// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Runners for the published test vectors in `rules/vectors/`. Used by `cargo test` and by
//! `dsverify vectors run`, so both check exactly the same thing.

use crate::drc20::{drc20_like, fmt_amount, parse_op, Op};
use crate::envelope::{self as env, Parsed};
use crate::koinu::{gather, route, BlockRouter, Dest, Held, TxInput, SUBSIDY_KOINU};
use crate::owner::owner_of_script;
use crate::planner::{plan_block, LocRow, PartialRow, Prefetch};
use crate::tx::{decode_tx_hex, parse_txid, txid_hex, RawIn, RawOut, RawTx, Txid};
use serde_json::Value;
use std::path::Path;

pub type VResult = Result<usize, String>;

macro_rules! ensure {
    ($c:expr, $($a:tt)*) => { if !$c { return Err(format!($($a)*)); } };
}

fn read(dir: &Path, f: &str) -> Result<String, String> {
    std::fs::read_to_string(dir.join(f)).map_err(|e| format!("{}: {e}", dir.join(f).display()))
}
fn json(dir: &Path, f: &str) -> Result<Value, String> {
    serde_json::from_str(&read(dir, f)?).map_err(|e| format!("{f}: {e}"))
}
fn hx(s: &str) -> Result<Vec<u8>, String> {
    hex::decode(s).map_err(|e| format!("hex: {e}"))
}

fn show(p: Parsed) -> String {
    match p {
        Parsed::None => "None\t\t".into(),
        Parsed::Partial => "Partial\t\t".into(),
        Parsed::Complete(i) => format!("Complete\t{}\t{}", hex::encode(i.content_type), hex::encode(i.body)),
    }
}

/// `envelope/envelope_cases.{in,expected}`: input scripts and the expected envelope verdict.
pub fn envelope(dir: &Path) -> VResult {
    let inp = read(dir, "envelope/envelope_cases.in")?;
    let exp = read(dir, "envelope/envelope_cases.expected")?;
    let mut n = 0;
    for (line, e) in inp.lines().zip(exp.lines()) {
        let mut f = line.split('\t');
        let id = f.next().unwrap_or("");
        let scripts: Vec<Vec<u8>> = f.next().unwrap_or("").split(',').map(hx).collect::<Result<_, _>>()?;
        let got = format!("{id}\t{}", show(env::parse(&scripts)));
        ensure!(got == e, "envelope case {id}: got {got:?}, want {e:?}");
        n += 1;
    }
    ensure!(n > 0 && inp.lines().count() == exp.lines().count(), "envelope vectors: line counts differ");
    Ok(n)
}

fn canon(s: &str) -> String {
    let (i, f) = s.split_once('.').unwrap_or((s, ""));
    let i = i.trim_start_matches('0');
    let f = f.trim_end_matches('0');
    let i = if i.is_empty() { "0" } else { i };
    if f.is_empty() {
        i.to_string()
    } else {
        format!("{i}.{f}")
    }
}

/// `drc20/json_bodies.json`: inscription bodies and the DRC-20 operation they are (or are not).
pub fn drc20_bodies(dir: &Path) -> VResult {
    let cases = json(dir, "drc20/json_bodies.json")?;
    let mut n = 0;
    for c in cases.as_array().ok_or("json_bodies.json: not an array")? {
        let id = c["id"].as_str().unwrap_or("?");
        let b = hx(c["body_hex"].as_str().unwrap_or(""))?;
        ensure!(drc20_like(&b) == c["drc20_like"].as_bool().unwrap_or(false), "{id}: drc20-like");
        let got = parse_op(&b);
        n += 1;
        if c.get("ledger_throws").is_some() {
            ensure!(got.is_some(), "{id}: a deeply nested body still reads");
            continue;
        }
        let w = &c["ledger_op"];
        let s = |k: &str| w[k].as_str().map(str::to_string);
        match got {
            None => ensure!(w.is_null(), "{id}: should read as {w}"),
            Some(Op::Mint { tick, amt }) => {
                ensure!(s("op").as_deref() == Some("mint") && s("tick") == Some(tick) && s("amt") == Some(amt), "{id}: mint differs from {w}")
            }
            Some(Op::Transfer { tick, amt }) => {
                ensure!(s("op").as_deref() == Some("transfer") && s("tick") == Some(tick) && s("amt") == Some(amt), "{id}: transfer differs from {w}")
            }
            Some(Op::Deploy { tick, max, lim, dec }) => ensure!(
                s("op").as_deref() == Some("deploy")
                    && s("tick") == Some(tick)
                    && s("max").map(|x| canon(&x)) == Some(fmt_amount(max))
                    && s("lim").map(|x| canon(&x)) == Some(fmt_amount(lim))
                    && w["dec"].as_i64() == Some(dec as i64),
                "{id}: deploy differs from {w}"
            ),
        }
    }
    Ok(n)
}

fn dest(v: &Value) -> Option<Dest> {
    Some(match v[0].as_str()? {
        "out" => Dest::Output { vout: v[1].as_u64()? as u32, off: v[2].as_str()?.parse().ok()? },
        "fee" => Dest::Fee { off: v[1].as_str()?.parse().ok()? },
        _ => Dest::Lost { off: v[1].as_str()?.parse().ok()? },
    })
}

/// `koinu/routing_cases.json`: blocks of transactions and where every inscription's koinu ends up.
pub fn koinu(dir: &Path) -> VResult {
    let cases = json(dir, "koinu/routing_cases.json")?;
    let mut items = 0usize;
    for (ci, c) in cases.as_array().ok_or("routing_cases.json: not an array")?.iter().enumerate() {
        let txs = c["blk"].as_array().ok_or("blk")?;
        let exp = c["expect"].as_array().ok_or("expect")?;
        let mut order: Vec<usize> = (1..txs.len()).collect();
        order.push(0);
        let mut br = BlockRouter::new();
        for (pos, &k) in order.iter().enumerate() {
            let t = &txs[k];
            let coinbase = k == 0;
            let ins: Vec<TxInput> = if coinbase {
                vec![]
            } else {
                t["ins"]
                    .as_array()
                    .ok_or("ins")?
                    .iter()
                    .map(|x| TxInput {
                        value: x["value"].as_u64().unwrap_or(0),
                        held: x["held"]
                            .as_array()
                            .map(|a| a.iter().map(|h| Held { off: h["off"].as_u64().unwrap_or(0), iid: h["iid"].as_u64().unwrap_or(0) }).collect())
                            .unwrap_or_default(),
                    })
                    .collect()
            };
            let outs: Vec<u64> = t["outs"].as_array().ok_or("outs")?.iter().map(|v| v.as_u64().unwrap_or(0)).collect();
            let (g, open) = gather(&ins).map_err(|e| format!("case {ci}: {e:?}"))?;
            let new = if open { t["newIid"].as_u64() } else { None };
            let placed = route(&g, new, br.flotsam(), coinbase, &outs).map_err(|e| format!("case {ci}: {e:?}"))?;
            if !coinbase {
                br.after_tx(k, &placed, t["fee"].as_u64().unwrap_or(0)).map_err(|e| format!("case {ci}: {e:?}"))?;
            }
            let e = &exp[pos];
            ensure!(e["gateOpen"].as_bool() == Some(open), "koinu case {ci} tx {k}: gate");
            let ep = e["placed"].as_array().ok_or("placed")?;
            ensure!(ep.len() == placed.len(), "koinu case {ci} tx {k}: {} placed, want {}", placed.len(), ep.len());
            for (a, b) in placed.iter().zip(ep) {
                let want =
                    (b["iid"].as_u64(), b["new"].as_bool(), b["fromFee"].as_bool(), b["src"].as_str().and_then(|s| s.parse::<u64>().ok()), dest(&b["dest"]));
                ensure!(want == (Some(a.iid), Some(a.new), Some(a.from_fee), Some(a.src_off), Some(a.dest)), "koinu case {ci} tx {k}: got {a:?}, want {b}");
                items += 1;
            }
        }
    }
    Ok(items)
}

fn dest_of(dvout: i64, off: u64) -> Dest {
    match dvout {
        -1 => Dest::Fee { off },
        -2 => Dest::Lost { off },
        v => Dest::Output { vout: v as u32, off },
    }
}

fn coinbase() -> RawTx {
    RawTx {
        txid: [0; 32],
        ins: vec![RawIn { prev: [0; 32], vout: u32::MAX, script: vec![0x6a] }],
        outs: vec![RawOut { value: SUBSIDY_KOINU, spk: vec![0x51] }],
    }
}

/// One mainnet reveal vector, as published.
pub struct Reveal {
    pub id: String,
    pub height: u64,
    pub txid: String,
    pub raw_hex: String,
}

/// `inscriptions/mainnet_reveals.json`: real reveal transactions, the koinu they spend, and the verdict.
/// Returns the vectors so a caller can also check `raw_hex` against a node.
pub fn mainnet_reveals(dir: &Path) -> Result<Vec<Reveal>, String> {
    let cases = json(dir, "inscriptions/mainnet_reveals.json")?;
    let mut out = Vec::new();
    for c in cases.as_array().ok_or("mainnet_reveals.json: not an array")? {
        let name = c["id"].as_str().unwrap_or("?").to_string();
        let raw = c["raw_hex"].as_str().unwrap_or("");
        let reveal = decode_tx_hex(raw).map_err(|e| format!("{name}: {e}"))?;
        ensure!(txid_hex(&reveal.txid) == c["txid"].as_str().unwrap_or(""), "{name}: raw transaction does not hash to its txid");
        let mut pre = Prefetch::default();
        for (i, x) in c["inputs"].as_array().ok_or("inputs")?.iter().enumerate() {
            if let Some(v) = x["value"].as_str() {
                let r = reveal.ins.get(i).ok_or(format!("{name}: input {i}"))?;
                pre.values.insert((r.prev, r.vout), v.parse().map_err(|_| format!("{name}: value"))?);
            }
        }
        let mut expect: Vec<(u64, Dest)> = Vec::new();
        let mut syn = 1_000_000u64;
        for s in c["held"].as_array().ok_or("held")? {
            let vin = s["vin"].as_u64().unwrap_or(0) as usize;
            let num = |k: &str| s[k].as_str().and_then(|x| x.parse::<i64>().ok()).unwrap_or(0);
            let (lo, hi, delta) = (num("lo") as u64, num("hi") as u64, num("delta"));
            let n = s["n"].as_u64().unwrap_or(1);
            let r = reveal.ins.get(vin).ok_or(format!("{name}: vin {vin}"))?;
            let op = (r.prev, r.vout);
            let val = *pre.values.get(&op).ok_or(format!("{name}: no value for vin {vin}"))?;
            let mut offs = vec![lo];
            if n >= 2 {
                offs.push(hi - 1);
            }
            while (offs.len() as u64) < n {
                offs.push(lo);
            }
            for off in offs {
                pre.loc.entry(op).or_default().push(LocRow { off, iid: syn, val });
                expect.push((syn, dest_of(s["dvout"].as_i64().unwrap_or(0), (off as i64 + delta) as u64)));
                syn += 1;
            }
        }
        if let Some(p) = c["parent"].as_object() {
            let txids: Vec<Txid> =
                p["txids"].as_array().ok_or("parent txids")?.iter().map(|t| parse_txid(t.as_str().unwrap_or("")).map_err(|e| e.0)).collect::<Result<_, _>>()?;
            let scripts: Vec<Vec<u8>> =
                p["scripts"].as_array().ok_or("parent scripts")?.iter().map(|s| hx(s.as_str().unwrap_or(""))).collect::<Result<_, _>>()?;
            pre.partial.insert(*txids.last().ok_or("parent")?, PartialRow { txids, scripts });
        }
        let plan = plan_block(&[coinbase(), reveal.clone()], &pre, 1).map_err(|e| format!("{name}: {e}"))?;
        let v = &plan.txs[1];
        let mut got = v.moves.clone();
        got.sort_by_key(|m| m.0);
        expect.sort_by_key(|m| m.0);
        ensure!(got == expect, "{name}: held inscriptions moved to {got:?}, want {expect:?}");
        let st = &c["expect"];
        if st["created"].as_bool() == Some(true) {
            let k = v.created.as_ref().ok_or(format!("{name}: should create an inscription"))?;
            ensure!(Some(k.inscription_id.as_str()) == st["id"].as_str(), "{name}: inscription id");
            let off = st["off"].as_str().and_then(|x| x.parse().ok()).unwrap_or(0);
            ensure!(k.dest == dest_of(st["vout"].as_i64().unwrap_or(0), off), "{name}: landing");
            ensure!(Some(k.kind as i64) == st["kind"].as_i64(), "{name}: kind");
            ensure!(Some(String::from_utf8_lossy(&k.content_type).as_ref()) == st["ctype"].as_str(), "{name}: content type");
            if let Some(b) = st["body_hex"].as_str() {
                ensure!(hex::encode(&k.body) == b, "{name}: body");
            }
            if let (Some(a), Dest::Output { vout, .. }) = (st["addr"].as_str(), k.dest) {
                ensure!(owner_of_script(&reveal.outs[vout as usize].spk).key() == a, "{name}: owner");
            }
        } else {
            ensure!(v.created.is_none(), "{name}: must not create an inscription");
        }
        out.push(Reveal { id: name, height: c["height"].as_u64().unwrap_or(0), txid: c["txid"].as_str().unwrap_or("").into(), raw_hex: raw.into() });
    }
    Ok(out)
}

/// `names/claims.json`: real first valid claims (the reveal transaction) and claim bodies.
/// Returns the mainnet claims so a caller can check them against a node.
pub fn names(dir: &Path) -> Result<(usize, Vec<Reveal>), String> {
    let v = json(dir, "names/claims.json")?;
    let mut n = 0;
    let mut out = Vec::new();
    for c in v["mainnet_claims"].as_array().ok_or("mainnet_claims")? {
        let id = c["id"].as_str().unwrap_or("?").to_string();
        let raw = c["raw_hex"].as_str().unwrap_or("");
        let t = decode_tx_hex(raw).map_err(|e| format!("{id}: {e}"))?;
        ensure!(txid_hex(&t.txid) == c["txid"].as_str().unwrap_or(""), "{id}: raw transaction does not hash to its txid");
        let script = t.ins.first().map(|i| i.script.clone()).unwrap_or_default();
        let Parsed::Complete(i) = env::parse(&[script]) else { return Err(format!("{id}: no complete envelope in input 0")) };
        let got = crate::names::reg_claim(&i.body).map(|x| x.name);
        ensure!(got.as_deref() == c["expect"]["name"].as_str(), "{id}: claim reads as {got:?}");
        n += 1;
        out.push(Reveal { id, height: c["height"].as_u64().unwrap_or(0), txid: txid_hex(&t.txid), raw_hex: raw.into() });
    }
    for c in v["bodies"].as_array().ok_or("bodies")? {
        let id = c["id"].as_str().unwrap_or("?");
        let got = crate::names::reg_claim(&hx(c["body_hex"].as_str().unwrap_or(""))?).map(|x| x.name);
        ensure!(got.as_deref() == c["expect"]["name"].as_str(), "{id}: claim reads as {got:?}");
        n += 1;
    }
    Ok((n, out))
}
