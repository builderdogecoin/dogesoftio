// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! `dsverify inscription`: walk a claimed location back, hop by hop, to the reveal, using only Core.
//! Every hop maps the koinu offset from the output side to the input side (first in, first out),
//! picks the input that holds it, and repeats until it reaches the transaction that wrote the envelope.

use crate::api::{seg, Api};
use ds_core_rpc::Client;
use ds_rules::envelope::{self as env, Parsed};
use ds_rules::koinu::{input_for_offset, output_offset_to_input_space};
use ds_rules::owner::owner_of_script;
use ds_rules::tx::{decode_tx_hex, parse_txid, txid_hex, RawTx, Txid};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub struct Claim {
    pub txid: Txid,
    pub vout: u32,
    pub off: u64,
    pub exact: bool,
    pub owner: Option<String>,
}

pub fn claim_from_api(api: &Api, id: &str) -> Result<Claim, String> {
    let v = api.get(&format!("/api/explorer/inscriptions/{}", seg(id)))?;
    let i = &v["inscription"];
    let l = &i["location"];
    let txid = parse_txid(l["txid"].as_str().ok_or("API answer has no location")?).map_err(|e| e.0)?;
    let off = l["sat_offset"].as_u64();
    Ok(Claim {
        txid,
        vout: l["vout"].as_u64().ok_or("API location has no vout")? as u32,
        off: off.unwrap_or(0),
        exact: off.is_some(),
        owner: l["owner_address"].as_str().or(i["owner_address"].as_str()).map(str::to_string),
    })
}

pub fn parse_at(s: &str) -> Result<Claim, String> {
    let p: Vec<&str> = s.split(':').collect();
    if p.len() < 2 || p.len() > 3 {
        return Err("--at wants TXID:VOUT or TXID:VOUT:OFFSET".into());
    }
    Ok(Claim {
        txid: parse_txid(p[0]).map_err(|e| e.0)?,
        vout: p[1].parse().map_err(|_| "bad vout")?,
        off: p.get(2).map(|x| x.parse()).transpose().map_err(|_| "bad offset")?.unwrap_or(0),
        exact: p.len() == 3,
        owner: None,
    })
}

struct Txs<'a> {
    rpc: &'a Client,
    cache: HashMap<Txid, RawTx>,
}
impl Txs<'_> {
    fn get(&mut self, t: &Txid) -> Result<RawTx, String> {
        if let Some(x) = self.cache.get(t) {
            return Ok(x.clone());
        }
        let id = txid_hex(t);
        let raw = self.rpc.raw_tx(&id).map_err(|e| format!("Core getrawtransaction {id}: {e}"))?;
        let x = decode_tx_hex(&raw).map_err(|e| format!("{id}: {e}"))?;
        if x.txid != *t {
            return Err(format!("Core returned a transaction that does not hash to {id}"));
        }
        if self.cache.len() > 50_000 {
            self.cache.clear();
        }
        self.cache.insert(*t, x.clone());
        Ok(x)
    }
    fn prefetch(&mut self, ids: &[Txid]) -> Result<(), String> {
        let want: Vec<String> = ids.iter().filter(|t| !self.cache.contains_key(*t)).map(txid_hex).collect();
        for chunk in want.chunks(100) {
            for (id, r) in chunk.iter().zip(self.rpc.raw_txs(chunk).map_err(|e| format!("Core: {e}"))?) {
                let x = decode_tx_hex(&r.map_err(|e| format!("Core getrawtransaction {id}: {e}"))?).map_err(|e| format!("{id}: {e}"))?;
                if txid_hex(&x.txid) != *id {
                    return Err(format!("Core returned a transaction that does not hash to {id}"));
                }
                self.cache.insert(x.txid, x);
            }
        }
        Ok(())
    }
}

/// Content type and body.
type Found = (Vec<u8>, Vec<u8>);

/// Does `t`'s input 0 finish an envelope whose chain starts at `first`? Returns the inscription.
fn completes(txs: &mut Txs<'_>, t: &RawTx, first: &Txid) -> Result<Option<Found>, String> {
    if t.ins.is_empty() || !env::can_be_inscription_part(&t.ins[0].script) {
        return Ok(None);
    }
    let mut chain = vec![(t.txid, t.ins[0].script.clone())];
    let mut cur = t.clone();
    while chain.len() < 256 && cur.txid != *first {
        let p = txs.get(&cur.ins[0].prev)?;
        if p.ins.is_empty() || p.is_coinbase() || !env::can_be_inscription_part(&p.ins[0].script) {
            break;
        }
        chain.push((p.txid, p.ins[0].script.clone()));
        cur = p;
    }
    chain.reverse();
    for s in 0..chain.len() {
        if chain[s].0 != *first {
            continue;
        }
        let scripts: Vec<Vec<u8>> = chain[s..].iter().map(|c| c.1.clone()).collect();
        if let Parsed::Complete(i) = env::parse(&scripts) {
            // the chain must not complete earlier than this transaction
            if (1..scripts.len()).all(|k| !matches!(env::parse(&scripts[..k]), Parsed::Complete(_))) {
                return Ok(Some((i.content_type, i.body)));
            }
        }
    }
    Ok(None)
}

pub struct Outcome {
    pub ok: bool,
    pub undecided: bool,
}

pub fn run(rpc: &Client, id: &str, claim: &Claim, max_hops: usize) -> Result<Outcome, String> {
    let first = parse_txid(id.strip_suffix("i0").ok_or("Doginals inscription ids end in i0")?).map_err(|e| e.0)?;
    let mut txs = Txs { rpc, cache: HashMap::new() };
    let short = |t: &Txid| txid_hex(t);
    // 1. where it is now: unspent, and whose
    let utxo = rpc.tx_out(&txid_hex(&claim.txid), claim.vout).map_err(|e| format!("Core gettxout: {e}"))?;
    let Some(u) = utxo else {
        println!("  now at   {}:{}  SPENT  (the claimed location is not current)  DIFF", short(&claim.txid), claim.vout);
        return Ok(Outcome { ok: false, undecided: false });
    };
    let spk = hex::decode(u["scriptPubKey"]["hex"].as_str().unwrap_or("")).map_err(|_| "gettxout: bad scriptPubKey")?;
    let owner = owner_of_script(&spk).key();
    // 2. walk back
    let (mut t, mut vout, mut off) = (claim.txid, claim.vout, claim.off);
    let mut hops = 0usize;
    loop {
        let x = txs.get(&t)?;
        let outs: Vec<u64> = x.outs.iter().map(|o| o.value).collect();
        let Some(s) = output_offset_to_input_space(&outs, vout as usize, off) else {
            println!("  hop {hops:<4} {}:{vout} offset {off} is outside the output  DIFF", short(&t));
            return Ok(Outcome { ok: false, undecided: false });
        };
        if x.is_coinbase() {
            println!("  hop {hops:<4} {}:{vout}  reached a coinbase: this Doginal was spent as a fee on the way.", short(&t));
            println!("           A back-walk cannot follow fees; `dsverify replay` covers that case.");
            return Ok(Outcome { ok: false, undecided: true });
        }
        if s == 0 {
            if let Some((ct, body)) = completes(&mut txs, &x, &first)? {
                let digest = hex::encode(Sha256::digest(&body));
                println!("  reveal   {}  envelope in input 0  {} bytes  {}  OK", short(&t), body.len(), String::from_utf8_lossy(&ct));
                println!("           content sha256 {digest}");
                println!("  hops     {hops}");
                println!("  now at   {}:{}  offset {}  unspent  owner {owner}  OK", short(&claim.txid), claim.vout, claim.off);
                if !claim.exact {
                    println!("  note     no exact offset was given; offset 0 was checked");
                }
                println!("  note     a back-walk cannot see whether these koinu were already inscribed before the reveal; `dsverify replay` checks that too");
                let mut ok = true;
                if let Some(o) = &claim.owner {
                    let same = *o == owner;
                    ok &= same;
                    println!("  API says owner {o}  location {}:{}:{}  {}", short(&claim.txid), claim.vout, claim.off, if same { "PASS" } else { "DIFF" });
                }
                return Ok(Outcome { ok, undecided: false });
            }
        }
        hops += 1;
        if hops > max_hops {
            println!("  stopped after {max_hops} hops (raise --max-hops)");
            return Ok(Outcome { ok: false, undecided: true });
        }
        let prevs: Vec<Txid> = x.ins.iter().map(|i| i.prev).collect();
        txs.prefetch(&prevs)?;
        let mut vals = Vec::with_capacity(x.ins.len());
        for i in &x.ins {
            let p = txs.get(&i.prev)?;
            vals.push(p.outs.get(i.vout as usize).ok_or("prevout missing")?.value);
        }
        let Some((i, o)) = input_for_offset(&vals, s) else {
            println!("  hop {hops:<4} {}:{vout} offset {off} maps past the inputs  DIFF", short(&t));
            return Ok(Outcome { ok: false, undecided: false });
        };
        if x.txid == first {
            println!("  hop {hops:<4} reached {} but the koinu came from input {i} offset {o}, not the first koinu  DIFF", short(&t));
            return Ok(Outcome { ok: false, undecided: false });
        }
        if hops <= 3 || hops % 100 == 0 {
            println!("  hop {hops:<4} {}:{vout} offset {off}  <- input {i}", short(&t));
        }
        t = x.ins[i].prev;
        vout = x.ins[i].vout;
        off = o;
    }
}
