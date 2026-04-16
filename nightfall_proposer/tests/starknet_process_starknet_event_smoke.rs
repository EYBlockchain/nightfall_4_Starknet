#[tokio::test]
async fn process_starknet_event_smoke() {
    let ev = lib::nightfall_events::NightfallEvent::DepositEscrowed {
        tx_hash: lib::chain_client::types::TxHash([0u8; 32]),
        commitment: [0u8; 32],
        token_id: [0u8; 32],
        value: lib::chain_client::types::U256([0u8; 32]),
        depositor: lib::chain_client::types::Address([0u8; 32]),
    };

    nightfall_proposer::services::process_starknet_events::process_starknet_event(ev)
        .await
        .expect("process_starknet_event");
}
