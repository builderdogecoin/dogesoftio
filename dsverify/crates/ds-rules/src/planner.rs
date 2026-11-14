// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! The block planner: everything one block does to inscriptions, as a pure function.
//!
//! In: the block's transactions plus the state rows it touches (inscription locations, partial envelope
//! chains, exact prevout values). Out: every effect (created inscriptions, moves, rows to delete and
//! insert). No node, no database. If a prevout value is needed and not supplied, the planner says which
//! ones ([`PlanError::NeedValues`]) and the caller fetches them from Core and plans again.

use crate::drc20::inscription_kind;
use crate::envelope::{can_be_inscription_part, parse_with_parent, Parsed};
use crate::koinu::{self as k, Dest, Held, TxInput};
use crate::tx::{txid_hex, OutPoint, RawTx, Txid};
use std::collections::{HashMap, HashSet};

/// One inscription at offset `off` of an outpoint worth `val` koinu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocRow {
    pub off: u64,
    pub iid: u64,
    pub val: u64,
}

/// A partial envelope chain: its txids and input-0 scriptSigs, oldest first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartialRow {
    pub txids: Vec<Txid>,
    pub scripts: Vec<Vec<u8>>,
}

/// State rows read before planning (only the keys the block touches).
#[derive(Clone, Debug, Default)]
pub struct Prefetch {
    pub loc: HashMap<OutPoint, Vec<LocRow>>,
    pub partial: HashMap<Txid, PartialRow>,
    /// Exact values of prevouts created before this block.
    pub values: HashMap<OutPoint, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    pub iid: u64,
    /// First chain txid + `i0`.
    pub inscription_id: String,
    pub content_type: Vec<u8>,
    pub body: Vec<u8>,
    pub kind: u8,
    pub dest: Dest,
}

/// What happened to inscriptions in one transaction.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TxVerdict {
    pub created: Option<Created>,
    /// The first koinu of the inputs was already inscribed: nothing could be created.
    pub gate_closed: bool,
    /// Existing inscriptions moved by this transaction (coinbase: the block's fee pool).
    pub moves: Vec<(u64, Dest)>,
}

#[derive(Clone, Debug, Default)]
pub struct BlockPlan {
    /// Per transaction, block order.
    pub txs: Vec<TxVerdict>,
    pub loc_deleted: Vec<(OutPoint, Vec<LocRow>)>,
    pub loc_inserted: Vec<(OutPoint, LocRow)>,
    pub partial_deleted: Vec<(Txid, PartialRow)>,
    pub partial_inserted: Vec<(Txid, PartialRow)>,
    pub next_iid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// These prevout values are needed (fetch them and plan again).
    NeedValues(Vec<OutPoint>),
    Sat(k::SatError),
    Invalid(String),
}

impl std::fmt::Display for PlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlanError::NeedValues(v) => write!(f, "need {} prevout values", v.len()),
            PlanError::Sat(e) => write!(f, "koinu arithmetic: {e}"),
            PlanError::Invalid(m) => f.write_str(m),
        }
    }
}
impl std::error::Error for PlanError {}

impl From<k::SatError> for PlanError {
    fn from(e: k::SatError) -> Self {
        PlanError::Sat(e)
    }
}

/// Processing order inside a block: transactions 1..n in block order, then the coinbase.
pub fn reference_order(n: usize) -> Vec<usize> {
    (1..n).chain(std::iter::once(0)).collect()
}

/// Plan one block. `txs` in block order (coinbase first). `next_iid` numbers new inscriptions.
pub fn plan_block(txs: &[RawTx], pre: &Prefetch, next_iid: u64) -> Result<BlockPlan, PlanError> {
    let n = txs.len();
    if n == 0 {
        return Err(PlanError::Invalid("empty block".into()));
    }
    let by_txid: HashMap<Txid, usize> = txs.iter().enumerate().map(|(k, t)| (t.txid, k)).collect();
    let mut missing: Vec<OutPoint> = Vec::new();
    let mut missing_set: HashSet<OutPoint> = HashSet::new();
    let value_of = |op: &OutPoint, missing: &mut Vec<OutPoint>, missing_set: &mut HashSet<OutPoint>| -> Result<u64, PlanError> {
        if let Some(&k) = by_txid.get(&op.0) {
            return txs[k].outs.get(op.1 as usize).map(|o| o.value).ok_or_else(|| PlanError::Invalid(format!("no output {}:{}", txid_hex(&op.0), op.1)));
        }
        match pre.values.get(op) {
            Some(v) => Ok(*v),
            None => {
                if missing_set.insert(*op) {
                    missing.push(*op);
                }
                Ok(0)
            }
        }
    };

    let mut created_rows: HashMap<OutPoint, Vec<LocRow>> = HashMap::new();
    let mut created_order: Vec<OutPoint> = Vec::new();
    let mut consumed_db: HashSet<OutPoint> = HashSet::new();
    let mut loc_deleted: Vec<(OutPoint, Vec<LocRow>)> = Vec::new();
    let mut part_new: HashMap<Txid, PartialRow> = HashMap::new();
    let mut part_new_order: Vec<Txid> = Vec::new();
    let mut part_deleted_db: HashSet<Txid> = HashSet::new();
    let mut partial_deleted: Vec<(Txid, PartialRow)> = Vec::new();

    let mut verdicts: Vec<TxVerdict> = vec![TxVerdict::default(); n];
    let mut next = next_iid;
    let order = reference_order(n);
    let mut fees_known_to = 0usize;
    let mut reward: u64 = k::SUBSIDY_KOINU;
    let mut flotsam: Vec<k::Flotsam> = Vec::new();

    for (pos, &ti) in order.iter().enumerate() {
        let tx = &txs[ti];
        let coinbase = ti == 0;
        // 1. inscriptions on the inputs
        let mut ins_rows: Vec<(usize, Vec<LocRow>)> = Vec::new();
        if !coinbase {
            for (i, x) in tx.ins.iter().enumerate() {
                let op = (x.prev, x.vout);
                let rows = if let Some(r) = created_rows.remove(&op) {
                    Some(r)
                } else if !consumed_db.contains(&op) {
                    pre.loc.get(&op).cloned().inspect(|r| {
                        consumed_db.insert(op);
                        loc_deleted.push((op, r.clone()));
                    })
                } else {
                    None
                };
                if let Some(mut r) = rows {
                    if !r.is_empty() {
                        r.sort_by_key(|a| (a.off, a.iid));
                        ins_rows.push((i, r));
                    }
                }
            }
        }
        let mut gathered: Vec<(u64, u64)> = Vec::new();
        let mut gate_open = true;
        if let Some(&(last, _)) = ins_rows.last() {
            let rows_at: HashMap<usize, &Vec<LocRow>> = ins_rows.iter().map(|(i, r)| (*i, r)).collect();
            let mut inputs: Vec<TxInput> = Vec::with_capacity(last + 1);
            for i in 0..=last {
                match rows_at.get(&i) {
                    Some(r) => inputs.push(TxInput { value: r[0].val, held: r.iter().map(|x| Held { off: x.off, iid: x.iid }).collect() }),
                    None => {
                        let x = &tx.ins[i];
                        let v = value_of(&(x.prev, x.vout), &mut missing, &mut missing_set)?;
                        inputs.push(TxInput { value: v, held: Vec::new() });
                    }
                }
            }
            let (g, open) = k::gather(&inputs)?;
            gathered = g;
            gate_open = open;
        }
        // 2. a new inscription (the gate is checked before the fee pool joins the coinbase)
        let mut new_iid: Option<u64> = None;
        let mut created: Option<Created> = None;
        if gate_open && !tx.ins.is_empty() {
            let in0 = &tx.ins[0];
            if can_be_inscription_part(&in0.script) {
                let parent_key = in0.prev;
                let parent: Option<PartialRow> = if let Some(p) = part_new.get(&parent_key) {
                    Some(p.clone())
                } else if !part_deleted_db.contains(&parent_key) {
                    pre.partial.get(&parent_key).cloned()
                } else {
                    None
                };
                let (r, chain_scripts) = parse_with_parent(parent.as_ref().map(|p| p.scripts.clone()), in0.script.clone());
                let mut chain_txids: Vec<Txid> = parent.as_ref().map(|p| p.txids.clone()).unwrap_or_default();
                chain_txids.push(tx.txid);
                let mut drop_parent = |part_new: &mut HashMap<Txid, PartialRow>| {
                    if let Some(pp) = parent.as_ref() {
                        if part_new.remove(&parent_key).is_none() {
                            part_deleted_db.insert(parent_key);
                            partial_deleted.push((parent_key, pp.clone()));
                        }
                    }
                };
                match r {
                    Parsed::Partial => {
                        drop_parent(&mut part_new);
                        if part_new.insert(tx.txid, PartialRow { txids: chain_txids, scripts: chain_scripts }).is_none() {
                            part_new_order.push(tx.txid);
                        }
                    }
                    Parsed::Complete(insc) => {
                        drop_parent(&mut part_new);
                        let iid = next;
                        next += 1;
                        let kind = inscription_kind(&insc.content_type, &insc.body);
                        new_iid = Some(iid);
                        created = Some(Created {
                            iid,
                            inscription_id: format!("{}i0", txid_hex(&chain_txids[0])),
                            content_type: insc.content_type,
                            body: insc.body,
                            kind,
                            dest: Dest::Lost { off: 0 },
                        });
                    }
                    Parsed::None => {}
                }
            }
        }
        verdicts[ti].gate_closed = !gate_open;
        let fl: Vec<k::Flotsam> = if coinbase { flotsam.clone() } else { Vec::new() };
        if gathered.is_empty() && new_iid.is_none() && fl.is_empty() {
            continue;
        }
        // 3. route first in, first out over the outputs
        let outs: Vec<u64> = tx.outs.iter().map(|o| o.value).collect();
        let placed = k::route(&gathered, new_iid, &fl, coinbase, &outs)?;
        let mut fee_here = false;
        for p in &placed {
            if let Dest::Output { vout, off } = p.dest {
                let op = (tx.txid, vout);
                let e = created_rows.entry(op).or_insert_with(|| {
                    created_order.push(op);
                    Vec::new()
                });
                e.push(LocRow { off, iid: p.iid, val: outs[vout as usize] });
            }
            if matches!(p.dest, Dest::Fee { .. }) {
                fee_here = true;
            }
            if p.new {
                if let Some(c) = created.as_mut() {
                    c.dest = p.dest;
                }
            } else {
                verdicts[ti].moves.push((p.iid, p.dest));
            }
        }
        verdicts[ti].created = created;
        // fee pool bookkeeping: the reward before this transaction needs the fees of every earlier one
        if !coinbase && fee_here {
            while fees_known_to < pos {
                let t = &txs[order[fees_known_to]];
                let mut s: u64 = 0;
                for x in &t.ins {
                    let v = value_of(&(x.prev, x.vout), &mut missing, &mut missing_set)?;
                    s = s.checked_add(v).ok_or(k::SatError::Overflow)?;
                }
                let o = t.output_total().ok_or(k::SatError::Overflow)?;
                if missing.is_empty() {
                    let fee = s.checked_sub(o).ok_or_else(|| PlanError::Invalid(format!("negative fee in {}", txid_hex(&t.txid))))?;
                    reward = reward.checked_add(fee).ok_or(k::SatError::Overflow)?;
                }
                fees_known_to += 1;
            }
            for p in &placed {
                if matches!(p.dest, Dest::Fee { .. }) {
                    // ord-dogecoin: reward + input-space offset, not minus the output total (see koinu.rs)
                    let coff = reward.checked_add(p.src_off).ok_or(k::SatError::Overflow)?;
                    flotsam.push(k::Flotsam { iid: p.iid, foff: p.src_off, tx_index: ti, coff });
                }
            }
        }
    }
    if !missing.is_empty() {
        return Err(PlanError::NeedValues(missing));
    }
    let mut loc_inserted = Vec::new();
    for op in created_order {
        if let Some(rows) = created_rows.get(&op) {
            for r in rows {
                loc_inserted.push((op, r.clone()));
            }
        }
    }
    let mut partial_inserted = Vec::new();
    for t in part_new_order {
        if let Some(p) = part_new.get(&t) {
            partial_inserted.push((t, p.clone()));
        }
    }
    Ok(BlockPlan { txs: verdicts, loc_deleted, loc_inserted, partial_deleted, partial_inserted, next_iid: next })
}
