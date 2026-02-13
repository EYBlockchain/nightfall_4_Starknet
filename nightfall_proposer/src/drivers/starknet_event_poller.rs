use configuration::settings::get_settings;
use lib::chain_client::get_chain_client;
use lib::chain_client::polling::{poll_events_forever, PollConfig};
use lib::chain_client::types::{BlockNumber, EventFilter};
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
        start_block: BlockNumber(0),
    };

    let filter = EventFilter {
        contract: None,
        keys: vec![],
    };

    static TICKS: AtomicU64 = AtomicU64::new(0);

    let _ = poll_events_forever(client, filter, cfg, |events| {
        let tick = TICKS.fetch_add(1, Ordering::Relaxed) + 1;
        info!("starknet poller tick #{tick}: received {} events", events.len());
        eprintln!("starknet poller tick #{tick}: received {} events", events.len());
        let _ = std::io::stderr().flush();
    })
    .await;

    log::warn!("starknet poller: poll loop returned unexpectedly");
}
