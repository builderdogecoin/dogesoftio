# 02. Koinu tracking

An inscription rides one koinu. When a transaction spends it:

1. Input koinu are lined up in input order.
2. Outputs are filled from that line in output order (first in, first out).
3. The inscription lands in the output whose range covers its koinu, at the matching offset.

## Spent as a fee {#fee}

If the koinu falls past the last output, it is part of the fee. It goes to the block's coinbase transaction, or is **lost** if the coinbase can't hold it.

Test vectors: `vectors/inscriptions/`.
