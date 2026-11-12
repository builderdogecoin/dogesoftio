# @dogesoft/sdk

**Dogecoin, Doginals, DRC-20 Coins and .doge names in a few lines of JavaScript or TypeScript.**

A small, typed client for the public Doge Soft Explorer API. No node, no key, no indexer of your own, and zero dependencies. Much simple. Very typed.

```bash
npm i @dogesoft/sdk
```

```js
import { DogeSoft, dogeFromKoinu } from "@dogesoft/sdk";

const ds = new DogeSoft(); // https://explorer.dogesoft.io

const health  = await ds.health();                 // indexed_block_height, chain_tip, blocks_behind_tip
const doge    = await ds.balance("D...");          // balance_koinu is a bigint
console.log(dogeFromKoinu(doge.balance_koinu), "DOGE");
const coins   = await ds.drc20Holdings("D...");    // available / transferable as exact strings
const coin    = await ds.drc20("DU$T");            // supply, deploy, holder count
const name    = await ds.resolveName("soft.doge");
const doginal = await ds.inscription("<txid>i0");  // genesis and current location
```

## Features

- Typed responses (`src/index.d.ts`) for health, DOGE balance, DRC-20, inscriptions and names
- Node 18+ and any runtime with `fetch`; ES module; no dependencies
- Every `*koinu*` field is an exact `bigint`, even above 2^53. Helpers: `dogeFromKoinu`, `dogeToKoinu`
- DRC-20 amounts stay exact decimal strings; `drc20Units(amount, decimals)` gives an exact `bigint`
- Polite automatic backoff on HTTP `429` (honours `Retry-After`, `maxRetries` default 3)

## Limits and partner keys

Reads are free. Outside callers are rate limited, so cache what you can; the client backs off on `429` for you. Partners get a key (`new DogeSoft({ apiKey })`, sent as `x-api-key`): DM [@dogesoftio](https://x.com/dogesoftio) on X, or send the [developer request form](https://docs.dogesoft.io/explorer/api).

## Test

```bash
npm test                     # mocked, offline
DOGESOFT_LIVE=1 npm test     # plus one light live read of /api/health
```

Prefer raw HTTP? See [docs/api-quickstart.md](../docs/api-quickstart.md).

License: Apache-2.0.
