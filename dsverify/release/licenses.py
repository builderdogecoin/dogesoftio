#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Write THIRD_PARTY_LICENSES.md from `cargo about generate --format json` output (dsverify and
sdk-rust) and the npm production dependencies of sdk-js.

  (cd dsverify && cargo about generate --format json > /tmp/a.json)
  (cd sdk-rust && cargo about generate --format json > /tmp/b.json)
  python3 dsverify/release/licenses.py /tmp/a.json /tmp/b.json sdk-js/package.json > THIRD_PARTY_LICENSES.md
"""
import json
import sys

OWN = {"ds-rules", "ds-core-rpc", "dsverify", "dogesoft"}


def main(a, b, pkg):
    by_text = {}
    crates = {}
    for path, product in ((a, "dsverify"), (b, "sdk-rust")):
        for lic in json.load(open(path))["licenses"]:
            users = [u["crate"] for u in lic["used_by"] if u["crate"]["name"] not in OWN]
            if not users:
                continue
            key = (lic["id"], lic["text"].strip())
            e = by_text.setdefault(key, {"id": lic["id"], "name": lic["name"], "text": lic["text"].strip(), "users": set()})
            for c in users:
                e["users"].add((c["name"], c["version"]))
                crates.setdefault((c["name"], c["version"]), set()).add(product)
    deps = json.load(open(pkg)).get("dependencies", {})
    out = []
    w = out.append
    w("# Third-party licenses\n")
    w("dsverify, sdk-rust and sdk-js use standard open-source libraries from crates.io and npm. Each keeps its own license, listed below. "
      "This file is generated with `cargo about generate --format json` (Rust) and `dsverify/release/licenses.py`; sdk-js reads its production dependencies from `package.json`.\n")
    w("Users run their own stock Dogecoin Core; it is not bundled.\n")
    w("## sdk-js (npm)\n")
    w("No runtime dependencies." if not deps else "\n".join(f"- `{k}` {v}" for k, v in sorted(deps.items())))
    w("")
    w(f"## Rust crates ({len(crates)})\n")
    w("| Crate | Version | License | Used in |")
    w("|---|---|---|---|")
    lic_of = {}
    for e in by_text.values():
        for c in e["users"]:
            lic_of.setdefault(c, set()).add(e["id"])
    for (n, v), prods in sorted(crates.items()):
        w(f"| `{n}` | {v} | {' / '.join(sorted(lic_of.get((n, v), {'?'})))} | {', '.join(sorted(prods))} |")
    w("")
    w("## License texts\n")
    for e in sorted(by_text.values(), key=lambda e: (e["id"], sorted(e["users"]))):
        users = ", ".join(f"`{n} {v}`" for n, v in sorted(e["users"]))
        w(f"### {e['name']} ({e['id']})\n")
        w(f"Used by: {users}\n")
        w("```text")
        w(e["text"].replace("```", "'''"))
        w("```\n")
    sys.stdout.write("\n".join(out))


if __name__ == "__main__":
    main(*sys.argv[1:4])
