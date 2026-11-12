# Checkpoint 6,453,225

| File | What it holds |
|---|---|
| `summary.json` | Height, Core block hash, coin, balance and lock counts |
| `balances.sha256` | SHA-256 of the sorted balance table |
| `holders.sha256` | SHA-256 of the sorted holder-count table |
| `locks.sha256` | SHA-256 of the sorted open-transfer-lock table |
| `tokens.sha256` | SHA-256 of the sorted Coin table |
| `diff.json` | The comparison result: zero differences |

Block 6,453,225 is `2cc71f47a57624e4d0e47906c6d071628ae7dca6d8ab69035ed261223e80b2df`.

Recheck it: `dsverify replay --to 6453225`, then `dsverify proof check 6453225`.

A full replay to this height takes hours on a home machine, and it is the check that needs no trust at all:
`dsverify replay --to 6453225`, then `dsverify proof export --out <folder>` writes the four sorted tables from your own replay,
and their SHA-256 must match the `.sha256` files here.
