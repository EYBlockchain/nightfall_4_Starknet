pub mod types;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use types::{BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChainClientError {
	#[error("not supported by backend")]
	NotSupported,
	#[error("rpc error: {0}")]
	Rpc(String),
	#[error("invalid calldata")]
	InvalidCalldata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTransaction {
	pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TxReceipt {
	pub tx_hash: TxHash,
	pub success: bool,
}

#[async_trait]
pub trait ChainClient: Send + Sync {
	async fn is_connected(&self) -> bool;

	async fn chain_id(&self) -> Result<ChainId, ChainClientError>;

	async fn block_number(&self) -> Result<BlockNumber, ChainClientError>;

	async fn get_events(
		&self,
		filter: EventFilter,
		from_block: BlockNumber,
		to_block: BlockNumber,
	) -> Result<Vec<RawEvent>, ChainClientError>;

	async fn call_view(
		&self,
		contract: ContractId,
		calldata: Vec<u8>,
	) -> Result<Vec<u8>, ChainClientError>;

	async fn send_transaction(
		&self,
		tx: SignedTransaction,
	) -> Result<TxHash, ChainClientError>;

	async fn wait_for_confirmation(
		&self,
		tx_hash: TxHash,
		confirmations: u64,
	) -> Result<TxReceipt, ChainClientError>;
}
