// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! A tiny reader for the public Doge Soft Explorer API, used only for `--api` comparisons.
//! Outside callers are rate limited; on HTTP 429 we wait as told (`Retry-After`) and try again, a few times.

use serde_json::Value;
use std::time::Duration;

pub struct Api {
    base: String,
    agent: ureq::Agent,
}

impl Api {
    pub fn new(base: &str) -> Api {
        let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(30)).user_agent(concat!("dsverify/", env!("CARGO_PKG_VERSION"))).build();
        Api { base: base.trim_end_matches('/').to_string(), agent }
    }

    pub fn get(&self, path: &str) -> Result<Value, String> {
        let url = format!("{}{}", self.base, path);
        let mut wait = 2u64;
        for attempt in 0..4 {
            match self.agent.get(&url).call() {
                Ok(r) => return r.into_json::<Value>().map_err(|e| format!("{url}: {e}")),
                Err(ureq::Error::Status(429, r)) if attempt < 3 => {
                    let s = r.header("retry-after").and_then(|v| v.parse::<u64>().ok()).unwrap_or(wait).min(60);
                    eprintln!("API rate limit reached: waiting {s}s");
                    std::thread::sleep(Duration::from_secs(s));
                    wait *= 2;
                }
                Err(ureq::Error::Status(c, _)) => return Err(format!("{url}: HTTP {c}")),
                Err(e) => return Err(format!("{url}: {e}")),
            }
        }
        Err(format!("{url}: still rate limited"))
    }
}

/// Percent-encode one path segment.
pub fn seg(s: &str) -> String {
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
