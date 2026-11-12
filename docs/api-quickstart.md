# API quickstart

The Doge Soft Explorer API is the same index the Explorer, the wallet and the marketplace read. Reads are public: no key, no signup. All paths are relative to `https://explorer.dogesoft.io`. Full reference: [docs.dogesoft.io/explorer/api](https://docs.dogesoft.io/explorer/api).

## Health first

```bash
curl https://explorer.dogesoft.io/api/health
```

Look at `indexed_block_height`, `chain_tip` and `blocks_behind_tip`. If the index is catching up, wait a moment before treating "not found" as final.

## Copy-paste recipes

```bash
# DOGE balance, spendable UTXOs and history for an address (amounts in koinu)
curl https://explorer.dogesoft.io/api/wallet/address/<address>/balance
curl https://explorer.dogesoft.io/api/wallet/address/<address>/utxos
curl https://explorer.dogesoft.io/api/wallet/address/<address>/history

# DRC-20 Coins held by an address
curl https://explorer.dogesoft.io/api/wallet/address/<address>/drc20

# One DRC-20 Coin: supply, deploy, holder count ($ is %24 in a URL)
curl "https://explorer.dogesoft.io/api/explorer/drc20/DU%24T"

# A transaction, with parsed DRC-20 operations and inscriptions
curl https://explorer.dogesoft.io/api/explorer/tx/<txid>

# A Doginal: metadata, then the content itself
curl https://explorer.dogesoft.io/api/explorer/inscriptions/<txid>i0
curl -o doginal.bin https://explorer.dogesoft.io/content/<txid>i0

# .doge names
curl https://explorer.dogesoft.io/api/names/resolve/soft.doge
curl "https://explorer.dogesoft.io/api/names/search?q=soft"

# Search anything: txid, inscription id, address or height
curl "https://explorer.dogesoft.io/api/explorer/search?q=<anything>"
```

## In JavaScript (Node 18+)

```js
const API = "https://explorer.dogesoft.io/api";
const get = (path) => fetch(API + path).then((r) => r.json());

const health = await get("/health");
console.log("indexed:", health.indexed_block_height, "tip:", health.chain_tip);

const coins = await get("/wallet/address/D.../drc20");
console.log(coins);
```

Prefer a typed client? Use the official SDKs: [sdk-js](../sdk-js/) and [sdk-rust](../sdk-rust/).

## Fair use and limits

Reads are free and need no key. Outside callers are rate limited per visitor IP, on API calls (`/api/...`) and on inscription content (`/content/...`), with both a per-minute and a daily allowance. Doge Soft's own apps are not affected.

Over the limit, you get HTTP `429` with a friendly message and a `Retry-After` header. Wait that long and retry; the official SDKs and dsverify do this for you. Cache what you can, and poll `/api/health` rather than hammering a route. Call the API from your backend; browser requests from other websites aren't enabled by default.

**Need more, or browser access from your own domain?** Partners get a key: DM [@dogesoftio](https://x.com/dogesoftio) on X, or send the [developer access request](https://docs.dogesoft.io/explorer/api) form (`POST /api/public/developer-request`). Tell us about your project and we'll get you sorted, Doge Kin.

## Good manners

- Amounts are in **koinu** (1 DOGE = 100,000,000 koinu).
- A DRC-20 balance can lag a fresh transfer by a moment; poll `/api/health` and re-check before deciding a transfer failed.
- Broadcast (`POST /api/tx/broadcast`) takes fully signed hex only and is not a free public relay.
