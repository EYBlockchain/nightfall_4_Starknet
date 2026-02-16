#[tokio::test]
async fn process_starknet_event_smoke() {
    let ev = lib::nightfall_events::NightfallEvent::BlockProposed {
        block_number: 1,
        proposer: lib::chain_client::types::Address([0u8; 32]),
        transactions_root: [0u8; 32],
        timestamp: 123,
    };

    nightfall_client::services::process_starknet_events::process_starknet_event(ev)
        .await
        .expect("process_starknet_event");
}
