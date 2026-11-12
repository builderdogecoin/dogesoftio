# Rules overview

DRC-20 and Doginals have no on-chain validation. Dogecoin happily confirms a malformed or over-limit inscription; it's the **indexer** that decides what counts. Doge Soft follows the Doginals and DRC-20 protocol rules strictly, adds nothing of its own, and shows exactly how each one is checked so anyone can test it. This page is the friendly summary; the precise spec is in [`rules/`](../rules/).

## Where history starts

The index reads every block with full history, straight from Dogecoin Core.

## Inscriptions (Doginals)

How the index follows the Doginals protocol:

| Rule | In plain English |
|---|---|
| Envelope from input 0 only | An inscription is read from the first input of the reveal transaction. Data hidden in other inputs doesn't create one. |
| Reinscription creates nothing | If the koinu being inscribed already carries an inscription, the new envelope does not create a second one. |
| Koinu tracking | Inscriptions ride a specific koinu. When it moves, outputs are filled in order (first in, first out), so we always know which output carries it. |
| Spent as a fee | If an inscription's koinu ends up in the fee, it goes to the block's coinbase (the miner's reward transaction), or is lost if the coinbase can't hold it. |
| OP_RETURN burns | Sending an inscription into an `OP_RETURN` output burns it. |
| Not inscriptions | Broken `cdrc-20` entries are not inscriptions. `.dogemap` tiles are not names. |

## DRC-20 Coins

Doge Soft follows the DRC-20 protocol rules strictly. Every deploy, mint and transfer is checked against all **14 protocol rules**, in order:

1. **Exactly 4 bytes.** `tick` is exactly 4 bytes of UTF-8, measured on the raw string as inscribed. A tick with a NUL byte is skipped. A leading or trailing space is a real byte of the tick, so `" abc"` and `"abc "` are valid and distinct from each other and from `"abc"`.
2. **Case doesn't matter.** Ticks are case-insensitive and stored lowercase (plain Unicode lowercase, letter by letter), after rule 1. `DOGI`, `Dogi` and `dogi` are one Coin.
3. **Look-alikes are never merged.** `𝕏`, `𝑿`, `ｘ` and `x` are different ticks, and so are `d0ge` and `doge`.
4. **Strict numbers.** `max`, `lim`, `amt` and `dec` are JSON strings. Amounts are ASCII digits with an optional `.` and digits on both sides (`1.`, `.5`, `1e8`, `+1` and spaces are rejected; `007` is fine). At most 18 decimals, never more than the Coin's `dec`, above zero, and no more than 18,446,744,073,709,551,615. `dec` is 18 when absent, otherwise a string from `"0"` to `"18"`.
5. **First deploy wins.** The first valid deploy of a tick, in chain order, creates the Coin.
6. **Invalid deploys are skipped.** A broken deploy doesn't block the tick; the next valid deploy can still win.
7. **Deploy before mint.** A mint only counts after a valid deploy, in chain order.
8. **Respect the limit.** A mint with `amt` above `lim` gets nothing.
9. **Max supply is max supply.** The mint that crosses `max` gets only the remainder. Once `max` is reached, mints get nothing.
10. **Transfers spent as a fee.** A transfer inscription spent as a fee when it is inscribed is ignored. One spent as a fee when it is sent goes back to the sender. Details below.
11. **Inscribe, then send.** A transfer counts only when it is inscribed with enough available balance, then sent. Each transfer inscription works once; later moves change nothing.
12. **Balances come from the chain.** Only valid mints and sent transfers change balances, in chain order.
13. **Holders.** An address holds a Coin while its available plus transferable balance is above zero.
14. **`doge` is not a DRC-20 Coin.** The exact tick `doge` (any case) is never listed. Its look-alikes are their own ticks.

- **Deploy**: one valid deploy per tick; the first inscribed wins.
- **Mint**: up to the per-mint limit, until the max supply is used up.
- **Transfer**: a two-step inscribe-then-send. Inscribing earmarks (locks) an amount; sending the inscription moves it.
- **Rule 10, transfers spent as a fee**: a transfer inscription spent as a fee when it is inscribed is ignored, and no balance changes. One spent as a fee when it is sent goes back to the sender's available balance. Nobody else gets it.
- **Holders**: an address is a holder while its available balance or its open transfer locks are above zero. The holder count changes only when an address's total crosses zero.

## How we know it's right

At block 6,453,225 the live index had zero differences against a strict ledger rebuilt from Dogecoin Core: 71,689 coins, 1,702,438 balances, 528,006 open locks, and every holder count. See [`proofs/`](../proofs/).
