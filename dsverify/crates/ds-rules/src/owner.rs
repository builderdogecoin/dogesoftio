// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! Who owns an output, read from the script bytes only (never from node labels).
//!
//! * P2PKH: the `D...` address (version 0x1e).
//! * P2SH: the `9...`/`A...` address (version 0x16).
//! * P2PK, compressed or uncompressed: the P2PKH address of that public key.
//! * Any script starting with `OP_RETURN` (0x6a): `nulldata`.
//! * Anything else: `script:<hex>` up to 64 bytes, otherwise the first 32 bytes in hex plus `..`.

use ripemd::Ripemd160;
use sha2::{Digest, Sha256};

/// Owner string for a nulldata output.
pub const NULLDATA: &str = "nulldata";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Owner {
    Address(String),
    Nulldata,
    Script(String),
}

impl Owner {
    /// The owner key as the ledger stores it.
    pub fn key(&self) -> String {
        match self {
            Owner::Address(a) | Owner::Script(a) => a.clone(),
            Owner::Nulldata => NULLDATA.to_string(),
        }
    }
}

pub fn hash160(b: &[u8]) -> [u8; 20] {
    Ripemd160::digest(Sha256::digest(b)).into()
}

pub fn base58check(version: u8, payload: &[u8]) -> String {
    let mut p = Vec::with_capacity(1 + payload.len());
    p.push(version);
    p.extend_from_slice(payload);
    bs58::encode(p).with_check().into_string()
}

/// Mainnet version bytes.
pub const P2PKH_VERSION: u8 = 0x1e;
pub const P2SH_VERSION: u8 = 0x16;

pub fn owner_of_script(spk: &[u8]) -> Owner {
    let n = spk.len();
    if n == 25 && spk[0] == 0x76 && spk[1] == 0xa9 && spk[2] == 0x14 && spk[23] == 0x88 && spk[24] == 0xac {
        return Owner::Address(base58check(P2PKH_VERSION, &spk[3..23]));
    }
    if n == 23 && spk[0] == 0xa9 && spk[1] == 0x14 && spk[22] == 0x87 {
        return Owner::Address(base58check(P2SH_VERSION, &spk[2..22]));
    }
    if (n == 35 && spk[0] == 0x21 && spk[34] == 0xac) || (n == 67 && spk[0] == 0x41 && spk[66] == 0xac) {
        return Owner::Address(base58check(P2PKH_VERSION, &hash160(&spk[1..n - 1])));
    }
    if n > 0 && spk[0] == 0x6a {
        return Owner::Nulldata;
    }
    Owner::Script(if n <= 64 { format!("script:{}", hex::encode(spk)) } else { format!("script:{}..", hex::encode(&spk[..32])) })
}

/// Is this a well-formed mainnet address (P2PKH or P2SH)?
pub fn is_mainnet_address(s: &str) -> bool {
    match bs58::decode(s).with_check(None).into_vec() {
        Ok(v) => v.len() == 21 && (v[0] == P2PKH_VERSION || v[0] == P2SH_VERSION),
        Err(_) => false,
    }
}
