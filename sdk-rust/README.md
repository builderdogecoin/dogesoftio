# dogesoft (Rust)

**An async Rust client for the Doge Soft Explorer API: Dogecoin, Doginals, DRC-20 Coins and .doge names.**

```bash
cargo add dogesoft
cargo add tokio --features macros,rt-multi-thread
```

```rust
#[tokio::main]
async fn main() -> Result<(), dogesoft::Error> {
    let ds = dogesoft::Client::default(); // https://explorer.dogesoft.io
    let health = ds.health().await?;
    println!("indexed {} / tip {}", health.indexed_block_height, health.chain_tip);

    let doge = ds.balance("D...").await?; // balance_koinu: u64
    println!("{} DOGE", doge.doge());

    for c in ds.drc20_holdings("D...").await?.balances {
        println!("{} available {} transferable {}", c.tick, c.available, c.transferable);
    }
    Ok(())
}
```

## Features

- `Client::default()`, `health`, `balance`, `drc20_holdings`, `drc20`, `resolve_name`, `inscription`, `content_url`
- DOGE amounts in koinu (`u64`), helpers `koinu_to_doge` and `doge_to_koinu`
- DRC-20 amounts as exact decimal strings, `drc20_units` for an exact `u128`, `is_valid_drc20_amount` for a mint or transfer amount
- Polite automatic backoff on HTTP `429` (honours `Retry-After`, `max_retries` default 3)
- rustls TLS, no OpenSSL

## Limits and partner keys

Reads are free. Outside callers are rate limited, so cache what you can; the client backs off on `429` for you. Partners get a key (`Client::default().with_api_key(..)`, sent as `x-api-key`): DM [@dogesoftio](https://x.com/dogesoftio) on X, or send the [developer request form](https://docs.dogesoft.io/explorer/api).

## Test

```bash
cargo test                   # local mock server, offline
cargo test -- --ignored      # one light live read of /api/health
```

License: Apache-2.0.
