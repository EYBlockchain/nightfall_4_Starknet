use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use thiserror::Error;

const DEFAULT_RPC_URL: &str = "http://localhost:5050/rpc";
const DEFAULT_SIERRA_ARTIFACT: &str = "../cairo1_dummy_emitter/target/dev/cairo1_dummy_emitter_DummyEmitter.contract_class.json";
const DEFAULT_CASM_ARTIFACT: &str = "../cairo1_dummy_emitter/target/dev/cairo1_dummy_emitter_DummyEmitter.compiled_contract_class.json";
const DEFAULT_ADDRESS_FILE: &str = "../artifacts/dummy_emitter_address.txt";

#[derive(Parser, Debug)]
#[command(author, version, about = "Starknet emitter tool")]
struct Cli {
    #[arg(long, global = true, default_value = DEFAULT_RPC_URL)]
    rpc_url: String,
    #[arg(long, global = true)]
    private_key: Option<String>,
    #[command(subcommand)]
    command: Command,
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
        write!(formatter, "{}: {}", self.code, self.message)
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
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let rpc = RpcClient::new(cli.rpc_url.clone());

    match &cli.command {
        Command::Ping => ping(&rpc).await?,
        Command::Deploy(args) => deploy_stub(&cli, args),
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

fn deploy_stub(cli: &Cli, args: &DeployArgs) {
    println!("deploy not implemented yet");
    println!("rpc_url={}", cli.rpc_url);
    println!("private_key_supplied={}", cli.private_key.is_some());
    println!("sierra_artifact={}", args.sierra_artifact.display());
    println!("casm_artifact={}", args.casm_artifact.display());
    println!("out_file={}", args.out_file.display());
}

fn emit_block_proposed_stub(cli: &Cli, args: &EmitBlockProposedArgs) {
    println!("emit-block-proposed not implemented yet");
    println!("rpc_url={}", cli.rpc_url);
    println!("private_key_supplied={}", cli.private_key.is_some());
    println!("contract={}", args.contract);
    println!("block_number={}", args.block_number);
    println!("transactions_root={}", args.transactions_root);
    println!("timestamp={}", args.timestamp);
}

fn emit_deposit_escrowed_stub(cli: &Cli, args: &EmitDepositEscrowedArgs) {
    println!("emit-deposit-escrowed not implemented yet");
    println!("rpc_url={}", cli.rpc_url);
    println!("private_key_supplied={}", cli.private_key.is_some());
    println!("contract={}", args.contract);
    println!("commitment={}", args.commitment);
    println!("token_id={}", args.token_id);
    println!("value_low={}", args.value_low);
    println!("value_high={}", args.value_high);
}
