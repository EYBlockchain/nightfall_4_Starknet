use super::{ChainClient, ChainClientError, SignedTransaction, TxReceipt};
use super::types::{Address, BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};
use async_trait::async_trait;
use hex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize)]
struct StarknetGetEventsRequest {
    filter: StarknetEventFilter,
    chunk_size: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    continuation_token: Option<String>,
}

#[derive(Debug, Serialize)]
struct StarknetEventFilter {
    #[serde(skip_serializing_if = "Option::is_none")]
    from_block: Option<StarknetBlockId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    to_block: Option<StarknetBlockId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    keys: Vec<Vec<String>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum StarknetBlockId {
    Number { block_number: u64 },
}

#[derive(Debug, Deserialize)]
struct StarknetGetEventsResult {
    events: Vec<StarknetEmittedEvent>,
    continuation_token: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StarknetEmittedEvent {
    from_address: String,
    keys: Vec<String>,
    data: Vec<String>,
    #[serde(default)]
    block_number: Option<u64>,
    #[serde(default)]
    transaction_hash: Option<String>,
}

fn hex_to_32_bytes(s: &str) -> Result<[u8; 32], ChainClientError> {
    let hex = s.strip_prefix("0x").unwrap_or(s);
    if hex.is_empty() || hex.len() > 64 {
        return Err(ChainClientError::Rpc("invalid hex length".to_string()));
    }

    let mut decoded = Vec::with_capacity(hex.len().div_ceil(2));
    let mut i = 0usize;
    if hex.len() % 2 == 1 {
        let b = u8::from_str_radix(&format!("0{}", &hex[0..1]), 16)
            .map_err(|_| ChainClientError::Rpc("invalid hex".to_string()))?;
        decoded.push(b);
        i = 1;
    }
    while i < hex.len() {
        let b = u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| ChainClientError::Rpc("invalid hex".to_string()))?;
        decoded.push(b);
        i += 2;
    }
    if decoded.len() > 32 {
        return Err(ChainClientError::Rpc("invalid hex length".to_string()));
    }
    let mut out = [0u8; 32];
    out[32 - decoded.len()..].copy_from_slice(&decoded);
    Ok(out)
}

fn hex_felts_to_bytes(felts: &[String]) -> Result<Vec<u8>, ChainClientError> {
    let mut out = Vec::with_capacity(felts.len() * 32);
    for f in felts {
        out.extend_from_slice(&hex_to_32_bytes(f)?);
    }
    Ok(out)
}

fn bytes32_to_starknet_hex(value: [u8; 32]) -> String {
    format!("0x{}", hex::encode(value))
}

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
            params: vec![params],
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
        filter: EventFilter,
        from_block: BlockNumber,
        to_block: BlockNumber,
    ) -> Result<Vec<RawEvent>, ChainClientError> {
        let address = filter.contract.map(|c| bytes32_to_starknet_hex(c.0 .0));

        let keys = if filter.keys.is_empty() {
            vec![]
        } else {
            vec![filter
                .keys
                .into_iter()
                .map(bytes32_to_starknet_hex)
                .collect::<Vec<_>>()]
        };

        let mut continuation_token: Option<String> = None;
        let mut out: Vec<RawEvent> = Vec::new();

        loop {
            let req = StarknetGetEventsRequest {
                filter: StarknetEventFilter {
                    from_block: Some(StarknetBlockId::Number {
                        block_number: from_block.0,
                    }),
                    to_block: Some(StarknetBlockId::Number {
                        block_number: to_block.0,
                    }),
                    address: address.clone(),
                    keys: keys.clone(),
                },
                chunk_size: 1000,
                continuation_token: continuation_token.clone(),
            };

            let res: StarknetGetEventsResult = self.rpc_call("starknet_getEvents", req).await?;

            for e in res.events {
                let block_number = e
                    .block_number
                    .map(BlockNumber)
                    .unwrap_or(from_block);

                let tx_hash = if let Some(h) = e.transaction_hash {
                    TxHash(hex_to_32_bytes(&h)?)
                } else {
                    TxHash([0u8; 32])
                };

                let contract = ContractId(Address(hex_to_32_bytes(&e.from_address)?));

                let keys = e
                    .keys
                    .into_iter()
                    .map(|k| hex_to_32_bytes(&k))
                    .collect::<Result<Vec<_>, _>>()?;

                let data = hex_felts_to_bytes(&e.data)?;

                out.push(RawEvent {
                    block_number,
                    tx_hash,
                    contract,
                    keys,
                    data,
                });
            }

            continuation_token = res.continuation_token;
            if continuation_token.is_none() {
                break;
            }
        }

        Ok(out)
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
