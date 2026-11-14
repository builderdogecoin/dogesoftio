// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Koinu tracking: rule 02 of the Doge Soft rules (`rules/spec/02-koinu-tracking.md`).
//!
//! Pure arithmetic, no node and no database. The caller supplies exact koinu values (never floats) and
//! the inscriptions each spent outpoint holds; this module says where every inscription goes and
//! whether a reveal may create a new inscription.
//!
//! * Inputs are lined up in input order and outputs are filled in output order (first in, first out).
//!   Zero-value outputs never hold an inscription.
//! * Order inside a block: every non-coinbase transaction in block order, then the coinbase.
//! * A koinu past the last output is spent as a fee. It joins the coinbase at offset
//!   `reward_before_tx + input-space offset`, where `reward_before_tx` is the block subsidy plus the
//!   fees of every earlier non-coinbase transaction in the block. If the coinbase outputs do not reach
//!   that offset, the inscription is lost. This follows ord-dogecoin, the reference Doginals indexer
//!   (`offset: self.reward + flotsam.offset`), on purpose: upstream ord (Bitcoin) later subtracts the
//!   spending transaction's output total (ordinals/ord#1971), and following it would move or un-lose
//!   fee-spent inscriptions that Doginals indexes already place.
//! * The gate: a reveal may create an inscription only if the first koinu of its inputs is not already
//!   inscribed (reinscription creates nothing).

/// Block subsidy from height 600,000 on: a fixed 10,000 DOGE (every block the replay reads).
pub const SUBSIDY_KOINU: u64 = 10_000 * 100_000_000;

/// One inscription sitting at koinu offset `off` of an outpoint (0 = its first koinu).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Held {
    pub off: u64,
    pub iid: u64,
}

/// One spent input: its exact koinu value and the inscriptions it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TxInput {
    pub value: u64,
    pub held: Vec<Held>,
}

/// Where an inscription ends up after a transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Dest {
    /// In output `vout`, at offset `off` inside that output.
    Output { vout: u32, off: u64 },
    /// Spent as a fee by a non-coinbase transaction (`off` = input-space offset minus output total).
    Fee { off: u64 },
    /// The coinbase could not hold it (`off` = coinbase-space offset minus coinbase output total).
    Lost { off: u64 },
}

/// One routed inscription.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub iid: u64,
    /// Created by this transaction.
    pub new: bool,
    /// Came from the block's fee pool (coinbase only).
    pub from_fee: bool,
    /// Offset in the transaction's input space (or coinbase space) before routing.
    pub src_off: u64,
    pub dest: Dest,
}

/// A fee-spent inscription waiting for the coinbase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Flotsam {
    pub iid: u64,
    /// Input-space offset in transaction `tx_index`.
    pub foff: u64,
    pub tx_index: usize,
    /// Offset in the coinbase's space.
    pub coff: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SatError {
    Overflow,
    TooManyOutputs,
}

impl std::fmt::Display for SatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SatError {}

#[derive(Clone, Copy, Debug)]
struct Item {
    off: u64,
    iid: u64,
    new: bool,
    from_fee: bool,
}

/// Step 1 of a transaction: input-space offsets of every inscription on the inputs (up to the last
/// inscribed input), and the gate (`true` = the first koinu is not inscribed, so a reveal may create one).
pub fn gather(inputs: &[TxInput]) -> Result<(Vec<(u64, u64)>, bool), SatError> {
    let mut out = Vec::new();
    let Some(last) = inputs.iter().rposition(|i| !i.held.is_empty()) else {
        return Ok((out, true));
    };
    let mut acc: u64 = 0;
    for inp in &inputs[..=last] {
        for h in &inp.held {
            out.push((acc.checked_add(h.off).ok_or(SatError::Overflow)?, h.iid));
        }
        acc = acc.checked_add(inp.value).ok_or(SatError::Overflow)?;
    }
    let gate_open = out.iter().all(|(o, _)| *o != 0);
    Ok((out, gate_open))
}

/// Steps 2 and 3: add the new inscription (offset 0) when the envelope completed one, add the block's
/// fee pool on the coinbase, then route first in, first out over the outputs.
pub fn route(gathered: &[(u64, u64)], new_iid: Option<u64>, coinbase_flotsam: &[Flotsam], is_coinbase: bool, outputs: &[u64]) -> Result<Vec<Placed>, SatError> {
    if outputs.len() > u32::MAX as usize {
        return Err(SatError::TooManyOutputs);
    }
    let mut items: Vec<Item> = gathered.iter().map(|&(off, iid)| Item { off, iid, new: false, from_fee: false }).collect();
    if let Some(iid) = new_iid {
        items.push(Item { off: 0, iid, new: true, from_fee: false });
    }
    if is_coinbase {
        for f in coinbase_flotsam {
            items.push(Item { off: f.coff, iid: f.iid, new: false, from_fee: true });
        }
    }
    // stable sort: equal offsets keep their order
    items.sort_by_key(|a| a.off);
    let mut ends = Vec::with_capacity(outputs.len());
    let mut s: u64 = 0;
    for v in outputs {
        s = s.checked_add(*v).ok_or(SatError::Overflow)?;
        ends.push(s);
    }
    let total_out = s;
    let n = outputs.len();
    let mut j = 0usize;
    let mut placed = Vec::with_capacity(items.len());
    for f in items {
        while j < n && f.off >= ends[j] {
            j += 1;
        }
        let dest = if j < n {
            let start = if j > 0 { ends[j - 1] } else { 0 };
            Dest::Output { vout: j as u32, off: f.off - start }
        } else if !is_coinbase {
            Dest::Fee { off: f.off - total_out }
        } else {
            Dest::Lost { off: f.off - total_out }
        };
        placed.push(Placed { iid: f.iid, new: f.new, from_fee: f.from_fee, src_off: f.off, dest });
    }
    Ok(placed)
}

/// Per-block reward bookkeeping for the fee pool.
#[derive(Debug, Clone)]
pub struct BlockRouter {
    reward: u64,
    flotsam: Vec<Flotsam>,
}

impl Default for BlockRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockRouter {
    pub fn new() -> Self {
        Self { reward: SUBSIDY_KOINU, flotsam: Vec::new() }
    }
    /// Record the fee-spent items of non-coinbase transaction `tx_index`, then add its fee to the reward.
    /// Call for every non-coinbase transaction in block order, even with nothing to record.
    pub fn after_tx(&mut self, tx_index: usize, placed: &[Placed], fee: u64) -> Result<(), SatError> {
        for p in placed {
            if let Dest::Fee { .. } = p.dest {
                // ord-dogecoin: reward + input-space offset, not minus the output total (module docs)
                let coff = self.reward.checked_add(p.src_off).ok_or(SatError::Overflow)?;
                self.flotsam.push(Flotsam { iid: p.iid, foff: p.src_off, tx_index, coff });
            }
        }
        self.reward = self.reward.checked_add(fee).ok_or(SatError::Overflow)?;
        Ok(())
    }
    pub fn flotsam(&self) -> &[Flotsam] {
        &self.flotsam
    }
}

/// Reverse of [`route`] for one koinu: which input (and offset inside it) feeds input-space offset `x`.
pub fn input_for_offset(input_values: &[u64], x: u64) -> Option<(usize, u64)> {
    let mut acc: u64 = 0;
    for (i, v) in input_values.iter().enumerate() {
        let end = acc.checked_add(*v)?;
        if x < end {
            return Some((i, x - acc));
        }
        acc = end;
    }
    None
}

/// Input-space offset of koinu `off` of output `vout`.
pub fn output_offset_to_input_space(outputs: &[u64], vout: usize, off: u64) -> Option<u64> {
    let before = outputs.get(..vout)?.iter().try_fold(0u64, |a, v| a.checked_add(*v))?;
    if off >= *outputs.get(vout)? {
        return None;
    }
    before.checked_add(off)
}
