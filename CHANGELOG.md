# Changelog

All notable changes to this repo. Dates are in Atlantic Time.

## v1.0.1 (2026-11-12)

- SDK amount helpers (`dogeToKoinu`, `drc20Units`, `doge_to_koinu`, `drc20_units`) now read amounts the same way ds-rules does: numbers only as strings, digits on both sides of a dot, no whitespace, plus, exponent or comma, and no more fraction digits than the decimals. Invalid amounts are rejected, not coerced.

## v1.0.0 (2026-11-12)

- The Doge Soft repo goes public: docs, rules and test vectors, the 6,453,225 proof, dsverify, sdk-js and sdk-rust.
