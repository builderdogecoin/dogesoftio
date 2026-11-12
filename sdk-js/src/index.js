// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
// @dogesoft/sdk: a small client for the public Doge Soft Explorer API.

export const DEFAULT_BASE_URL = "https://explorer.dogesoft.io";
export const KOINU_PER_DOGE = 100_000_000n;

/** Thrown for any non-2xx answer (after 429 retries are used up). */
export class DogeSoftError extends Error {
  constructor(message, status, body) {
    super(message);
    this.name = "DogeSoftError";
    this.status = status;
    this.body = body;
  }
}

// Integer fields whose name says koinu become exact bigint, even past 2^53.
const KOINU_FIELD = /"([A-Za-z0-9_]*koinu[A-Za-z0-9_]*)"\s*:\s*(-?\d+)(?=\s*[,}\]])/g;

/** Parse an API body. Every `*koinu*` integer becomes a bigint. */
export function parseBody(text) {
  return JSON.parse(text.replace(KOINU_FIELD, '"$1":"\\u0000koinu:$2"'), (_k, v) =>
    typeof v === "string" && v.startsWith("\u0000koinu:") ? BigInt(v.slice(7)) : v,
  );
}

/** Format koinu (bigint) as a DOGE string with 8 decimals: 937600979n -> "9.37600979". */
export function dogeFromKoinu(koinu) {
  const k = BigInt(koinu);
  const neg = k < 0n;
  const a = neg ? -k : k;
  const frac = (a % KOINU_PER_DOGE).toString().padStart(8, "0");
  return `${neg ? "-" : ""}${a / KOINU_PER_DOGE}.${frac}`;
}

/** Parse a DOGE amount string to koinu (bigint): "9.37600979" -> 937600979n. */
export function dogeToKoinu(doge) {
  const m = /^(\d+)(?:\.(\d{0,8}))?$/.exec(String(doge).trim());
  if (!m) throw new RangeError(`not a DOGE amount: ${doge}`);
  return BigInt(m[1]) * KOINU_PER_DOGE + BigInt((m[2] || "").padEnd(8, "0"));
}

/**
 * Parse a DRC-20 decimal string to an exact bigint in units of 10^-decimals:
 * drc20Units("492.5", 18) -> 492500000000000000000n.
 */
export function drc20Units(amount, decimals = 18) {
  const m = /^(\d+)(?:\.(\d+))?$/.exec(String(amount).trim());
  if (!m) throw new RangeError(`not a DRC-20 amount: ${amount}`);
  const frac = (m[2] || "").replace(/0+$/, "");
  if (frac.length > decimals) throw new RangeError(`${amount} has more than ${decimals} decimals`);
  return BigInt(m[1]) * 10n ** BigInt(decimals) + BigInt(frac.padEnd(decimals, "0") || "0");
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const seg = (s) => encodeURIComponent(String(s));

export class DogeSoft {
  /**
   * @param {object} [opts]
   * @param {string} [opts.baseUrl] default https://explorer.dogesoft.io
   * @param {string} [opts.apiKey]  partner key, sent as `x-api-key`
   * @param {number} [opts.maxRetries] retries on HTTP 429 (default 3)
   * @param {typeof fetch} [opts.fetch] custom fetch (tests, proxies)
   */
  constructor(opts = {}) {
    this.baseUrl = (opts.baseUrl || DEFAULT_BASE_URL).replace(/\/+$/, "");
    this.apiKey = opts.apiKey;
    this.maxRetries = opts.maxRetries ?? 3;
    this.fetch = opts.fetch || globalThis.fetch;
    this.sleep = opts.sleep || sleep;
    if (typeof this.fetch !== "function") throw new Error("no fetch available: use Node 18+ or pass opts.fetch");
  }

  /** GET a path under the base URL. Backs off politely on 429 (honours Retry-After). */
  async get(path) {
    const headers = { accept: "application/json", "user-agent": "dogesoft-sdk-js/1.0.0" };
    if (this.apiKey) headers["x-api-key"] = this.apiKey;
    let wait = 1000;
    for (let attempt = 0; ; attempt++) {
      const r = await this.fetch(this.baseUrl + path, { headers });
      const text = await r.text();
      if (r.status === 429 && attempt < this.maxRetries) {
        const ra = Number(r.headers?.get?.("retry-after"));
        await this.sleep(Number.isFinite(ra) && ra > 0 ? Math.min(ra, 60) * 1000 : wait);
        wait *= 2;
        continue;
      }
      if (!r.ok) throw new DogeSoftError(`GET ${path}: HTTP ${r.status}`, r.status, text);
      return parseBody(text);
    }
  }

  /** Index height, chain tip and lag. */
  health() {
    return this.get("/api/health");
  }
  /** DOGE balance of an address; `balance_koinu` is a bigint. */
  balance(address) {
    return this.get(`/api/wallet/address/${seg(address)}/balance`);
  }
  /** DRC-20 Coins held by an address. */
  drc20Holdings(address) {
    return this.get(`/api/wallet/address/${seg(address)}/drc20`);
  }
  /** One DRC-20 Coin: supply, deploy, holder count, recent operations. */
  drc20(tick) {
    return this.get(`/api/explorer/drc20/${seg(tick)}`);
  }
  /** Resolve a .doge name (or any Doge Soft name ending). */
  resolveName(name) {
    return this.get(`/api/names/resolve/${seg(name)}`);
  }
  /** A Doginal's metadata, genesis and current location. */
  inscription(id) {
    return this.get(`/api/explorer/inscriptions/${seg(id)}`);
  }
  /** URL of a Doginal's content. */
  contentUrl(id) {
    return `${this.baseUrl}/content/${seg(id)}`;
  }
}

export default DogeSoft;
