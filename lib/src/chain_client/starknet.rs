use super::{ChainClient, ChainClientError, SignedTransaction, TxReceipt};
use super::types::{BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

pub struct StarknetChainClient {
    rpc_url: Url,
    http: Client,
}

impl StarknetChainClient {
    pub fn new(rpc_url: String) -> Self {
        let rpc_url = rpc_url
            .parse::<Url>()
            .expect("invalid starknet_client_url");
        Self {
            rpc_url,
            http: Client::new(),
        }
    }

    async fn rpc_call<TParams, TResult>(
        &self,
        method: &'static str,
        params: TParams,
    ) -> Result<TResult, ChainClientError>
    where
        TParams: Serialize,
        TResult: for<'de> Deserialize<'de>,
    {
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            params,
            id: 1u64,
        };

        let response = self
            .http
            .post(self.rpc_url.clone())
            .json(&request)
            .send()
            .await
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;

        let status = response.status();
        let body = response
            .json::<JsonRpcResponse<TResult>>()
            .await
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;

        if let Some(err) = body.error {
            return Err(ChainClientError::Rpc(format!(
                "starknet rpc error {status}: {} (code {})",
                err.message, err.code
            )));
        }

        body.result.ok_or_else(|| {
            ChainClientError::Rpc(format!(
                "starknet rpc invalid response {status}: missing result"
            ))
        })
    }
}

#[derive(Debug, Serialize)]
struct JsonRpcRequest<TParams> {
    jsonrpc: &'static str,
    method: &'static str,
    params: TParams,
    id: u64,
}

#[derive(Debug, Deserialize)]
struct JsonRpcResponse<TResult> {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    result: Option<TResult>,
    error: Option<JsonRpcError>,
    #[allow(dead_code)]
    id: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

#[async_trait]
impl ChainClient for StarknetChainClient {
    async fn is_connected(&self) -> bool {
        self.chain_id().await.is_ok()
    }

    async fn chain_id(&self) -> Result<ChainId, ChainClientError> {
        let chain_id_hex: String = self.rpc_call("starknet_chainId", Vec::<u8>::new()).await?;
        ChainId::from_hex_str(&chain_id_hex)
            .map_err(|e| ChainClientError::Rpc(format!("invalid chain id: {e}")))
    }

    async fn block_number(&self) -> Result<BlockNumber, ChainClientError> {
        let n: u64 = self
            .rpc_call("starknet_blockNumber", Vec::<u8>::new())
            .await?;
        Ok(BlockNumber(n))
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
