use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use sha3::{Digest, Keccak256};
use std::path::PathBuf;

const DEFAULT_SENDER: &str =
    "0x127fd5f1fe78a71f8bcd1fec63e3fe2f0486b6ecd5c86a0466c3a21fa5cfcec";

#[derive(Parser, Debug)]
#[command(author, version, about = "Katana dev-mode deploy/invoke tool")]
struct Args {
    #[arg(long, default_value = "http://localhost:5050")]
    rpc_url: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Check RPC connectivity
    Ping,
    /// Declare + deploy DummyEmitter, write address to file
    Deploy {
        /// .contract_class.json from `scarb build`
        #[arg(long)]
        artifact: PathBuf,
        /// .compiled_contract_class.json (CASM) from `scarb build`
        #[arg(long)]
        compiled_artifact: Option<PathBuf>,
        /// Pre-computed compiled class hash (if you don't pass compiled_artifact)
        #[arg(long)]
        compiled_class_hash: Option<String>,
        /// Where to write the deployed address
        #[arg(long, default_value = "../artifacts/dummy_emitter_address.txt")]
        out_file: PathBuf,
        /// Katana pre-funded account address
        #[arg(long, default_value = DEFAULT_SENDER)]
        sender: String,
    },
    /// Invoke emit_block_proposed
    EmitBlockProposed {
        #[arg(long)]
        contract: String,
        #[arg(long, default_value = DEFAULT_SENDER)]
        sender: String,
        #[arg(long, default_value = "1")]
        block_number: u64,
        #[arg(long, default_value = "0xdeadbeef")]
        transactions_root: String,
        #[arg(long, default_value = "1700000000")]
        timestamp: u64,
    },
    /// Invoke emit_deposit_escrowed
    EmitDepositEscrowed {
        #[arg(long)]
        contract: String,
        #[arg(long, default_value = DEFAULT_SENDER)]
        sender: String,
        #[arg(long, default_value = "0xaaaa")]
        commitment: String,
        #[arg(long, default_value = "0x1")]
        token_id: String,
        #[arg(long, default_value = "0x64")]
        value_low: String,
        #[arg(long, default_value = "0x0")]
        value_high: String,
    },
    /// Fetch events from Katana
    GetEvents {
        #[arg(long)]
        contract: Option<String>,
        #[arg(long, default_value_t = 0)]
        from_block: u64,
        #[arg(long, default_value_t = 10000)]
        to_block: u64,
    },
}

// ── JSON-RPC helper ─────────────────────────────────────────────────

struct Rpc {
    http: reqwest::Client,
    url: String,
}

impl Rpc {
    fn new(url: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            url: url.into(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let body = json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
        eprintln!(">>> {method}");
        let resp = self
            .http
            .post(&self.url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("POST {method}"))?;
        let v: Value = resp.json().await.context("parse json")?;
        if let Some(e) = v.get("error") {
            bail!("RPC {method}: {e}");
        }
        v.get("result")
            .cloned()
            .ok_or_else(|| anyhow!("no result from {method}: {v}"))
    }

    async fn nonce(&self, addr: &str) -> Result<String> {
        let r = self
            .call("starknet_getNonce", json!(["latest", addr]))
            .await?;
        Ok(r.as_str().unwrap_or("0x0").to_string())
    }
}

// ── Starknet helpers ────────────────────────────────────────────────

/// starknet_keccak(name) = keccak256(name) with top 6 bits zeroed (250-bit).
fn sn_keccak(name: &str) -> String {
    let mut hasher = Keccak256::new();
    hasher.update(name.as_bytes());
    let hash = hasher.finalize();
    let mut b = [0u8; 32];
    b.copy_from_slice(&hash);
    b[0] &= 0x03; // mask top 6 bits → 250-bit
    format!("0x{}", hex::encode(b))
}

fn invoke_v3(sender: &str, calldata: &[String], nonce: &str) -> Value {
    json!({
        "type": "INVOKE",
        "sender_address": sender,
        "calldata": calldata,
        "version": "0x3",
        "signature": [],
        "nonce": nonce,
        "tip": "0x0",
        "resource_bounds": {
            "l1_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
            "l2_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" }
        },
        "paymaster_data": [],
        "account_deployment_data": [],
        "nonce_data_availability_mode": "L1",
        "fee_data_availability_mode": "L1",
    })
}

/// Try to extract the deployed contract address from a tx receipt.
fn extract_address(receipt: &Value) -> Option<String> {
    for key in &["events", "receipt"] {
        let events = if *key == "events" {
            receipt.get("events")
        } else {
            receipt.get("receipt").and_then(|r| r.get("events"))
        };
        if let Some(arr) = events.and_then(|v| v.as_array()) {
            for ev in arr {
                if let Some(data) = ev.get("data").and_then(|d| d.as_array()) {
                    if let Some(s) = data.first().and_then(|v| v.as_str()) {
                        return Some(s.to_string());
                    }
                }
            }
        }
    }
    None
}

// ── main ────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let rpc = Rpc::new(&args.rpc_url);

    match args.cmd {
        // ── ping ────────────────────────────────────────────────
        Cmd::Ping => {
            let bn = rpc.call("starknet_blockNumber", json!([])).await?;
            println!("ok: block_number={bn}");
        }

        // ── deploy ─────────────────────────────────────────────
        Cmd::Deploy {
            artifact,
            compiled_artifact,
            compiled_class_hash,
            out_file,
            sender,
        } => {
            let art_str = std::fs::read_to_string(&artifact).context("read artifact")?;
            let mut contract_class: Value =
                serde_json::from_str(&art_str).context("parse artifact")?;

            // Starknet API expects `abi` as a JSON string, not array
            if let Some(abi) = contract_class.get("abi") {
                if abi.is_array() {
                    let abi_str = serde_json::to_string(abi).unwrap();
                    contract_class["abi"] = json!(abi_str);
                }
            }
            // Strip debug info (not needed)
            contract_class.as_object_mut().map(|m| m.remove("sierra_program_debug_info"));

            // Resolve compiled_class_hash: from flag, or via `starkli class-hash`, or "0x0"
            let cch = if let Some(h) = compiled_class_hash {
                h
            } else if let Some(ref casm_path) = compiled_artifact {
                // Try starkli
                let out = std::process::Command::new("starkli")
                    .args(["class-hash", &casm_path.to_string_lossy()])
                    .output()
                    .context("run starkli class-hash")?;
                if !out.status.success() {
                    bail!(
                        "starkli class-hash failed: {}",
                        String::from_utf8_lossy(&out.stderr)
                    );
                }
                String::from_utf8_lossy(&out.stdout).trim().to_string()
            } else {
                eprintln!("WARNING: no compiled_class_hash or compiled_artifact; using 0x0");
                "0x0".into()
            };
            println!("compiled_class_hash: {cch}");

            // ---- declare (v3) ----
            println!("==> Declaring…");
            let nonce = rpc.nonce(&sender).await?;
            let decl = json!({
                "type": "DECLARE",
                "sender_address": sender,
                "contract_class": contract_class,
                "compiled_class_hash": cch,
                "version": "0x3",
                "signature": [],
                "nonce": nonce,
                "tip": "0x0",
                "resource_bounds": {
                    "l1_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" },
                    "l2_gas": { "max_amount": "0x0", "max_price_per_unit": "0x0" }
                },
                "paymaster_data": [],
                "account_deployment_data": [],
                "nonce_data_availability_mode": "L1",
                "fee_data_availability_mode": "L1",
            });
            let dr = rpc
                .call(
                    "starknet_addDeclareTransaction",
                    json!([decl]),
                )
                .await;

            let class_hash = match dr {
                Ok(v) => {
                    let ch = v
                        .get("class_hash")
                        .and_then(|x| x.as_str())
                        .ok_or_else(|| anyhow!("no class_hash: {v}"))?
                        .to_string();
                    println!("class_hash: {ch}");
                    ch
                }
                Err(e) => {
                    let m = format!("{e}");
                    if m.to_lowercase().contains("already") {
                        println!("Already declared – {m}");
                        bail!("Restart Katana or supply class_hash manually.");
                    }
                    return Err(e);
                }
            };

            // ---- deploy via UDC ----
            let udc =
                "0x041a78e741e5af2fec34b695679bc6891742439f7afb8484ecd7766661ad02bf";
            let sel = sn_keccak("deployContract");

            // Wait briefly for the declare block to be mined so nonce updates
            println!("Waiting 2s for declare block…");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            let nonce2 = rpc.nonce(&sender).await?;
            let cd: Vec<String> = vec![
                "0x1".into(),           // num_calls
                udc.into(),             // to (UDC)
                sel.clone(),            // selector
                "0x4".into(),           // calldata_len
                class_hash.clone(),     // arg: classHash
                "0x1234".into(),        // arg: salt
                "0x0".into(),           // arg: unique (false)
                "0x0".into(),           // arg: constructor_calldata_len
            ];

            println!("==> Deploying via UDC (sel={sel})…");
            let tx = invoke_v3(&sender, &cd, &nonce2);
            let res = rpc
                .call(
                    "starknet_addInvokeTransaction",
                    json!([tx]),
                )
                .await?;
            let txh = res
                .get("transaction_hash")
                .and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("no tx hash: {res}"))?;
            println!("deploy tx: {txh}");

            println!("Waiting 3s for block…");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;

            let receipt = rpc
                .call("starknet_getTransactionReceipt", json!([txh]))
                .await?;
            println!(
                "receipt:\n{}",
                serde_json::to_string_pretty(&receipt).unwrap_or_default()
            );

            let addr = extract_address(&receipt).ok_or_else(|| {
                anyhow!("Could not find address in receipt. Check output above.")
            })?;
            println!("\n✅  contract: {addr}");
            if let Some(p) = out_file.parent() {
                std::fs::create_dir_all(p)?;
            }
            std::fs::write(&out_file, &addr)?;
            println!("wrote {}", out_file.display());
        }

        // ── emit-block-proposed ────────────────────────────────
        Cmd::EmitBlockProposed {
            contract,
            sender,
            block_number,
            transactions_root,
            timestamp,
        } => {
            let sel = sn_keccak("emit_block_proposed");
            let nonce = rpc.nonce(&sender).await?;
            let cd: Vec<String> = vec![
                "0x1".into(),
                contract,
                sel.clone(),
                "0x3".into(),
                format!("0x{:x}", block_number),
                transactions_root,
                format!("0x{:x}", timestamp),
            ];
            println!("==> emit_block_proposed (sel={sel})");
            let tx = invoke_v3(&sender, &cd, &nonce);
            let r = rpc
                .call(
                    "starknet_addInvokeTransaction",
                    json!([tx]),
                )
                .await?;
            println!("result: {r}");
        }

        // ── emit-deposit-escrowed ──────────────────────────────
        Cmd::EmitDepositEscrowed {
            contract,
            sender,
            commitment,
            token_id,
            value_low,
            value_high,
        } => {
            let sel = sn_keccak("emit_deposit_escrowed");
            let nonce = rpc.nonce(&sender).await?;
            let cd: Vec<String> = vec![
                "0x1".into(),
                contract,
                sel.clone(),
                "0x4".into(),
                commitment,
                token_id,
                value_low,
                value_high,
            ];
            println!("==> emit_deposit_escrowed (sel={sel})");
            let tx = invoke_v3(&sender, &cd, &nonce);
            let r = rpc
                .call(
                    "starknet_addInvokeTransaction",
                    json!([tx]),
                )
                .await?;
            println!("result: {r}");
        }

        // ── get-events ─────────────────────────────────────────
        Cmd::GetEvents {
            contract,
            from_block,
            to_block,
        } => {
            let mut f = json!({
                "from_block": {"block_number": from_block},
                "to_block":   {"block_number": to_block},
                "keys": [],
            });
            if let Some(a) = contract {
                f["address"] = json!(a);
            }
            let ev = rpc
                .call(
                    "starknet_getEvents",
                    json!([{"filter": f, "chunk_size": 100}]),
                )
                .await?;
            println!("{}", serde_json::to_string_pretty(&ev)?);
        }
    }
    Ok(())
}
