#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Build every dsverify release archive, its SHA-256 checksums and a CycloneDX SBOM.
# Reproducible: same source + same pinned toolchain (rust-toolchain.toml) + same zig and cargo-zigbuild
# versions below give byte-identical binaries, whatever directory you build in.
#
#   pip install ziglang==0.17.1 && cargo install cargo-zigbuild --version 0.24.1 --locked
#   cargo install cargo-cyclonedx --version 0.5.10 --locked
#   ./release/build.sh            # from the dsverify/ directory; output in dist/
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)"
SRC="$(pwd)"
export SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH:-$(git log -1 --format=%ct 2>/dev/null || echo 1794355200)}"
export CARGO_INCREMENTAL=0
# No build paths in the binary: remap the source tree and the cargo home.
export RUSTFLAGS="--remap-path-prefix=${SRC}=/dsverify --remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo -C strip=symbols"

declare -A NAMES=(
  [x86_64-unknown-linux-musl]=linux-x64
  [aarch64-unknown-linux-musl]=linux-arm64
  [x86_64-apple-darwin]=macos-x64
  [aarch64-apple-darwin]=macos-arm64
  [x86_64-pc-windows-gnu]=windows-x64
)
rm -rf dist && mkdir -p dist/stage
for t in x86_64-unknown-linux-musl aarch64-unknown-linux-musl x86_64-apple-darwin aarch64-apple-darwin x86_64-pc-windows-gnu; do
  cargo zigbuild --release --locked --target "$t" -p dsverify
  n="dsverify-${NAMES[$t]}"
  d="dist/stage/$n"
  mkdir -p "$d"
  if [[ $t == *windows* ]]; then cp "target/$t/release/dsverify.exe" "$d/"; else cp "target/$t/release/dsverify" "$d/"; fi
  cp ../LICENSE README.md "$d/"
  # fixed timestamps, owners and order so the archives are reproducible too
  find "$d" -exec touch -h -d "@${SOURCE_DATE_EPOCH}" {} +
  if [[ $t == *windows* ]]; then
    python3 - "$n" "$SOURCE_DATE_EPOCH" <<'PY'
import os, sys, time, zipfile
n, epoch = sys.argv[1], int(sys.argv[2])
dt = time.gmtime(max(epoch, 315532800))[:6]
with zipfile.ZipFile(f"dist/{n}.zip", "w", zipfile.ZIP_DEFLATED, compresslevel=9) as z:
    for root, _, files in sorted(os.walk(f"dist/stage/{n}")):
        for f in sorted(files):
            p = os.path.join(root, f)
            i = zipfile.ZipInfo(os.path.relpath(p, "dist/stage"), dt)
            i.external_attr = (0o755 if f.endswith(".exe") else 0o644) << 16
            i.compress_type = zipfile.ZIP_DEFLATED
            z.writestr(i, open(p, "rb").read(), compresslevel=9)
PY
  else
    tar --sort=name --owner=0 --group=0 --numeric-owner --mtime="@${SOURCE_DATE_EPOCH}" -C dist/stage -cf - "$n" | gzip -n -9 > "dist/$n.tar.gz"
  fi
done
rm -rf dist/stage
cargo cyclonedx --format json --target all >/dev/null
# drop local paths and the wall-clock time so the SBOM is reproducible too
python3 - "$SRC" "$SOURCE_DATE_EPOCH" crates/dsverify/dsverify.cdx.json "dist/dsverify-${VERSION}.cdx.json" <<'PY'
import json, sys, time
src, epoch, i, o = sys.argv[1], int(sys.argv[2]), sys.argv[3], sys.argv[4]
t = open(i).read().replace("path+file://" + src, "path+file:///dsverify")
d = json.loads(t)
d.setdefault("metadata", {})["timestamp"] = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(epoch))
open(o, "w").write(json.dumps(d, indent=2, sort_keys=True) + "\n")
PY
find crates -name '*.cdx.json' -delete
(cd dist && sha256sum dsverify-* | LC_ALL=C sort -k2 > SHA256SUMS)
cat dist/SHA256SUMS
