# .doge names

A name like `soft.doge` is a text inscription on Dogecoin. Whoever holds the inscription owns the name. No accounts, no renewals, no expiry. The chain is the authority.

## The rules

- Names are `p:dns` inscriptions held in your own wallet.
- **First valid claim wins**, ordered by block height, then position in the block. Later claims are ignored.
- Ownership follows the inscription. Send the inscription, send the name.
- Holders are re-checked against the unspent coin set before any sale or record update.
- Doge Soft never holds names, keys or coins for you.

## Get one

Register, manage Pay and URL records, and buy or sell names at **[registrar.dogesoft.io](https://registrar.dogesoft.io)**. Endings include `.doge`, `.dogecoin`, `.doginals`, `.soft` and `.xd`.

## Look one up

```bash
curl https://explorer.dogesoft.io/api/names/resolve/soft.doge
curl https://explorer.dogesoft.io/api/names/address/<address>
curl "https://explorer.dogesoft.io/api/names/search?q=soft"
curl https://explorer.dogesoft.io/api/names/stats
```

## Verify one

```bash
dsverify replay          # once; later runs only add new blocks
dsverify name soft.doge --api https://explorer.dogesoft.io
```

More: [docs.dogesoft.io/names/registrar](https://docs.dogesoft.io/names/registrar).
