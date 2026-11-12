// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! JSON exactly as the Doge Soft ledger reads DRC-20 bodies (JavaScript `JSON.parse` semantics).
//!
//! Numbers are only checked for syntax (`1e400` parses), there is no nesting limit (iterative, so deep
//! nesting cannot overflow the stack), duplicate keys keep the last value at every depth, and strings are
//! UTF-16 so a lone surrogate (`"\\ud800"`) parses. `has_lone` reports a lone surrogate anywhere in the
//! final value (keys and values, any depth; an overwritten duplicate does not count).

use std::collections::HashMap;

/// One top-level member value: a string (UTF-16 units) or anything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Member {
    Str(Vec<u16>),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    /// Top-level object members with their last value; None when the value is not an object.
    pub members: Option<HashMap<Vec<u16>, Member>>,
    pub has_lone: bool,
}

impl Parsed {
    pub fn get(&self, key: &str) -> Option<&Member> {
        let k: Vec<u16> = key.encode_utf16().collect();
        self.members.as_ref()?.get(&k)
    }
}

pub fn units_have_lone(u: &[u16]) -> bool {
    let mut i = 0;
    while i < u.len() {
        let c = u[i];
        if (0xD800..=0xDBFF).contains(&c) {
            if i + 1 < u.len() && (0xDC00..=0xDFFF).contains(&u[i + 1]) {
                i += 2;
                continue;
            }
            return true;
        }
        if (0xDC00..=0xDFFF).contains(&c) {
            return true;
        }
        i += 1;
    }
    false
}

enum Frame {
    // key -> lone flag of (key || its last value); `cur` is the key being filled
    Obj { keys: HashMap<Vec<u16>, bool>, cur: Vec<u16>, cur_lone: bool },
    Arr { lone: bool },
}

fn ws(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

fn hex4(b: &[u8], i: usize) -> Option<u16> {
    let s = b.get(i..i + 4)?;
    let mut v: u16 = 0;
    for &c in s {
        let d = (c as char).to_digit(16)? as u16;
        v = v * 16 + d;
    }
    Some(v)
}

/// `"`-started string at `i` -> (units, index after the closing quote)
fn string(s: &str, mut i: usize) -> Option<(Vec<u16>, usize)> {
    let b = s.as_bytes();
    i += 1;
    let mut u = Vec::new();
    loop {
        let c = *b.get(i)?;
        match c {
            b'"' => return Some((u, i + 1)),
            b'\\' => {
                let e = *b.get(i + 1)?;
                i += 2;
                match e {
                    b'"' => u.push(0x22),
                    b'\\' => u.push(0x5C),
                    b'/' => u.push(0x2F),
                    b'b' => u.push(0x08),
                    b'f' => u.push(0x0C),
                    b'n' => u.push(0x0A),
                    b'r' => u.push(0x0D),
                    b't' => u.push(0x09),
                    b'u' => {
                        u.push(hex4(b, i)?);
                        i += 4;
                    }
                    _ => return None,
                }
            }
            0x00..=0x1F => return None,
            0x20..=0x7F => {
                u.push(c as u16);
                i += 1;
            }
            _ => {
                let ch = s.get(i..)?.chars().next()?;
                let mut buf = [0u16; 2];
                u.extend_from_slice(ch.encode_utf16(&mut buf));
                i += ch.len_utf8();
            }
        }
    }
}

fn number(b: &[u8], mut i: usize) -> Option<usize> {
    if b.get(i) == Some(&b'-') {
        i += 1;
    }
    match b.get(i)? {
        b'0' => i += 1,
        b'1'..=b'9' => {
            while matches!(b.get(i), Some(b'0'..=b'9')) {
                i += 1;
            }
        }
        _ => return None,
    }
    if b.get(i) == Some(&b'.') {
        i += 1;
        if !matches!(b.get(i), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(b.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(b.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        if !matches!(b.get(i), Some(b'0'..=b'9')) {
            return None;
        }
        while matches!(b.get(i), Some(b'0'..=b'9')) {
            i += 1;
        }
    }
    Some(i)
}

/// `JSON.parse(s)`; None where it would throw.
pub fn parse(s: &str) -> Option<Parsed> {
    let b = s.as_bytes();
    let mut st: Vec<Frame> = Vec::new();
    let mut top: Option<HashMap<Vec<u16>, Member>> = None;
    let mut top_is_obj = false;
    let root_lone;
    let mut i = ws(b, 0);
    // value just finished: (member for the top object, lone flag of that value)
    'value: loop {
        let (mut member, mut lone): (Member, bool);
        i = ws(b, i);
        match *b.get(i)? {
            b'{' => {
                i = ws(b, i + 1);
                if st.is_empty() {
                    top_is_obj = true;
                    top = Some(HashMap::new());
                }
                if b.get(i) == Some(&b'}') {
                    i += 1;
                    member = Member::Other;
                    lone = false;
                } else {
                    if b.get(i) != Some(&b'"') {
                        return None;
                    }
                    let (k, j) = string(s, i)?;
                    i = ws(b, j);
                    if b.get(i) != Some(&b':') {
                        return None;
                    }
                    i += 1;
                    let kl = units_have_lone(&k);
                    st.push(Frame::Obj { keys: HashMap::new(), cur: k, cur_lone: kl });
                    continue 'value;
                }
            }
            b'[' => {
                i = ws(b, i + 1);
                if b.get(i) == Some(&b']') {
                    i += 1;
                    member = Member::Other;
                    lone = false;
                } else {
                    st.push(Frame::Arr { lone: false });
                    continue 'value;
                }
            }
            b'"' => {
                let (u, j) = string(s, i)?;
                i = j;
                lone = units_have_lone(&u);
                member = Member::Str(u);
            }
            b't' if b[i..].starts_with(b"true") => {
                i += 4;
                member = Member::Other;
                lone = false;
            }
            b'f' if b[i..].starts_with(b"false") => {
                i += 5;
                member = Member::Other;
                lone = false;
            }
            b'n' if b[i..].starts_with(b"null") => {
                i += 4;
                member = Member::Other;
                lone = false;
            }
            b'-' | b'0'..=b'9' => {
                i = number(b, i)?;
                member = Member::Other;
                lone = false;
            }
            _ => return None,
        }
        // attach the finished value to its container, closing containers as they end
        loop {
            let depth = st.len();
            match st.last_mut() {
                None => {
                    root_lone = lone;
                    i = ws(b, i);
                    if i != b.len() {
                        return None;
                    }
                    break 'value;
                }
                Some(Frame::Obj { keys, cur, cur_lone }) => {
                    let fl = *cur_lone || lone;
                    keys.insert(cur.clone(), fl);
                    if depth == 1 && top_is_obj {
                        top.as_mut().expect("top object").insert(cur.clone(), member.clone());
                    }
                    i = ws(b, i);
                    match b.get(i)? {
                        b',' => {
                            i = ws(b, i + 1);
                            if b.get(i) != Some(&b'"') {
                                return None;
                            }
                            let (k, j) = string(s, i)?;
                            i = ws(b, j);
                            if b.get(i) != Some(&b':') {
                                return None;
                            }
                            i += 1;
                            *cur_lone = units_have_lone(&k);
                            *cur = k;
                            continue 'value;
                        }
                        b'}' => {
                            i += 1;
                            let l = keys.values().any(|f| *f);
                            st.pop();
                            member = Member::Other;
                            lone = l;
                        }
                        _ => return None,
                    }
                }
                Some(Frame::Arr { lone: al }) => {
                    *al = *al || lone;
                    i = ws(b, i);
                    match b.get(i)? {
                        b',' => {
                            i += 1;
                            continue 'value;
                        }
                        b']' => {
                            i += 1;
                            let l = *al;
                            st.pop();
                            member = Member::Other;
                            lone = l;
                        }
                        _ => return None,
                    }
                }
            }
        }
    }
    Some(Parsed { members: if top_is_obj { top } else { None }, has_lone: root_lone })
}

/// The top-level object as a flat serde Value (strings kept, every other value null), or None when
/// JSON.parse would throw, the value is not an object, or the ledger's `hasLone` rejects it.
pub fn ledger_object(s: &str) -> Option<serde_json::Value> {
    let p = parse(s)?;
    if p.has_lone {
        return None;
    }
    let mut m = serde_json::Map::new();
    for (k, v) in p.members? {
        let key = String::from_utf16(&k).ok()?;
        let val = match v {
            Member::Str(u) => serde_json::Value::String(String::from_utf16(&u).ok()?),
            Member::Other => serde_json::Value::Null,
        };
        m.insert(key, val);
    }
    Some(serde_json::Value::Object(m))
}

/// JS `String.prototype.trim` whitespace (WhiteSpace + LineTerminator), as UTF-16 units.
pub fn js_trim(u: &[u16]) -> &[u16] {
    let ws = |c: u16| {
        matches!(c, 0x09 | 0x0A | 0x0B | 0x0C | 0x0D | 0x20 | 0xA0 | 0x1680 | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000 | 0xFEFF)
            || (0x2000..=0x200A).contains(&c)
    };
    let mut a = 0;
    let mut z = u.len();
    while a < z && ws(u[a]) {
        a += 1;
    }
    while z > a && ws(u[z - 1]) {
        z -= 1;
    }
    &u[a..z]
}
