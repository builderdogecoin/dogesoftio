# The Doge Soft Orbit

Everything has a job. Everything talks. And everything agrees that Dogecoin Core is the source of truth. GitHub draws this map for you:

```mermaid
mindmap
  root((dogesoftio))
    Core Truth
      Dogecoin Core node
      Full UTXO and sat tracking
      Rust indexer
    Explorer
      explorer.dogesoft.io
      Public API
      Rate limits and partner keys
    Protocols
      DRC-20
      Doginals
      Names p:dns
      Doge Soft Standard ds
      Dogemaps
    Inscriber
      Creates inscriptions
      Used by Launch, Marketplace, Bridge, Origin
    Wallets
      Extension
      Mobile Android
      Mobile Apple
    Marketplace
      Doginals listings
      Names marketplace
      Collections
    Launchpad
      Collection minting
      LAUNCH verbs
    Handles
      dogesites.com
      dogesoft.io/yourhandle
    Bridge
      L2
    Origin
      Attestations
    Social
      X @dogesoftio
      Doge Den
    GitHub
      dsverify CLI
      rules and test vectors
      proofs
      standard spec
      sdk-js and sdk-rust
      docs.dogesoft.io
```

## Who does what

| Piece | Job |
|---|---|
| Dogecoin Core | The chain. The final word on every coin and transaction. |
| Indexer | Watches each block and records the UTXO set, which koinu each inscription rides, and every DRC-20 deploy, mint, transfer and balance. |
| Explorer | The website and read API of that same index. |
| Wallets | Hold the keys and sign on your device. The extension injects `window.dogesoft` for apps. |
| Inscriber | Writes inscriptions and the `ds` tag for Launch, Marketplace, Bridge and Origin. It does not keep the ledger. |
| Marketplace | Listings and offers are signed records; a sale is one Dogecoin transaction with a Doge Soft tag. |
| Launchpad | Collection launches with `LAUNCH_DEPLOY` and `LAUNCH_MINT`. |
| Handles | Your page at dogesoft.io/yourhandle, and building with dogesites.com. |
| Bridge | Moves DRC-20 Coins between Dogecoin L1 and the Doge Soft L2. |
| Origin | Attestations: signed Dogecoin facts, and an overview of how the Doge Soft tech fits together. |
| GitHub | This repo: rules, vectors, proofs, verifier and SDKs. |

Deeper dive: [docs.dogesoft.io/architecture](https://docs.dogesoft.io/architecture).
