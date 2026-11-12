# Proofs

**Every figure we publish, tied to a Dogecoin Core block hash.**

## Checkpoint 6,453,225 (DRC-20)

The live Doge Soft index was compared with a strict ledger rebuilt from Dogecoin Core. Result: **zero differences**.

| Compared | Count | Differences |
|---|---:|---:|
| DRC-20 Coins (ticks) | 71,689 | 0 |
| Balances | 1,702,438 | 0 |
| Open transfer locks | 528,006 | 0 |
| Holders (address and Coin pairs) | 1,750,018 | 0 |
| Holder counts | 69,443 coins with holders | 0 |

Core block hash at 6,453,225: `2cc71f47a57624e4d0e47906c6d071628ae7dca6d8ab69035ed261223e80b2df`

## Files

```text
checkpoints/6453225/
  summary.json      height, Core block hash, counts, table format
  balances.sha256   hash of the sorted balance table (tick, owner, available, transferable)
  holders.sha256    hash of the sorted holder-count table
  locks.sha256      hash of the sorted open-transfer-lock table
  tokens.sha256     hash of the sorted Coin table (supply, limit, decimals, minted, deploy)
  diff.json         the zero-diff result
```

## Check it yourself

```bash
dogecoin-cli getblockhash 6453225                      # compare with the hash above
dsverify --db cp6453225.db replay --to 6453225         # your own replay, stopped at the checkpoint (takes hours)
dsverify --db cp6453225.db proof check 6453225         # block hash, then all four table hashes
```

Table format: UTF-8, one record per line, fields separated by a tab, every line ending in a newline, lines sorted by their bytes. Amounts are plain decimals with no trailing zeros (`0` for zero). `dsverify proof export --out <dir>` writes the same tables from your replay, so you can diff them line by line.

License: CC0-1.0.
