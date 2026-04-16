use configuration::settings::get_settings;
use lib::chain_client::get_chain_client;
use lib::chain_client::polling::{poll_events_forever, PollConfig};
use lib::chain_client::ChainClientError;
use lib::chain_client::types::{Address, BlockNumber, ContractId, EventFilter};
use lib::starknet_event_decoder;
use crate::services::process_starknet_events;
use log::info;
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::Write;
use tokio::time::{sleep, Duration};

pub async fn start_starknet_event_poller() {
    info!("starknet poller: task started");

    tokio::spawn(async {
        let mut i: u64 = 0;
        loop {
            i += 1;
            info!("starknet poller: heartbeat #{i}");
            eprintln!("starknet poller: heartbeat #{i}");
            let _ = std::io::stderr().flush();
            sleep(Duration::from_secs(2)).await;
        }
    });

    let settings = get_settings();

    let client = match get_chain_client(settings).await {
        Ok(c) => c,
        Err(e) => {
            log::error!("failed to init chain client: {e:?}");
            return;
        }
    };

    info!("starknet poller: chain client initialized");

    let cfg = PollConfig {
        poll_interval: std::time::Duration::from_secs(2),
        chunk_size_blocks: 1000,
        genesis_block: BlockNumber(settings.genesis_block as u64),
        finality_lag_blocks: settings.starknet_finality_lag_blocks,
        rewind_blocks: settings.starknet_rewind_blocks,
    };

    let contract = match settings.starknet_events_contract_address.trim() {
        "" => None,
        hex => match Address::from_hex_str(hex) {
            Ok(addr) => Some(ContractId(addr)),
            Err(e) => {
                log::error!(
                    "invalid NF4_STARKNET_EVENTS_CONTRACT_ADDRESS / starknet_events_contract_address: {hex} ({e})"
                );
                return;
            }
        },
    };

    let filter = EventFilter { contract, keys: vec![] };

    static TICKS: AtomicU64 = AtomicU64::new(0);

    let _ = poll_events_forever(
        client,
        filter,
        cfg,
        settings.nightfall_proposer.db_url.clone(),
        "nightfall_proposer_starknet_poller",
        |events| async move {
        let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        info!("starknet poller tick #{tick}: received {} events", events.len());
        eprintln!("starknet poller tick #{tick}: received {} events", events.len());
        let _ = std::io::stderr().flush();

        for (idx, ev) in events.iter().enumerate() {
            info!(
                "starknet event[{idx}]: block={} contract=0x{} keys={} data_len={} tx=0x{}",
                ev.block_number.0,
                hex::encode(ev.contract.0 .0),
                ev.keys.len(),
                ev.data.len(),
                hex::encode(ev.tx_hash.0)
            );

            match starknet_event_decoder::starknet::decode_dummy_emitter_event(ev) {
                Ok(decoded) => {
                    info!("starknet decoded event[{idx}]: {decoded:?}");
                    process_starknet_events::process_starknet_event(decoded)
                        .await
                        .map_err(|e| ChainClientError::Rpc(format!("process_starknet_event failed: {e}")))?;
                }
                Err(e) => info!("starknet undecoded event[{idx}]: {e}"),
            }
        }
        Ok(())
    },
    )
    .await;

    log::warn!("starknet poller: poll loop returned unexpectedly");
}
