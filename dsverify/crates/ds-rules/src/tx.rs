// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Raw Dogecoin transactions and blocks, decoded from the bytes Dogecoin Core returns.
//!
//! Nothing is taken on trust: a transaction id is the double SHA-256 of its bytes, a block hash is the
//! double SHA-256 of its 80-byte header, and a block's transactions must hash to the header's merkle root.

use sha2::{Digest, Sha256};

/// A transaction id in display byte order (the hex Dogecoin Core prints).
pub type Txid = [u8; 32];
/// An outpoint: (txid, output index).
pub type OutPoint = (Txid, u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError(pub String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DecodeError {}

fn err<T>(m: impl Into<String>) -> Result<T, DecodeError> {
    Err(DecodeError(m.into()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawIn {
    pub prev: Txid,
    pub vout: u32,
    pub script: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawOut {
    pub value: u64,
    pub spk: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawTx {
    pub txid: Txid,
    pub ins: Vec<RawIn>,
    pub outs: Vec<RawOut>,
}

impl RawTx {
    pub fn is_coinbase(&self) -> bool {
        self.ins.len() == 1 && self.ins[0].prev == [0u8; 32] && self.ins[0].vout == u32::MAX
    }
    pub fn output_total(&self) -> Option<u64> {
        self.outs.iter().try_fold(0u64, |a, o| a.checked_add(o.value))
    }
}

pub fn sha256d(b: &[u8]) -> [u8; 32] {
    let a = Sha256::digest(b);
    let c = Sha256::digest(a);
    c.into()
}

pub fn txid_hex(t: &Txid) -> String {
    hex::encode(t)
}

pub fn parse_txid(s: &str) -> Result<Txid, DecodeError> {
    let v = hex::decode(s.trim()).map_err(|_| DecodeError(format!("bad txid {s}")))?;
    v.try_into().map_err(|_| DecodeError(format!("txid {s} is not 32 bytes")))
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let end = self.i.checked_add(n).ok_or_else(|| DecodeError("length overflow".into()))?;
        let s = self.b.get(self.i..end).ok_or_else(|| DecodeError("truncated".into()))?;
        self.i = end;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32, DecodeError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, DecodeError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn varint(&mut self) -> Result<u64, DecodeError> {
        let f = self.take(1)?[0];
        Ok(match f {
            0xfd => u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as u64,
            0xfe => self.u32()? as u64,
            0xff => self.u64()?,
            n => n as u64,
        })
    }
    fn len(&mut self) -> Result<usize, DecodeError> {
        let n = self.varint()?;
        usize::try_from(n).map_err(|_| DecodeError("length too large".into()))
    }
}

/// Read one transaction starting at `r.i`. Witness serialization is accepted only where it is allowed
/// (an AuxPoW parent coinbase); Dogecoin transactions themselves are legacy serialized.
fn read_tx(r: &mut Reader<'_>, allow_witness: bool) -> Result<RawTx, DecodeError> {
    let start = r.i;
    r.take(4)?;
    let mut nin = r.len()?;
    let mut witness = false;
    if nin == 0 && r.b.get(r.i).copied().unwrap_or(0) != 0 {
        if !allow_witness {
            return err("witness-serialized transaction where Dogecoin does not allow one");
        }
        witness = true;
        r.take(1)?;
        nin = r.len()?;
    }
    let mut ins = Vec::with_capacity(nin.min(10_000));
    for _ in 0..nin {
        let mut prev: Txid = r.take(32)?.try_into().unwrap();
        prev.reverse();
        let vout = r.u32()?;
        let l = r.len()?;
        let script = r.take(l)?.to_vec();
        r.take(4)?;
        ins.push(RawIn { prev, vout, script });
    }
    let nout = r.len()?;
    let mut outs = Vec::with_capacity(nout.min(100_000));
    for _ in 0..nout {
        let value = r.u64()?;
        let l = r.len()?;
        let spk = r.take(l)?.to_vec();
        outs.push(RawOut { value, spk });
    }
    if witness {
        for _ in 0..nin {
            let items = r.len()?;
            for _ in 0..items {
                let l = r.len()?;
                r.take(l)?;
            }
        }
    }
    r.take(4)?;
    let mut txid = sha256d(&r.b[start..r.i]);
    txid.reverse();
    Ok(RawTx { txid, ins, outs })
}

/// Decode one legacy-serialized transaction; its txid is computed from its bytes.
pub fn decode_tx(raw: &[u8]) -> Result<RawTx, DecodeError> {
    let mut r = Reader { b: raw, i: 0 };
    let t = read_tx(&mut r, false)?;
    if r.i != raw.len() {
        return err(format!("transaction has {} trailing bytes", raw.len() - r.i));
    }
    Ok(t)
}

pub fn decode_tx_hex(raw_hex: &str) -> Result<RawTx, DecodeError> {
    decode_tx(&hex::decode(raw_hex.trim()).map_err(|_| DecodeError("raw tx is not hex".into()))?)
}

/// A decoded block.
#[derive(Clone, Debug)]
pub struct Block {
    /// Block hash, display order.
    pub hash: [u8; 32],
    /// Previous block hash, display order.
    pub prev: [u8; 32],
    pub merkle_root: [u8; 32],
    pub txs: Vec<RawTx>,
}

/// Merkle root (internal byte order) of display-order txids.
pub fn merkle_root(txids: &[Txid]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }
    let mut level: Vec<[u8; 32]> = txids
        .iter()
        .map(|t| {
            let mut x = *t;
            x.reverse();
            x
        })
        .collect();
    while level.len() > 1 {
        if level.len() % 2 == 1 {
            level.push(*level.last().unwrap());
        }
        level = level
            .chunks(2)
            .map(|p| {
                let mut b = [0u8; 64];
                b[..32].copy_from_slice(&p[0]);
                b[32..].copy_from_slice(&p[1]);
                sha256d(&b)
            })
            .collect();
    }
    level[0]
}

/// Decode a raw block (`getblock <hash> false`), merge-mined (AuxPoW) blocks included, and check that the
/// header hashes to the block hash and the transactions hash to the merkle root.
pub fn decode_block(raw: &[u8]) -> Result<Block, DecodeError> {
    if raw.len() < 80 {
        return err("block shorter than a header");
    }
    let header = &raw[..80];
    let version = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let mut hash = sha256d(header);
    hash.reverse();
    let mut prev: [u8; 32] = header[4..36].try_into().unwrap();
    prev.reverse();
    let merkle: [u8; 32] = header[36..68].try_into().unwrap();
    let mut r = Reader { b: raw, i: 80 };
    if version & 0x100 != 0 {
        // AuxPoW: parent coinbase (may be witness serialized), parent hash, two merkle branches, parent header
        read_tx(&mut r, true)?;
        r.take(32)?;
        for _ in 0..2 {
            let n = r.len()?;
            r.take(n.checked_mul(32).ok_or_else(|| DecodeError("branch overflow".into()))?)?;
            r.take(4)?;
        }
        r.take(80)?;
    }
    let n = r.len()?;
    let mut txs = Vec::with_capacity(n.min(100_000));
    for _ in 0..n {
        txs.push(read_tx(&mut r, false)?);
    }
    if r.i != raw.len() {
        return err(format!("block has {} trailing bytes", raw.len() - r.i));
    }
    let ids: Vec<Txid> = txs.iter().map(|t| t.txid).collect();
    if merkle_root(&ids) != merkle {
        return err("transactions do not hash to the header merkle root");
    }
    let mut merkle_display = merkle;
    merkle_display.reverse();
    Ok(Block { hash, prev, merkle_root: merkle_display, txs })
}

pub fn decode_block_hex(raw_hex: &str) -> Result<Block, DecodeError> {
    decode_block(&hex::decode(raw_hex.trim()).map_err(|_| DecodeError("raw block is not hex".into()))?)
}
