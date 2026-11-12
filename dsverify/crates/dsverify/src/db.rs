// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! The local replay state: koinu positions of every inscription, open multi-transaction envelopes, and the
//! DRC-20 Coin and .doge name state built from them. One SQLite file on your own disk.

use ds_rules::drc20::{fmt_amount, parse_scaled};
use ds_rules::ledger::{Balance, Event, LedgerStore, Lock, Token};
use ds_rules::planner::{LocRow, PartialRow};
use ds_rules::tx::{OutPoint, Txid};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::Path;

pub type Res<T> = Result<T, String>;
/// A name's winning claim: inscription number, inscription id, height, position, claims seen.
pub type NameRow = (u64, String, u64, u32, u64);
fn e(x: rusqlite::Error) -> String {
    format!("local database: {x}")
}

pub struct Db {
    pub conn: Connection,
    open_locks: HashSet<u64>,
    failed: Option<String>,
}

const SCHEMA: &str = "
PRAGMA journal_mode=WAL;
PRAGMA synchronous=NORMAL;
CREATE TABLE IF NOT EXISTS meta(k TEXT PRIMARY KEY, v TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS blocks(height INTEGER PRIMARY KEY, hash TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS loc(txid BLOB NOT NULL, vout INTEGER NOT NULL, off INTEGER NOT NULL, iid INTEGER NOT NULL, val INTEGER NOT NULL, PRIMARY KEY(txid, vout, off, iid)) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS loc_iid ON loc(iid);
CREATE TABLE IF NOT EXISTS partial(txid BLOB PRIMARY KEY, chain TEXT NOT NULL) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS tok(tick TEXT PRIMARY KEY, tick_raw TEXT, iid INTEGER, insc_id TEXT, height INTEGER, txidx INTEGER, txid TEXT, deployer TEXT, max TEXT, lim TEXT, dec INTEGER, minted TEXT) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS bal(tick TEXT NOT NULL, addr TEXT NOT NULL, avail TEXT NOT NULL, trans TEXT NOT NULL, PRIMARY KEY(tick, addr)) WITHOUT ROWID;
CREATE TABLE IF NOT EXISTS lck(iid INTEGER PRIMARY KEY, txid TEXT, vout INTEGER, tick TEXT, owner TEXT, amt TEXT, height INTEGER);
CREATE TABLE IF NOT EXISTS ev(seq INTEGER PRIMARY KEY AUTOINCREMENT, height INTEGER, txidx INTEGER, txid TEXT, iid INTEGER, op TEXT, tick TEXT, addr TEXT, amt TEXT, credit TEXT, status TEXT);
CREATE TABLE IF NOT EXISTS names(name TEXT PRIMARY KEY, iid INTEGER, insc_id TEXT, height INTEGER, txidx INTEGER, claims INTEGER NOT NULL DEFAULT 1) WITHOUT ROWID;
";

impl Db {
    pub fn open(path: &Path) -> Res<Db> {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).map_err(|x| format!("{}: {x}", p.display()))?;
        }
        let conn = Connection::open(path).map_err(e)?;
        conn.execute_batch(SCHEMA).map_err(e)?;
        let mut open_locks = HashSet::new();
        {
            let mut st = conn.prepare("SELECT iid FROM lck").map_err(e)?;
            let rows = st.query_map([], |r| r.get::<_, i64>(0)).map_err(e)?;
            for r in rows {
                open_locks.insert(r.map_err(e)? as u64);
            }
        }
        Ok(Db { conn, open_locks, failed: None })
    }

    pub fn open_existing(path: &Path) -> Res<Db> {
        if !path.exists() {
            return Err(format!("no local replay at {} yet: run `dsverify replay` first", path.display()));
        }
        Db::open(path)
    }

    pub fn meta(&self, k: &str) -> Option<String> {
        self.conn.query_row("SELECT v FROM meta WHERE k=?1", [k], |r| r.get(0)).optional().ok().flatten()
    }
    pub fn set_meta(&self, k: &str, v: &str) -> Res<()> {
        self.conn.execute("INSERT INTO meta(k,v) VALUES(?1,?2) ON CONFLICT(k) DO UPDATE SET v=excluded.v", params![k, v]).map(|_| ()).map_err(e)
    }
    /// Last fully applied height, if any.
    pub fn height(&self) -> Option<u64> {
        self.meta("height").and_then(|v| v.parse().ok())
    }
    pub fn block_hash(&self, h: u64) -> Option<String> {
        self.conn.query_row("SELECT hash FROM blocks WHERE height=?1", [h as i64], |r| r.get(0)).optional().ok().flatten()
    }
    pub fn put_block(&self, h: u64, hash: &str) -> Res<()> {
        self.conn.execute("INSERT OR REPLACE INTO blocks(height,hash) VALUES(?1,?2)", params![h as i64, hash]).map(|_| ()).map_err(e)
    }
    pub fn begin(&self) -> Res<()> {
        self.conn.execute_batch("BEGIN").map_err(e)
    }
    pub fn commit(&mut self) -> Res<()> {
        if let Some(f) = self.failed.take() {
            let _ = self.conn.execute_batch("ROLLBACK");
            return Err(f);
        }
        self.conn.execute_batch("COMMIT").map_err(e)
    }

    pub fn loc_rows(&self, op: &OutPoint) -> Res<Vec<LocRow>> {
        let mut st = self.conn.prepare_cached("SELECT off, iid, val FROM loc WHERE txid=?1 AND vout=?2").map_err(e)?;
        let rows = st
            .query_map(params![&op.0[..], op.1 as i64], |r| {
                Ok(LocRow { off: r.get::<_, i64>(0)? as u64, iid: r.get::<_, i64>(1)? as u64, val: r.get::<_, i64>(2)? as u64 })
            })
            .map_err(e)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(e)
    }
    pub fn del_loc(&self, op: &OutPoint) -> Res<()> {
        self.conn.prepare_cached("DELETE FROM loc WHERE txid=?1 AND vout=?2").map_err(e)?.execute(params![&op.0[..], op.1 as i64]).map(|_| ()).map_err(e)
    }
    pub fn put_loc(&self, op: &OutPoint, r: &LocRow) -> Res<()> {
        self.conn
            .prepare_cached("INSERT OR REPLACE INTO loc(txid,vout,off,iid,val) VALUES(?1,?2,?3,?4,?5)")
            .map_err(e)?
            .execute(params![&op.0[..], op.1 as i64, r.off as i64, r.iid as i64, r.val as i64])
            .map(|_| ())
            .map_err(e)
    }
    /// Where inscription `iid` is now: outpoint and koinu offset.
    pub fn where_is(&self, iid: u64) -> Res<Option<(Txid, u32, u64)>> {
        self.conn
            .query_row("SELECT txid, vout, off FROM loc WHERE iid=?1", [iid as i64], |r| {
                let t: Vec<u8> = r.get(0)?;
                let mut id = [0u8; 32];
                id.copy_from_slice(&t);
                Ok((id, r.get::<_, i64>(1)? as u32, r.get::<_, i64>(2)? as u64))
            })
            .optional()
            .map_err(e)
    }

    pub fn partial(&self, txid: &Txid) -> Res<Option<PartialRow>> {
        let c: Option<String> =
            self.conn.prepare_cached("SELECT chain FROM partial WHERE txid=?1").map_err(e)?.query_row([&txid[..]], |r| r.get(0)).optional().map_err(e)?;
        Ok(c.and_then(|c| {
            let v: serde_json::Value = serde_json::from_str(&c).ok()?;
            let dec = |k: &str| -> Option<Vec<Vec<u8>>> { v[k].as_array()?.iter().map(|x| hex::decode(x.as_str()?).ok()).collect() };
            let txids = dec("txids")?.into_iter().map(|t| t.try_into().ok()).collect::<Option<Vec<Txid>>>()?;
            Some(PartialRow { txids, scripts: dec("scripts")? })
        }))
    }
    pub fn del_partial(&self, txid: &Txid) -> Res<()> {
        self.conn.prepare_cached("DELETE FROM partial WHERE txid=?1").map_err(e)?.execute([&txid[..]]).map(|_| ()).map_err(e)
    }
    pub fn put_partial(&self, txid: &Txid, p: &PartialRow) -> Res<()> {
        let j = serde_json::json!({
            "txids": p.txids.iter().map(hex::encode).collect::<Vec<_>>(),
            "scripts": p.scripts.iter().map(hex::encode).collect::<Vec<_>>(),
        });
        self.conn
            .prepare_cached("INSERT OR REPLACE INTO partial(txid, chain) VALUES(?1,?2)")
            .map_err(e)?
            .execute(params![&txid[..], j.to_string()])
            .map(|_| ())
            .map_err(e)
    }

    pub fn has_open_lock(&self, iid: u64) -> bool {
        self.open_locks.contains(&iid)
    }

    /// First valid claim wins: keep the earliest, count the rest.
    pub fn name_claim(&self, name: &str, iid: u64, insc_id: &str, height: u64, txidx: u32) -> Res<()> {
        self.conn
            .prepare_cached("INSERT INTO names(name,iid,insc_id,height,txidx) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(name) DO UPDATE SET claims=claims+1")
            .map_err(e)?
            .execute(params![name, iid as i64, insc_id, height as i64, txidx as i64])
            .map(|_| ())
            .map_err(e)
    }
    pub fn name(&self, name: &str) -> Res<Option<NameRow>> {
        self.conn
            .query_row("SELECT iid, insc_id, height, txidx, claims FROM names WHERE name=?1", [name], |r| {
                Ok((r.get::<_, i64>(0)? as u64, r.get(1)?, r.get::<_, i64>(2)? as u64, r.get::<_, i64>(3)? as u32, r.get::<_, i64>(4)? as u64))
            })
            .optional()
            .map_err(e)
    }

    fn fail(&mut self, x: rusqlite::Error) {
        if self.failed.is_none() {
            self.failed = Some(e(x));
        }
    }

    pub fn token_row(&self, tick: &str) -> Res<Option<Token>> {
        self.conn
            .query_row("SELECT tick,tick_raw,iid,insc_id,height,txidx,txid,deployer,max,lim,dec,minted FROM tok WHERE tick=?1", [tick], |r| {
                Ok(Token {
                    tick: r.get(0)?,
                    tick_raw: r.get(1)?,
                    iid: r.get::<_, i64>(2)? as u64,
                    inscription_id: r.get(3)?,
                    height: r.get::<_, i64>(4)? as u64,
                    txidx: r.get::<_, i64>(5)? as u32,
                    txid: r.get(6)?,
                    deployer: r.get(7)?,
                    max: parse_scaled(&r.get::<_, String>(8)?).unwrap_or(0),
                    lim: parse_scaled(&r.get::<_, String>(9)?).unwrap_or(0),
                    dec: r.get::<_, i64>(10)? as u32,
                    minted: parse_scaled(&r.get::<_, String>(11)?).unwrap_or(0),
                })
            })
            .optional()
            .map_err(e)
    }
    pub fn balance_row(&self, tick: &str, owner: &str) -> Res<Balance> {
        let r: Option<(String, String)> = self
            .conn
            .prepare_cached("SELECT avail, trans FROM bal WHERE tick=?1 AND addr=?2")
            .map_err(e)?
            .query_row([tick, owner], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()
            .map_err(e)?;
        Ok(r.map(|(a, t)| Balance { avail: parse_scaled(&a).unwrap_or(0), trans: parse_scaled(&t).unwrap_or(0) }).unwrap_or_default())
    }
    pub fn holder_count(&self, tick: &str) -> Res<u64> {
        self.conn
            .query_row("SELECT count(*) FROM bal WHERE tick=?1 AND (avail<>'0' OR trans<>'0')", [tick], |r| r.get::<_, i64>(0))
            .map(|n| n as u64)
            .map_err(e)
    }
}

impl LedgerStore for Db {
    fn token(&mut self, tick: &str) -> Option<Token> {
        match self.token_row(tick) {
            Ok(t) => t,
            Err(x) => {
                self.failed.get_or_insert(x);
                None
            }
        }
    }
    fn put_token(&mut self, t: &Token) {
        let r = self.conn.prepare_cached("INSERT OR REPLACE INTO tok(tick,tick_raw,iid,insc_id,height,txidx,txid,deployer,max,lim,dec,minted) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)").and_then(|mut s| {
            s.execute(params![t.tick, t.tick_raw, t.iid as i64, t.inscription_id, t.height as i64, t.txidx as i64, t.txid, t.deployer, fmt_amount(t.max), fmt_amount(t.lim), t.dec as i64, fmt_amount(t.minted)])
        });
        if let Err(x) = r {
            self.fail(x)
        }
    }
    fn balance(&mut self, tick: &str, owner: &str) -> Balance {
        match self.balance_row(tick, owner) {
            Ok(b) => b,
            Err(x) => {
                self.failed.get_or_insert(x);
                Balance::default()
            }
        }
    }
    fn put_balance(&mut self, tick: &str, owner: &str, b: Balance) {
        let r = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO bal(tick,addr,avail,trans) VALUES(?1,?2,?3,?4)")
            .and_then(|mut s| s.execute(params![tick, owner, fmt_amount(b.avail), fmt_amount(b.trans)]));
        if let Err(x) = r {
            self.fail(x)
        }
    }
    fn add_lock(&mut self, l: Lock) {
        self.open_locks.insert(l.iid);
        let r = self
            .conn
            .prepare_cached("INSERT OR REPLACE INTO lck(iid,txid,vout,tick,owner,amt,height) VALUES(?1,?2,?3,?4,?5,?6,?7)")
            .and_then(|mut s| s.execute(params![l.iid as i64, l.txid, l.vout as i64, l.tick, l.owner, fmt_amount(l.amt), l.height as i64]));
        if let Err(x) = r {
            self.fail(x)
        }
    }
    fn take_lock(&mut self, iid: u64) -> Option<Lock> {
        if !self.open_locks.remove(&iid) {
            return None;
        }
        let r = self
            .conn
            .query_row("SELECT txid,vout,tick,owner,amt,height FROM lck WHERE iid=?1", [iid as i64], |r| {
                Ok(Lock {
                    iid,
                    txid: r.get(0)?,
                    vout: r.get::<_, i64>(1)? as u32,
                    tick: r.get(2)?,
                    owner: r.get(3)?,
                    amt: parse_scaled(&r.get::<_, String>(4)?).unwrap_or(0),
                    height: r.get::<_, i64>(5)? as u64,
                })
            })
            .and_then(|l| self.conn.execute("DELETE FROM lck WHERE iid=?1", [iid as i64]).map(|_| l));
        match r {
            Ok(l) => Some(l),
            Err(x) => {
                self.fail(x);
                None
            }
        }
    }
    fn event(&mut self, v: Event) {
        let r = self
            .conn
            .prepare_cached("INSERT INTO ev(height,txidx,txid,iid,op,tick,addr,amt,credit,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)")
            .and_then(|mut s| s.execute(params![v.height as i64, v.txidx as i64, v.txid, v.iid as i64, v.op, v.tick, v.addr, v.amt, v.credit, v.status]));
        if let Err(x) = r {
            self.fail(x)
        }
    }
}
