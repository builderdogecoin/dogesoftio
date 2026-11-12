# Security

Keeping every Doge Kin safe comes first.

## Reporting a vulnerability

**Please don't open a public issue.** Report it confidentially through one of these:

- GitHub vulnerability reporting: the **Security** tab of this repo, then **Report a vulnerability**.
- Email: koinu@dogesoft.io

Include what you found, how to reproduce it, and what it affects. We aim to reply within 72 hours and will keep you updated until it's fixed. With your permission we'll thank you by name once it's resolved.

## In scope

- dsverify, the SDKs and anything else in this repo
- Wrong results from the rules or the verifier (for example, a balance the rules count incorrectly)
- Doge Soft public services: explorer.dogesoft.io, docs.dogesoft.io, the wallets and extension

## Never

Doge Soft will never ask for your seed phrase, private key or password, in an issue, a DM or anywhere else. Anyone asking is not us.

## Verifying downloads

Releases come with SHA-256 checksums and signatures. Check them before you run a binary. Signing key fingerprint: `ECCD 069F 5C96 AB76 9A7D  B313 32B3 F85E C536 116B` (Doge Soft Release Signing). The public key is in [`dsverify/release/dogesoft-release-key.asc`](dsverify/release/dogesoft-release-key.asc).
