// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! # dogesoft
//!
//! An async client for the public [Doge Soft Explorer](https://explorer.dogesoft.io) API: Dogecoin,
//! Doginals, DRC-20 Coins and .doge names. No node and no key needed for reads.
//!
//! ```no_run
//! # async fn demo() -> Result<(), dogesoft::Error> {
//! let ds = dogesoft::Client::default(); // https://explorer.dogesoft.io
//! let h = ds.health().await?;
//! println!("indexed {} / tip {}", h.indexed_block_height, h.chain_tip);
//! let coins = ds.drc20_holdings("D...").await?;
//! for c in coins.balances {
//!     println!("{} available {} transferable {}", c.tick, c.available, c.transferable);
//! }
//! # Ok(()) }
//! ```
//!
//! DOGE amounts are koinu (`u64`, 1 DOGE = 100,000,000 koinu). DRC-20 amounts are exact decimal
//! strings; [`drc20_units`] turns one into an exact integer, and [`is_valid_drc20_amount`] checks a mint
//! or transfer amount against the ds-rules amount rule.
//!
//! Outside callers are rate limited. On HTTP 429 the client waits (honouring `Retry-After`)
//! and tries again, up to [`Client::max_retries`] times. Partners get a key ([`Client::with_api_key`]):
//! DM [@dogesoftio](https://x.com/dogesoftio) on X or send the developer request form at
//! <https://docs.dogesoft.io/explorer/api>.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::Duration;

pub const DEFAULT_BASE_URL: &str = "https://explorer.dogesoft.io";
pub const KOINU_PER_DOGE: u64 = 100_000_000;

#[derive(Debug)]
pub enum Error {
    /// Network or TLS trouble.
    Http(reqwest::Error),
    /// The API answered with this status (after any 429 retries).
    Status { status: u16, body: String },
    /// The answer was not the JSON we expected.
    Decode(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Http(e) => write!(f, "http: {e}"),
            Error::Status { status, .. } => write!(f, "HTTP {status}"),
            Error::Decode(e) => write!(f, "decode: {e}"),
        }
    }
}
impl std::error::Error for Error {}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Health {
    pub status: String,
    #[serde(default)]
    pub version: String,
    pub indexed_block_height: u64,
    pub chain_tip: u64,
    pub blocks_behind_tip: u64,
    #[serde(default)]
    pub drc20_operations_indexed_to: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Balance {
    pub address: String,
    pub balance_koinu: u64,
    #[serde(default)]
    pub confirmed_balance_koinu: u64,
    #[serde(default)]
    pub unconfirmed_balance_koinu: u64,
    #[serde(default)]
    pub utxo_count: u64,
}
impl Balance {
    /// The balance as a DOGE string with 8 decimals.
    pub fn doge(&self) -> String {
        koinu_to_doge(self.balance_koinu)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drc20Holding {
    pub tick: String,
    pub available: String,
    pub transferable: String,
    #[serde(default)]
    pub balance: String,
    #[serde(default)]
    pub decimals: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drc20Holdings {
    pub address: String,
    pub balances: Vec<Drc20Holding>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drc20Token {
    pub ticker: String,
    #[serde(default)]
    pub ticker_onchain: String,
    pub max_supply: String,
    pub limit_per_mint: String,
    pub decimals: u32,
    pub deploy_height: u64,
    pub deploy_txid: String,
    pub deployer_address: String,
    pub minted_supply: String,
    pub holder_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drc20Holder {
    pub address: String,
    pub balance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Drc20Coin {
    pub token: Drc20Token,
    #[serde(default)]
    pub holders: Vec<Drc20Holder>,
    #[serde(default)]
    pub operations: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NameAnswer {
    pub name: String,
    pub found: bool,
    #[serde(default)]
    pub valid: bool,
    pub holder: Option<String>,
    pub inscription_id: Option<String>,
    pub txid: Option<String>,
    pub block_height: Option<u64>,
    pub claim_count: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Outpoint {
    pub txid: String,
    pub vout: u32,
    pub outpoint: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Location {
    pub txid: String,
    pub vout: u32,
    pub outpoint: String,
    pub sat_offset: Option<u64>,
    pub owner_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Inscription {
    pub inscription_id: String,
    pub mime_type: Option<String>,
    pub owner_address: Option<String>,
    pub content_sha256: Option<String>,
    pub content_length: Option<u64>,
    pub block_height: Option<u64>,
    pub genesis: Option<Outpoint>,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, Deserialize)]
struct InscriptionWrap {
    inscription: Inscription,
}

/// The client. Cheap to clone.
#[derive(Debug, Clone)]
pub struct Client {
    base: String,
    http: reqwest::Client,
    api_key: Option<String>,
    /// Retries on HTTP 429 (default 3).
    pub max_retries: u32,
}

impl Default for Client {
    fn default() -> Self {
        Client::new(DEFAULT_BASE_URL)
    }
}

fn seg(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

impl Client {
    pub fn new(base_url: &str) -> Client {
        let http = reqwest::Client::builder()
            .user_agent(concat!("dogesoft-sdk-rust/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(30))
            .build()
            .expect("http client");
        Client { base: base_url.trim_end_matches('/').to_string(), http, api_key: None, max_retries: 3 }
    }

    /// Use a partner key (sent as `x-api-key`).
    pub fn with_api_key(mut self, key: impl Into<String>) -> Client {
        self.api_key = Some(key.into());
        self
    }

    /// GET a path under the base URL and decode it.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, Error> {
        let mut wait = Duration::from_secs(1);
        let mut attempt = 0;
        loop {
            let mut rq = self.http.get(format!("{}{}", self.base, path)).header("accept", "application/json");
            if let Some(k) = &self.api_key {
                rq = rq.header("x-api-key", k);
            }
            let r = rq.send().await?;
            let status = r.status().as_u16();
            if status == 429 && attempt < self.max_retries {
                let ra = r.headers().get("retry-after").and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
                tokio::time::sleep(ra.map(|s| Duration::from_secs(s.min(60))).unwrap_or(wait)).await;
                wait *= 2;
                attempt += 1;
                continue;
            }
            let body = r.text().await?;
            if !(200..300).contains(&status) {
                return Err(Error::Status { status, body });
            }
            return serde_json::from_str(&body).map_err(|e| Error::Decode(e.to_string()));
        }
    }

    pub async fn health(&self) -> Result<Health, Error> {
        self.get("/api/health").await
    }
    /// DOGE balance of an address, in koinu.
    pub async fn balance(&self, address: &str) -> Result<Balance, Error> {
        self.get(&format!("/api/wallet/address/{}/balance", seg(address))).await
    }
    /// DRC-20 Coins held by an address.
    pub async fn drc20_holdings(&self, address: &str) -> Result<Drc20Holdings, Error> {
        self.get(&format!("/api/wallet/address/{}/drc20", seg(address))).await
    }
    /// One DRC-20 Coin: supply, deploy and holder count.
    pub async fn drc20(&self, tick: &str) -> Result<Drc20Coin, Error> {
        self.get(&format!("/api/explorer/drc20/{}", seg(tick))).await
    }
    /// Resolve a .doge name.
    pub async fn resolve_name(&self, name: &str) -> Result<NameAnswer, Error> {
        self.get(&format!("/api/names/resolve/{}", seg(name))).await
    }
    /// A Doginal's metadata, genesis and location.
    pub async fn inscription(&self, id: &str) -> Result<Inscription, Error> {
        self.get::<InscriptionWrap>(&format!("/api/explorer/inscriptions/{}", seg(id))).await.map(|w| w.inscription)
    }
    /// URL of a Doginal's content.
    pub fn content_url(&self, id: &str) -> String {
        format!("{}/content/{}", self.base, seg(id))
    }
}

/// `937600979` -> `"9.37600979"`.
pub fn koinu_to_doge(koinu: u64) -> String {
    format!("{}.{:08}", koinu / KOINU_PER_DOGE, koinu % KOINU_PER_DOGE)
}

/// Read an amount with the same format rules as ds-rules: ASCII digits, optional one dot with digits
/// on both sides, no whitespace, plus, exponent or comma, and no more fraction digits than `decimals`.
/// Invalid input is rejected, not coerced. Zero is accepted (balances can be 0).
fn units_from_amount(amount: &str, decimals: u32) -> Option<u128> {
    let (i, f) = match amount.split_once('.') {
        Some((i, f)) => (i, f),
        None => (amount, ""),
    };
    let digits = |p: &str| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
    if !digits(i) || (amount.contains('.') && !digits(f)) || f.len() > decimals as usize {
        return None;
    }
    let scale = 10u128.checked_pow(decimals)?;
    let fv: u128 = if f.is_empty() { 0 } else { format!("{f:0<width$}", width = decimals as usize).parse().ok()? };
    i.parse::<u128>().ok()?.checked_mul(scale)?.checked_add(fv)
}

/// `"9.37600979"` -> `937600979`. At most 8 decimals.
pub fn doge_to_koinu(doge: &str) -> Option<u64> {
    units_from_amount(doge, 8)?.try_into().ok()
}

/// A DRC-20 decimal string as an exact integer of `10^-decimals` units: `("492.5", 18)` -> `492.5 * 10^18`.
/// At most uint64 max whole tokens, like ds-rules. Zero is accepted (balances can be 0); check a mint or
/// transfer amount with [`is_valid_drc20_amount`].
pub fn drc20_units(amount: &str, decimals: u32) -> Option<u128> {
    let units = units_from_amount(amount, decimals)?;
    match 10u128.checked_pow(decimals).and_then(|scale| scale.checked_mul(u128::from(u64::MAX))) {
        Some(max) if units > max => None,
        _ => Some(units),
    }
}

/// The full ds-rules amount rule for a DRC-20 mint or transfer: the format above, at most `decimals`
/// fraction digits (`decimals` 0 to 18), above zero and at most uint64 max.
pub fn is_valid_drc20_amount(amount: &str, decimals: u32) -> bool {
    decimals <= 18 && matches!(drc20_units(amount, decimals), Some(units) if units > 0)
}
