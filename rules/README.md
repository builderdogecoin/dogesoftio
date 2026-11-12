# Protocol rules

**The Doginals and DRC-20 protocol rules exactly as the Doge Soft index follows them, with test cases you can run.**

DRC-20 Coins and Doginals have no on-chain validation, so an index is only as good as how strictly it follows the protocol. Doge Soft follows the Doginals and DRC-20 protocol rules strictly, adds nothing of its own, and shows exactly how each rule is checked against Dogecoin Core so every Doge Kin can test it and hold us to it. Friendly summary: [docs/rules-overview.md](../docs/rules-overview.md).

## Spec

| File | Covers |
|---|---|
| [`spec/01-inscriptions.md`](spec/01-inscriptions.md) | Envelope from input 0 only; reinscription creates nothing |
| [`spec/02-koinu-tracking.md`](spec/02-koinu-tracking.md) | First-in first-out koinu offsets; fee spends go to the coinbase or are lost |
| [`spec/03-drc20.md`](spec/03-drc20.md) | The 15 DRC-20 protocol rules as the index checks them, including rule 11 for fee spends, and the holder rule |
| [`spec/04-burns.md`](spec/04-burns.md) | `OP_RETURN` burns |
| [`spec/05-not-inscriptions.md`](spec/05-not-inscriptions.md) | Broken `cdrc-20` entries; `.dogemap` tiles are not names |
| [`spec/06-chain-source.md`](spec/06-chain-source.md) | dsverify reads the chain from your own Dogecoin Core |

## Test vectors

[`vectors/`](vectors/) holds real mainnet transactions with the result each rule requires. Run them against your own node with `dsverify vectors run`.

## Changes

Every rule change is dated and says the block it applies from, in `CHANGELOG.md`.

## License

Our write-up of the rules: CC-BY-4.0 (share and adapt, credit Doge Soft). The rules themselves belong to the protocols. Vectors: CC0 (use them for anything). See [LICENSING.md](../LICENSING.md).
