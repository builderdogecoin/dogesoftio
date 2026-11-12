// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Unit tests for every Doge Soft rule, plus the published vectors in `rules/vectors/`.

use ds_rules::drc20::{amount, fmt_amount, parse_op, tick_key, tick_ok, Op, SCALE};
use ds_rules::envelope::{self as env, Parsed};
use ds_rules::koinu::*;
use ds_rules::ledger::{on_inscription, on_move, MemStore, NewInscription, SendTo};
use ds_rules::names::{normalize, reg_claim, XD};
use ds_rules::owner::{owner_of_script, Owner};
use ds_rules::planner::*;
use ds_rules::tx::*;
use serde_json::Value;
use std::path::PathBuf;

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../rules/vectors")
}
fn h(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}
fn inp(value: u64, held: &[(u64, u64)]) -> TxInput {
    TxInput { value, held: held.iter().map(|&(off, iid)| Held { off, iid }).collect() }
}

// ------------------------------------------------------------------ 01 envelopes

fn body(r: Parsed) -> Option<String> {
    match r {
        Parsed::Complete(i) => Some(String::from_utf8(i.body).unwrap()),
        _ => None,
    }
}

// input-0 scriptSigs of six audited mainnet deploys
const OXAN_4748189: &str = "5a004830450221009b2c7b1caf8333cfeed84321ddf0d9e1b15960f92bd40cf86a3e5dc6baa4d9fc022025ebca72067623898afa9356f9a3c8c88ce6a1a657e72e715007484268647ba0014c99512103c3c2e7ba3d48a76f5db2b23ebc394b36077cc4bdc5bfe31ffc67f261c8bc168c51ae036f726418746578742f706c61696e3b636861727365743d7574662d384c517b22646563223a31382c226c696d223a2231303030222c226d6178223a223231303030303030222c226f70223a226465706c6f79222c2270223a226472632d3230222c227469636b223a224f58414e227d75757575";
const ARMY_4747909: &str = "5a00483045022100ae4f1b7dd0712c406dd5608d825994d004738393e8962c294e19e9377d9db2e502200672ff412e9b37e965afc8ecc8cdef959f96b71ace3e42a3b1235c688b89a6cf014c97512103c24ca5788ca06a64555863ca44b8d9a20ab78991d90fa081355da8d0e13e1f8051ae036f726418746578742f706c61696e3b636861727365743d7574662d384c4f7b22646563223a31382c226c696d223a22373737222c226d6178223a2237373737373737222c226f70223a226465706c6f79222c2270223a226472632d3230222c227469636b223a2241524d59227d75757575";
const PRIM_4760653: &str = "5a004830450221008ef8c0f9243e22fdcdf3aeb093d71965317b7f00ce82dfe957867974ccc095a102202ec1eef11dd31df540d4e80fe82490554bf95fadeea313b7759f27cd9a98a81f014c965121037708d3b7c6865d51e0fbfb5eec008a57de2ae890ca0896b686c5873abceb171f51ae036f726418746578742f706c61696e3b636861727365743d7574662d384c4e7b22646563223a31382c226c696d223a2231303039222c226d6178223a223130303039222c226f70223a226465706c6f79222c2270223a226472632d3230222c227469636b223a225052494d227d75757575";
const ARMY_4751087: &str = "036f72645117746578742f706c61696e3b636861727365743d75746638004c5d7b0a20202270223a20226472632d3230222c0a2020226f70223a20226465706c6f79222c0a2020227469636b223a202261726d79222c0a2020226d6178223a20223230313331323036222c0a2020226c696d223a202232303133220a7d473044022073f4e282ae108a41d2c73f60a2d8164cac00f9af69cd00f2f463d1f71aad3a670220446d717ba985885fe0bd75e178960860ef2d94fbe6b02ec1dd5b07224083591001292102f932b7ef7cedd93ffa93f2dbc90bb701da6487e6ef61b0a60cb209e05edaf31ead757575757551";
const PRIM_5859907: &str = "036f7264510a746578742f706c61696e004c4c7b2270223a226472632d3230222c226f70223a226465706c6f79222c227469636b223a227072696d222c226d6178223a223231303030303030222c226c696d223a223231303030303030227d47304402206e87a9ee70f04f968f26eddb458cbb5e166a337e5ff748ba0b50607b6df7083402201275e491690d7c3edbe554f0e170d20deeacbb71e41e40c8ae14e8707ba42526012921035ae7aae57ccd68881d2959d5a1a86912677b9ab6c8da137642e4006dc406f20aad757575757551";
const DOGI_4630091: &str = "036f72645119746578742f706c61696e3b20636861727365743d7574662d38004c647b200d0a20202270223a20226472632d3230222c0d0a2020226f70223a20226465706c6f79222c0d0a2020227469636b223a2022646f6769222c0d0a2020226d6178223a20223231303030303030222c0d0a2020226c696d223a202231303030220d0a7d4830450221008f20b47cab433bb680114700b7ec5c140d74522120d9746fd16ce20ec07f3a4502207ea121b03b038cb6838cacb1d6e5ce191f745b1af8940ec0079d3bfda26ed5e80129210321802b1bbff4781a29049a8fa84e71ed1e553ba16c8c196c0b0149c3f283a988ad757575757551";

#[test]
fn data_hidden_in_a_redeem_script_is_not_an_inscription() {
    for s in [OXAN_4748189, ARMY_4747909, PRIM_4760653] {
        assert_eq!(env::parse(&[h(s)]), Parsed::None);
    }
}

#[test]
fn standard_deploys_are_inscriptions_with_their_json() {
    assert!(body(env::parse(&[h(ARMY_4751087)])).unwrap().contains("\"tick\": \"army\""));
    assert!(body(env::parse(&[h(PRIM_5859907)])).unwrap().contains("\"max\":\"21000000\""));
    assert!(body(env::parse(&[h(DOGI_4630091)])).unwrap().contains("\"tick\": \"dogi\""));
}

#[test]
fn op_if_wrapper_is_not_a_doginals_envelope() {
    assert_eq!(env::parse(&[h("0063036f7264510a746578742f706c61696e00017868")]), Parsed::None);
}

#[test]
fn multi_tx_envelope() {
    let t1 = h("036f7264520a746578742f706c61696e51026162");
    let t2 = h("00026364");
    assert_eq!(env::parse(std::slice::from_ref(&t1)), Parsed::Partial);
    let parent = env::partial_chain_of(vec![t1.clone()]);
    assert!(parent.is_some());
    assert_eq!(body(env::parse_with_parent(parent, t2.clone()).0).unwrap(), "abcd");
    assert_eq!(env::parse_with_parent(None, t2).0, Parsed::None);
    // a fresh `ord` script spending a partial parent is read as a continuation and fails
    assert_eq!(env::parse_with_parent(env::partial_chain_of(vec![t1]), h(PRIM_5859907)).0, Parsed::None);
}

fn d(b: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    if b.len() <= 75 {
        v.push(b.len() as u8);
    } else {
        v.push(0x4c);
        v.push(b.len() as u8);
    }
    v.extend_from_slice(b);
    v
}
const JS: &[u8] = br#"{"p":"drc-20","op":"transfer","tick":"dogi","amt":"5"}"#;
fn tag_form(extra: &[&[u8]]) -> Vec<u8> {
    let mut v = [d(b"ord"), vec![0x51], d(b"text/plain;charset=utf-8")].concat();
    for e in extra {
        v.extend(d(e));
    }
    v.extend([vec![0x00], d(JS), d(&[0x30; 71]), d(&[0x21; 41])].concat());
    v
}

#[test]
fn extra_tag_pushes_never_invalidate_an_inscription() {
    let s = tag_form(&[b"TRANSFER"]);
    assert_eq!(env::parse_strict(std::slice::from_ref(&s)), Parsed::Partial);
    assert_eq!(body(env::parse(&[s])).unwrap().as_bytes(), JS);
    for extra in [vec![&b"MINT"[..]], vec![&b"BRIDGE_L1_RELEASE"[..], &b"bid:42"[..]], vec![&b"T"[..]; 9]] {
        assert_eq!(body(env::parse(&[tag_form(&extra)])).unwrap().as_bytes(), JS, "{extra:?}");
    }
    // more than 9, non-printable, or longer than 80 bytes: not skipped
    assert_eq!(env::parse(&[tag_form(&[&b"T"[..]; 10])]), Parsed::Partial);
    assert_eq!(env::parse(&[tag_form(&[&[0xde, 0xad, 0xbe, 0xef]])]), Parsed::Partial);
    assert_eq!(env::parse(&[tag_form(&[&[b'A'; 81]])]), Parsed::Partial);
}

#[test]
fn historical_pushdata_length_reading() {
    // OP_PUSHDATA2 with length bytes (0x00, 0x00) reads as 77 bytes
    let mut s = vec![0x4d, 0x00, 0x00];
    s.extend(vec![7u8; 77]);
    let p = env::read_pushes(&s).unwrap();
    assert_eq!(p.len(), 1);
    assert_eq!(p[0].data.len(), 77);
    assert!(env::read_pushes(&[0x4d, 0x00, 0x00, 1, 2]).is_none());
    assert!(env::read_pushes(&[0x4f]).is_none(), "OP_1NEGATE is not a push here");
    assert_eq!(env::push_number(&[]), Some(0));
    assert_eq!(env::push_number(&[1, 1]), Some(257));
    assert_eq!(env::push_number(&[0; 9]), None);
}

#[test]
fn envelope_corpus_matches_production_parser() {
    let inp = std::fs::read_to_string(vectors().join("envelope/envelope_cases.in")).unwrap();
    let exp = std::fs::read_to_string(vectors().join("envelope/envelope_cases.expected")).unwrap();
    let mut n = 0;
    for (line, e) in inp.lines().zip(exp.lines()) {
        let mut f = line.split('\t');
        let id = f.next().unwrap();
        let scripts: Vec<Vec<u8>> = f.next().unwrap_or("").split(',').map(|x| hex::decode(x).unwrap()).collect();
        let got = match env::parse(&scripts) {
            Parsed::None => format!("{id}\tNone\t\t"),
            Parsed::Partial => format!("{id}\tPartial\t\t"),
            Parsed::Complete(i) => format!("{id}\tComplete\t{}\t{}", hex::encode(i.content_type), hex::encode(i.body)),
        };
        assert_eq!(got, e, "case {id}");
        n += 1;
    }
    assert_eq!(n, 4000);
}

// ------------------------------------------------------------------ 02 koinu tracking

#[test]
fn clean_inputs_open_the_gate_and_new_lands_on_first_koinu_of_output_0() {
    let (g, open) = gather(&[inp(100_000, &[]), inp(5, &[])]).unwrap();
    assert!(g.is_empty() && open);
    let p = route(&g, Some(7), &[], false, &[100_000, 4]).unwrap();
    assert_eq!(p, vec![Placed { iid: 7, new: true, from_fee: false, src_off: 0, dest: Dest::Output { vout: 0, off: 0 } }]);
}

#[test]
fn first_koinu_already_inscribed_closes_the_gate() {
    let (_, open) = gather(&[inp(100_000, &[(0, 1)]), inp(50, &[])]).unwrap();
    assert!(!open);
    assert!(gather(&[inp(1000, &[(1, 1)])]).unwrap().1);
    let (g, open) = gather(&[inp(1000, &[]), inp(500, &[(0, 2)])]).unwrap();
    assert!(open);
    assert_eq!(g, vec![(1000, 2)]);
}

#[test]
fn fifo_and_zero_value_outputs_hold_nothing() {
    let (g, _) = gather(&[inp(300, &[(299, 1)]), inp(200, &[(0, 2), (150, 3)])]).unwrap();
    let p = route(&g, None, &[], false, &[300, 0, 100, 90]).unwrap();
    assert_eq!(
        p.iter().map(|x| x.dest).collect::<Vec<_>>(),
        vec![Dest::Output { vout: 0, off: 299 }, Dest::Output { vout: 2, off: 0 }, Dest::Output { vout: 3, off: 50 }]
    );
}

#[test]
fn fee_spent_goes_to_coinbase_at_reward_plus_offset_or_is_lost() {
    let mut br = BlockRouter::new();
    br.after_tx(1, &[], 1000).unwrap();
    let (g, _) = gather(&[inp(1000, &[(900, 9)])]).unwrap();
    let p = route(&g, None, &[], false, &[500]).unwrap();
    assert_eq!(p[0].dest, Dest::Fee { off: 400 });
    br.after_tx(2, &p, 500).unwrap();
    assert_eq!(br.flotsam()[0].coff, SUBSIDY_KOINU + 1000 + 900);
    let cb = route(&[], None, br.flotsam(), true, &[SUBSIDY_KOINU + 1500]).unwrap();
    assert_eq!(cb[0].dest, Dest::Lost { off: 400 });
    let cb2 = route(&[], None, br.flotsam(), true, &[SUBSIDY_KOINU, 5000]).unwrap();
    assert_eq!(cb2[0].dest, Dest::Output { vout: 1, off: 1900 });
    assert_eq!(route(&[], Some(1), &[], false, &[]).unwrap()[0].dest, Dest::Fee { off: 0 });
}

#[test]
fn overflow_is_an_error_not_a_wrap() {
    assert_eq!(gather(&[inp(u64::MAX, &[]), inp(1, &[(0, 1)]), inp(1, &[(0, 2)])]), Err(SatError::Overflow));
}

#[test]
fn reverse_offset_helpers() {
    assert_eq!(input_for_offset(&[300, 200], 299), Some((0, 299)));
    assert_eq!(input_for_offset(&[300, 200], 300), Some((1, 0)));
    assert_eq!(input_for_offset(&[300, 200], 500), None);
    assert_eq!(output_offset_to_input_space(&[300, 0, 100], 2, 5), Some(305));
    assert_eq!(output_offset_to_input_space(&[300], 0, 300), None);
}

#[test]
fn koinu_routing_vectors() {
    let raw = std::fs::read_to_string(vectors().join("koinu/routing_cases.json")).unwrap();
    let cases: Value = serde_json::from_str(&raw).unwrap();
    let mut n_items = 0usize;
    for (ci, c) in cases.as_array().unwrap().iter().enumerate() {
        let txs = c["blk"].as_array().unwrap();
        let exp = c["expect"].as_array().unwrap();
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
                    .unwrap()
                    .iter()
                    .map(|x| TxInput {
                        value: x["value"].as_u64().unwrap(),
                        held: x["held"].as_array().unwrap().iter().map(|h| Held { off: h["off"].as_u64().unwrap(), iid: h["iid"].as_u64().unwrap() }).collect(),
                    })
                    .collect()
            };
            let outs: Vec<u64> = t["outs"].as_array().unwrap().iter().map(|v| v.as_u64().unwrap()).collect();
            let (g, open) = gather(&ins).unwrap();
            let new = if open { t["newIid"].as_u64() } else { None };
            let placed = route(&g, new, br.flotsam(), coinbase, &outs).unwrap();
            if !coinbase {
                br.after_tx(k, &placed, t["fee"].as_u64().unwrap()).unwrap();
            }
            let e = &exp[pos];
            assert_eq!(e["gateOpen"].as_bool().unwrap(), open, "case {ci} tx {k} gate");
            let ep = e["placed"].as_array().unwrap();
            assert_eq!(ep.len(), placed.len(), "case {ci} tx {k} count");
            for (a, b) in placed.iter().zip(ep) {
                let dd = &b["dest"];
                let want = match dd[0].as_str().unwrap() {
                    "out" => Dest::Output { vout: dd[1].as_u64().unwrap() as u32, off: dd[2].as_str().unwrap().parse().unwrap() },
                    "fee" => Dest::Fee { off: dd[1].as_str().unwrap().parse().unwrap() },
                    _ => Dest::Lost { off: dd[1].as_str().unwrap().parse().unwrap() },
                };
                assert_eq!(
                    (a.iid, a.new, a.from_fee, a.src_off, a.dest),
                    (
                        b["iid"].as_u64().unwrap(),
                        b["new"].as_bool().unwrap(),
                        b["fromFee"].as_bool().unwrap(),
                        b["src"].as_str().unwrap().parse::<u64>().unwrap(),
                        want
                    ),
                    "case {ci} tx {k}"
                );
                n_items += 1;
            }
        }
    }
    assert!(n_items > 10_000, "only {n_items} items compared");
}

// ------------------------------------------------------------------ the block planner

const T: fn(u8) -> Txid = |n| [n; 32];
fn coinbase(id: u8, outs: &[u64]) -> RawTx {
    RawTx {
        txid: T(id),
        ins: vec![RawIn { prev: [0; 32], vout: u32::MAX, script: vec![0x6a] }],
        outs: outs.iter().map(|v| RawOut { value: *v, spk: vec![0x51] }).collect(),
    }
}
fn tx(id: u8, ins: &[(Txid, u32, Vec<u8>)], outs: &[u64]) -> RawTx {
    RawTx {
        txid: T(id),
        ins: ins.iter().map(|(p, v, s)| RawIn { prev: *p, vout: *v, script: s.clone() }).collect(),
        outs: outs.iter().map(|v| RawOut { value: *v, spk: vec![0x51] }).collect(),
    }
}
fn envelope(b: &str) -> Vec<u8> {
    let mut s = h("036f726451");
    s.push(10);
    s.extend_from_slice(b"text/plain");
    s.push(0);
    s.push(b.len() as u8);
    s.extend_from_slice(b.as_bytes());
    s
}

#[test]
fn kind_classification() {
    use ds_rules::drc20::inscription_kind;
    assert_eq!(inscription_kind(b"text/plain;charset=utf-8", b"x"), 1);
    assert_eq!(inscription_kind(b" \xa0TEXT/Plain", b"x"), 1);
    assert_eq!(inscription_kind(b"application/json", b"[]"), 1);
    assert_eq!(inscription_kind(b"image/png", br#" {"p":" DRC-20 ","op":"mint"}"#), 2);
    assert_eq!(inscription_kind(b"image/png", br#"{"p":"other"}"#), 0);
    assert_eq!(inscription_kind(b"image/png", b"\x89PNG"), 0);
}

#[test]
fn gate_closed_reveal_creates_nothing() {
    let mut pre = Prefetch::default();
    pre.loc.insert((T(1), 0), vec![LocRow { off: 0, iid: 7, val: 100_000 }]);
    pre.partial.insert(T(1), PartialRow { txids: vec![T(1)], scripts: vec![h("036f7264520a746578742f706c61696e51026162")] });
    let b = vec![coinbase(0, &[1]), tx(2, &[(T(1), 0, envelope(r#"{"p":"drc-20","op":"deploy"}"#))], &[100_000])];
    let p = plan_block(&b, &pre, 100).unwrap();
    assert!(p.txs[1].gate_closed && p.txs[1].created.is_none());
    assert_eq!(p.txs[1].moves, vec![(7, Dest::Output { vout: 0, off: 0 })]);
    assert!(p.partial_deleted.is_empty() && p.partial_inserted.is_empty());
    assert_eq!(p.next_iid, 100);
}

#[test]
fn later_koinu_inscribed_keeps_gate_open() {
    let mut pre = Prefetch::default();
    pre.loc.insert((T(1), 0), vec![LocRow { off: 5, iid: 7, val: 100_000 }]);
    let b = vec![coinbase(0, &[1]), tx(2, &[(T(1), 0, envelope("hi"))], &[3, 99_000])];
    let p = plan_block(&b, &pre, 100).unwrap();
    let c = p.txs[1].created.as_ref().unwrap();
    assert_eq!((c.iid, c.kind, c.dest), (100, 1, Dest::Output { vout: 0, off: 0 }));
    assert_eq!(c.inscription_id, format!("{}i0", hex::encode(T(2))));
    assert_eq!(p.txs[1].moves, vec![(7, Dest::Output { vout: 1, off: 2 })]);
}

#[test]
fn multi_tx_envelope_in_one_block_and_across_blocks() {
    let first = h("036f7264520a746578742f706c61696e51026162");
    let second = h("00026364");
    let b = vec![coinbase(0, &[1]), tx(1, &[(T(9), 0, first.clone())], &[1000]), tx(2, &[(T(1), 0, second.clone())], &[1000])];
    let p = plan_block(&b, &Prefetch::default(), 50).unwrap();
    assert!(p.txs[1].created.is_none());
    let c = p.txs[2].created.as_ref().unwrap();
    assert_eq!(c.inscription_id, format!("{}i0", hex::encode(T(1))));
    assert_eq!(c.body, b"abcd");
    let mut pre = Prefetch::default();
    pre.partial.insert(T(1), PartialRow { txids: vec![T(1)], scripts: vec![first] });
    let p2 = plan_block(&[coinbase(0, &[1]), tx(2, &[(T(1), 0, second)], &[1000])], &pre, 50).unwrap();
    assert!(p2.txs[1].created.is_some());
    assert_eq!(p2.partial_deleted.len(), 1);
}

#[test]
fn fee_inscription_goes_to_the_coinbase() {
    let s = SUBSIDY_KOINU;
    let mut pre = Prefetch::default();
    pre.loc.insert((T(5), 0), vec![LocRow { off: 60, iid: 3, val: 100 }]);
    let b = vec![coinbase(0, &[s + 20, 100]), tx(1, &[(T(6), 0, vec![])], &[90]), tx(2, &[(T(5), 0, vec![])], &[50])];
    match plan_block(&b, &pre, 1) {
        Err(PlanError::NeedValues(v)) => assert_eq!(v, vec![(T(6), 0)]),
        r => panic!("expected NeedValues, got {r:?}"),
    }
    pre.values.insert((T(6), 0), 100);
    let p = plan_block(&b, &pre, 1).unwrap();
    assert_eq!(p.txs[2].moves, vec![(3, Dest::Fee { off: 10 })]);
    assert_eq!(p.txs[0].moves, vec![(3, Dest::Output { vout: 1, off: 50 })]);
    let mut b2 = b.clone();
    b2[0] = coinbase(0, &[s]);
    assert_eq!(plan_block(&b2, &pre, 1).unwrap().txs[0].moves, vec![(3, Dest::Lost { off: 70 })]);
}

fn dest_of(dvout: i64, off: u64) -> Dest {
    match dvout {
        -1 => Dest::Fee { off },
        -2 => Dest::Lost { off },
        v => Dest::Output { vout: v as u32, off },
    }
}

#[test]
fn mainnet_reveal_vectors() {
    let raw = std::fs::read_to_string(vectors().join("inscriptions/mainnet_reveals.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&raw).unwrap();
    assert_eq!(cases.len(), 12);
    let (mut refused, mut created) = (0, 0);
    for c in &cases {
        let name = c["id"].as_str().unwrap();
        let reveal = decode_tx_hex(c["raw_hex"].as_str().unwrap()).unwrap();
        assert_eq!(txid_hex(&reveal.txid), c["txid"].as_str().unwrap(), "{name}");
        let mut pre = Prefetch::default();
        for (i, x) in c["inputs"].as_array().unwrap().iter().enumerate() {
            if let Some(v) = x["value"].as_str() {
                pre.values.insert((reveal.ins[i].prev, reveal.ins[i].vout), v.parse().unwrap());
            }
        }
        let mut expect: Vec<(u64, Dest)> = Vec::new();
        let mut next_syn = 1_000_000u64;
        for s in c["held"].as_array().unwrap() {
            let vin = s["vin"].as_u64().unwrap() as usize;
            let lo: u64 = s["lo"].as_str().unwrap().parse().unwrap();
            let hi: u64 = s["hi"].as_str().unwrap().parse().unwrap();
            let n = s["n"].as_u64().unwrap();
            let delta: i64 = s["delta"].as_str().unwrap().parse().unwrap();
            let op = (reveal.ins[vin].prev, reveal.ins[vin].vout);
            let val = pre.values[&op];
            let mut offs = vec![lo];
            if n >= 2 {
                offs.push(hi - 1);
            }
            while (offs.len() as u64) < n {
                offs.push(lo);
            }
            for off in offs {
                pre.loc.entry(op).or_default().push(LocRow { off, iid: next_syn, val });
                expect.push((next_syn, dest_of(s["dvout"].as_i64().unwrap(), (off as i64 + delta) as u64)));
                next_syn += 1;
            }
        }
        if let Some(p) = c["parent"].as_object() {
            let txids: Vec<Txid> = p["txids"].as_array().unwrap().iter().map(|t| parse_txid(t.as_str().unwrap()).unwrap()).collect();
            let scripts: Vec<Vec<u8>> = p["scripts"].as_array().unwrap().iter().map(|s| h(s.as_str().unwrap())).collect();
            pre.partial.insert(*txids.last().unwrap(), PartialRow { txids, scripts });
        }
        let plan = plan_block(&[coinbase(0, &[SUBSIDY_KOINU]), reveal.clone()], &pre, 1).unwrap();
        let v = &plan.txs[1];
        let mut got = v.moves.clone();
        got.sort_by_key(|m| m.0);
        expect.sort_by_key(|m| m.0);
        assert_eq!(got, expect, "{name}: held inscriptions move as the rules say");
        let st = &c["expect"];
        if st["created"].as_bool().unwrap() {
            created += 1;
            let k = v.created.as_ref().unwrap_or_else(|| panic!("{name}: should create"));
            assert_eq!(k.inscription_id, st["id"].as_str().unwrap(), "{name}");
            assert_eq!(k.dest, dest_of(st["vout"].as_i64().unwrap(), st["off"].as_str().unwrap().parse().unwrap()), "{name}");
            assert_eq!(k.kind as i64, st["kind"].as_i64().unwrap(), "{name}");
            assert_eq!(String::from_utf8_lossy(&k.content_type), st["ctype"].as_str().unwrap(), "{name}");
            if let Some(b) = st["body_hex"].as_str() {
                assert_eq!(hex::encode(&k.body), b, "{name}");
            }
            if let Some(a) = st["addr"].as_str() {
                let Dest::Output { vout, .. } = k.dest else { panic!("{name}") };
                assert_eq!(owner_of_script(&reveal.outs[vout as usize].spk).key(), a, "{name}: owner");
            }
        } else {
            assert!(v.created.is_none(), "{name}: must not create");
            if name.contains("already_inscribed") {
                assert!(v.gate_closed, "{name}");
                refused += 1;
            }
        }
    }
    assert_eq!((refused, created), (4, 6));
}

#[test]
fn decode_tx_checks_layout_and_hash() {
    let raw = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff0603e355620101ffffffff01c0ecc3c2e90000001976a914f5ed609ea6c6bacc5314e9a1a4eabd45457849d088ac00000000";
    let t = decode_tx_hex(raw).unwrap();
    assert!(t.is_coinbase());
    let mut want = sha256d(&h(raw));
    want.reverse();
    assert_eq!(t.txid, want);
    assert!(decode_tx_hex(&(raw.to_string() + "00")).is_err());
    assert!(decode_tx_hex(&raw[..raw.len() - 2]).is_err());
}

#[test]
fn merge_mined_block_6444515_decodes_and_hashes_to_its_merkle_root() {
    let raw = std::fs::read_to_string(vectors().join("inscriptions/block_6444515.hex")).unwrap();
    let b = decode_block_hex(&raw).unwrap();
    assert_eq!(b.txs.len(), 4);
    assert!(b.txs[0].is_coinbase());
    assert_eq!(hex::encode(&b.txs[0].outs[0].spk), "76a914f5ed609ea6c6bacc5314e9a1a4eabd45457849d088ac");
    // one flipped byte in a transaction breaks the merkle check
    let mut bad = h(raw.trim());
    let l = bad.len();
    bad[l - 10] ^= 1;
    assert!(decode_block(&bad).is_err());
}

// ------------------------------------------------------------------ 03 DRC-20

#[test]
fn tick_rules() {
    assert!(tick_ok("dogi") && tick_ok("DU$T") && tick_ok("\u{1D54F}"));
    assert!(tick_ok(" abc") && tick_ok("abc "));
    assert_ne!(tick_key(" abc"), tick_key("abc "));
    assert_ne!(tick_key("DJT "), tick_key(" DJT"));
    assert!(!tick_ok("dogis") && !tick_ok("dog") && !tick_ok("ab\0c"));
    assert_eq!(tick_key("DoGi"), "dogi");
    assert_eq!(tick_key("DU$T"), "du$t");
    assert_ne!(tick_key("\u{1D54F}"), tick_key("\u{1D47F}"));
    assert_eq!(tick_key("\u{FF38}A"), "\u{FF58}a");
    assert_ne!(tick_key("d0ge"), "doge");
}

#[test]
fn number_rules() {
    assert_eq!(amount("1").unwrap().value, SCALE);
    assert_eq!(amount("007").unwrap().value, 7 * SCALE);
    assert_eq!(amount("0.5").unwrap().frac_digits, 1);
    for bad in ["1.", ".5", " 1", "1 ", "+1", "-1", "1e8", "0", "0.000", "１", "1_0", "18446744073709551616", "18446744073709551615.1", "1.0000000000000000001"]
    {
        assert!(amount(bad).is_none(), "{bad:?}");
    }
    assert!(amount("18446744073709551615").is_some());
    assert!(amount("18446744073709551615.000").is_some());
    assert_eq!(fmt_amount(amount("12.3400").unwrap().value), "12.34");
}

#[test]
fn op_rules() {
    let p = |s: &str| parse_op(s.as_bytes());
    assert_eq!(p(r#"{"p":"drc-20","op":"mint","tick":"dogi","amt":"5"}"#), Some(Op::Mint { tick: "dogi".into(), amt: "5".into() }));
    for bad in [
        r#"{"p":"DRC-20","op":"mint","tick":"dogi","amt":"5"}"#,
        r#"{"p":"drc-20","op":"MINT","tick":"dogi","amt":"5"}"#,
        r#"{"p":"drc-20","op":"mint","tick":"dogi","amt":5}"#,
        r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":21000000}"#,
        r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"1","dec":8}"#,
        r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"1","dec":"19"}"#,
        r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"1.5","dec":"0"}"#,
    ] {
        assert_eq!(p(bad), None, "{bad}");
    }
    assert_eq!(p(r#"{"p":"drc-20","op":"mint","tick":" dog","amt":"5"}"#), Some(Op::Mint { tick: " dog".into(), amt: "5".into() }));
    // `decimals` is ignored, `dec` absent = 18, lim absent = max
    assert_eq!(
        p(r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"21","decimals":"2"}"#),
        Some(Op::Deploy { tick: "dogi".into(), max: 21 * SCALE, lim: 21 * SCALE, dec: 18 })
    );
    assert_eq!(p(r#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"21","dec":"08"}"#).map(|o| matches!(o, Op::Deploy { dec: 8, .. })), Some(true));
    // extra fields never decide the op
    assert!(p(r#"{"p":"drc-20","op":"transfer","tick":"dogi","amt":"5","to":"Dxxx","ds":{"action":"BUY"}}"#).is_some());
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

#[test]
fn drc20_json_body_vectors() {
    let raw = std::fs::read_to_string(vectors().join("drc20/json_bodies.json")).unwrap();
    let cases: Vec<Value> = serde_json::from_str(&raw).unwrap();
    assert_eq!(cases.len(), 86);
    let (mut ops, mut deep) = (0, 0);
    for c in &cases {
        let name = c["id"].as_str().unwrap();
        let b = h(c["body_hex"].as_str().unwrap());
        assert_eq!(ds_rules::drc20::drc20_like(&b), c["drc20_like"].as_bool().unwrap(), "{name}: drc20-like");
        let got = parse_op(&b);
        if c.get("ledger_throws").is_some() {
            deep += 1;
            assert!(got.is_some(), "{name}: deeply nested body still reads");
            continue;
        }
        let want = &c["ledger_op"];
        match got {
            None => assert!(want.is_null(), "{name}: should be {want}"),
            Some(op) => {
                ops += 1;
                match op {
                    Op::Mint { tick, amt } => assert_eq!(
                        (want["op"].as_str(), want["tick"].as_str(), want["amt"].as_str()),
                        (Some("mint"), Some(tick.as_str()), Some(amt.as_str())),
                        "{name}"
                    ),
                    Op::Transfer { tick, amt } => assert_eq!(
                        (want["op"].as_str(), want["tick"].as_str(), want["amt"].as_str()),
                        (Some("transfer"), Some(tick.as_str()), Some(amt.as_str())),
                        "{name}"
                    ),
                    Op::Deploy { tick, max, lim, dec } => {
                        assert_eq!(want["op"].as_str(), Some("deploy"), "{name}");
                        assert_eq!(want["tick"].as_str(), Some(tick.as_str()), "{name}");
                        assert_eq!(want["max"].as_str().map(canon), Some(fmt_amount(max)), "{name}");
                        assert_eq!(want["lim"].as_str().map(canon), Some(fmt_amount(lim)), "{name}");
                        assert_eq!(want["dec"].as_i64(), Some(dec as i64), "{name}");
                    }
                }
            }
        }
    }
    assert_eq!((ops, deep), (32, 1));
}

#[test]
fn json_reader_unit_cases() {
    use ds_rules::json::{parse, Member};
    let p = parse(r#"{"a":"x","a":1,"b":"\ud83d\udc15"}"#).unwrap();
    assert_eq!(p.get("a"), Some(&Member::Other));
    assert!(!p.has_lone);
    assert!(parse(r#"{"x":["\udc00"]}"#).unwrap().has_lone);
    assert!(parse("[1e400,-1e400]").is_some());
    assert!(parse(&("[".repeat(200_000) + &"]".repeat(200_000))).is_some());
    for bad in ["", "{", "{\"a\"}", "[1,]", "01", "1.", "\u{feff}{}", "NaN", "'a'"] {
        assert!(parse(bad).is_none(), "{bad:?}");
    }
}

#[test]
fn owner_keys_come_from_script_bytes() {
    // P2PKH
    let o = owner_of_script(&h("76a914f5ed609ea6c6bacc5314e9a1a4eabd45457849d088ac"));
    assert!(matches!(&o, Owner::Address(a) if a.starts_with('D')));
    // P2SH
    assert!(matches!(owner_of_script(&h("a914000102030405060708090a0b0c0d0e0f1011121387")), Owner::Address(a) if a.starts_with('9') || a.starts_with('A')));
    // P2PK compressed -> P2PKH address of the key
    let pk = "0321802b1bbff4781a29049a8fa84e71ed1e553ba16c8c196c0b0149c3f283a988";
    let p2pk = owner_of_script(&h(&format!("21{pk}ac")));
    let p2pkh = owner_of_script(&h(&format!("76a914{}88ac", hex::encode(ds_rules::owner::hash160(&h(pk))))));
    assert_eq!(p2pk, p2pkh);
    // any OP_RETURN start is nulldata; others are script keys
    for s in ["6a", "6a0474657374", "6a6a"] {
        assert_eq!(owner_of_script(&h(s)), Owner::Nulldata);
    }
    assert_eq!(owner_of_script(&h("516a")).key(), "script:516a");
    let long = vec![0x51u8; 70];
    assert_eq!(owner_of_script(&long).key(), format!("script:{}..", hex::encode(&long[..32])));
}

// ------------------------------------------------------------------ the ledger

fn ins<'a>(h: u64, iid: u64, owner: Option<&str>, body: &'a [u8], txid: &'a str) -> NewInscription<'a> {
    NewInscription { height: h, txidx: 1, reveal_txid: txid, iid, inscription_id: txid, landing: owner.map(|o| (0, o.to_string())), body }
}

#[test]
fn ledger_deploy_mint_transfer_and_rule_11() {
    let mut s = MemStore::default();
    let dep = br#"{"p":"drc-20","op":"deploy","tick":"DOGI","max":"100","lim":"40"}"#;
    on_inscription(&mut s, &ins(1, 1, Some("A"), dep, "t1"));
    // second deploy of the same key (any case) is skipped
    on_inscription(&mut s, &ins(2, 2, Some("B"), br#"{"p":"drc-20","op":"deploy","tick":"dogi","max":"5"}"#, "t2"));
    assert_eq!(s.tokens["dogi"].max, 100 * SCALE);
    // mints: over lim rejected, partial last mint, then nothing
    let m = |a: &str| String::from(r#"{"p":"drc-20","op":"mint","tick":"dogi","amt":""#) + a + r#""}"#;
    on_inscription(&mut s, &ins(3, 3, Some("A"), m("41").as_bytes(), "t3"));
    on_inscription(&mut s, &ins(3, 4, Some("A"), m("40").as_bytes(), "t4"));
    on_inscription(&mut s, &ins(3, 5, Some("A"), m("40").as_bytes(), "t5"));
    on_inscription(&mut s, &ins(3, 6, Some("B"), m("40").as_bytes(), "t6"));
    on_inscription(&mut s, &ins(3, 7, Some("B"), m("1").as_bytes(), "t7"));
    // mint spent as a fee at inscription time: ignored
    on_inscription(&mut s, &ins(3, 8, None, m("1").as_bytes(), "t8"));
    assert_eq!(s.tokens["dogi"].minted, 100 * SCALE);
    assert_eq!(s.balance_of("dogi", "A"), (80 * SCALE, 0));
    assert_eq!(s.balance_of("dogi", "B"), (20 * SCALE, 0));
    let st: Vec<&str> = s.events.iter().map(|e| e.status.as_str()).collect();
    assert_eq!(st, ["ok", "ticker_exists", "over_lim", "ok", "ok", "partial_remainder", "max_reached", "inscribed_into_fee"]);
    // transfer: inscribe locks, send moves
    let t = |a: &str| String::from(r#"{"p":"drc-20","op":"transfer","tick":"dogi","amt":""#) + a + r#""}"#;
    assert!(on_inscription(&mut s, &ins(4, 10, Some("A"), t("30").as_bytes(), "x10")));
    assert!(!on_inscription(&mut s, &ins(4, 11, Some("A"), t("51").as_bytes(), "x11")), "insufficient available balance");
    assert_eq!(s.balance_of("dogi", "A"), (50 * SCALE, 30 * SCALE));
    assert_eq!(s.holder_count("dogi"), 2);
    assert!(on_move(&mut s, 5, 1, "s1", 10, &SendTo::Owner("C".into())));
    assert!(!on_move(&mut s, 6, 1, "s2", 10, &SendTo::Owner("D".into())), "each transfer inscription is used once");
    assert_eq!(s.balance_of("dogi", "C"), (30 * SCALE, 0));
    // rule 11: spent as a fee in the send step -> back to the sender
    assert!(on_inscription(&mut s, &ins(7, 12, Some("A"), t("10").as_bytes(), "x12")));
    assert!(on_move(&mut s, 7, 2, "s3", 12, &SendTo::Fee));
    assert_eq!(s.balance_of("dogi", "A"), (50 * SCALE, 0));
    // rule 11: spent as a fee at inscription time -> ignored, nothing locked
    assert!(!on_inscription(&mut s, &ins(8, 13, None, t("10").as_bytes(), "x13")));
    assert_eq!(s.events.last().unwrap().status, "inscribed_into_fee_ignored");
    // a send into OP_RETURN burns
    assert!(on_inscription(&mut s, &ins(9, 14, Some("A"), t("5").as_bytes(), "x14")));
    assert!(on_move(&mut s, 9, 1, "s4", 14, &SendTo::Owner("nulldata".into())));
    assert_eq!(s.balance_of("dogi", "A"), (45 * SCALE, 0));
    assert_eq!(s.events.last().unwrap().status, "burned_nulldata from A insc x14");
    // the native coin's ticker is never listed
    on_inscription(&mut s, &ins(9, 15, Some("A"), br#"{"p":"drc-20","op":"deploy","tick":"DOGE","max":"5"}"#, "x15"));
    assert!(!s.tokens.contains_key("doge"));
}

trait BalOf {
    fn balance_of(&self, t: &str, o: &str) -> (u128, u128);
}
impl BalOf for MemStore {
    fn balance_of(&self, t: &str, o: &str) -> (u128, u128) {
        self.balances.get(&(t.to_string(), o.to_string())).map(|b| (b.avail, b.trans)).unwrap_or((0, 0))
    }
}

// ------------------------------------------------------------------ names

#[test]
fn name_claims() {
    assert_eq!(reg_claim(b"soft.doge").unwrap().name, "soft.doge");
    assert_eq!(reg_claim(b"  Soft.DOGE \n").unwrap().name, "soft.doge");
    assert_eq!(reg_claim(br#"{"p":"dns","op":"reg","name":"DogeSoft.doge"}"#).unwrap().name, "dogesoft.doge");
    assert_eq!(reg_claim(br#"{"p":"dns","op":"register","name":"wow.soft","registrar":"dogesoft.io"}"#).unwrap().name, "wow.soft");
    assert_eq!(reg_claim(b"wow.xd").unwrap().ending, XD);
    for bad in [
        &b"1234.dogemap"[..],
        b"a.b.doge",
        b"nodot",
        b".doge",
        b"-x.doge",
        b"x-.doge",
        b"x.123",
        b"two words.doge",
        br#"{"p":"dns","op":"burn","name":"a.doge"}"#,
        br#"{"p":"drc-20","op":"mint","tick":"dogi","amt":"1"}"#,
        br#"{"p":"dns","op":"reg","name":5}"#,
    ] {
        assert!(reg_claim(bad).is_none(), "{:?}", String::from_utf8_lossy(bad));
    }
    // a JSON claim's name is cut at the first space, like the registrar index
    assert_eq!(normalize("  DogeSoft.DOGE \n junk").unwrap().name, "dogesoft.doge");
    assert_eq!(normalize("Wow.XD").unwrap().name, format!("wow.{XD}"));
}

#[test]
fn published_vectors_through_the_shared_runner() {
    use ds_rules::vectors;
    let d = vectors();
    assert_eq!(vectors::envelope(&d).unwrap(), 4000);
    assert_eq!(vectors::drc20_bodies(&d).unwrap(), 86);
    assert!(vectors::koinu(&d).unwrap() > 10_000);
    assert_eq!(vectors::mainnet_reveals(&d).unwrap().len(), 12);
    let (n, claims) = vectors::names(&d).unwrap();
    assert_eq!((n, claims.len()), (20, 4));
}

#[test]
fn smart_quotes_and_other_non_ascii_where_a_key_belongs_are_not_json() {
    use ds_rules::json::parse;
    // seen on mainnet: a curly quote opening the first key
    assert!(parse("{\u{201c}p\u{201d}:\"drc-20\"}").is_none());
    assert!(parse("{\"p\":\"drc-20\",\u{201c}op\u{201d}:\"mint\"}").is_none());
    assert!(parse("{\u{00e9}}").is_none());
    assert_eq!(parse_op("{\u{201c}p\u{201d}:\u{201c}drc-20\u{201d},\u{201c}op\u{201d}:\u{201c}mint\u{201d}}".as_bytes()), None);
}
