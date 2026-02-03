use super::{ChainClient, ChainClientError, SignedTransaction, TxReceipt};
use super::types::{BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};
use async_trait::async_trait;

pub struct StarknetChainClient;

impl StarknetChainClient {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ChainClient for StarknetChainClient {
    async fn is_connected(&self) -> bool {
        false
    }

    async fn chain_id(&self) -> Result<ChainId, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient not implemented yet".to_string(),
        ))
    }

    async fn block_number(&self) -> Result<BlockNumber, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient not implemented yet".to_string(),
        ))
    }

    async fn get_events(
        &self,
        _filter: EventFilter,
        _from_block: BlockNumber,
        _to_block: BlockNumber,
    ) -> Result<Vec<RawEvent>, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient.get_events not implemented yet".to_string(),
        ))
    }

    async fn call_view(
        &self,
        _contract: ContractId,
        _calldata: Vec<u8>,
    ) -> Result<Vec<u8>, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient.call_view not implemented yet".to_string(),
        ))
    }

    async fn send_transaction(&self, _tx: SignedTransaction) -> Result<TxHash, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient.send_transaction not implemented yet".to_string(),
        ))
    }

    async fn wait_for_confirmation(
        &self,
        _tx_hash: TxHash,
        _confirmations: u64,
    ) -> Result<TxReceipt, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "StarknetChainClient.wait_for_confirmation not implemented yet".to_string(),
        ))
    }
}
