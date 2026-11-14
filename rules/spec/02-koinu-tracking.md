# 02. Koinu tracking

An inscription rides one koinu. When a transaction spends it:

1. Input koinu are lined up in input order.
2. Outputs are filled from that line in output order (first in, first out).
3. The inscription lands in the output whose range covers its koinu, at the matching offset.

## Spent as a fee {#fee}

If the koinu falls past the last output, it is part of the fee. It goes to the block's coinbase transaction, or is **lost** if the coinbase can't hold it.

Its place in the coinbase is `reward_before_tx + X`. `X` is the koinu's offset in the spending transaction's inputs, and `reward_before_tx` is the block subsidy plus the fees of every earlier non-coinbase transaction in the block. If the coinbase outputs end before that place, the inscription is lost.

This follows ord-dogecoin, the reference Doginals indexer, on purpose. Upstream ord (Bitcoin) later changed the place to `reward_before_tx + X - total_out`, where `total_out` is the spending transaction's output total ([ordinals/ord#1971](https://github.com/ordinals/ord/pull/1971)). ord-dogecoin never took that change, and following it would move or un-lose fee-spent inscriptions that Doginals indexes already place.

Test vectors: `vectors/inscriptions/`.
