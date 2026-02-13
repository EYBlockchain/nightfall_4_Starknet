use super::types::{BlockNumber, EventFilter, RawEvent};
use super::{ChainClient, ChainClientError};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

const MAX_BACKOFF_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct PollConfig {
    pub poll_interval: Duration,
    pub chunk_size_blocks: u64,
    pub start_block: BlockNumber,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            chunk_size_blocks: 1000,
            start_block: BlockNumber(0),
        }
    }
}

pub async fn poll_events_forever(
    client: Arc<dyn ChainClient>,
    filter: EventFilter,
    cfg: PollConfig,
    mut on_events: impl FnMut(Vec<RawEvent>) + Send + 'static,
) -> Result<(), ChainClientError> {
    let mut cursor = cfg.start_block;
    let mut backoff = cfg.poll_interval;

    loop {
        let latest = match client.block_number().await {
            Ok(b) => {
                backoff = cfg.poll_interval;
                b
            }
            Err(e) => {
                log::warn!(
                    "poller: block_number failed: {e:?}; backing off {backoff:?}"
                );
                sleep(backoff).await;
                backoff = Duration::from_secs(std::cmp::min(
                    MAX_BACKOFF_SECS,
                    (backoff.as_secs().max(1)).saturating_mul(2),
                ));
                continue;
            }
        };
        if latest.0 < cursor.0 {
            log::debug!(
                "poller: waiting for chain to reach cursor (latest={}, cursor={})",
                latest.0,
                cursor.0
            );
            sleep(cfg.poll_interval).await;
            continue;
        }

        let to = BlockNumber(std::cmp::min(latest.0, cursor.0.saturating_add(cfg.chunk_size_blocks)));

        let events = match client.get_events(filter.clone(), cursor, to).await {
            Ok(evs) => {
                backoff = cfg.poll_interval;
                evs
            }
            Err(e) => {
                log::warn!(
                    "poller: get_events failed for range {}..={}: {e:?}; backing off {backoff:?}",
                    cursor.0,
                    to.0
                );
                sleep(backoff).await;
                backoff = Duration::from_secs(std::cmp::min(
                    MAX_BACKOFF_SECS,
                    (backoff.as_secs().max(1)).saturating_mul(2),
                ));
                continue;
            }
        };

        log::info!(
            "poller: polled blocks {}..={} (latest={}), got {} events",
            cursor.0,
            to.0,
            latest.0,
            events.len()
        );

        on_events(events);

        // Advance cursor to avoid re-reading the same range.
        cursor = BlockNumber(to.0.saturating_add(1));
        sleep(cfg.poll_interval).await;
    }
}
