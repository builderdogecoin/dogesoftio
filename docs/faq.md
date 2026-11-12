# FAQ

**What is Doge Soft?**
The friendly layer for Dogecoin: wallet, explorer, DRC-20 Coins, Doginals, .doge names, marketplace, launches and a bridge, all reading one Core-verified index.

**Do I need to run a node?**
No, not to use the Explorer, the API or the SDKs. You only need your own Dogecoin Core if you want to verify us with dsverify.

**Does dsverify need a special or modified Dogecoin Core?**
No. Any stock Dogecoin Core with `txindex=1` works.

**What's published here?**
The rules, test vectors, proofs, dsverify and the SDKs. Together they let you check every answer the Doge Soft Rust indexer gives against your own node.

**What does "Core-verified" mean?**
Every number we publish can be traced to a Dogecoin Core block and transaction. At block 6,453,225 our DRC-20 index had zero differences against a strict ledger rebuilt from Core.

**Why "coins" and not "tokens"?**
DRC-20 Coins are inscribed directly on Dogecoin L1 using the same UTXO model as DOGE. No contract, no IOU.

**Is the API free?**
Reads are free and need no key. Outside callers are rate limited (per IP, per minute and per day), so cache what you can and back off on HTTP `429`. Building something bigger? Partners get a key: DM [@dogesoftio](https://x.com/dogesoftio) on X, or send the [developer access request](https://docs.dogesoft.io/explorer/api) form (`POST /api/public/developer-request`). Tell us about your project and we'll get you sorted, Doge Kin.

**I found a DIFF. What now?**
Much thanks! Open a bug report with the full command output. If it's real, we fix it and record it in the rules changelog.

**Will Doge Soft ever ask for my seed phrase?**
Never. Nothing from Doge Soft will ask for a seed phrase, private key or password to "verify", "sync" or "unlock rewards". If something does, it isn't us.

**Can I build on the Doge Soft Standard?**
Yes. The spec is open (CC-BY-4.0). Build away and credit Doge Soft.

**Where do I ask for help?**
See [SUPPORT.md](../SUPPORT.md).
