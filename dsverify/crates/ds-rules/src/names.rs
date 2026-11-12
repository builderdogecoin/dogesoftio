// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! .doge names: `p:dns` claims. The first valid `reg` claim for a name wins (block height, then position
//! in the block); ownership follows the claim inscription. `.dogemap` tiles are not names.
//!
//! A `reg` claim is either
//! * plain text that is exactly one name: `label.ending` (label: 1-63 of `a-z 0-9 -`, no hyphen at
//!   either end; ending: `xd` or its letter forms, or 1-24 of `a-z 0-9 -`, not all digits, no hyphen
//!   at either end), surrounding whitespace allowed, any letter case; or
//! * JSON `{"p":"dns","op":"reg","name":"label.ending"}` (`"op":"register"` means the same).

use crate::json;

/// The `.xd` ending in its real letters (double-struck X, capital eth).
pub const XD: &str = "\u{1D54F}\u{00D0}";
const XD_LOW: &str = "\u{1D54F}\u{00F0}";
/// Endings that are not names.
pub const NON_NAME_ENDINGS: &[&str] = &["dogemap"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    pub name: String,
    pub label: String,
    pub ending: String,
}

fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{A0}' | '\u{1680}' | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}'
    ) || ('\u{2000}'..='\u{200A}').contains(&c)
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

/// Text that was UTF-8 read as Latin-1 (`.xd` letters garbled): put the real characters back.
pub fn repair_mojibake(raw: &str) -> String {
    if !raw.chars().all(|c| (c as u32) <= 255) || !(raw.contains('\u{F0}') || raw.contains('\u{C3}')) {
        return raw.to_string();
    }
    let bytes: Vec<u8> = raw.chars().map(|c| c as u32 as u8).collect();
    let utf = String::from_utf8_lossy(&bytes).into_owned();
    if utf.contains(XD) && !utf.contains('\u{FFFD}') {
        utf
    } else {
        raw.to_string()
    }
}

pub fn canon_ending(e: &str) -> String {
    if e.eq_ignore_ascii_case("xd") || e == XD || e == XD_LOW {
        return XD.to_string();
    }
    let c: Vec<char> = e.chars().collect();
    if c.len() == 2 && (c[0] == '\u{1D54F}' || c[0] == '\u{1D569}') && (c[1] == '\u{D0}' || c[1] == '\u{F0}') {
        return XD.to_string();
    }
    e.to_lowercase()
}

/// Name normalization: trim, cut at the first whitespace, exactly one dot with text on both sides,
/// lowercase label, canonical ending.
pub fn normalize(raw: &str) -> Option<Name> {
    let r = repair_mojibake(raw);
    let mut s = js_trim(&r);
    if let Some(p) = s.find(is_js_space) {
        s = &s[..p];
    }
    if s.is_empty() {
        return None;
    }
    let dot = s.find('.')?;
    if dot == 0 || s[dot + 1..].contains('.') {
        return None;
    }
    let label = s[..dot].to_lowercase();
    let ending = canon_ending(&s[dot + 1..]);
    if label.is_empty() || ending.is_empty() {
        return None;
    }
    Some(Name { name: format!("{label}.{ending}"), label, ending })
}

fn label_ok(l: &str) -> bool {
    let b = l.as_bytes();
    !b.is_empty() && b.len() <= 63 && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-') && b[0] != b'-' && b[b.len() - 1] != b'-'
}

fn ending_ok(e: &str) -> bool {
    if e.eq_ignore_ascii_case("xd") {
        return true;
    }
    let c: Vec<char> = e.chars().collect();
    if c.len() == 2 && c[0] == '\u{1D54F}' && (c[1] == '\u{D0}' || c[1] == '\u{F0}') {
        return true;
    }
    let b = e.as_bytes();
    !b.is_empty()
        && b.len() <= 24
        && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
        && !b.iter().all(|c| c.is_ascii_digit())
}

/// A `reg` claim in an inscription body, if it is one.
pub fn reg_claim(body: &[u8]) -> Option<Name> {
    let text = repair_mojibake(&String::from_utf8_lossy(body));
    // plain text: exactly one name
    let t = js_trim(&text);
    if let Some((l, e)) = t.split_once('.') {
        if label_ok(l) && ending_ok(e) {
            let n = normalize(t)?;
            return (!NON_NAME_ENDINGS.contains(&n.ending.as_str())).then_some(n);
        }
    }
    // JSON
    let p = json::parse(&text)?;
    let get = |k: &str| match p.get(k) {
        Some(json::Member::Str(u)) => String::from_utf16(u).ok(),
        _ => None,
    };
    if get("p").as_deref() != Some("dns") {
        return None;
    }
    let op = get("op")?;
    if op != "reg" && op != "register" {
        return None;
    }
    normalize(&get("name")?)
}
