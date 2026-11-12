// SPDX-License-Identifier: Apache-2.0
use dogesoft::{doge_to_koinu, drc20_units, koinu_to_doge, Client, Error};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A tiny HTTP server: answers each request with the next canned response and records the paths.
async fn serve(answers: Vec<(u16, &'static str, &'static str)>) -> (String, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = l.local_addr().unwrap();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let s2 = seen.clone();
    tokio::spawn(async move {
        for (status, extra, body) in answers {
            let (mut c, _) = l.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let n = c.read(&mut buf).await.unwrap();
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            s2.lock().unwrap().push(req.lines().next().unwrap_or("").to_string());
            let resp =
                format!("HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n{extra}\r\n{body}", body.len());
            c.write_all(resp.as_bytes()).await.unwrap();
        }
    });
    (format!("http://{addr}"), seen)
}

#[tokio::test]
async fn health_and_holdings() {
    let (base, seen) = serve(vec![
        (200, "", r#"{"status":"healthy","version":"1.0.0","indexed_block_height":6453516,"chain_tip":6453517,"blocks_behind_tip":1,"extra":true}"#),
        (200, "", r#"{"address":"DLJR","balances":[{"tick":"DU$T","available":"39999981.84","transferable":"492.000000000000000000","balance":"40000473.84","decimals":18}]}"#),
    ])
    .await;
    let ds = Client::new(&base);
    let h = ds.health().await.unwrap();
    assert_eq!((h.indexed_block_height, h.chain_tip, h.blocks_behind_tip), (6453516, 6453517, 1));
    let c = ds.drc20_holdings("DLJR").await.unwrap();
    assert_eq!(c.balances[0].tick, "DU$T");
    assert_eq!(drc20_units(&c.balances[0].transferable, 18), Some(492 * 10u128.pow(18)));
    let s = seen.lock().unwrap();
    assert_eq!(s[0], "GET /api/health HTTP/1.1");
    assert_eq!(s[1], "GET /api/wallet/address/DLJR/drc20 HTTP/1.1");
}

#[tokio::test]
async fn balance_is_u64_koinu_and_ticks_are_encoded() {
    let (base, seen) = serve(vec![
        (200, "", r#"{"address":"D1","balance_koinu":937600979,"balance_doge":"9.37600979","utxo_count":117}"#),
        (200, "", r#"{"token":{"ticker":"du$t","ticker_onchain":"DU$T","max_supply":"100000000","limit_per_mint":"100000000","decimals":18,"deploy_height":5648609,"deploy_txid":"85","deployer_address":"D7","minted_supply":"100000000","holder_count":139},"operations":[],"holders":[]}"#),
    ])
    .await;
    let ds = Client::new(&base);
    let b = ds.balance("D1").await.unwrap();
    assert_eq!(b.balance_koinu, 937_600_979u64);
    assert_eq!(b.doge(), "9.37600979");
    assert_eq!(ds.drc20("DU$T").await.unwrap().token.holder_count, 139);
    assert_eq!(seen.lock().unwrap()[1], "GET /api/explorer/drc20/DU%24T HTTP/1.1");
}

#[tokio::test(start_paused = true)]
async fn backs_off_on_429_then_succeeds() {
    let (base, seen) = serve(vec![
        (429, "retry-after: 1\r\n", "slow down"),
        (429, "", "slow down"),
        (200, "", r#"{"name":"soft.doge","found":true,"valid":true,"holder":"DBTA","inscription_id":"ffi0","claim_count":2}"#),
    ])
    .await;
    let n = Client::new(&base).resolve_name("soft.doge").await.unwrap();
    assert_eq!(n.holder.as_deref(), Some("DBTA"));
    assert_eq!(seen.lock().unwrap().len(), 3);
}

#[tokio::test(start_paused = true)]
async fn gives_up_after_max_retries() {
    let (base, _) = serve(vec![(429, "", "slow down"), (429, "", "slow down")]).await;
    let mut ds = Client::new(&base);
    ds.max_retries = 1;
    match ds.health().await {
        Err(Error::Status { status: 429, .. }) => {}
        r => panic!("{r:?}"),
    }
}

#[tokio::test]
async fn inscription_unwraps() {
    let (base, _) = serve(vec![(200, "", r#"{"inscription":{"inscription_id":"ffi0","mime_type":"text/plain","location":{"txid":"ff","vout":0,"outpoint":"ff:0","sat_offset":null,"owner_address":"DBTA"}}}"#)]).await;
    let i = Client::new(&base).inscription("ffi0").await.unwrap();
    assert_eq!(i.location.unwrap().owner_address.as_deref(), Some("DBTA"));
}

#[test]
fn amounts() {
    assert_eq!(koinu_to_doge(5), "0.00000005");
    assert_eq!(doge_to_koinu("9.37600979"), Some(937_600_979));
    assert_eq!(doge_to_koinu("1"), Some(100_000_000));
    assert_eq!(doge_to_koinu("1.000000001"), None);
    assert_eq!(drc20_units("0.5", 1), Some(5));
    assert_eq!(drc20_units("0.55", 1), None);
    assert_eq!(drc20_units("-1", 18), None);
}

#[tokio::test]
#[ignore = "live read; run with --ignored"]
async fn live_health() {
    let h = Client::default().health().await.unwrap();
    assert!(h.indexed_block_height > 6_000_000);
}
