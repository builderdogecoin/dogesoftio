// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! DRC-20 operation rules: rule 03 of the Doge Soft rules (`rules/spec/03-drc20.md`).
//!
//! A DRC-20 operation is decided only by the inscription's JSON body. Envelope tag pushes and extra JSON
//! fields never decide or invalidate an operation, and there is no height cutoff.

use crate::json;

/// All amounts are scaled integers: value x 10^18.
pub const SCALE: u128 = 1_000_000_000_000_000_000;
/// Largest whole value allowed: uint64 max.
pub const U64_MAX: u128 = u64::MAX as u128;
/// The only blocked ticker key (the native coin).
pub const RESERVED_TICK: &str = "doge";

/// A parsed DRC-20 operation, before it is checked against the ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Deploy { tick: String, max: u128, lim: u128, dec: u32 },
    Mint { tick: String, amt: String },
    Transfer { tick: String, amt: String },
}

impl Op {
    pub fn tick(&self) -> &str {
        match self {
            Op::Deploy { tick, .. } | Op::Mint { tick, .. } | Op::Transfer { tick, .. } => tick,
        }
    }
}

/// A valid amount: its scaled value and how many fraction digits were inscribed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amount {
    pub value: u128,
    pub frac_digits: usize,
}

/// Rule 5 + strict number format: ASCII digits with at most one `.` that has digits on both sides, at
/// most 18 fraction digits, at most uint64 max, and greater than zero. Leading zeros are fine.
pub fn amount(s: &str) -> Option<Amount> {
    let (ip, fp) = match s.split_once('.') {
        Some((i, f)) => (i, f),
        None => (s, ""),
    };
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    if !digits(ip) || (s.contains('.') && !digits(fp)) {
        return None;
    }
    if fp.len() > 18 {
        return None;
    }
    let it = ip.trim_start_matches('0');
    if it.len() > 20 {
        return None;
    }
    let iv: u128 = if it.is_empty() { 0 } else { it.parse().ok()? };
    if iv > U64_MAX || (iv == U64_MAX && fp.bytes().any(|b| b != b'0')) {
        return None;
    }
    let mut f = fp.to_string();
    while f.len() < 18 {
        f.push('0');
    }
    let fv: u128 = f.parse().ok()?;
    let v = iv * SCALE + fv;
    (v > 0).then_some(Amount { value: v, frac_digits: fp.len() })
}

/// Format a scaled value the way the ledger prints it (no trailing fraction zeros).
pub fn fmt_amount(v: u128) -> String {
    let i = v / SCALE;
    let f = v % SCALE;
    if f == 0 {
        return i.to_string();
    }
    let fs = format!("{f:018}");
    format!("{i}.{}", fs.trim_end_matches('0'))
}

/// Parse a ledger-printed decimal back to a scaled value.
pub fn parse_scaled(s: &str) -> Option<u128> {
    let (i, f) = s.split_once('.').unwrap_or((s, ""));
    if f.len() > 18 || i.is_empty() || !i.bytes().all(|b| b.is_ascii_digit()) || !f.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let mut fs = f.to_string();
    while fs.len() < 18 {
        fs.push('0');
    }
    i.parse::<u128>().ok()?.checked_mul(SCALE)?.checked_add(fs.parse::<u128>().ok()?)
}

/// Rule 2: the ticker key is the plain Unicode lowercase of the raw tick, one character at a time (no
/// trimming, no normalization, no look-alike folding).
pub fn tick_key(tick: &str) -> String {
    tick.chars().flat_map(|c| c.to_lowercase()).collect()
}

/// Rule 1: exactly 4 bytes of UTF-8 as inscribed, no NUL byte. Leading or trailing
/// whitespace is a real byte of the tick, not trimmed or rejected.
pub fn tick_ok(tick: &str) -> bool {
    tick.len() == 4 && !tick.contains('\0')
}

fn member_str(p: &json::Parsed, key: &str) -> Result<Option<String>, ()> {
    match p.get(key) {
        None => Ok(None),
        Some(json::Member::Str(u)) => String::from_utf16(u).map(Some).map_err(|_| ()),
        Some(json::Member::Other) => Err(()),
    }
}

/// Read an inscription body as a DRC-20 operation, exactly as the ledger does: strict UTF-8, whitespace
/// trimmed, `JSON.parse`, an object with `"p":"drc-20"` exactly and no lone surrogate anywhere, an exact
/// lowercase `op`, a string `tick` that passes [`tick_ok`], and string-only number fields.
pub fn parse_op(body: &[u8]) -> Option<Op> {
    let s = std::str::from_utf8(body).ok()?;
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let p = json::parse(s)?;
    p.members.as_ref()?;
    if member_str(&p, "p").ok()?.as_deref() != Some("drc-20") {
        return None;
    }
    if p.has_lone {
        return None;
    }
    let op = member_str(&p, "op").ok().flatten().unwrap_or_default();
    let tick = member_str(&p, "tick").ok()??;
    if !tick_ok(&tick) {
        return None;
    }
    match op.as_str() {
        "deploy" => {
            let dec = match member_str(&p, "dec").ok()? {
                None => 18,
                Some(d) => {
                    if d.is_empty() || !d.bytes().all(|b| b.is_ascii_digit()) {
                        return None;
                    }
                    let z = d.trim_start_matches('0');
                    let n: u32 = if z.is_empty() {
                        0
                    } else if z.len() > 9 {
                        99
                    } else {
                        z.parse().ok()?
                    };
                    if n > 18 {
                        return None;
                    }
                    n
                }
            };
            let mx = member_str(&p, "max").ok()??;
            let m = amount(&mx)?;
            if m.frac_digits > dec as usize {
                return None;
            }
            let l = match member_str(&p, "lim").ok()? {
                None => m,
                Some(ls) => amount(&ls)?,
            };
            if l.frac_digits > dec as usize {
                return None;
            }
            Some(Op::Deploy { tick, max: m.value, lim: l.value, dec })
        }
        "mint" => Some(Op::Mint { tick, amt: member_str(&p, "amt").ok()?? }),
        "transfer" => Some(Op::Transfer { tick, amt: member_str(&p, "amt").ok()?? }),
        _ => None,
    }
}

/// Is a body "DRC-20 like"? At most 4096 bytes, first non-blank byte `{`, and a JSON object whose `p`,
/// trimmed and lowercased, is `drc-20`. Bodies of other content types are kept only when this is true.
pub fn drc20_like(body: &[u8]) -> bool {
    if body.len() > 4096 {
        return false;
    }
    let i = body.iter().position(|c| !matches!(*c, 0x20 | 0x0a | 0x0d | 0x09)).unwrap_or(body.len());
    if body.get(i) != Some(&b'{') {
        return false;
    }
    let s = String::from_utf8_lossy(body);
    let Some(parsed) = json::parse(&s) else { return false };
    match parsed.get("p") {
        Some(json::Member::Str(p)) => String::from_utf16_lossy(json::js_trim(p)).to_lowercase() == "drc-20",
        _ => false,
    }
}

/// Content type starts with `text/plain` or `application/json` (trimmed, case-insensitive).
pub fn is_text_json(ct: &[u8]) -> bool {
    let ws = |c: &u8| matches!(*c, 0x09..=0x0d | 0x20 | 0xa0);
    let s = ct.iter().position(|c| !ws(c)).unwrap_or(ct.len());
    let e = ct.iter().rposition(|c| !ws(c)).map(|p| p + 1).unwrap_or(s);
    let t: Vec<u8> = ct[s..e.max(s)].iter().map(|c| c.to_ascii_lowercase()).collect();
    t.starts_with(b"text/plain") || t.starts_with(b"application/json")
}

/// 1 = text or JSON content type, 2 = another content type with a DRC-20-like body, 0 = anything else.
/// Only kind 1 and 2 bodies are read by DRC-20 and names.
pub fn inscription_kind(ct: &[u8], body: &[u8]) -> u8 {
    if is_text_json(ct) {
        1
    } else if drc20_like(body) {
        2
    } else {
        0
    }
}
