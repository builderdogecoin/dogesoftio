// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Canonical state listings and their SHA-256, the format published in `proofs/checkpoints/<height>/`.
//!
//! Every listing is UTF-8, one record per line, fields joined by a tab, each line ending in `\n`, lines
//! sorted by their bytes. Amounts are plain decimals with no trailing zeros (`0` for zero).
//!
//! - `balances`: `tick  owner  available  transferable` for every owner with either above zero
//! - `holders`:  `tick  holder_count` for every Coin with at least one holder
//! - `locks`:    `inscription_txid  tick  owner  amount` for every open transfer inscription
//! - `tokens`:   `tick  max  lim  dec  minted  deploy_height  deploy_txid  deployer`

use crate::db::{Db, Res};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const LISTINGS: [&str; 4] = ["balances", "holders", "locks", "tokens"];

fn rows(db: &Db, sql: &str, n: usize) -> Res<Vec<String>> {
    let mut st = db.conn.prepare(sql).map_err(|e| e.to_string())?;
    let it = st
        .query_map([], |r| {
            let mut f = Vec::with_capacity(n);
            for i in 0..n {
                let v: rusqlite::types::Value = r.get(i)?;
                f.push(match v {
                    rusqlite::types::Value::Integer(x) => x.to_string(),
                    rusqlite::types::Value::Text(s) => s,
                    rusqlite::types::Value::Null => String::new(),
                    other => format!("{other:?}"),
                });
            }
            Ok(f.join("\t") + "\n")
        })
        .map_err(|e| e.to_string())?;
    it.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Build one listing, sorted, as bytes.
pub fn listing(db: &Db, name: &str) -> Res<Vec<u8>> {
    let mut lines = match name {
        "balances" => rows(db, "SELECT tick, addr, avail, trans FROM bal WHERE avail<>'0' OR trans<>'0'", 4)?,
        "holders" => {
            let mut m: BTreeMap<String, u64> = BTreeMap::new();
            for l in rows(db, "SELECT tick FROM bal WHERE avail<>'0' OR trans<>'0'", 1)? {
                *m.entry(l.trim_end_matches('\n').to_string()).or_default() += 1;
            }
            m.into_iter().map(|(t, n)| format!("{t}\t{n}\n")).collect()
        }
        "locks" => rows(db, "SELECT txid, tick, owner, amt FROM lck", 4)?,
        "tokens" => rows(db, "SELECT tick, max, lim, dec, minted, height, txid, deployer FROM tok", 8)?,
        _ => return Err(format!("unknown listing {name}")),
    };
    lines.sort_unstable_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    Ok(lines.concat().into_bytes())
}

pub fn sha256_hex(b: &[u8]) -> String {
    hex::encode(Sha256::digest(b))
}

pub fn line_count(b: &[u8]) -> usize {
    b.iter().filter(|c| **c == b'\n').count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ds_rules::ledger::{on_inscription, on_move, NewInscription, SendTo};

    fn ins<'a>(iid: u64, owner: &str, body: &'a [u8], txid: &'a str) -> NewInscription<'a> {
        NewInscription { height: 4_700_000 + iid, txidx: 1, reveal_txid: txid, iid, inscription_id: txid, landing: Some((0, owner.to_string())), body }
    }

    #[test]
    fn listings_are_canonical_and_rules_hold_in_sqlite() {
        let p = std::env::temp_dir().join(format!("dsverify-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&p);
        let mut d = Db::open(&p).unwrap();
        d.begin().unwrap();
        on_inscription(&mut d, &ins(1, "DA", br#"{"p":"drc-20","op":"deploy","tick":"WoW!","max":"100","lim":"60","dec":"2"}"#, "t1"));
        on_inscription(&mut d, &ins(2, "DA", br#"{"p":"drc-20","op":"mint","tick":"wow!","amt":"60"}"#, "t2"));
        on_inscription(&mut d, &ins(3, "DB", br#"{"p":"drc-20","op":"mint","tick":"wow!","amt":"40.5"}"#, "t3"));
        assert!(on_inscription(&mut d, &ins(4, "DA", br#"{"p":"drc-20","op":"transfer","tick":"wow!","amt":"10.25"}"#, "t4")));
        assert!(on_inscription(&mut d, &ins(5, "DA", br#"{"p":"drc-20","op":"transfer","tick":"wow!","amt":"1"}"#, "t5")));
        assert!(on_move(&mut d, 4_700_009, 2, "s1", 4, &SendTo::Owner("DC".into())));
        assert!(on_move(&mut d, 4_700_009, 3, "s2", 5, &SendTo::Fee));
        assert!(!d.has_open_lock(4));
        d.commit().unwrap();
        let bal = String::from_utf8(listing(&d, "balances").unwrap()).unwrap();
        assert_eq!(bal, "wow!\tDA\t49.75\t0\nwow!\tDB\t40\t0\nwow!\tDC\t10.25\t0\n");
        assert_eq!(String::from_utf8(listing(&d, "holders").unwrap()).unwrap(), "wow!\t3\n");
        assert_eq!(String::from_utf8(listing(&d, "locks").unwrap()).unwrap(), "");
        assert_eq!(String::from_utf8(listing(&d, "tokens").unwrap()).unwrap(), "wow!\t100\t60\t2\t100\t4700001\tt1\tDA\n");
        assert_eq!(d.holder_count("wow!").unwrap(), 3);
        let b = listing(&d, "balances").unwrap();
        assert_eq!(line_count(&b), 3);
        assert_eq!(sha256_hex(b""), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        drop(d);
        let _ = std::fs::remove_file(&p);
    }
}
