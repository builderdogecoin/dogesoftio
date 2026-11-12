// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! The DRC-20 ledger: the 15 Doge Soft rules applied in chain order.
//!
//! Feed it every kind 1/2 inscription the block planner creates and every move of an open transfer
//! inscription, in reference order (non-coinbase transactions in block order, then the coinbase; inside
//! a transaction the new inscription first, then the moves). Storage is behind [`LedgerStore`] so the
//! same rules run in memory (tests) or on disk (dsverify).

use crate::drc20::{amount, fmt_amount, parse_op, tick_key, Op, RESERVED_TICK};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tick: String,
    pub tick_raw: String,
    pub iid: u64,
    pub inscription_id: String,
    pub height: u64,
    pub txidx: u32,
    pub txid: String,
    pub deployer: String,
    pub max: u128,
    pub lim: u128,
    pub dec: u32,
    pub minted: u128,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Balance {
    /// Available.
    pub avail: u128,
    /// Locked in open transfer inscriptions.
    pub trans: u128,
}

/// An open transfer inscription (inscribed, not yet sent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    pub iid: u64,
    pub txid: String,
    pub vout: u32,
    pub tick: String,
    pub owner: String,
    pub amt: u128,
    pub height: u64,
}

/// One ledger event (every decision is recorded, accepted or not).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub height: u64,
    pub txidx: u32,
    pub txid: String,
    pub iid: u64,
    pub op: &'static str,
    pub tick: String,
    pub addr: Option<String>,
    pub amt: Option<String>,
    pub credit: Option<String>,
    pub status: String,
}

pub trait LedgerStore {
    fn token(&mut self, tick: &str) -> Option<Token>;
    fn put_token(&mut self, t: &Token);
    fn balance(&mut self, tick: &str, owner: &str) -> Balance;
    fn put_balance(&mut self, tick: &str, owner: &str, b: Balance);
    fn add_lock(&mut self, l: Lock);
    /// Remove and return the open lock on inscription `iid`, if any.
    fn take_lock(&mut self, iid: u64) -> Option<Lock>;
    fn event(&mut self, e: Event);
}

/// A new inscription with a readable body.
#[derive(Debug, Clone)]
pub struct NewInscription<'a> {
    pub height: u64,
    pub txidx: u32,
    /// The transaction that completed the envelope.
    pub reveal_txid: &'a str,
    pub iid: u64,
    pub inscription_id: &'a str,
    /// Output it landed in and that output's owner key; `None` when it was spent as a fee.
    pub landing: Option<(u32, String)>,
    pub body: &'a [u8],
}

/// Where the first move of an open transfer inscription took it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendTo {
    /// Spent as a fee (or lost in the coinbase): rule 11, back to the sender.
    Fee,
    /// An output with this owner key (`nulldata` burns).
    Owner(String),
}

#[allow(clippy::too_many_arguments)]
fn ev(
    s: &mut dyn LedgerStore,
    n: &NewInscription<'_>,
    op: &'static str,
    tick: &str,
    addr: Option<&str>,
    amt: Option<String>,
    credit: Option<String>,
    status: &str,
) {
    s.event(Event {
        height: n.height,
        txidx: n.txidx,
        txid: n.reveal_txid.to_string(),
        iid: n.iid,
        op,
        tick: tick.to_string(),
        addr: addr.map(str::to_string),
        amt,
        credit,
        status: status.to_string(),
    });
}

/// Apply a new inscription. Returns `true` when it became an open transfer lock (the caller watches its
/// next move).
pub fn on_inscription(s: &mut dyn LedgerStore, n: &NewInscription<'_>) -> bool {
    let Some(o) = parse_op(n.body) else { return false };
    let key = tick_key(o.tick());
    if key == RESERVED_TICK {
        return false;
    }
    let fee = n.landing.is_none();
    let addr = n.landing.as_ref().map(|(_, a)| a.as_str()).unwrap_or("");
    match o {
        Op::Deploy { tick, max, lim, dec } => {
            if fee {
                ev(s, n, "deploy_skipped", &key, None, None, None, "inscribed_into_fee");
                return false;
            }
            if s.token(&key).is_some() {
                ev(s, n, "deploy_skipped", &key, Some(addr), None, None, "ticker_exists");
                return false;
            }
            ev(s, n, "deploy", &key, Some(addr), Some(fmt_amount(max)), Some(format!("{}/{}", fmt_amount(lim), dec)), "ok");
            s.put_token(&Token {
                tick: key,
                tick_raw: tick,
                iid: n.iid,
                inscription_id: n.inscription_id.to_string(),
                height: n.height,
                txidx: n.txidx,
                txid: n.reveal_txid.to_string(),
                deployer: addr.to_string(),
                max,
                lim,
                dec,
                minted: 0,
            });
            false
        }
        Op::Mint { amt, .. } => {
            let t = s.token(&key);
            let a = amount(&amt);
            let why = if fee {
                Some("inscribed_into_fee")
            } else if a.is_none() {
                Some("invalid_amount")
            } else if t.as_ref().is_some_and(|t| a.unwrap().frac_digits > t.dec as usize) {
                Some("amt_exceeds_dec")
            } else if t.is_none() {
                Some("not_deployed")
            } else if a.unwrap().value > t.as_ref().unwrap().lim {
                Some("over_lim")
            } else if t.as_ref().unwrap().minted >= t.as_ref().unwrap().max {
                Some("max_reached")
            } else {
                None
            };
            match (why, t) {
                (None, Some(mut t)) => {
                    let v = a.unwrap().value;
                    let credit = if t.minted + v > t.max { t.max - t.minted } else { v };
                    t.minted += credit;
                    s.put_token(&t);
                    let mut b = s.balance(&key, addr);
                    b.avail += credit;
                    s.put_balance(&key, addr, b);
                    let st = if credit == v { "ok" } else { "partial_remainder" };
                    ev(s, n, "mint", &key, Some(addr), Some(amt), Some(fmt_amount(credit)), st);
                }
                (w, _) => {
                    ev(s, n, "mint_skipped", &key, (!fee).then_some(addr), Some(amt), Some("0".into()), w.unwrap_or("rejected"));
                }
            }
            false
        }
        Op::Transfer { amt, .. } => {
            let t = s.token(&key);
            let a = amount(&amt);
            let mut why = if fee {
                Some("inscribed_into_fee_ignored")
            } else if a.is_none() {
                Some("invalid_amount")
            } else if t.as_ref().is_some_and(|t| a.unwrap().frac_digits > t.dec as usize) {
                Some("amt_exceeds_dec")
            } else {
                None
            };
            if why.is_none() {
                let v = a.unwrap().value;
                let mut b = s.balance(&key, addr);
                if b.avail >= v {
                    b.avail -= v;
                    b.trans += v;
                    s.put_balance(&key, addr, b);
                } else {
                    why = Some("insufficient_balance");
                }
            }
            if let Some(w) = why {
                ev(s, n, "transfer_inscription_invalid", &key, (!fee).then_some(addr), Some(amt), Some("0".into()), w);
                return false;
            }
            let v = a.unwrap().value;
            ev(s, n, "transfer_inscription", &key, Some(addr), Some(amt), Some(fmt_amount(v)), "inscribed_to_self");
            let (vout, _) = n.landing.clone().unwrap();
            s.add_lock(Lock { iid: n.iid, txid: n.reveal_txid.to_string(), vout, tick: key, owner: addr.to_string(), amt: v, height: n.height });
            true
        }
    }
}

/// Apply the first move of inscription `iid` if it is an open transfer lock. Returns `true` if it was.
pub fn on_move(s: &mut dyn LedgerStore, height: u64, txidx: u32, spend_txid: &str, iid: u64, to: &SendTo) -> bool {
    let Some(l) = s.take_lock(iid) else { return false };
    let mut b0 = s.balance(&l.tick, &l.owner);
    b0.trans -= l.amt;
    s.put_balance(&l.tick, &l.owner, b0);
    let (dest, how): (Option<String>, &str) = match to {
        SendTo::Fee => (Some(l.owner.clone()), "spent_as_fee_back_to_owner"),
        SendTo::Owner(o) if o == crate::owner::NULLDATA => (None, "burned_nulldata"),
        SendTo::Owner(o) => (Some(o.clone()), if *o == l.owner { "self_send_unlock" } else { "locked_carrier_moved" }),
    };
    if let Some(d) = &dest {
        let mut b = s.balance(&l.tick, d);
        b.avail += l.amt;
        s.put_balance(&l.tick, d, b);
    }
    s.event(Event {
        height,
        txidx,
        txid: spend_txid.to_string(),
        iid,
        op: "transfer",
        tick: l.tick.clone(),
        addr: dest,
        amt: Some(fmt_amount(l.amt)),
        credit: Some(fmt_amount(l.amt)),
        status: format!("{how} from {} insc {}", l.owner, l.txid),
    });
    true
}

/// An in-memory store (tests and small replays).
#[derive(Debug, Default)]
pub struct MemStore {
    pub tokens: HashMap<String, Token>,
    pub balances: HashMap<(String, String), Balance>,
    pub locks: HashMap<u64, Lock>,
    pub events: Vec<Event>,
}

impl LedgerStore for MemStore {
    fn token(&mut self, tick: &str) -> Option<Token> {
        self.tokens.get(tick).cloned()
    }
    fn put_token(&mut self, t: &Token) {
        self.tokens.insert(t.tick.clone(), t.clone());
    }
    fn balance(&mut self, tick: &str, owner: &str) -> Balance {
        self.balances.get(&(tick.to_string(), owner.to_string())).copied().unwrap_or_default()
    }
    fn put_balance(&mut self, tick: &str, owner: &str, b: Balance) {
        self.balances.insert((tick.to_string(), owner.to_string()), b);
    }
    fn add_lock(&mut self, l: Lock) {
        self.locks.insert(l.iid, l);
    }
    fn take_lock(&mut self, iid: u64) -> Option<Lock> {
        self.locks.remove(&iid)
    }
    fn event(&mut self, e: Event) {
        self.events.push(e);
    }
}

impl MemStore {
    /// Rule 14: holders are owners whose available or locked balance is above zero.
    pub fn holder_count(&self, tick: &str) -> usize {
        self.balances.iter().filter(|((t, _), b)| t == tick && (b.avail > 0 || b.trans > 0)).count()
    }
}
