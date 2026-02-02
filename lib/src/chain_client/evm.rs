use crate::chain_client::{
    ChainClient, ChainClientError, SignedTransaction, TxReceipt,
};
use crate::chain_client::types::{
    Address, BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash,
};
use async_trait::async_trait;

#[cfg(feature = "backend_evm")]
use alloy::{
    primitives::{B256, Bytes, FixedBytes},
    providers::Provider,
    rpc::types::Filter,
};

#[cfg(feature = "backend_evm")]
use std::sync::Arc;

#[cfg(feature = "backend_evm")]
#[derive(Clone)]
pub struct EvmChainClient {
    provider: Arc<dyn Provider>,
}

#[cfg(feature = "backend_evm")]
impl EvmChainClient {
    pub fn new(provider: Arc<dyn Provider>) -> Self {
        Self { provider }
    }

    fn address_to_alloy(addr: Address) -> alloy::primitives::Address {
        alloy::primitives::Address::from_slice(&addr.0[12..])
    }

    fn b256_to_bytes32(value: &B256) -> [u8; 32] {
        value.0
    }

    fn bytes_to_vec_u8(value: Bytes) -> Vec<u8> {
        value.to_vec()
    }

    fn filter_from_event_filter(filter: EventFilter, from: BlockNumber, to: BlockNumber) -> Filter {
        let mut f = Filter::new().from_block(from.0).to_block(to.0);
        if let Some(contract) = filter.contract {
            let a = Self::address_to_alloy(contract.0);
            f = f.address(a);
        }
        // NOTE: We keep keys generic; on EVM we interpret them as topic0/topic1/...
        // If keys is empty, no topic filtering is applied.
        if !filter.keys.is_empty() {
            let topics: Vec<FixedBytes<32>> = filter
                .keys
                .into_iter()
                .map(|k| FixedBytes::<32>::from_slice(&k))
                .collect();
            f = f.event_signature(topics);
        }
        f
    }
}

#[cfg(feature = "backend_evm")]
#[async_trait]
impl ChainClient for EvmChainClient {
    async fn is_connected(&self) -> bool {
        self.provider.get_net_version().await.is_ok()
    }

    async fn chain_id(&self) -> Result<ChainId, ChainClientError> {
        let id = self
            .provider
            .get_chain_id()
            .await
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;
        Ok(ChainId(id))
    }

    async fn block_number(&self) -> Result<BlockNumber, ChainClientError> {
        let n = self
            .provider
            .get_block_number()
            .await
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;
        Ok(BlockNumber(n))
    }

    async fn get_events(
        &self,
        filter: EventFilter,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<Vec<RawEvent>, ChainClientError> {
        let f = Self::filter_from_event_filter(filter, from_block, to_block);
        let logs = self
            .provider
            .get_logs(&f)
            .await
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;

        let mut out = Vec::with_capacity(logs.len());
        for log in logs {
            // alloy::rpc::types::Log
            let block_number = log
                .block_number
                .map(|b| BlockNumber(b))
                .unwrap_or(from_block);

            let tx_hash = log
                .transaction_hash
                .map(|h| TxHash(h.into()))
                .unwrap_or(TxHash([0u8; 32]));

            let contract = ContractId(Address::from(log.address()));

            // topics: Vec<B256>
            let keys = log.topics().iter().map(Self::b256_to_bytes32).collect::<Vec<_>>();

            let data = Self::bytes_to_vec_u8(log.data().data.clone());

            out.push(RawEvent {
                block_number,
                tx_hash,
                contract,
                keys,
                data,
            });
        }

        Ok(out)
    }

    async fn call_view(
        &self,
        _contract: ContractId,
        _calldata: Vec<u8>,
    ) -> Result<Vec<u8>, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "EVM call_view not implemented yet".to_string(),
        ))
    }

    async fn send_transaction(
        &self,
        _tx: SignedTransaction,
    ) -> Result<TxHash, ChainClientError> {
        Err(ChainClientError::NotSupported(
            "EVM send_transaction not implemented yet".to_string(),
        ))
    }

    async fn wait_for_confirmation(
        &self,
        tx_hash: TxHash,
        _confirmations: u64,
    ) -> Result<TxReceipt, ChainClientError> {
        Ok(TxReceipt {
            tx_hash,
            success: true,
        })
    }
}

// Help avoid unused warnings when the feature is not enabled.
#[cfg(not(feature = "backend_evm"))]
pub struct EvmChainClient;
