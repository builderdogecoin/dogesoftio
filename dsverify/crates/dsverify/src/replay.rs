// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Forward replay against your own Dogecoin Core. Every block is fetched raw,
//! decoded and hash-checked here, then run through the same rule code the Doge Soft Rust indexer uses.
//! State is saved after every batch, so you can stop and resume any time.

use crate::db::{Db, Res};
use ds_core_rpc::Client;
use ds_rules::koinu::Dest;
use ds_rules::ledger::{on_inscription, on_move, NewInscription, SendTo};
use ds_rules::names::reg_claim;
use ds_rules::owner::owner_of_script;
use ds_rules::planner::{plan_block, reference_order, PlanError, Prefetch};
use ds_rules::tx::{decode_block_hex, decode_tx_hex, txid_hex, Block, OutPoint, Txid};
use ds_rules::REPLAY_ANCHOR;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

pub struct Opts {
    pub to: Option<u64>,
    pub confirmations: u64,
    pub batch: u64,
    pub quiet: bool,
}

/// Small bounded cache of prevout values fetched from Core.
#[derive(Default)]
pub struct ValueCache {
    map: HashMap<OutPoint, u64>,
}

impl ValueCache {
    fn fetch(&mut self, rpc: &Client, want: &[OutPoint]) -> Res<()> {
        let mut txids: Vec<Txid> = want.iter().filter(|op| !self.map.contains_key(op)).map(|op| op.0).collect();
        txids.sort();
        txids.dedup();
        if self.map.len() > 2_000_000 {
            self.map.clear();
        }
        for chunk in txids.chunks(200) {
            let hexes: Vec<String> = chunk.iter().map(txid_hex).collect();
            let got = rpc.raw_txs(&hexes).map_err(|e| format!("Core: {e}"))?;
            for (id, r) in hexes.iter().zip(got) {
                let raw = r.map_err(|e| format!("Core getrawtransaction {id}: {e} (is txindex=1 on?)"))?;
                let t = decode_tx_hex(&raw).map_err(|e| format!("transaction {id}: {e}"))?;
                if txid_hex(&t.txid) != *id {
                    return Err(format!("Core returned a transaction that hashes to {} for {id}", txid_hex(&t.txid)));
                }
                for (v, o) in t.outs.iter().enumerate() {
                    self.map.insert((t.txid, v as u32), o.value);
                }
            }
        }
        Ok(())
    }
}

pub fn apply_block(db: &mut Db, rpc: &Client, cache: &mut ValueCache, h: u64, b: &Block) -> Res<()> {
    let mut pre = Prefetch::default();
    let mut seen: HashSet<OutPoint> = HashSet::new();
    for t in b.txs.iter().skip(1) {
        for (i, x) in t.ins.iter().enumerate() {
            let op = (x.prev, x.vout);
            if seen.insert(op) {
                let rows = db.loc_rows(&op)?;
                if !rows.is_empty() {
                    pre.loc.insert(op, rows);
                }
            }
            if i == 0 && !pre.partial.contains_key(&x.prev) {
                if let Some(p) = db.partial(&x.prev)? {
                    pre.partial.insert(x.prev, p);
                }
            }
        }
    }
    let next_iid: u64 = db.meta("next_iid").and_then(|v| v.parse().ok()).unwrap_or(0);
    let plan = loop {
        match plan_block(&b.txs, &pre, next_iid) {
            Ok(p) => break p,
            Err(PlanError::NeedValues(v)) => {
                cache.fetch(rpc, &v)?;
                for op in v {
                    let val = *cache.map.get(&op).ok_or_else(|| format!("Core has no output {}:{}", txid_hex(&op.0), op.1))?;
                    pre.values.insert(op, val);
                }
            }
            Err(e) => return Err(format!("block {h}: {e}")),
        }
    };
    for (op, _) in &plan.loc_deleted {
        db.del_loc(op)?;
    }
    for (op, r) in &plan.loc_inserted {
        db.put_loc(op, r)?;
    }
    for (t, _) in &plan.partial_deleted {
        db.del_partial(t)?;
    }
    for (t, p) in &plan.partial_inserted {
        db.put_partial(t, p)?;
    }
    db.set_meta("next_iid", &plan.next_iid.to_string())?;

    // DRC-20 and names, in reference order; inside a transaction the new inscription comes first.
    for k in reference_order(b.txs.len()) {
        let tx = &b.txs[k];
        let v = &plan.txs[k];
        let txid = txid_hex(&tx.txid);
        if let Some(c) = &v.created {
            if c.kind > 0 {
                let landing = match c.dest {
                    Dest::Output { vout, .. } => Some((vout, owner_of_script(&tx.outs[vout as usize].spk).key())),
                    _ => None,
                };
                on_inscription(
                    db,
                    &NewInscription { height: h, txidx: k as u32, reveal_txid: &txid, iid: c.iid, inscription_id: &c.inscription_id, landing, body: &c.body },
                );
                if let Some(n) = reg_claim(&c.body) {
                    db.name_claim(&n.name, c.iid, &c.inscription_id, h, k as u32)?;
                }
            }
        }
        for (iid, d) in &v.moves {
            if !db.has_open_lock(*iid) {
                continue;
            }
            let to = match d {
                Dest::Output { vout, .. } => SendTo::Owner(owner_of_script(&tx.outs[*vout as usize].spk).key()),
                Dest::Fee { .. } | Dest::Lost { .. } => SendTo::Fee,
            };
            on_move(db, h, k as u32, &txid, *iid, &to);
        }
    }
    Ok(())
}

/// Height of the first block a fresh replay reads, asked from Core by hash.
pub fn first_height(rpc: &Client) -> Res<u64> {
    let hd = rpc.block_header(REPLAY_ANCHOR).map_err(|e| format!("Core getblockheader: {e} (is this a mainnet node?)"))?;
    hd.get("height").and_then(|v| v.as_u64()).ok_or_else(|| "Core getblockheader: no height".to_string())
}

pub fn run(db: &mut Db, rpc: &Client, o: &Opts) -> Res<u64> {
    let tip = rpc.block_count().map_err(|e| format!("Core: {e}"))?;
    let safe = tip.saturating_sub(o.confirmations);
    let target = o.to.map(|t| t.min(safe)).unwrap_or(safe);
    let mut h = match db.height() {
        Some(x) => x + 1,
        None => first_height(rpc)?,
    };
    if o.to.is_some_and(|t| t > safe) && !o.quiet {
        eprintln!("note: stopping at {target}, {} blocks behind the tip ({tip}) for reorg safety", o.confirmations);
    }
    if h > target {
        if !o.quiet {
            eprintln!("already at {}; nothing to do (target {target})", h - 1);
        }
        return Ok(h - 1);
    }
    let mut cache = ValueCache::default();
    let t0 = Instant::now();
    let start = h;
    let mut last = Instant::now();
    while h <= target {
        let end = (h + o.batch - 1).min(target);
        let heights: Vec<serde_json::Value> = (h..=end).map(|x| serde_json::json!([x])).collect();
        let hashes: Vec<String> = rpc
            .batch("getblockhash", &heights)
            .map_err(|e| format!("Core: {e}"))?
            .into_iter()
            .map(|r| r.map_err(|e| format!("Core getblockhash: {e}")).and_then(|v| v.as_str().map(str::to_string).ok_or("bad getblockhash".to_string())))
            .collect::<Res<_>>()?;
        let params: Vec<serde_json::Value> = hashes.iter().map(|x| serde_json::json!([x, false])).collect();
        let raws = rpc.batch("getblock", &params).map_err(|e| format!("Core: {e}"))?;
        db.begin()?;
        for (i, r) in raws.into_iter().enumerate() {
            let hh = h + i as u64;
            let raw = r.map_err(|e| format!("Core getblock {hh}: {e}"))?;
            let raw = raw.as_str().ok_or("bad getblock answer")?;
            let b = decode_block_hex(raw).map_err(|e| format!("block {hh}: {e}"))?;
            if txid_hex(&b.hash) != hashes[i] {
                return Err(format!("block {hh}: header hashes to {} but Core said {}", txid_hex(&b.hash), hashes[i]));
            }
            if let Some(prev) = db.block_hash(hh - 1) {
                if prev != txid_hex(&b.prev) {
                    return Err(format!(
                        "block {hh} does not build on the saved block {} (a reorg deeper than --confirmations); start a fresh replay with a new --db",
                        hh - 1
                    ));
                }
            }
            apply_block(db, rpc, &mut cache, hh, &b)?;
            db.put_block(hh, &hashes[i])?;
        }
        db.set_meta("height", &end.to_string())?;
        db.set_meta("tip_hash", &hashes[hashes.len() - 1])?;
        db.commit()?;
        h = end + 1;
        if !o.quiet && last.elapsed().as_secs() >= 10 {
            let el = t0.elapsed().as_secs_f64().max(0.001);
            let bps = (h - start) as f64 / el;
            eprintln!("replay  height {end}  {:.1} blocks/s  {} to go", bps, target.saturating_sub(end));
            last = Instant::now();
        }
    }
    Ok(target)
}
