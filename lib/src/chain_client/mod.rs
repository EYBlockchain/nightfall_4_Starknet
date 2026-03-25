pub mod types;

pub mod signer;

pub mod polling;

#[cfg(feature = "backend_evm")]
pub mod evm;

#[cfg(feature = "backend_starknet")]
pub mod starknet;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::blockchain_client::BlockchainClientConnection;

use types::{BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};

#[derive(Debug, Clone, thiserror::Error)]
pub enum ChainClientError {
	#[error("not supported: {0}")]
	NotSupported(String),
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

/// Initialize a chain-neutral client selected by configuration.
///
/// This is the single entrypoint intended by US-006.
pub async fn get_chain_client(
	settings: &configuration::settings::Settings,
) -> Result<Arc<dyn ChainClient>, ChainClientError> {
	match settings.backend_kind {
		configuration::settings::BackendKind::Evm => {
			#[cfg(feature = "backend_evm")]
			{
				use crate::wallets::LocalWsClient;
				let ws = LocalWsClient::try_from_settings(settings)
					.await
					.map_err(|e| ChainClientError::Rpc(e.to_string()))?;
				let provider = ws.get_client();
				Ok(Arc::new(evm::EvmChainClient::new(provider)))
			}
			#[cfg(not(feature = "backend_evm"))]
			{
				Err(ChainClientError::NotSupported(
					"EVM backend selected but `backend_evm` feature is disabled".to_string(),
				))
			}
		}
		configuration::settings::BackendKind::Starknet => {
			#[cfg(feature = "backend_starknet")]
			{
				let url = settings.starknet_client_url.clone();
				if url.trim().is_empty() {
					return Err(ChainClientError::Rpc(
						"missing starknet_client_url".to_string(),
					));
				}

				let client = starknet::StarknetChainClient::new(url);

				if settings.starknet_verify_class_hash {
					if settings.starknet_expected_class_hash.trim().is_empty() {
						log::warn!(
							"starknet_verify_class_hash=true but NF4_STARKNET_EXPECTED_CLASS_HASH is empty; skipping verification"
						);
					} else {
						if settings.starknet_events_contract_address.trim().is_empty() {
							return Err(ChainClientError::Rpc(
								"starknet_verify_class_hash=true but NF4_STARKNET_EVENTS_CONTRACT_ADDRESS is empty"
									.to_string(),
							));
						}

						client
							.verify_class_hash(
								&settings.starknet_events_contract_address,
								&settings.starknet_expected_class_hash,
							)
							.await?;
					}
				}

				Ok(Arc::new(client))
			}
			#[cfg(not(feature = "backend_starknet"))]
			{
				Err(ChainClientError::NotSupported(
					"Starknet backend selected but `backend_starknet` feature is disabled".to_string(),
				))
			}
		}
	}
}

/// Initialize a chain-neutral signer selected by configuration.
pub async fn get_chain_signer(
	settings: &configuration::settings::Settings,
) -> Result<Arc<dyn signer::ChainSigner>, ChainClientError> {
	match settings.backend_kind {
		configuration::settings::BackendKind::Evm => {
			#[cfg(feature = "backend_evm")]
			{
				use crate::wallets::LocalWsClient;
				let ws = LocalWsClient::try_from_settings(settings)
					.await
					.map_err(|e| ChainClientError::Rpc(e.to_string()))?;
				Ok(Arc::new(signer::evm::EvmChainSigner::new(ws.get_wallet_type().clone())))
			}
			#[cfg(not(feature = "backend_evm"))]
			{
				Err(ChainClientError::NotSupported(
					"EVM backend selected but `backend_evm` feature is disabled".to_string(),
				))
			}
		}
		configuration::settings::BackendKind::Starknet => {
			#[cfg(feature = "backend_starknet")]
			{
				if settings.signing_key.trim().is_empty() {
					return Err(ChainClientError::Rpc(
						"missing NF4_SIGNING_KEY for Starknet backend".to_string(),
					));
				}
				if settings.starknet_account_address.trim().is_empty() {
					return Err(ChainClientError::Rpc(
						"missing NF4_STARKNET_ACCOUNT_ADDRESS for Starknet backend".to_string(),
					));
				}

				let signer = signer::starknet::StarknetSigner::from_hex_key(
					&settings.signing_key,
					&settings.starknet_account_address,
				)?;
				Ok(Arc::new(signer))
			}
			#[cfg(not(feature = "backend_starknet"))]
			{
				Err(ChainClientError::NotSupported(
					"Starknet backend selected but `backend_starknet` feature is disabled".to_string(),
				))
			}
		}
	}
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
