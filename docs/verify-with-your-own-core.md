# Verify with your own Core

Don't trust. Verify. dsverify asks **your** Dogecoin Core node, follows our published [rules](../rules/), and tells you whether a Doge Soft answer is right: `PASS` or `DIFF`.

## What you need

1. **Dogecoin Core**, the stock release. Nothing modified, nothing extra to install in Core.
2. These lines in `dogecoin.conf`:

   ```ini
   server=1
   txindex=1
   ```

   With `server=1`, Core writes a `.cookie` login file that dsverify reads by itself. If you set `rpcuser` and `rpcpassword` instead, dsverify reads those from `dogecoin.conf`. It never prints them.

   Turning on `txindex` on an existing node needs a one-time reindex (`-reindex`), which takes a while.
3. A fully synced, unpruned node. dsverify tells you if it isn't.
4. For DRC-20 Coins and names: free disk for the local replay (a few tens of GB) and some patience for the first run.

Where `dogecoin.conf` lives:

| System | Path |
|---|---|
| Windows | `%APPDATA%\Dogecoin\dogecoin.conf` |
| Linux | `~/.dogecoin/dogecoin.conf` |
| macOS | `~/Library/Application Support/Dogecoin/dogecoin.conf` |

## Install dsverify

Prebuilt, signed binaries for Windows, Linux and macOS are on the [Releases](https://github.com/builderdogecoin/dogesoftio/releases/latest) page, each with a SHA-256 checksum.

**Windows**
1. Download `dsverify-windows-x64.zip` from Releases and unzip it.
2. Open PowerShell in that folder.
3. Run `.\dsverify.exe doctor`

**Linux / macOS**
```bash
tar -xzf dsverify-<platform>.tar.gz
./dsverify doctor
```

Check the download first: `sha256sum dsverify-*` (Linux), `shasum -a 256 dsverify-*` (macOS) or `Get-FileHash .\dsverify-windows-x64.zip` (PowerShell), and compare with the checksum on the release.

## Your first checks

```bash
dsverify doctor                                  # is my node ready?
dsverify inscription <txid>i0 --api https://explorer.dogesoft.io
dsverify vectors run --node                      # every published edge case
dsverify replay                                  # build your own DRC-20 and names state (first run takes hours)
dsverify drc20 balance <address> <tick> --api https://explorer.dogesoft.io
dsverify drc20 holders <tick> --api https://explorer.dogesoft.io
dsverify name soft.doge --api https://explorer.dogesoft.io
dsverify proof check 6453225                     # recheck our published proof
```

Exit codes: `0` PASS, `1` DIFF, `2` could not check (for example, node not synced).

## What PASS means

- **Inscriptions**: dsverify walks the inscription's koinu back, hop by hop, to the reveal transaction's envelope in input 0, and confirms with your node that the current output is unspent.
- **DRC-20**: your own replay, built only from your Core's blocks with the open rules, gives the same balance or holder count. The first replay takes hours; after that it only adds new blocks. It trusts nothing, not even our checkpoint.
- **Names**: your replay found the same first valid claim, and the claim inscription sits in an unspent output owned by the same address.
- **Checkpoint**: `proof check` confirms the block hash with your Core, then hashes your replay's tables at that height and compares them with ours.

If the API is a few blocks ahead of your replay, a fresh balance can differ for a moment. dsverify prints both heights so you can tell.

## Found a DIFF?

Wonderful. Open a [bug report](../.github/ISSUE_TEMPLATE/bug_report.yml) with the full command output. Confirmed issues are fixed and recorded in the rules changelog.
