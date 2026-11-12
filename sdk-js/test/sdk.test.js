// SPDX-License-Identifier: Apache-2.0
import test from "node:test";
import assert from "node:assert/strict";
import { DogeSoft, DogeSoftError, parseBody, dogeFromKoinu, dogeToKoinu, drc20Units } from "../src/index.js";

function mock(routes) {
  const calls = [];
  const f = async (url, init) => {
    calls.push({ url, init });
    const path = new URL(url).pathname;
    let r = routes[path];
    if (Array.isArray(r)) r = r.shift();
    if (!r) return { ok: false, status: 404, headers: new Map(), text: async () => "{}" };
    const status = r.status ?? 200;
    return { ok: status < 300, status, headers: new Map(Object.entries(r.headers || {})), text: async () => r.body };
  };
  return { f, calls };
}

test("health and the default base URL", async () => {
  const { f, calls } = mock({ "/api/health": { body: '{"status":"healthy","indexed_block_height":6453516,"chain_tip":6453517,"blocks_behind_tip":1}' } });
  const ds = new DogeSoft({ fetch: f });
  const h = await ds.health();
  assert.equal(h.indexed_block_height, 6453516);
  assert.equal(calls[0].url, "https://explorer.dogesoft.io/api/health");
});

test("koinu fields are exact bigints, even above 2^53", async () => {
  const v = parseBody('{"balance_koinu":15000000000000000001,"balance_doge":"150000000000.00000001","n":5}');
  assert.equal(v.balance_koinu, 15000000000000000001n);
  assert.equal(v.n, 5);
  assert.equal(dogeFromKoinu(937600979n), "9.37600979");
  assert.equal(dogeFromKoinu(-5n), "-0.00000005");
  assert.equal(dogeToKoinu("9.37600979"), 937600979n);
  assert.equal(dogeToKoinu("1"), 100000000n);
  assert.equal(dogeToKoinu("0"), 0n);
  assert.equal(dogeToKoinu("0.00000001"), 1n);
  assert.throws(() => dogeToKoinu("1.000000001"));
  for (const bad of [" 1", "1 ", "1\n", "9.", ".5", "+1", "1e2", "1E2", "1.2e1", "1,000", "1,2", "", "1.2.3", 1]) {
    assert.throws(() => dogeToKoinu(bad));
  }
});

test("drc20 amounts stay exact", () => {
  assert.equal(drc20Units("492.000000000000000000"), 492n * 10n ** 18n);
  assert.equal(drc20Units("0.5", 1), 5n);
  assert.equal(drc20Units("1.20", 2), 120n);
  assert.throws(() => drc20Units("0.55", 1));
  assert.throws(() => drc20Units("-1"));
  // Trailing zeros still count as fraction digits, same as ds-rules.
  assert.throws(() => drc20Units("1.200", 2));
  for (const bad of [" 0.5", "0.5 ", "0.5\n", "1.", ".5", "+1", "1e2", "1,2", "", 0.5]) {
    assert.throws(() => drc20Units(bad, 1));
  }
});

test("paths are encoded ($ in tickers)", async () => {
  const { f, calls } = mock({
    "/api/explorer/drc20/DU%24T": { body: '{"token":{"ticker":"du$t","holder_count":139},"operations":[],"holders":[]}' },
    "/api/wallet/address/DLJR/drc20": { body: '{"address":"DLJR","balances":[{"tick":"DU$T","available":"1","transferable":"0","balance":"1","decimals":18}]}' },
    "/api/names/resolve/soft.doge": { body: '{"name":"soft.doge","found":true,"valid":true,"holder":"DBTA"}' },
    "/api/explorer/inscriptions/abci0": { body: '{"inscription":{"inscription_id":"abci0","location":{"txid":"abc","vout":0}}}' },
  });
  const ds = new DogeSoft({ fetch: f });
  assert.equal((await ds.drc20("DU$T")).token.holder_count, 139);
  assert.equal((await ds.drc20Holdings("DLJR")).balances[0].tick, "DU$T");
  assert.equal((await ds.resolveName("soft.doge")).holder, "DBTA");
  assert.equal((await ds.inscription("abci0")).inscription.location.vout, 0);
  assert.equal(calls[0].url, "https://explorer.dogesoft.io/api/explorer/drc20/DU%24T");
  assert.equal(ds.contentUrl("abci0"), "https://explorer.dogesoft.io/content/abci0");
});

test("429 backs off (Retry-After) then succeeds", async () => {
  const waits = [];
  const { f, calls } = mock({
    "/api/health": [{ status: 429, headers: { "retry-after": "2" }, body: "slow down" }, { status: 429, body: "slow down" }, { body: '{"status":"healthy"}' }],
  });
  const ds = new DogeSoft({ fetch: f, sleep: async (ms) => waits.push(ms) });
  assert.equal((await ds.health()).status, "healthy");
  assert.equal(calls.length, 3);
  assert.deepEqual(waits, [2000, 2000]);
});

test("429 gives up after maxRetries; other errors throw DogeSoftError", async () => {
  const { f } = mock({ "/api/health": { status: 429, body: "slow down" } });
  const ds = new DogeSoft({ fetch: f, maxRetries: 1, sleep: async () => {} });
  await assert.rejects(ds.health(), (e) => e instanceof DogeSoftError && e.status === 429);
  await assert.rejects(new DogeSoft({ fetch: f }).inscription("nope"), (e) => e.status === 404);
});

test("partner key header", async () => {
  const { f, calls } = mock({ "/api/health": { body: "{}" } });
  await new DogeSoft({ fetch: f, apiKey: "k" }).health();
  assert.equal(calls[0].init.headers["x-api-key"], "k");
});

test("live read of /api/health (set DOGESOFT_LIVE=1)", { skip: !process.env.DOGESOFT_LIVE }, async () => {
  const h = await new DogeSoft().health();
  assert.ok(h.indexed_block_height > 6_000_000);
});
