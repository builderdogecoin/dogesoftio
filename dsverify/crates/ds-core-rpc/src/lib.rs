// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! # ds-core-rpc
//!
//! A minimal, **read-only** JSON-RPC client for a stock Dogecoin Core node.
//!
//! * Only read methods can be called: the allowlist below is enforced before anything is sent, so this
//!   client can never send coins, sign, broadcast or touch a wallet.
//! * Credentials come from your own `dogecoin.conf`, the node's `.cookie` file, or environment
//!   variables. They are only ever placed in the HTTP `Authorization` header and are never printed,
//!   logged or included in an error message.

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The only RPC methods this client will call.
pub const READ_ONLY_METHODS: &[&str] =
    &["getblockchaininfo", "getnetworkinfo", "getblockcount", "getblockhash", "getblockheader", "getblock", "getrawtransaction", "gettxout"];

#[derive(Debug)]
pub enum RpcError {
    /// Could not reach the node (connection refused, timeout...).
    Transport(String),
    /// The node rejected the login.
    Unauthorized,
    /// The node answered with an RPC error.
    Rpc {
        code: i64,
        message: String,
    },
    /// The method is not on the read-only allowlist.
    NotAllowed(String),
    Config(String),
    Decode(String),
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::Transport(m) => write!(f, "cannot reach Dogecoin Core: {m}"),
            RpcError::Unauthorized => write!(f, "Dogecoin Core rejected the RPC login (check rpcuser/rpcpassword or the cookie file)"),
            RpcError::Rpc { code, message } => write!(f, "Dogecoin Core error {code}: {message}"),
            RpcError::NotAllowed(m) => write!(f, "RPC method {m} is not allowed (dsverify is read-only)"),
            RpcError::Config(m) => write!(f, "RPC settings: {m}"),
            RpcError::Decode(m) => write!(f, "unexpected answer from Dogecoin Core: {m}"),
        }
    }
}
impl std::error::Error for RpcError {}

pub type Result<T> = std::result::Result<T, RpcError>;

/// How to reach the node. `Debug` never shows the credentials.
#[derive(Clone)]
pub struct RpcConfig {
    pub url: String,
    user: String,
    pass: String,
    pub auth_source: &'static str,
}

impl std::fmt::Debug for RpcConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RpcConfig").field("url", &self.url).field("auth_source", &self.auth_source).finish()
    }
}

/// Default Dogecoin Core data directory for this system.
pub fn default_datadir() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("Dogecoin"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support/Dogecoin"))
    } else {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".dogecoin"))
    }
}

fn read_conf(path: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let mut out = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        if l.starts_with('[') {
            // only the top level and [main] apply to mainnet
            in_section = l != "[main]";
            continue;
        }
        if in_section {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            out.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}

impl RpcConfig {
    /// Explicit URL and login.
    pub fn new(url: impl Into<String>, user: impl Into<String>, pass: impl Into<String>) -> Self {
        RpcConfig { url: url.into(), user: user.into(), pass: pass.into(), auth_source: "explicit" }
    }

    /// Discover settings: environment (`DSVERIFY_RPC_URL`, `DSVERIFY_RPC_USER`, `DSVERIFY_RPC_PASSWORD`),
    /// then `dogecoin.conf` in `datadir` (or `conf`), then the `.cookie` file.
    pub fn discover(datadir: Option<&Path>, conf: Option<&Path>, url: Option<&str>) -> Result<Self> {
        let dd = datadir.map(Path::to_path_buf).or_else(default_datadir);
        let conf_path = conf.map(Path::to_path_buf).or_else(|| dd.as_ref().map(|d| d.join("dogecoin.conf")));
        let kv = conf_path.as_deref().map(read_conf).unwrap_or_default();
        let get = |k: &str| kv.iter().rev().find(|(a, _)| a == k).map(|(_, b)| b.clone());
        let host = get("rpcconnect").unwrap_or_else(|| "127.0.0.1".into());
        let port = get("rpcport").unwrap_or_else(|| "22555".into());
        let url = url.map(str::to_string).or_else(|| std::env::var("DSVERIFY_RPC_URL").ok()).unwrap_or_else(|| format!("http://{host}:{port}/"));
        if let (Ok(u), Ok(p)) = (std::env::var("DSVERIFY_RPC_USER"), std::env::var("DSVERIFY_RPC_PASSWORD")) {
            return Ok(RpcConfig { url, user: u, pass: p, auth_source: "environment" });
        }
        if let (Some(u), Some(p)) = (get("rpcuser"), get("rpcpassword")) {
            return Ok(RpcConfig { url, user: u, pass: p, auth_source: "dogecoin.conf" });
        }
        if let Some(d) = dd {
            if let Ok(c) = std::fs::read_to_string(d.join(".cookie")) {
                if let Some((u, p)) = c.trim().split_once(':') {
                    return Ok(RpcConfig { url, user: u.into(), pass: p.into(), auth_source: ".cookie" });
                }
            }
        }
        Err(RpcError::Config(
            "no RPC login found: set rpcuser/rpcpassword in dogecoin.conf, run Core with server=1 so it writes a .cookie, or set DSVERIFY_RPC_USER and DSVERIFY_RPC_PASSWORD".into(),
        ))
    }
}

fn b64(input: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut o = String::with_capacity(input.len().div_ceil(3) * 4);
    for c in input.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        o.push(T[(n >> 18) as usize & 63] as char);
        o.push(T[(n >> 12) as usize & 63] as char);
        o.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        o.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    o
}

/// The client.
pub struct Client {
    cfg: RpcConfig,
    agent: ureq::Agent,
    auth: String,
}

impl Client {
    pub fn new(cfg: RpcConfig) -> Self {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(120)).build();
        let auth = format!("Basic {}", b64(format!("{}:{}", cfg.user, cfg.pass).as_bytes()));
        Client { cfg, agent, auth }
    }

    pub fn url(&self) -> &str {
        &self.cfg.url
    }
    pub fn auth_source(&self) -> &str {
        self.cfg.auth_source
    }

    fn post(&self, body: &Value) -> Result<Value> {
        let r = self.agent.post(&self.cfg.url).set("Authorization", &self.auth).set("Content-Type", "application/json").send_json(body.clone());
        match r {
            Ok(resp) => resp.into_json::<Value>().map_err(|e| RpcError::Decode(e.to_string())),
            Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => Err(RpcError::Unauthorized),
            // Core answers RPC errors with HTTP 500 and a JSON body
            Err(ureq::Error::Status(_, resp)) => resp.into_json::<Value>().map_err(|e| RpcError::Decode(e.to_string())),
            Err(ureq::Error::Transport(t)) => Err(RpcError::Transport(t.kind().to_string())),
        }
    }

    fn check(method: &str) -> Result<()> {
        if READ_ONLY_METHODS.contains(&method) {
            Ok(())
        } else {
            Err(RpcError::NotAllowed(method.to_string()))
        }
    }

    fn unwrap(v: Value) -> Result<Value> {
        if let Some(e) = v.get("error").filter(|e| !e.is_null()) {
            return Err(RpcError::Rpc {
                code: e.get("code").and_then(Value::as_i64).unwrap_or(0),
                message: e.get("message").and_then(Value::as_str).unwrap_or("").to_string(),
            });
        }
        Ok(v.get("result").cloned().unwrap_or(Value::Null))
    }

    /// One call.
    pub fn call(&self, method: &str, params: Value) -> Result<Value> {
        Self::check(method)?;
        Self::unwrap(self.post(&json!({"jsonrpc": "1.0", "id": "dsverify", "method": method, "params": params}))?)
    }

    /// Many calls of the same method in one batch; results in input order.
    pub fn batch(&self, method: &str, params: &[Value]) -> Result<Vec<Result<Value>>> {
        Self::check(method)?;
        if params.is_empty() {
            return Ok(Vec::new());
        }
        let body: Vec<Value> = params.iter().enumerate().map(|(i, p)| json!({"jsonrpc": "1.0", "id": i, "method": method, "params": p})).collect();
        let v = self.post(&Value::Array(body))?;
        let arr = v.as_array().ok_or_else(|| RpcError::Decode("batch answer is not an array".into()))?;
        let mut out: Vec<Option<Result<Value>>> = (0..params.len()).map(|_| None).collect();
        for item in arr {
            let id = item.get("id").and_then(Value::as_u64).ok_or_else(|| RpcError::Decode("batch item without id".into()))? as usize;
            if id < out.len() {
                out[id] = Some(Self::unwrap(item.clone()));
            }
        }
        out.into_iter().map(|o| o.ok_or_else(|| RpcError::Decode("missing batch item".into()))).collect()
    }

    pub fn blockchain_info(&self) -> Result<Value> {
        self.call("getblockchaininfo", json!([]))
    }
    pub fn block_count(&self) -> Result<u64> {
        self.call("getblockcount", json!([]))?.as_u64().ok_or_else(|| RpcError::Decode("getblockcount".into()))
    }
    pub fn block_hash(&self, height: u64) -> Result<String> {
        self.call("getblockhash", json!([height]))?.as_str().map(str::to_string).ok_or_else(|| RpcError::Decode("getblockhash".into()))
    }
    /// Raw block hex (`getblock <hash> false`).
    pub fn raw_block(&self, hash: &str) -> Result<String> {
        self.call("getblock", json!([hash, false]))?.as_str().map(str::to_string).ok_or_else(|| RpcError::Decode("getblock".into()))
    }
    pub fn block_header(&self, hash: &str) -> Result<Value> {
        self.call("getblockheader", json!([hash, true]))
    }
    /// Raw transaction hex (needs `txindex=1` for transactions outside the wallet and mempool).
    pub fn raw_tx(&self, txid: &str) -> Result<String> {
        self.call("getrawtransaction", json!([txid, 0]))?.as_str().map(str::to_string).ok_or_else(|| RpcError::Decode("getrawtransaction".into()))
    }
    pub fn raw_txs(&self, txids: &[String]) -> Result<Vec<Result<String>>> {
        let p: Vec<Value> = txids.iter().map(|t| json!([t, 0])).collect();
        Ok(self
            .batch("getrawtransaction", &p)?
            .into_iter()
            .map(|r| r.and_then(|v| v.as_str().map(str::to_string).ok_or_else(|| RpcError::Decode("getrawtransaction".into()))))
            .collect())
    }
    /// The unspent output, or `None` when it is spent (or never existed).
    pub fn tx_out(&self, txid: &str, vout: u32) -> Result<Option<Value>> {
        let v = self.call("gettxout", json!([txid, vout, false]))?;
        Ok((!v.is_null()).then_some(v))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_methods_are_refused_before_sending() {
        let c = Client::new(RpcConfig::new("http://127.0.0.1:1/", "u", "p"));
        for m in ["sendrawtransaction", "sendtoaddress", "signrawtransaction", "dumpprivkey", "importprivkey", "walletpassphrase", "stop"] {
            assert!(matches!(c.call(m, json!([])), Err(RpcError::NotAllowed(_))), "{m}");
        }
    }

    #[test]
    fn debug_never_shows_credentials() {
        let c = RpcConfig::new("http://127.0.0.1:22555/", "alice", "s3cr3t-pass");
        let d = format!("{c:?}");
        assert!(!d.contains("alice") && !d.contains("s3cr3t"));
    }

    #[test]
    fn conf_reader_takes_top_level_and_main_only() {
        let dir = std::env::temp_dir().join(format!("dsrpc-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("dogecoin.conf");
        std::fs::write(&p, "server=1\nrpcuser=a\nrpcpassword=b\n[test]\nrpcport=1\n[main]\nrpcport=2\n").unwrap();
        let kv = read_conf(&p);
        assert!(kv.contains(&("rpcport".into(), "2".into())));
        assert!(!kv.contains(&("rpcport".into(), "1".into())));
        let c = RpcConfig::discover(Some(&dir), None, None).unwrap();
        assert_eq!(c.url, "http://127.0.0.1:2/");
        assert_eq!(c.auth_source, "dogecoin.conf");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn base64_matches_rfc4648() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"f"), "Zg==");
        assert_eq!(b64(b"fo"), "Zm8=");
        assert_eq!(b64(b"foo"), "Zm9v");
        assert_eq!(b64(b"foobar"), "Zm9vYmFy");
    }
}
