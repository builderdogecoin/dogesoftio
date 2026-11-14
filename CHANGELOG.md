# Changelog

All notable changes to this repo. Dates are in Atlantic Time.

## Unreleased

- SDK: `drc20Units` and `drc20_units` reject amounts above uint64 max whole tokens, like ds-rules, and `dogeToKoinu` rejects more than uint64 max koinu, like `doge_to_koinu`. Zero is still accepted, because balances can be 0.
- SDK: new `isValidDrc20Amount` (JS) and `is_valid_drc20_amount` (Rust) check a DRC-20 mint or transfer amount against the full ds-rules amount rule: the format, above zero and at most uint64 max.

## v1.0.1 (2026-11-12)

- SDK amount helpers (`dogeToKoinu`, `drc20Units`, `doge_to_koinu`, `drc20_units`) now read amounts with the same format rules as ds-rules: numbers only as strings, digits on both sides of a dot, no whitespace, plus, exponent or comma, and no more fraction digits than the decimals. Invalid formats are rejected, not coerced. They do not check that an amount is above zero (balances can be 0), and in this release `drc20Units`, `drc20_units` and `dogeToKoinu` did not check uint64 max (see Unreleased).

## v1.0.0 (2026-11-12)

- The Doge Soft repo goes public: docs, rules and test vectors, the 6,453,225 proof, dsverify, sdk-js and sdk-rust.
