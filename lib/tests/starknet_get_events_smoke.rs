#![cfg(feature = "backend_starknet")]

use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn starknet_get_events_smoke_katana() {
    // This is intentionally a smoke test. It should succeed against Katana
    // when a devnet is running locally.
    //
    // Run with:
    //   STARKNET_RPC_URL=http://127.0.0.1:5050 cargo test -p lib --features backend_starknet --test starknet_get_events_smoke
    let rpc_url = std::env::var("STARKNET_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:5050".to_string());

    // Minimal request that Katana accepts in practice.
    // Notes:
    // - Katana expects JSON-RPC params as a positional array.
    // - Block selectors should be shaped like `{ "block_number": N }`.
    let payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "starknet_getEvents",
        "params": [{
            "filter": {
                "from_block": { "block_number": 0 },
                "to_block": { "block_number": 0 },
                "keys": []
            },
            "chunk_size": 10
        }]
    });

    let http = Client::new();
    let res = http
        .post(rpc_url)
        .json(&payload)
        .send()
        .await
        .expect("http post failed");

    let value: serde_json::Value = res.json().await.expect("invalid json");
    assert!(value.get("error").is_none(), "rpc error: {value}");
    assert!(value.get("result").is_some(), "missing result: {value}");
}
