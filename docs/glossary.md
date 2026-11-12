# Glossary

Plain English for every Doge Kin. Most of it is standard Dogecoin vocabulary; a few words are Doge Soft being deliberate.

| Word | Plain English |
|---|---|
| **Dogecoin Core** | The official Dogecoin node software. It holds the whole chain and is the final judge of what happened. |
| **Node** | A computer running Dogecoin Core. Your node, your truth. |
| **RPC** | The way a program asks your node questions, like "what's in block 6,453,225?" |
| **txindex** | A Core setting (`txindex=1`) that lets your node look up any transaction by id. dsverify needs it. |
| **Block / height** | Dogecoin adds a block about every minute. Height is its number in the chain. |
| **Transaction (tx)** | A signed message that spends coins from inputs and creates new outputs. |
| **UTXO** | Unspent transaction output: a chunk of DOGE waiting at an address to be spent. Your balance is a pile of these. |
| **Koinu** | The smallest unit of DOGE. 1 DOGE = 100,000,000 koinu. APIs usually return koinu. |
| **Inscription** | Data (image, text, JSON) written into a Dogecoin transaction and tied to one specific koinu, so it can be owned and sent. |
| **Doginal** | A one-of-a-kind inscription on Dogecoin L1, collected like an NFT. |
| **Sat / koinu position** | Which exact koinu carries an inscription. Following it from transaction to transaction tells you who owns it. |
| **Commit / reveal** | The two transactions that create an inscription: the commit funds it, the reveal writes it on chain. |
| **Envelope** | The inscription payload inside the reveal transaction. Doge Soft reads it from input 0 only. |
| **DRC-20 Coin** | A fungible coin defined and moved by plain-text inscriptions: deploy, mint, transfer. No smart contract; the rules decide what counts. |
| **Tick** | A DRC-20 Coin's short name, like `DU$T`. |
| **Transfer lock** | When you inscribe a DRC-20 transfer, that amount is locked until the inscription is sent. |
| **Holder** | An address with a DRC-20 balance or open transfer locks above zero. |
| **.doge name** | A readable name (like `soft.doge`) inscribed as a `p:dns` claim. Whoever holds the inscription owns the name. |
| **Doge Soft Standard (`ds`)** | A tiny tag that says what a transaction is: a sale, a launch, a bridge deposit, a governance action. |
| **Indexer** | A program that reads every block and keeps the results (owners, balances) in a database. |
| **Verifier** | A program that checks an indexer's answer against Core instead of trusting it. That's dsverify. |
| **Test vector** | A real transaction plus the result the rules say it must produce. |
| **Proof / checkpoint** | A published snapshot of our numbers at a block, tied to that block's hash, that anyone can recheck. |
| **L1 / L2** | L1 is Dogecoin mainnet. L2 is the Doge Soft layer the Bridge connects to. |
| **PSBT** | A standard unsigned-transaction format used for marketplace listings and multi-party sales. |
