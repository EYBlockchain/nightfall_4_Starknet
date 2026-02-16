#![cfg(feature = "backend_starknet")]

//! Integration test: verify that real events from the deployed DummyEmitter
//! contract are visible via the NF4 `ChainClient::get_events` abstraction.
//!
//! Prerequisites:
//! - Katana running on localhost:5050
//! - DummyEmitter deployed & events emitted (via the Rust emitter tool)
//! - Contract address written to `starknet_assets/artifacts/dummy_emitter_address.txt`
//!
//! Run with:
//!   cargo test -p lib --features backend_starknet --test starknet_dummy_emitter_events -- --nocapture

use lib::chain_client::{
    types::{Address, BlockNumber, ContractId, EventFilter},
    ChainClient,
};
use std::sync::Arc;

fn read_contract_address() -> Option<String> {
    let paths = [
        "starknet_assets/artifacts/dummy_emitter_address.txt",
        "../starknet_assets/artifacts/dummy_emitter_address.txt",
    ];
    for p in &paths {
        if let Ok(s) = std::fs::read_to_string(p) {
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

#[tokio::test]
async fn dummy_emitter_events_visible_via_chain_client() {
    let rpc_url = std::env::var("STARKNET_RPC_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:5050".to_string());

    let addr_hex = match read_contract_address() {
        Some(a) => a,
        None => {
            eprintln!(
                "SKIP: no dummy_emitter_address.txt found. \
                 Deploy the DummyEmitter first with the Rust emitter tool."
            );
            return;
        }
    };

    println!("Using contract address: {addr_hex}");

    // Build the chain client
    let client: Arc<dyn ChainClient> = Arc::new(
        lib::chain_client::starknet::StarknetChainClient::new(rpc_url),
    );

    // Build a filter targeting the deployed contract
    let contract_addr = Address::from_hex_str(&addr_hex)
        .expect("parse contract address");
    let filter = EventFilter {
        contract: Some(ContractId(contract_addr)),
        keys: vec![],
    };

    // Get the current block number
    let head = client.block_number().await.expect("block_number");
    println!("Current head: {}", head.0);
    assert!(head.0 >= 2, "Expected at least block 2 (deploy + emit)");

    // Fetch events from block 0 to head
    let events = client
        .get_events(filter, BlockNumber(0), head)
        .await
        .expect("get_events");

    println!("Got {} events from contract {}", events.len(), addr_hex);
    for (i, ev) in events.iter().enumerate() {
        println!(
            "  event[{}]: keys={} data_bytes={} tx={}",
            i,
            ev.keys.len(),
            ev.data.len(),
            hex::encode(&ev.tx_hash.0[..4])
        );
    }

    // We should see at least the BlockProposed and DepositEscrowed events
    assert!(
        events.len() >= 2,
        "Expected at least 2 events (BlockProposed + DepositEscrowed), got {}",
        events.len()
    );
}
