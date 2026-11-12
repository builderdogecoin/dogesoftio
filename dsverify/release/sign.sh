#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Sign dist/SHA256SUMS with the Doge Soft release key (detached, ASCII-armored).
# The release key is never stored in this repo. Run on the signing machine, or in CI with the key
# imported from a secret. The published fingerprint is in SECURITY.md.
#
#   ./release/sign.sh <KEY_FINGERPRINT>
set -euo pipefail
cd "$(dirname "$0")/.."
FPR="${1:?usage: release/sign.sh <KEY_FINGERPRINT>}"
test -f dist/SHA256SUMS || { echo "run release/build.sh first" >&2; exit 2; }
gpg --batch --yes --local-user "$FPR" --armor --detach-sign --output dist/SHA256SUMS.asc dist/SHA256SUMS
gpg --batch --verify dist/SHA256SUMS.asc dist/SHA256SUMS
echo "signed dist/SHA256SUMS with $FPR"
