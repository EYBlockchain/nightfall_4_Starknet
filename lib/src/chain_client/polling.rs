use super::types::{BlockNumber, EventFilter, RawEvent};
use super::{ChainClient, ChainClientError};
use mongodb::bson::doc;
use mongodb::Client as MongoClient;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::future::Future;
use tokio::time::{sleep, Duration};

const MAX_BACKOFF_SECS: u64 = 30;
const CHECKPOINT_DB_NAME: &str = "nightfall_listener_state";
const CHECKPOINT_COLLECTION_NAME: &str = "event_listener_checkpoints";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PollCheckpointDoc {
    #[serde(rename = "_id")]
    checkpoint_id: String,
    chain_id_hex: String,
    next_block: i64,
    #[serde(default)]
    last_event_block_number: Option<i64>,
    #[serde(default)]
    last_event_block_hash_hex: Option<String>,
}

fn to_hex(bytes: [u8; 32]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn to_i64(value: u64, field: &str) -> Result<i64, ChainClientError> {
    i64::try_from(value)
        .map_err(|_| ChainClientError::Rpc(format!("{field} overflow while converting to i64")))
}

fn to_u64(value: i64, field: &str) -> Result<u64, ChainClientError> {
    u64::try_from(value)
        .map_err(|_| ChainClientError::Rpc(format!("{field} is negative in checkpoint")))
}

async fn load_checkpoint(
    mongo: &MongoClient,
    checkpoint_id: &'static str,
) -> Result<Option<PollCheckpointDoc>, ChainClientError> {
    let collection = mongo
        .database(CHECKPOINT_DB_NAME)
        .collection::<PollCheckpointDoc>(CHECKPOINT_COLLECTION_NAME);

    collection
        .find_one(doc! {"_id": checkpoint_id})
        .await
        .map_err(|e| ChainClientError::Rpc(format!("failed to load checkpoint: {e}")))
}

async fn save_checkpoint(
    mongo: &MongoClient,
    checkpoint_id: &'static str,
    chain_id_hex: &str,
    next_block: u64,
    last_event_block_number: Option<u64>,
    last_event_block_hash_hex: Option<String>,
) -> Result<(), ChainClientError> {
    let collection = mongo
        .database(CHECKPOINT_DB_NAME)
        .collection::<PollCheckpointDoc>(CHECKPOINT_COLLECTION_NAME);

    let doc = PollCheckpointDoc {
        checkpoint_id: checkpoint_id.to_string(),
        chain_id_hex: chain_id_hex.to_string(),
        next_block: to_i64(next_block, "next_block")?,
        last_event_block_number: match last_event_block_number {
            Some(v) => Some(to_i64(v, "last_event_block_number")?),
            None => None,
        },
        last_event_block_hash_hex,
    };

    collection
        .replace_one(doc! {"_id": checkpoint_id}, doc)
        .upsert(true)
        .await
        .map_err(|e| ChainClientError::Rpc(format!("failed to save checkpoint: {e}")))?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct PollConfig {
    pub poll_interval: Duration,
    pub chunk_size_blocks: u64,
    pub genesis_block: BlockNumber,
    pub finality_lag_blocks: u64,
    pub rewind_blocks: u64,
}

impl Default for PollConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(2),
            chunk_size_blocks: 1000,
            genesis_block: BlockNumber(0),
            finality_lag_blocks: 0,
            rewind_blocks: 0,
        }
    }
}

pub async fn poll_events_forever<F, Fut>(
    client: Arc<dyn ChainClient>,
    filter: EventFilter,
    cfg: PollConfig,
    checkpoint_db_url: String,
    checkpoint_id: &'static str,
    mut on_events: F,
) -> Result<(), ChainClientError>
where
    F: FnMut(Vec<RawEvent>) -> Fut + Send + 'static,
    Fut: Future<Output = Result<(), ChainClientError>> + Send,
{
    let mongo = MongoClient::with_uri_str(&checkpoint_db_url)
        .await
        .map_err(|e| ChainClientError::Rpc(format!("failed to connect checkpoint DB: {e}")))?;

    let chain_id = client.chain_id().await?;
    let chain_id_hex = to_hex(chain_id.0);

    let checkpoint = load_checkpoint(&mongo, checkpoint_id).await?;

    let mut cursor = if let Some(saved) = checkpoint {
        if saved.chain_id_hex != chain_id_hex {
            return Err(ChainClientError::Rpc(format!(
                "checkpoint chain id mismatch for {checkpoint_id}: saved={}, current={}",
                saved.chain_id_hex, chain_id_hex
            )));
        }

        let next_block = to_u64(saved.next_block, "next_block")?;
        let rewind = std::cmp::max(cfg.rewind_blocks, cfg.finality_lag_blocks);
        let rewound = next_block.saturating_sub(rewind);
        let resume = std::cmp::max(cfg.genesis_block.0, rewound);
        log::info!(
            "poller: loaded checkpoint {checkpoint_id} (next_block={}, resume_from={}, chain_id={})",
            next_block,
            resume,
            chain_id_hex
        );
        BlockNumber(resume)
    } else {
        log::info!(
            "poller: no checkpoint for {}; starting from genesis {} (chain_id={})",
            checkpoint_id,
            cfg.genesis_block.0,
            chain_id_hex
        );
        cfg.genesis_block
    };

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
        let safe_latest = latest.0.saturating_sub(cfg.finality_lag_blocks);
        if safe_latest < cursor.0 {
            log::debug!(
                "poller: waiting for safe head (latest={}, safe_latest={}, cursor={})",
                latest.0,
                safe_latest,
                cursor.0
            );
            sleep(cfg.poll_interval).await;
            continue;
        }

        let chunk_span = cfg.chunk_size_blocks.saturating_sub(1);
        let to = BlockNumber(std::cmp::min(
            safe_latest,
            cursor.0.saturating_add(chunk_span),
        ));

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

        let last_event_block_number = events.last().map(|e| e.block_number.0);
        let last_event_block_hash_hex = events.last().map(|e| to_hex(e.block_hash.0));

        on_events(events).await?;

        let next_block = to.0.saturating_add(1);
        save_checkpoint(
            &mongo,
            checkpoint_id,
            &chain_id_hex,
            next_block,
            last_event_block_number,
            last_event_block_hash_hex,
        )
        .await?;

        cursor = BlockNumber(next_block);

        if cursor.0 > safe_latest {
            sleep(cfg.poll_interval).await;
        }
    }
}
