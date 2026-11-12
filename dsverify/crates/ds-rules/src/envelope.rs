// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Doginals envelopes: rule 01 of the Doge Soft rules (`rules/spec/01-inscriptions.md`).
//!
//! An inscription is read from the scriptSig of **input 0** of a transaction, as a flat list of data
//! pushes, and nothing else (the redeem script is never opened):
//!
//! ```text
//! "ord"  <piece count n, not 0>  <content type>  <n-1> <data>  <n-2> <data> ...  <0> <data>
//! ```
//!
//! A long inscription may continue in later transactions. Each later transaction spends the previous
//! one in its input 0, and its input-0 scriptSig carries the next countdown pieces. Until the countdown
//! reaches 0 the chain is *partial*; the transaction that pushes piece 0 *completes* it, and the
//! inscription id is the first transaction of the chain plus `i0`.
//!
//! Push decoding follows the historical Doginals reading of every inscription already on chain, quirks
//! included, because changing it would change which inscriptions exist:
//! * `OP_0` pushes an empty item and `OP_1`..`OP_16` push one byte (1..16);
//! * `OP_PUSHDATA2` reads its length as `256 * b1 + 77` and `OP_PUSHDATA4` as
//!   `2^24 * b3 + 2^16 * b2 + 2^8 * b1 + 78` (the opcode byte stands in for the lowest length byte);
//! * any other opcode makes the script unreadable;
//! * a number is little-endian over at most 8 bytes, and an empty push is 0.
//!
//! Doge Soft rule (locked 2026-11-01): 1 to 9 extra tag pushes between the content type and the first
//! countdown piece of the FIRST script (each 1..=80 bytes of printable ASCII and not the expected piece
//! number) never invalidate an inscription on their own; they are skipped and never decide anything.

/// The marker every envelope starts with.
pub const PROTOCOL_ID: &[u8] = b"ord";
/// Most extra tag pushes skipped in the first script.
pub const MAX_EXTRA_PUSHES: usize = 9;
/// Longest extra tag push skipped.
pub const MAX_EXTRA_PUSH_LEN: usize = 80;

/// A completed inscription.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inscription {
    pub content_type: Vec<u8>,
    pub body: Vec<u8>,
}

/// What a chain of input-0 scriptSigs reads as.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Parsed {
    /// Not an inscription (or not a valid continuation).
    None,
    /// A valid start whose countdown has not reached 0 yet.
    Partial,
    /// A complete inscription.
    Complete(Inscription),
}

/// One push: where it starts in the script and the bytes it pushes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Push {
    pub start: usize,
    pub data: Vec<u8>,
}

/// Read a script as data pushes (historical length rules above). `None` when any byte is not a push or
/// a push runs past the end.
pub fn read_pushes(script: &[u8]) -> Option<Vec<Push>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < script.len() {
        let op = script[i] as usize;
        let rest = script.len() - i;
        let (head, len) = match op {
            0x00 => {
                out.push(Push { start: i, data: Vec::new() });
                i += 1;
                continue;
            }
            0x51..=0x60 => {
                out.push(Push { start: i, data: vec![(op - 0x50) as u8] });
                i += 1;
                continue;
            }
            0x01..=0x4b => (1usize, op),
            0x4c => {
                if rest < 2 {
                    return None;
                }
                (2, script[i + 1] as usize)
            }
            0x4d => {
                if rest < 3 {
                    return None;
                }
                (3, ((script[i + 1] as usize) << 8) + op)
            }
            0x4e => {
                if rest < 5 {
                    return None;
                }
                let l = ((script[i + 3] as usize) << 24) + ((script[i + 2] as usize) << 16) + ((script[i + 1] as usize) << 8) + op;
                (5, l)
            }
            _ => return None,
        };
        let end = i.checked_add(head)?.checked_add(len)?;
        if end > script.len() {
            return None;
        }
        out.push(Push { start: i, data: script[i + head..end].to_vec() });
        i = end;
    }
    Some(out)
}

/// A push read as a little-endian number: empty is 0, more than 8 bytes is not a number.
pub fn push_number(p: &[u8]) -> Option<u64> {
    if p.len() > 8 {
        return None;
    }
    Some(p.iter().rev().fold(0u64, |n, b| (n << 8) | *b as u64))
}

fn datas(script: &[u8]) -> Option<Vec<Vec<u8>>> {
    read_pushes(script).map(|v| v.into_iter().map(|p| p.data).collect())
}

/// Strict envelope read of a chain of input-0 scriptSigs, oldest first (no extra-push rule).
pub fn parse_strict(scripts: &[Vec<u8>]) -> Parsed {
    let Some(first) = scripts.first() else { return Parsed::None };
    let Some(p) = datas(first) else { return Parsed::None };
    if p.len() < 3 || p[0] != PROTOCOL_ID {
        return Parsed::None;
    }
    let Some(mut left) = push_number(&p[1]) else { return Parsed::None };
    if left == 0 {
        return Parsed::None;
    }
    let content_type = p[2].clone();
    let mut body = Vec::new();
    let mut items: Vec<Vec<u8>> = p[3..].to_vec();
    let mut next_script = 1usize;
    loop {
        // take countdown pairs from this script while they line up
        let mut k = 0usize;
        while left > 0 && k + 1 < items.len() {
            match push_number(&items[k]) {
                Some(n) if n == left - 1 => {
                    body.extend_from_slice(&items[k + 1]);
                    left -= 1;
                    k += 2;
                }
                _ => break,
            }
        }
        if left == 0 {
            return Parsed::Complete(Inscription { content_type, body });
        }
        // continue in the next transaction of the chain
        let Some(s) = scripts.get(next_script) else { return Parsed::Partial };
        next_script += 1;
        let Some(q) = datas(s) else { return Parsed::None };
        if q.len() < 2 || push_number(&q[0]) != Some(left - 1) {
            return Parsed::None;
        }
        items = q;
    }
}

fn is_extra_push(p: &[u8], expected_piece: u64) -> bool {
    !p.is_empty() && p.len() <= MAX_EXTRA_PUSH_LEN && p.iter().all(|c| (0x20..=0x7e).contains(c)) && push_number(p) != Some(expected_piece)
}

/// `"ord" <n> <type> <1..=9 extra tags> <n-1> <data> ...` -> the same script without the tags.
/// `None` when the script does not have exactly that shape.
pub fn strip_extra_pushes(script: &[u8]) -> Option<Vec<u8>> {
    let pushes = read_pushes(script)?;
    if pushes.len() < 6 || pushes[0].data != PROTOCOL_ID {
        return None;
    }
    let n = push_number(&pushes[1].data)?;
    if n == 0 {
        return None;
    }
    let mut i = 3;
    while i < pushes.len() && i - 3 < MAX_EXTRA_PUSHES && is_extra_push(&pushes[i].data, n - 1) {
        i += 1;
    }
    if i == 3 || i + 1 >= pushes.len() || push_number(&pushes[i].data) != Some(n - 1) {
        return None;
    }
    let mut out = script[..pushes[3].start].to_vec();
    out.extend_from_slice(&script[pushes[i].start..]);
    Some(out)
}

/// The Doge Soft envelope rule: the strict read, or the strict read of the chain with the first script's
/// extra tag pushes removed when that reads as an inscription (complete or partial).
pub fn parse(scripts: &[Vec<u8>]) -> Parsed {
    if scripts.is_empty() {
        return Parsed::None;
    }
    let strict = parse_strict(scripts);
    if matches!(strict, Parsed::Complete(_)) {
        return strict;
    }
    if let Some(s0) = strip_extra_pushes(&scripts[0]) {
        let mut c = scripts.to_vec();
        c[0] = s0;
        let r = parse_strict(&c);
        if r != Parsed::None {
            return r;
        }
    }
    strict
}

/// Only a script whose first push is `ord` or a number-sized push (<= 8 bytes) can start or continue
/// an inscription.
pub fn can_be_inscription_part(script: &[u8]) -> bool {
    match read_pushes(script) {
        Some(p) if !p.is_empty() => p[0].data == PROTOCOL_ID || p[0].data.len() <= 8,
        _ => false,
    }
}

/// Read `script` as the next link of `parent` (the partial chain its input 0 spends, if any).
/// Returns the verdict and the chain it was read as.
pub fn parse_with_parent(parent: Option<Vec<Vec<u8>>>, script: Vec<u8>) -> (Parsed, Vec<Vec<u8>>) {
    let mut chain = parent.unwrap_or_default();
    chain.push(script);
    (parse(&chain), chain)
}

/// Given input-0 scriptSigs of a run of ancestors (oldest first, the one before the oldest known not to
/// be partial), the partial chain held for the newest one, if any.
pub fn partial_chain_of(ancestors_oldest_first: Vec<Vec<u8>>) -> Option<Vec<Vec<u8>>> {
    let mut partial: Option<Vec<Vec<u8>>> = None;
    for s in ancestors_oldest_first {
        let (r, chain) = parse_with_parent(partial.take(), s);
        partial = (r == Parsed::Partial).then_some(chain);
    }
    partial
}
