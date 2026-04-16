use super::{ChainClient, ChainClientError, SignedTransaction, TxReceipt};
use super::types::{Address, BlockNumber, ChainId, ContractId, EventFilter, RawEvent, TxHash};
use async_trait::async_trait;
use hex;
use reqwest::Client;
use serde_json::Value;
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
    // Starknet / Katana uses minimal hex (no leading zeros) for addresses.
    let full = hex::encode(value);
    let stripped = full.trim_start_matches('0');
    if stripped.is_empty() {
        "0x0".to_string()
    } else {
        format!("0x{stripped}")
    }
}

fn tx_hash_to_hex(tx_hash: TxHash) -> String {
    bytes32_to_starknet_hex(tx_hash.0)
}

fn bytes_to_words(data: &[u8]) -> Result<Vec<[u8; 32]>, ChainClientError> {
    if data.is_empty() || data.len() % 32 != 0 {
        return Err(ChainClientError::InvalidCalldata);
    }

    let mut out = Vec::with_capacity(data.len() / 32);
    for chunk in data.chunks_exact(32) {
        let mut word = [0u8; 32];
        word.copy_from_slice(chunk);
        out.push(word);
    }
    Ok(out)
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

    pub async fn verify_class_hash(
        &self,
        contract_address: &str,
        expected_class_hash: &str,
    ) -> Result<(), ChainClientError> {
        #[derive(Debug, Serialize)]
        struct Params<'a> {
            block_id: &'a str,
            contract_address: &'a str,
        }

        let params = Params {
            block_id: "latest",
            contract_address,
        };

        let on_chain: String = self.rpc_call("starknet_getClassHashAt", params).await?;

        let normalise = |h: &str| {
            let stripped = h.strip_prefix("0x").unwrap_or(h).trim_start_matches('0');
            if stripped.is_empty() {
                "0".to_string()
            } else {
                stripped.to_ascii_lowercase()
            }
        };

        let on_chain_norm = normalise(&on_chain);
        let expected_norm = normalise(expected_class_hash);

        if on_chain_norm != expected_norm {
            return Err(ChainClientError::Rpc(format!(
                "starknet class hash mismatch for {contract_address}: expected=0x{expected_norm}, on_chain=0x{on_chain_norm}"
            )));
        }

        Ok(())
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
        let params_value = serde_json::to_value(&params)
            .map_err(|e| ChainClientError::Rpc(e.to_string()))?;
        let wrapped_params = wrap_rpc_params(params_value);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            method,
            params: wrapped_params,
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
struct JsonRpcRequest {
    jsonrpc: &'static str,
    method: &'static str,
    params: Value,
    id: u64,
}

pub(crate) fn wrap_rpc_params(value: Value) -> Value {
    match &value {
        Value::Null => Value::Array(vec![]),
        Value::Array(a) if a.is_empty() => Value::Array(vec![]),
        _ => Value::Array(vec![value]),
    }
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
        let chain_id_hex: String = self.rpc_call("starknet_chainId", Value::Null).await?;
        ChainId::from_hex_str(&chain_id_hex)
            .map_err(|e| ChainClientError::Rpc(format!("invalid chain id: {e}")))
    }

    async fn block_number(&self) -> Result<BlockNumber, ChainClientError> {
        let n: u64 = self
            .rpc_call("starknet_blockNumber", Value::Null)
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

            // Katana may return a continuation_token even when no events remain.
            // Break early to avoid an infinite loop.
            if res.events.is_empty() {
                break;
            }

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
        contract: ContractId,
        calldata: Vec<u8>,
    ) -> Result<Vec<u8>, ChainClientError> {
        #[derive(Debug, Serialize)]
        struct CallRequest {
            contract_address: String,
            entry_point_selector: String,
            calldata: Vec<String>,
        }

        #[derive(Debug, Serialize)]
        struct Params {
            request: CallRequest,
            block_id: &'static str,
        }

        let words = bytes_to_words(&calldata)?;
        let entry_point_selector = bytes32_to_starknet_hex(words[0]);
        let calldata_words = words
            .iter()
            .skip(1)
            .map(|w| bytes32_to_starknet_hex(*w))
            .collect::<Vec<_>>();

        let params = Params {
            request: CallRequest {
                contract_address: bytes32_to_starknet_hex(contract.0 .0),
                entry_point_selector,
                calldata: calldata_words,
            },
            block_id: "latest",
        };

        let result: Vec<String> = self.rpc_call("starknet_call", params).await?;
        hex_felts_to_bytes(&result)
    }

    async fn send_transaction(&self, tx: SignedTransaction) -> Result<TxHash, ChainClientError> {
        #[derive(Debug, Serialize)]
        struct Params {
            invoke_transaction: Value,
        }

        #[derive(Debug, Deserialize)]
        struct ResultPayload {
            transaction_hash: String,
        }

        let invoke_transaction: Value = serde_json::from_slice(&tx.bytes)
            .map_err(|e| ChainClientError::Rpc(format!("invalid starknet invoke payload json: {e}")))?;

        let params = Params { invoke_transaction };
        let result: ResultPayload = self
            .rpc_call("starknet_addInvokeTransaction", params)
            .await?;

        Ok(TxHash(hex_to_32_bytes(&result.transaction_hash)?))
    }

    async fn wait_for_confirmation(
        &self,
        tx_hash: TxHash,
        _confirmations: u64,
    ) -> Result<TxReceipt, ChainClientError> {
        #[derive(Debug, Serialize)]
        struct Params {
            transaction_hash: String,
        }

        #[derive(Debug, Deserialize)]
        struct Receipt {
            #[serde(default)]
            execution_status: Option<String>,
        }

        let params = Params {
            transaction_hash: tx_hash_to_hex(tx_hash),
        };
        let receipt: Receipt = self
            .rpc_call("starknet_getTransactionReceipt", params)
            .await?;

        let success = !matches!(
            receipt.execution_status.as_deref(),
            Some("REVERTED") | Some("REJECTED")
        );

        Ok(TxReceipt { tx_hash, success })
    }
}
