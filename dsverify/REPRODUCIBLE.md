# Reproducible builds

Every dsverify release can be rebuilt from this source and checked byte for byte against the published `SHA256SUMS`. Much verify. Very same bytes.

## What is pinned

| Piece | Version | Where |
|---|---|---|
| Rust toolchain | 1.99.1 | `rust-toolchain.toml` |
| Dependencies | exact versions | `Cargo.lock`, built with `--locked` |
| Cross linker | zig 0.17.1 (`pip install ziglang==0.17.1`) | `release/build.sh` |
| cargo-zigbuild | 0.24.1 | `release/build.sh` |
| cargo-cyclonedx (SBOM) | 0.5.10 | `release/build.sh` |
| Timestamps | `SOURCE_DATE_EPOCH` = time of the release commit | `release/build.sh` |

`release/build.sh` also strips symbols, remaps the source and cargo paths out of the binaries, and writes the archives with fixed timestamps, owners and file order.

## Rebuild a release

```bash
git clone https://github.com/builderdogecoin/dogesoftio /build/dogesoftio
cd /build/dogesoftio
git checkout v1.0.0
cd dsverify
./release/build.sh
diff dist/SHA256SUMS <(curl -sL https://github.com/builderdogecoin/dogesoftio/releases/download/v1.0.0/SHA256SUMS)
```

Linux, Windows and SBOM outputs are identical from any directory. For the macOS builds, the linker derives the binary's UUID from build paths, so clone to `/build/dogesoftio` (the path the release workflow uses) to get identical macOS archives.

## Check the signature

```bash
gpg --import dsverify/release/dogesoft-release-key.asc
gpg --verify SHA256SUMS.asc SHA256SUMS
sha256sum --ignore-missing -c SHA256SUMS
```

The release signing key fingerprint is listed in [SECURITY.md](../SECURITY.md).
