// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 Doge Soft
//! dsverify: check Doge Soft answers for Doginals, DRC-20 Coins and .doge names against your own
//! Dogecoin Core. Exit code 0 = PASS, 1 = DIFF, 2 = could not check.

mod api;
mod db;
mod proof;
mod replay;
mod walk;

use clap::{Parser, Subcommand};
use ds_core_rpc::{Client, RpcConfig};
use ds_rules::drc20::{fmt_amount, parse_scaled, tick_key};
use ds_rules::names::normalize;
use ds_rules::owner::owner_of_script;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const DEFAULT_API: &str = "https://explorer.dogesoft.io";

#[derive(Parser)]
#[command(name = "dsverify", version, about = "Check Doge Soft answers against your own Dogecoin Core. Exit 0 PASS, 1 DIFF, 2 could not check.")]
struct Cli {
    /// Dogecoin Core data directory (default: the standard one for your system)
    #[arg(long, global = true)]
    datadir: Option<PathBuf>,
    /// Path to dogecoin.conf (default: <datadir>/dogecoin.conf)
    #[arg(long, global = true)]
    conf: Option<PathBuf>,
    /// RPC URL (default: from dogecoin.conf, else http://127.0.0.1:22555/)
    #[arg(long, global = true)]
    rpc_url: Option<String>,
    /// Local replay database (default: dsverify/state.db next to Core's data directory)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Check the RPC login, mainnet, sync and txindex
    Doctor,
    /// Where a Doginal is and who owns it, traced back to its reveal
    Inscription {
        /// Inscription id, `<txid>i0`
        id: String,
        /// Compare with this API (default https://explorer.dogesoft.io) and take the claimed location from it
        #[arg(long, num_args = 0..=1, default_missing_value = DEFAULT_API)]
        api: Option<String>,
        /// Check this location instead: TXID:VOUT[:OFFSET]
        #[arg(long)]
        at: Option<String>,
        #[arg(long, default_value_t = 100_000)]
        max_hops: usize,
    },
    /// DRC-20 Coins, from your local replay
    Drc20 {
        #[command(subcommand)]
        cmd: Drc20Cmd,
    },
    /// A .doge name's first valid claim and current holder, from your local replay
    Name {
        name: String,
        #[arg(long, num_args = 0..=1, default_missing_value = DEFAULT_API)]
        api: Option<String>,
    },
    /// Replay the chain from your own Core into the local database (resumable, then incremental)
    Replay {
        /// Stop at this height
        #[arg(long)]
        to: Option<u64>,
        /// Stay this many blocks behind the tip
        #[arg(long, default_value_t = 12)]
        confirmations: u64,
        /// Blocks per RPC batch and commit
        #[arg(long, default_value_t = 50)]
        batch: u64,
    },
    /// The published test vectors
    Vectors {
        #[command(subcommand)]
        cmd: VectorsCmd,
    },
    /// Published checkpoints in proofs/
    Proof {
        #[command(subcommand)]
        cmd: ProofCmd,
    },
}

#[derive(Subcommand)]
enum Drc20Cmd {
    /// Available plus transferable balance of one address
    Balance {
        address: String,
        tick: String,
        #[arg(long, num_args = 0..=1, default_missing_value = DEFAULT_API)]
        api: Option<String>,
    },
    /// Holder count of a Coin
    Holders {
        tick: String,
        #[arg(long, num_args = 0..=1, default_missing_value = DEFAULT_API)]
        api: Option<String>,
    },
    /// Deploy, supply and mint progress of a Coin
    Token { tick: String },
}

#[derive(Subcommand)]
enum VectorsCmd {
    /// Run every vector; with --node also confirm the mainnet ones byte for byte against Core
    Run {
        #[arg(long, default_value = "rules/vectors")]
        dir: PathBuf,
        #[arg(long)]
        node: bool,
    },
}

#[derive(Subcommand)]
enum ProofCmd {
    /// Check a published checkpoint: the block hash against Core, and the state hashes against your replay
    Check {
        height: u64,
        #[arg(long, default_value = "proofs/checkpoints")]
        dir: PathBuf,
    },
    /// Write the canonical listings of your replay (to compare or publish)
    Export {
        #[arg(long)]
        out: PathBuf,
    },
}

const PASS: u8 = 0;
const DIFF: u8 = 1;
const UNSURE: u8 = 2;

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(c) => ExitCode::from(c),
        Err(e) => {
            eprintln!("dsverify: {e}");
            ExitCode::from(UNSURE)
        }
    }
}

fn rpc(cli: &Cli) -> Result<Client, String> {
    let cfg = RpcConfig::discover(cli.datadir.as_deref(), cli.conf.as_deref(), cli.rpc_url.as_deref()).map_err(|e| e.to_string())?;
    Ok(Client::new(cfg))
}

fn db_path(cli: &Cli) -> PathBuf {
    if let Some(p) = &cli.db {
        return p.clone();
    }
    let base = cli.datadir.clone().or_else(ds_core_rpc::default_datadir).unwrap_or_else(|| PathBuf::from("."));
    base.join("dsverify").join("state.db")
}

fn verdict(same: bool) -> &'static str {
    if same {
        "PASS"
    } else {
        "DIFF"
    }
}

fn local_height(d: &db::Db) -> String {
    d.height().map(|h| h.to_string()).unwrap_or_else(|| "none".into())
}

fn run(cli: &Cli) -> Result<u8, String> {
    match &cli.cmd {
        Cmd::Doctor => doctor(cli),
        Cmd::Inscription { id, api, at, max_hops } => {
            let c = rpc(cli)?;
            let claim = match (at, api) {
                (Some(a), _) => walk::parse_at(a)?,
                (None, Some(u)) => walk::claim_from_api(&api::Api::new(u), id)?,
                (None, None) => walk::claim_from_api(&api::Api::new(DEFAULT_API), id).map(|mut c| {
                    c.owner = None;
                    c
                })?,
            };
            let o = walk::run(&c, id, &claim, *max_hops)?;
            Ok(if o.undecided {
                UNSURE
            } else if o.ok {
                PASS
            } else {
                DIFF
            })
        }
        Cmd::Replay { to, confirmations, batch } => {
            let c = rpc(cli)?;
            let p = db_path(cli);
            let mut d = db::Db::open(&p)?;
            match d.height() {
                Some(h) => eprintln!("replay  database {}  resuming after {h}", p.display()),
                None => eprintln!("replay  database {}  new", p.display()),
            }
            let h = replay::run(&mut d, &c, &replay::Opts { to: *to, confirmations: *confirmations, batch: (*batch).max(1), quiet: false })?;
            println!("replay  done at height {h}  block {}", d.block_hash(h).unwrap_or_default());
            Ok(PASS)
        }
        Cmd::Drc20 { cmd } => drc20(cli, cmd),
        Cmd::Name { name, api } => name_cmd(cli, name, api.as_deref()),
        Cmd::Vectors { cmd: VectorsCmd::Run { dir, node } } => vectors(cli, dir, *node),
        Cmd::Proof { cmd } => proof_cmd(cli, cmd),
    }
}

fn doctor(cli: &Cli) -> Result<u8, String> {
    let cfg = RpcConfig::discover(cli.datadir.as_deref(), cli.conf.as_deref(), cli.rpc_url.as_deref()).map_err(|e| e.to_string())?;
    println!("  rpc      {}  login from {}", cfg.url, cfg.auth_source);
    let c = Client::new(cfg);
    let mut ok = true;
    let info = c.blockchain_info().map_err(|e| format!("Core: {e}"))?;
    let chain = info["chain"].as_str().unwrap_or("?");
    println!("  chain    {chain}  {}", verdict(chain == "main"));
    ok &= chain == "main";
    let (b, hd) = (info["blocks"].as_u64().unwrap_or(0), info["headers"].as_u64().unwrap_or(0));
    let synced = hd > 0 && hd.saturating_sub(b) <= 2;
    println!("  synced   blocks {b}  headers {hd}  {}", if synced { "OK" } else { "still syncing" });
    ok &= synced;
    let pruned = info["pruned"].as_bool().unwrap_or(false);
    println!("  pruned   {pruned}  {}", if pruned { "dsverify needs an unpruned node" } else { "OK" });
    ok &= !pruned;
    // txindex: an old confirmed transaction outside any wallet must be readable
    let probe = "0bd32d69ca2221f3fc34d99aa14bccc2af10eedc7514770ae842ab9a72468743";
    let tx = c.raw_tx(probe).is_ok();
    println!("  txindex  {}", if tx { "OK" } else { "off: add txindex=1 to dogecoin.conf and restart Core (it reindexes once)" });
    ok &= tx;
    let p = db_path(cli);
    if p.exists() {
        let d = db::Db::open(&p)?;
        println!("  replay   {}  at height {}", p.display(), local_height(&d));
    } else {
        println!("  replay   none yet at {}  (run `dsverify replay` for DRC-20 and names)", p.display());
    }
    Ok(if ok { PASS } else { UNSURE })
}

fn drc20(cli: &Cli, cmd: &Drc20Cmd) -> Result<u8, String> {
    let d = db::Db::open_existing(&db_path(cli))?;
    let h = local_height(&d);
    match cmd {
        Drc20Cmd::Balance { address, tick, api } => {
            let k = tick_key(tick);
            let b = d.balance_row(&k, address)?;
            println!("  local    {k}  {address}  available {}  transferable {}  (replay height {h})", fmt_amount(b.avail), fmt_amount(b.trans));
            let Some(u) = api else { return Ok(PASS) };
            let v = api::Api::new(u).get(&format!("/api/wallet/address/{}/drc20", api::seg(address)))?;
            let row = v["balances"]
                .as_array()
                .and_then(|a| a.iter().find(|r| r["tick"].as_str().map(tick_key).as_deref() == Some(k.as_str())))
                .cloned()
                .unwrap_or(Value::Null);
            let num = |x: &Value| x.as_str().and_then(parse_scaled).or_else(|| x.as_u64().map(|n| n as u128 * ds_rules::drc20::SCALE)).unwrap_or(0);
            let (aa, at) = (num(&row["available"]), num(&row["transferable"]));
            let same = aa == b.avail && at == b.trans;
            println!("  API says available {}  transferable {}  {}", fmt_amount(aa), fmt_amount(at), verdict(same));
            api_height_note(u);
            Ok(if same { PASS } else { DIFF })
        }
        Drc20Cmd::Holders { tick, api } => {
            let k = tick_key(tick);
            let n = d.holder_count(&k)?;
            println!("  local    {k}  holders {n}  (replay height {h})");
            let Some(u) = api else { return Ok(PASS) };
            let v = api::Api::new(u).get(&format!("/api/explorer/drc20/{}", api::seg(&k)))?;
            let a = v["token"]["holder_count"].as_u64().ok_or("API answer has no holder_count")?;
            println!("  API says holders {a}  {}", verdict(a == n));
            api_height_note(u);
            Ok(if a == n { PASS } else { DIFF })
        }
        Drc20Cmd::Token { tick } => {
            let k = tick_key(tick);
            match d.token_row(&k)? {
                None => {
                    println!("  local    {k}  not deployed (replay height {h})");
                    Ok(PASS)
                }
                Some(t) => {
                    println!("  coin     {} ({k})  max {}  lim {}  dec {}", t.tick_raw, fmt_amount(t.max), fmt_amount(t.lim), t.dec);
                    println!("  minted   {}  holders {}", fmt_amount(t.minted), d.holder_count(&k)?);
                    println!("  deploy   {}  height {}  by {}", t.inscription_id, t.height, t.deployer);
                    println!("  replay   height {h}");
                    Ok(PASS)
                }
            }
        }
    }
}

fn api_height_note(u: &str) {
    if let Ok(v) = api::Api::new(u).get("/api/health") {
        if let Some(x) = v["drc20_operations_indexed_to"].as_u64().or(v["indexed_block_height"].as_u64()) {
            println!("  note     API indexed to {x}; a DIFF can simply mean the two heights differ");
        }
    }
}

fn name_cmd(cli: &Cli, raw: &str, api: Option<&str>) -> Result<u8, String> {
    let n = normalize(raw).ok_or("that is not a valid name")?;
    let d = db::Db::open_existing(&db_path(cli))?;
    let c = rpc(cli)?;
    let h = local_height(&d);
    let Some((iid, insc, height, _txidx, claims)) = d.name(&n.name)? else {
        println!("  local    {}  no valid claim (replay height {h})", n.name);
        return match api {
            Some(u) => {
                let v = api::Api::new(u).get(&format!("/api/names/resolve/{}", api::seg(&n.name)))?;
                let found = v["found"].as_bool().unwrap_or(false);
                println!("  API says found {found}  {}", verdict(!found));
                Ok(if found { DIFF } else { PASS })
            }
            None => Ok(PASS),
        };
    };
    let holder = match d.where_is(iid)? {
        Some((t, vout, _)) => match c.tx_out(&ds_rules::tx::txid_hex(&t), vout).map_err(|e| format!("Core: {e}"))? {
            Some(u) => owner_of_script(&hex::decode(u["scriptPubKey"]["hex"].as_str().unwrap_or("")).unwrap_or_default()).key(),
            None => "unknown (moved after the replay height; run `dsverify replay` again)".into(),
        },
        None => "none (the claim inscription was spent as a fee)".into(),
    };
    println!("  local    {}  claim {insc}  height {height}  claims seen {claims}", n.name);
    println!("  holder   {holder}  (replay height {h})");
    let Some(u) = api else { return Ok(PASS) };
    let v = api::Api::new(u).get(&format!("/api/names/resolve/{}", api::seg(&n.name)))?;
    let (ai, ah) = (v["inscription_id"].as_str().unwrap_or(""), v["holder"].as_str().unwrap_or(""));
    let same = ai == insc && ah == holder;
    println!("  API says claim {ai}  holder {ah}  {}", verdict(same));
    Ok(if same { PASS } else { DIFF })
}

fn vectors(cli: &Cli, dir: &Path, node: bool) -> Result<u8, String> {
    use ds_rules::vectors as v;
    let mut bad = 0;
    let mut line = |name: &str, r: Result<usize, String>| match r {
        Ok(n) => println!("  {name:<22} {n:>7} checks  PASS"),
        Err(e) => {
            println!("  {name:<22} DIFF  {e}");
            bad += 1;
        }
    };
    line("envelope", v::envelope(dir));
    line("drc20 bodies", v::drc20_bodies(dir));
    line("koinu routing", v::koinu(dir));
    let reveals = v::mainnet_reveals(dir);
    line("mainnet reveals", reveals.as_ref().map(|r| r.len()).map_err(|e| e.clone()));
    let names = v::names(dir);
    line("names", names.as_ref().map(|r| r.0).map_err(|e| e.clone()));
    if node {
        let c = rpc(cli)?;
        let mut checked = 0;
        let claims = names.as_ref().map(|x| &x.1[..]).map_err(|e| e.clone())?;
        for r in reveals.as_ref().map_err(|e| e.clone())?.iter().chain(claims) {
            let raw = c.raw_tx(&r.txid).map_err(|e| format!("Core getrawtransaction {}: {e}", r.txid))?;
            let meta = c.call("getrawtransaction", json!([r.txid, 1])).map_err(|e| format!("Core: {e}"))?;
            let bh = meta["blockhash"].as_str().unwrap_or("");
            let height = c.block_header(bh).ok().and_then(|h| h["height"].as_u64()).unwrap_or(0);
            if raw != r.raw_hex || height != r.height {
                println!("  node     {}  DIFF  (raw bytes or height differ from your Core)", r.id);
                bad += 1;
            } else {
                checked += 1;
            }
        }
        println!("  {:<22} {checked:>7} checks  {}", "mainnet txs vs Core", verdict(bad == 0));
    }
    Ok(if bad == 0 { PASS } else { DIFF })
}

fn proof_cmd(cli: &Cli, cmd: &ProofCmd) -> Result<u8, String> {
    match cmd {
        ProofCmd::Export { out } => {
            let d = db::Db::open_existing(&db_path(cli))?;
            std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
            for l in proof::LISTINGS {
                let b = proof::listing(&d, l)?;
                std::fs::write(out.join(format!("{l}.tsv")), &b).map_err(|e| e.to_string())?;
                std::fs::write(out.join(format!("{l}.sha256")), format!("{}  {l}.tsv\n", proof::sha256_hex(&b))).map_err(|e| e.to_string())?;
                println!("  {l:<9} {:>9} lines  {}", proof::line_count(&b), proof::sha256_hex(&b));
            }
            println!("  height   {}", local_height(&d));
            Ok(PASS)
        }
        ProofCmd::Check { height, dir } => {
            let base = dir.join(height.to_string());
            let s: Value = serde_json::from_str(&std::fs::read_to_string(base.join("summary.json")).map_err(|e| format!("{}: {e}", base.display()))?)
                .map_err(|e| e.to_string())?;
            let want_hash = s["block_hash"].as_str().ok_or("summary.json has no block_hash")?;
            let c = rpc(cli)?;
            let got = c.block_hash(*height).map_err(|e| format!("Core: {e}"))?;
            println!("  block    {height}  {got}  {}", verdict(got == want_hash));
            if got != want_hash {
                return Ok(DIFF);
            }
            let p = db_path(cli);
            let d = match db::Db::open_existing(&p) {
                Ok(d) if d.height() == Some(*height) => d,
                Ok(d) => {
                    println!("  state    not checked: your replay is at {}, not {height}.", local_height(&d));
                    println!("           run `dsverify --db <new file> replay --to {height}` and check again");
                    return Ok(UNSURE);
                }
                Err(e) => {
                    println!("  state    not checked: {e}");
                    return Ok(UNSURE);
                }
            };
            let mut all = true;
            for l in proof::LISTINGS {
                let want = std::fs::read_to_string(base.join(format!("{l}.sha256"))).map_err(|e| e.to_string())?;
                let want = want.split_whitespace().next().unwrap_or("").to_string();
                let got = proof::sha256_hex(&proof::listing(&d, l)?);
                println!("  {l:<9} {got}  {}", verdict(got == want));
                all &= got == want;
            }
            Ok(if all { PASS } else { DIFF })
        }
    }
}
