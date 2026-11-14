# Test vectors

Every edge case the rules care about, with the answer the rules require. `cargo test` in [`dsverify/`](../../dsverify/) runs all of them, and so does:

```bash
dsverify vectors run            # offline
dsverify vectors run --node     # also matches every mainnet transaction byte for byte with your own Core
```

```text
vectors/
  inscriptions/mainnet_reveals.json   12 real mainnet reveals: envelope, content, landing output, owner,
                                      and reveals whose first koinu already carried a Doginal (no new one)
  inscriptions/block_6444515.hex      a real merge-mined block, for block decoding and the merkle check
  envelope/envelope_cases.in          4,000 envelope scripts (one per line: id, then scripts in hex)
  envelope/envelope_cases.expected    the verdict for each: None, Partial, or Complete + content type + body
  koinu/routing_cases.json            4,000 generated blocks: first-in first-out routing, the reinscription
                                      gate, fee spends to the coinbase, lost koinu, zero-value outputs
  drc20/json_bodies.json              86 DRC-20 bodies: the strict JSON reading, numbers, ticks, duplicates
  names/claims.json                   4 real first valid .doge claims, plus 20 claim bodies
  schema.json                         the shape of every file above
```

The mainnet vectors are real Dogecoin transactions; `--node` proves it on your own machine. The envelope corpus, koinu blocks, DRC-20 bodies and name bodies are built to hit edge cases fast, and their expected answers come from the Doge Soft Rust indexer's own rule code.

Shape of one mainnet reveal (trimmed):

```json
{
  "id": "inscription-reveal-ordi_deploy_981d3c56",
  "rule": "spec/01-inscriptions.md#envelope",
  "height": 4708116,
  "txid": "981d3c56...",
  "raw_hex": "0100000001...",
  "inputs": [{ "prev": "...", "vout": 0, "value": "100000" }],
  "expect": { "created": true, "id": "981d3c56...i0", "vout": 0, "off": "0", "kind": 1, "addr": "D..." }
}
```

License: CC0-1.0. Use them to test any code you like, Doge Kin.
