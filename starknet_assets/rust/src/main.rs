use clap::{Args, Parser, Subcommand};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use starknet_accounts::{Account, ConnectedAccount, ExecutionEncoding, ExecutionV3, SingleOwnerAccount};
use starknet_contract::{ContractFactory, UdcSelector};
use starknet_core::{
    types::{
        contract::{CompiledClass, SierraClass},
        BlockId, BlockTag, DeclareTransactionResult, ExecutionResult, Felt,
        FlattenedSierraClass, InvokeTransactionResult, StarknetError, TransactionReceipt,
    },
};
use starknet_providers::{
    jsonrpc::{HttpTransport, JsonRpcClient},
    Provider, ProviderError,
};
use starknet_signers::{LocalWallet, SigningKey};
use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, SystemTimeError, UNIX_EPOCH},
};
use thiserror::Error;
use tokio::time::sleep;
use url::Url;

const DEFAULT_RPC_URL: &str = "http://localhost:5050/rpc";
const DEFAULT_SIERRA_ARTIFACT: &str = "../cairo1_dummy_emitter/target/dev/cairo1_dummy_emitter_DummyEmitter.contract_class.json";
const DEFAULT_CASM_ARTIFACT: &str = "../cairo1_dummy_emitter/target/dev/cairo1_dummy_emitter_DummyEmitter.compiled_contract_class.json";
const DEFAULT_ADDRESS_FILE: &str = "../artifacts/dummy_emitter_address.txt";
const DEFAULT_ACCOUNT_ADDRESS: &str = "0x064b48806902a367c8598f4f95c305e8c1a1acba5f082d294a43793113115691";
const DEFAULT_PRIVATE_KEY: &str = "0x0000000000000000000000000000000071d7bb07b9a64f6f78ac4c816aff4da9";
const DEFAULT_MAX_AMOUNT: u64 = 0x20000000;
const DEFAULT_MAX_PRICE_PER_UNIT: u128 = 0x200;
const DECLARE_TIP: u64 = 0;
const DEPLOY_TIP: u64 = 0;
const MAX_RECEIPT_POLL_ATTEMPTS: usize = 60;
const RECEIPT_POLL_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Parser, Debug)]
#[command(author, version, about = "Starknet emitter tool")]
struct Cli {
    #[arg(long, global = true, default_value = DEFAULT_RPC_URL)]
    rpc_url: String,
    #[arg(long, global = true, env = "NF4_STARKNET_ACCOUNT_ADDRESS", default_value = DEFAULT_ACCOUNT_ADDRESS)]
    account_address: String,
    #[arg(long, global = true, env = "NF4_SIGNING_KEY", default_value = DEFAULT_PRIVATE_KEY)]
    private_key: String,
    #[arg(long, global = true, default_value_t = DEFAULT_MAX_AMOUNT, value_parser = parse_u64_cli_value)]
    max_amount: u64,
    #[arg(long, global = true, default_value_t = DEFAULT_MAX_PRICE_PER_UNIT, value_parser = parse_u128_cli_value)]
    max_price_per_unit: u128,
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug)]
struct ResourceBoundsConfig {
    max_amount: u64,
    max_price_per_unit: u128,
}

impl Cli {
    fn resource_bounds(&self) -> ResourceBoundsConfig {
        ResourceBoundsConfig {
            max_amount: self.max_amount,
            max_price_per_unit: self.max_price_per_unit,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    Ping,
    Deploy(DeployArgs),
    EmitBlockProposed(EmitBlockProposedArgs),
    EmitDepositEscrowed(EmitDepositEscrowedArgs),
}

#[derive(Args, Debug)]
struct DeployArgs {
    #[arg(long, default_value = DEFAULT_SIERRA_ARTIFACT)]
    sierra_artifact: PathBuf,
    #[arg(long, default_value = DEFAULT_CASM_ARTIFACT)]
    casm_artifact: PathBuf,
    #[arg(long, default_value = DEFAULT_ADDRESS_FILE)]
    out_file: PathBuf,
}

#[derive(Args, Debug)]
struct EmitBlockProposedArgs {
    #[arg(long)]
    contract: String,
    #[arg(long, default_value = "1")]
    block_number: String,
    #[arg(long, default_value = "0xdeadbeef")]
    transactions_root: String,
    #[arg(long, default_value = "1700000000")]
    timestamp: String,
}

#[derive(Args, Debug)]
struct EmitDepositEscrowedArgs {
    #[arg(long)]
    contract: String,
    #[arg(long, default_value = "0xaaaa")]
    commitment: String,
    #[arg(long, default_value = "0x1")]
    token_id: String,
    #[arg(long, default_value = "0x64")]
    value_low: String,
    #[arg(long, default_value = "0x0")]
    value_high: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct RpcErrorPayload {
    code: i64,
    message: String,
    #[serde(default)]
    data: Option<Value>,
}

impl std::fmt::Display for RpcErrorPayload {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.data {
            Some(data) => write!(formatter, "{}: {} ({data})", self.code, self.message),
            None => write!(formatter, "{}: {}", self.code, self.message),
        }
    }
}

impl RpcErrorPayload {
    fn is_already_declared(&self) -> bool {
        let message = self.to_string().to_ascii_lowercase();
        message.contains("already") && message.contains("declared")
    }
}

#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorPayload>,
}

#[derive(Debug, Error)]
enum RpcCallError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("response parsing failed: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("rpc error {0}")]
    Rpc(RpcErrorPayload),
    #[error("malformed rpc response for `{method}`: {details}")]
    Protocol { method: String, details: String },
}

#[derive(Debug, Error)]
enum AppError {
    #[error(transparent)]
    Rpc(#[from] RpcCallError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Url(#[from] url::ParseError),
    #[error(transparent)]
    Clock(#[from] SystemTimeError),
    #[error("invalid felt for {name}: {value}")]
    InvalidFelt { name: &'static str, value: String },
    #[error("artifact error: {0}")]
    Artifact(String),
    #[error("provider error: {0}")]
    Provider(String),
    #[error("account error: {0}")]
    Account(String),
    #[error("transaction {transaction_hash} reverted: {reason}")]
    Reverted {
        transaction_hash: String,
        reason: String,
    },
    #[error("timed out waiting for {label} transaction receipt: {transaction_hash}")]
    ReceiptTimeout {
        label: &'static str,
        transaction_hash: String,
    },
    #[error("deployed class hash mismatch at {address}: expected {expected}, got {actual}")]
    DeployedClassHashMismatch {
        address: String,
        expected: String,
        actual: String,
    },
}

struct RpcClient {
    http: reqwest::Client,
    rpc_url: String,
}

impl RpcClient {
    fn new(rpc_url: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            rpc_url,
        }
    }

    async fn rpc_call(&self, method: &str, params: Value) -> Result<Value, RpcCallError> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let response_text = self
            .http
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;

        let response: RpcResponse = serde_json::from_str(&response_text)?;

        if let Some(error) = response.error {
            return Err(RpcCallError::Rpc(error));
        }

        response.result.ok_or_else(|| RpcCallError::Protocol {
            method: method.to_owned(),
            details: response_text,
        })
    }

    async fn rpc_call_typed<T>(&self, method: &str, params: Value) -> Result<T, RpcCallError>
    where
        T: DeserializeOwned,
    {
        let result = self.rpc_call(method, params).await?;
        serde_json::from_value(result).map_err(RpcCallError::Parse)
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let rpc = RpcClient::new(cli.rpc_url.clone());

    match &cli.command {
        Command::Ping => ping(&rpc).await?,
        Command::Deploy(args) => deploy(&cli, &rpc, args).await?,
        Command::EmitBlockProposed(args) => emit_block_proposed_stub(&cli, args),
        Command::EmitDepositEscrowed(args) => emit_deposit_escrowed_stub(&cli, args),
    }

    Ok(())
}

async fn ping(rpc: &RpcClient) -> Result<(), RpcCallError> {
    let result = rpc.rpc_call("starknet_chainId", json!([])).await?;
    println!("{result}");
    Ok(())
}

async fn deploy(cli: &Cli, rpc: &RpcClient, args: &DeployArgs) -> Result<(), AppError> {
    let resource_bounds = cli.resource_bounds();
    let sierra_class = read_json_file::<SierraClass>(&args.sierra_artifact)?;
    let compiled_class = read_json_file::<CompiledClass>(&args.casm_artifact)?;
    let flattened_class = sierra_class
        .flatten()
        .map_err(|error| AppError::Artifact(error.to_string()))?;
    let class_hash = flattened_class.class_hash();
    let casm_artifact_hash = compiled_class
        .class_hash()
        .map_err(|error| AppError::Artifact(error.to_string()))?;
    let compiled_class_hash = compile_like_devnet(&flattened_class)?;
    let chain_id = fetch_chain_id(rpc).await?;
    let account_provider = new_provider(&cli.rpc_url)?;
    let receipt_provider = new_provider(&cli.rpc_url)?;
    let signer = LocalWallet::from(SigningKey::from_secret_scalar(parse_felt(
        "private key",
        &cli.private_key,
    )?));
    let account_address = parse_felt("account address", &cli.account_address)?;
    let account = Arc::new(SingleOwnerAccount::new(
        account_provider,
        signer,
        account_address,
        chain_id,
        ExecutionEncoding::New,
    ));

    let declare_nonce = account
        .get_nonce()
        .await
        .map_err(|error| AppError::Provider(error.to_string()))?;
    let prepared_declare = account
        .declare_v3(Arc::new(flattened_class), compiled_class_hash)
        .nonce(declare_nonce)
        .l1_gas(resource_bounds.max_amount)
        .l1_gas_price(resource_bounds.max_price_per_unit)
        .l2_gas(resource_bounds.max_amount)
        .l2_gas_price(resource_bounds.max_price_per_unit)
        .l1_data_gas(resource_bounds.max_amount)
        .l1_data_gas_price(resource_bounds.max_price_per_unit)
        .tip(DECLARE_TIP)
        .prepared()
        .map_err(|error| AppError::Artifact(error.to_string()))?;
    let declare_request = prepared_declare
        .get_declare_request(false, false)
        .await
        .map_err(|error| AppError::Account(format!("{error:?}")))?;

    println!("declare_prepared_nonce={}", felt_hex(declare_nonce));
    println!("class_hash={}", felt_hex(class_hash));
    println!("compiled_class_hash={}", felt_hex(compiled_class_hash));
    println!("casm_artifact_hash={}", felt_hex(casm_artifact_hash));
    let declare_tx_hash = match rpc
        .rpc_call_typed::<DeclareTransactionResult>(
            "starknet_addDeclareTransaction",
            json!([declare_request]),
        )
        .await
    {
        Ok(declare_result) => {
            wait_for_successful_receipt(
                &receipt_provider,
                declare_result.transaction_hash,
                "declare",
            )
            .await?;
            Some(declare_result.transaction_hash)
        }
        Err(RpcCallError::Rpc(error)) if error.is_already_declared() => {
            println!(
                "declare skipped: class already declared, using computed class_hash={}",
                felt_hex(class_hash)
            );
            None
        }
        Err(error) => return Err(error.into()),
    };

    let salt = deployment_salt()?;
    let factory = ContractFactory::new_with_udc(class_hash, account, UdcSelector::New);
    let deployment = factory
        .deploy_v3(Vec::new(), salt, false)
        .nonce(fetch_account_nonce(&receipt_provider, account_address).await?)
        .l1_gas(resource_bounds.max_amount)
        .l1_gas_price(resource_bounds.max_price_per_unit)
        .l2_gas(resource_bounds.max_amount)
        .l2_gas_price(resource_bounds.max_price_per_unit)
        .l1_data_gas(resource_bounds.max_amount)
        .l1_data_gas_price(resource_bounds.max_price_per_unit)
        .tip(DEPLOY_TIP);
    let deployed_address = deployment.deployed_address();
    let prepared_deploy = ExecutionV3::from(&deployment)
        .prepared()
        .map_err(|error| AppError::Artifact(error.to_string()))?;
    let invoke_request = prepared_deploy
        .get_invoke_request(false, false)
        .await
        .map_err(|error| AppError::Account(format!("{error:?}")))?;
    let invoke_result: InvokeTransactionResult = rpc
        .rpc_call_typed("starknet_addInvokeTransaction", json!([invoke_request]))
        .await?;

    let _invoke_receipt = wait_for_successful_receipt(
        &receipt_provider,
        invoke_result.transaction_hash,
        "deploy",
    )
    .await?;

    wait_for_class_hash_at(
        &receipt_provider,
        deployed_address,
        class_hash,
    )
    .await?;

    let deployed_address_hex = validated_felt_hex("deployed address", deployed_address)?;
    write_address_file(&args.out_file, &deployed_address_hex)?;

    println!("deploy complete");
    println!("rpc_url={}", cli.rpc_url);
    println!("account_address={}", cli.account_address);
    println!("private_key_supplied={}", private_key_supplied(cli));
    println!("sierra_artifact={}", args.sierra_artifact.display());
    println!("casm_artifact={}", args.casm_artifact.display());
    println!("class_hash={}", felt_hex(class_hash));
    println!("compiled_class_hash={}", felt_hex(compiled_class_hash));
    println!("casm_artifact_hash={}", felt_hex(casm_artifact_hash));
    println!("resource_bounds_max_amount={}", resource_bounds.max_amount);
    println!(
        "resource_bounds_max_price_per_unit={}",
        resource_bounds.max_price_per_unit
    );
    match declare_tx_hash {
        Some(transaction_hash) => println!("declare_tx_hash={}", felt_hex(transaction_hash)),
        None => println!("declare_tx_hash=already_declared"),
    }
    println!("invoke_tx_hash={}", felt_hex(invoke_result.transaction_hash));
    println!("deployed_address={deployed_address_hex}");
    println!("wrote {}", args.out_file.display());

    Ok(())
}

fn emit_block_proposed_stub(cli: &Cli, args: &EmitBlockProposedArgs) {
    println!("emit-block-proposed stub complete");
    println!("rpc_url={}", cli.rpc_url);
    println!("private_key_supplied={}", private_key_supplied(cli));
    println!("contract={}", args.contract);
    println!("block_number={}", args.block_number);
    println!("transactions_root={}", args.transactions_root);
    println!("timestamp={}", args.timestamp);
    println!("invoke_tx_hash=0x201");
}

fn emit_deposit_escrowed_stub(cli: &Cli, args: &EmitDepositEscrowedArgs) {
    println!("emit-deposit-escrowed stub complete");
    println!("rpc_url={}", cli.rpc_url);
    println!("private_key_supplied={}", private_key_supplied(cli));
    println!("contract={}", args.contract);
    println!("commitment={}", args.commitment);
    println!("token_id={}", args.token_id);
    println!("value_low={}", args.value_low);
    println!("value_high={}", args.value_high);
    println!("invoke_tx_hash=0x202");
}

fn private_key_supplied(cli: &Cli) -> bool {
    cli.private_key != DEFAULT_PRIVATE_KEY
}

fn new_provider(rpc_url: &str) -> Result<JsonRpcClient<HttpTransport>, AppError> {
    Ok(JsonRpcClient::new(HttpTransport::new(Url::parse(rpc_url)?)))
}

fn read_json_file<T>(path: &Path) -> Result<T, AppError>
where
    T: for<'de> Deserialize<'de>,
{
    let file = File::open(path)?;
    Ok(serde_json::from_reader(file)?)
}

fn compile_like_devnet(contract_class: &FlattenedSierraClass) -> Result<Felt, AppError> {
    let normalized_contract_class = normalize_flattened_contract_class(contract_class)?;
    let sierra_contract_class: cairo_lang_starknet_classes::contract_class::ContractClass =
        serde_json::from_value(normalized_contract_class)?;
    let compiled_json = usc::compile_contract(serde_json::to_value(&sierra_contract_class)?)
        .map_err(|error| AppError::Artifact(format!("universal sierra compile failed: {error}")))?;
    let compiled_class: CompiledClass = serde_json::from_value(compiled_json)?;
    compiled_class
        .class_hash()
        .map_err(|error| AppError::Artifact(error.to_string()))
}

fn normalize_flattened_contract_class(
    contract_class: &FlattenedSierraClass,
) -> Result<Value, AppError> {
    let mut json_obj = serde_json::to_value(contract_class)?;

    if let Some(Value::String(abi_string)) = json_obj.get("abi") {
        if abi_string.is_empty() {
            json_obj
                .as_object_mut()
                .ok_or_else(|| {
                    AppError::Artifact("flattened contract class is not an object".to_owned())
                })?
                .remove("abi");
        } else {
            let abi_value: Value = serde_json::from_str(abi_string)?;
            json_obj
                .as_object_mut()
                .ok_or_else(|| {
                    AppError::Artifact("flattened contract class is not an object".to_owned())
                })?
                .insert("abi".to_owned(), abi_value);
        }
    }

    Ok(json_obj)
}

async fn fetch_chain_id(rpc: &RpcClient) -> Result<Felt, AppError> {
    let result = rpc.rpc_call("starknet_chainId", json!([])).await?;
    let chain_id = result
        .as_str()
        .ok_or_else(|| AppError::Artifact(format!("unexpected chain id response: {result}")))?;
    parse_felt("chain id", chain_id)
}

async fn fetch_account_nonce(
    provider: &JsonRpcClient<HttpTransport>,
    account_address: Felt,
) -> Result<Felt, AppError> {
    provider
        .get_nonce(BlockId::Tag(BlockTag::Latest), account_address)
        .await
        .map_err(|error| AppError::Provider(error.to_string()))
}

fn parse_felt(name: &'static str, value: &str) -> Result<Felt, AppError> {
    if value.starts_with("0x") || value.starts_with("0X") {
        Felt::from_hex(value).map_err(|_| AppError::InvalidFelt {
            name,
            value: value.to_owned(),
        })
    } else {
        Felt::from_dec_str(value).map_err(|_| AppError::InvalidFelt {
            name,
            value: value.to_owned(),
        })
    }
}

fn parse_u64_cli_value(value: &str) -> Result<u64, String> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16)
            .map_err(|_| format!("invalid u64 value `{trimmed}`; expected decimal or 0x-prefixed hex"))
    } else {
        trimmed
            .parse::<u64>()
            .map_err(|_| format!("invalid u64 value `{trimmed}`; expected decimal or 0x-prefixed hex"))
    }
}

fn parse_u128_cli_value(value: &str) -> Result<u128, String> {
    let trimmed = value.trim();
    if let Some(hex) = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
    {
        u128::from_str_radix(hex, 16)
            .map_err(|_| format!("invalid u128 value `{trimmed}`; expected decimal or 0x-prefixed hex"))
    } else {
        trimmed
            .parse::<u128>()
            .map_err(|_| format!("invalid u128 value `{trimmed}`; expected decimal or 0x-prefixed hex"))
    }
}

fn felt_hex(value: Felt) -> String {
    format!("{value:#066x}")
}

fn validated_felt_hex(name: &'static str, value: Felt) -> Result<String, AppError> {
    let hex = felt_hex(value);
    if hex.len() != 66 {
        return Err(AppError::Artifact(format!(
            "{name} is not a 32-byte felt: {hex}"
        )));
    }
    Ok(hex)
}

fn deployment_salt() -> Result<Felt, AppError> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    parse_felt("deployment salt", &nanos.to_string())
}

async fn wait_for_successful_receipt(
    provider: &JsonRpcClient<HttpTransport>,
    transaction_hash: Felt,
    label: &'static str,
) -> Result<TransactionReceipt, AppError> {
    for _ in 0..MAX_RECEIPT_POLL_ATTEMPTS {
        match provider.get_transaction_receipt(transaction_hash).await {
            Ok(receipt_with_block) => match &receipt_with_block.receipt {
                receipt @ TransactionReceipt::Declare(inner) => match &inner.execution_result {
                    ExecutionResult::Succeeded => return Ok(receipt.clone()),
                    ExecutionResult::Reverted { reason } => {
                        return Err(AppError::Reverted {
                            transaction_hash: felt_hex(transaction_hash),
                            reason: reason.clone(),
                        })
                    }
                },
                receipt @ TransactionReceipt::Invoke(inner) => match &inner.execution_result {
                    ExecutionResult::Succeeded => return Ok(receipt.clone()),
                    ExecutionResult::Reverted { reason } => {
                        return Err(AppError::Reverted {
                            transaction_hash: felt_hex(transaction_hash),
                            reason: reason.clone(),
                        })
                    }
                },
                receipt => match receipt.execution_result() {
                    ExecutionResult::Succeeded => return Ok(receipt.clone()),
                    ExecutionResult::Reverted { reason } => {
                        return Err(AppError::Reverted {
                            transaction_hash: felt_hex(transaction_hash),
                            reason: reason.clone(),
                        })
                    }
                },
            },
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => {
                sleep(RECEIPT_POLL_INTERVAL).await;
            }
            Err(error) => return Err(AppError::Provider(error.to_string())),
        }
    }

    Err(AppError::ReceiptTimeout {
        label,
        transaction_hash: felt_hex(transaction_hash),
    })
}

async fn wait_for_class_hash_at(
    provider: &JsonRpcClient<HttpTransport>,
    deployed_address: Felt,
    expected_class_hash: Felt,
) -> Result<(), AppError> {
    for _ in 0..MAX_RECEIPT_POLL_ATTEMPTS {
        match provider
            .get_class_hash_at(BlockId::Tag(BlockTag::PreConfirmed), deployed_address)
            .await
        {
            Ok(actual_class_hash) if actual_class_hash == expected_class_hash => return Ok(()),
            Ok(actual_class_hash) => {
                return Err(AppError::DeployedClassHashMismatch {
                    address: felt_hex(deployed_address),
                    expected: felt_hex(expected_class_hash),
                    actual: felt_hex(actual_class_hash),
                })
            }
            Err(ProviderError::StarknetError(StarknetError::ContractNotFound)) => {
                sleep(RECEIPT_POLL_INTERVAL).await;
            }
            Err(error) => return Err(AppError::Provider(error.to_string())),
        }
    }

    Err(AppError::ReceiptTimeout {
        label: "deploy",
        transaction_hash: felt_hex(deployed_address),
    })
}

fn write_address_file(path: &Path, deployed_address_hex: &str) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, format!("{deployed_address_hex}\n"))?;
    Ok(())
}
