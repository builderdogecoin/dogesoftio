# Contributing to Doge Soft

Much welcome, Doge Kin! Whether you fix a typo, add a test vector or find a DIFF, you make the Orbit stronger.

## Ways to help

- **Report a DIFF.** If dsverify or your own node disagrees with the Doge Soft Explorer, open a bug report with the full output. This is the most valuable thing you can do.
- **Improve the docs.** Clearer words for newcomers are always welcome.
- **Suggest a test vector.** A real mainnet transaction that tests a rule edge case.
- **Code.** dsverify and the SDKs welcome pull requests.

## Before you open a pull request

1. Open an issue first for anything bigger than a small fix, so we can agree on the approach.
2. Keep each pull request to one change.
3. Run the checks for the folder you touched (for Rust: `cargo fmt`, `cargo clippy`, `cargo test --locked`).
4. Write clearly. On Doge Soft they are **DRC-20 Coins**, not tokens, and amounts in APIs are **koinu**.
5. Never include keys, seed phrases, passwords, `.env` files, server addresses or anything secret.
6. Turn on the pre-commit guard once per clone: `git config core.hooksPath .githooks`. It runs [gitleaks](https://github.com/gitleaks/gitleaks) when installed, plus basic checks for keys, RPC logins, home paths and raw IPs.

## Rule changes

The rules decide real balances, so changes are careful: an issue explaining the case, a test vector showing it, and the block height the change applies from. Maintainers decide; every accepted change goes in `rules/CHANGELOG.md`.

## Sign-off and licensing

By contributing you agree your contribution is licensed under the license of the folder you changed (see [LICENSING.md](LICENSING.md)) and that it is your own work. Please sign your commits.

## Be kind

Everyone here follows our [Code of Conduct](CODE_OF_CONDUCT.md). Much respect, very kindness.
