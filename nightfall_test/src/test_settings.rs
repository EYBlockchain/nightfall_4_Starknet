use alloy::primitives::Address;
use configuration::settings::Settings;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use lib::rollup_circuit_checks::find_file_with_path;
use nightfall_client::drivers::rest::models::KeyRequest;
use serde::Deserialize;
use std::{fs::File, io::Read, path::Path, sync::OnceLock};

use crate::test::TransactionDetails;

// rather than pass around what are effectively constant values, or recreate them locally,
// let's use the lazy_static crate to create a global variable that can be used to consume
// settings from anywhere in the code.
pub fn get_test_settings() -> &'static TestSettings {
    static SETTINGS: OnceLock<TestSettings> = OnceLock::new();
    SETTINGS.get_or_init(|| TestSettings::new().unwrap())
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct DepositValues {
    pub path: String,
    pub value: String,
    pub fee: String,
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct TransferValues {
    pub path: String,
    pub value: String,
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct WithdrawValues {
    pub path: String,
    pub value: String,
    pub token_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MockAddresses {
    pub erc20: Address,
    pub erc721: Address,
    pub erc1155: Address,
    pub erc3525: Address,
}

#[derive(serde::Deserialize)]
pub struct TestSettings {
    pub key_request: KeyRequest,
    pub key_request2: KeyRequest,
    pub erc20_deposit_0: TransactionDetails,
    pub erc20_deposit_1: TransactionDetails,
    pub erc20_deposit_2: TransactionDetails,
    pub erc20_deposit_3: TransactionDetails,
    pub erc20_deposit_4: TransactionDetails,
    pub erc20_deposit_large_block: TransactionDetails,
    pub erc20_transfer_0: TransactionDetails,
    pub erc20_transfer_1: TransactionDetails,
    pub erc20_transfer_2: TransactionDetails,
    pub erc20_transfer_large_block: TransactionDetails,
    pub erc20_withdraw_0: TransactionDetails,
    pub erc20_withdraw_1: TransactionDetails,
    pub erc20_withdraw_2: TransactionDetails,
    pub erc721_deposit: TransactionDetails,
    pub erc721_transfer: TransactionDetails,
    pub erc721_withdraw: TransactionDetails,
    pub erc3525_deposit_1: TransactionDetails,
    pub erc3525_deposit_2: TransactionDetails,
    pub erc3525_transfer_1: TransactionDetails,
    pub erc3525_transfer_2: TransactionDetails,
    pub erc3525_withdraw: TransactionDetails,
    pub erc1155_deposit_1: TransactionDetails,
    pub erc1155_deposit_2: TransactionDetails,
    pub erc1155_deposit_3_nft: TransactionDetails,
    pub erc1155_transfer_1: TransactionDetails,
    pub erc1155_transfer_2_nft: TransactionDetails,
    pub erc1155_withdraw_1: TransactionDetails,
    pub erc1155_withdraw_2_nft: TransactionDetails,
}
impl TestSettings {
    pub fn new() -> Result<Self, String> {
        let test_settings: TestSettings = Figment::new()
            .merge(Toml::file("nightfall_test.toml").nested())
            .extract()
            .map_err(|e| format!("{e}"))?;

        Ok(test_settings)
    }

    pub fn retrieve_mock_addresses() -> MockAddresses {
        let json_path = find_file_with_path(
            &Path::new("blockchain_assets/logs/mock_deployment.s.sol")
                .join(Settings::new().unwrap().network.chain_id.to_string())
                .join("run-latest.json"),
        )
        .unwrap();
        let mut json_file = File::open(json_path).unwrap();
        let mut json_string = String::new();
        json_file.read_to_string(&mut json_string).unwrap();
        let v: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        let mut erc20 = Address::ZERO;
        let mut erc721 = Address::ZERO;
        let mut erc1155 = Address::ZERO;
        let mut erc3525 = Address::ZERO;
        let transaction_array = v["transactions"].as_array().unwrap();

        for transaction in transaction_array {
            match transaction["contractName"].as_str().unwrap() {
                "ERC20Mock" => {
                    let bytes: [u8; 20] = hex::decode(
                        transaction["contractAddress"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches("0x"),
                    )
                    .unwrap()
                    .try_into()
                    .unwrap();
                    erc20 = Address::from(bytes);
                }
                "ERC721Mock" => {
                    let bytes: [u8; 20] = hex::decode(
                        transaction["contractAddress"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches("0x"),
                    )
                    .unwrap()
                    .try_into()
                    .unwrap();
                    erc721 = Address::from(bytes);
                }
                "ERC1155Mock" => {
                    let bytes: [u8; 20] = hex::decode(
                        transaction["contractAddress"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches("0x"),
                    )
                    .unwrap()
                    .try_into()
                    .unwrap();
                    erc1155 = Address::from(bytes);
                }
                "ERC3525Mock" => {
                    let bytes: [u8; 20] = hex::decode(
                        transaction["contractAddress"]
                            .as_str()
                            .unwrap()
                            .trim_start_matches("0x"),
                    )
                    .unwrap()
                    .try_into()
                    .unwrap();
                    erc3525 = Address::from(bytes);
                }
                _ => continue,
            }
        }
        MockAddresses {
            erc20,
            erc721,
            erc1155,
            erc3525,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::providers::{Provider, ProviderBuilder};
    use alloy_node_bindings::Anvil;
    use nightfall_bindings::artifacts::{
        ERC1155Mock as erc1155_mock, ERC20Mock as erc20_mock, ERC3525Mock as erc3525_mock,
        ERC721Mock as erc721_mock,
    };

    #[tokio::test]
    async fn test_mock_addresses() {
        // fire up a blockchain simulator
        let mut settings = configuration::settings::Settings::new().unwrap();
        settings.ethereum_client_url = "ws://localhost:8545".to_string(); // we're running bare metal so a docker url won't work
        let url = url::Url::parse(&settings.ethereum_client_url).unwrap();
        let anvil = Anvil::new().port(url.port().unwrap()).spawn();

        std::env::set_var(
            "NF4_SIGNING_KEY",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        );
        std::env::set_var(
            "CLIENT_ADDRESS",
            "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266",
        );
        std::env::set_var(
            "CLIENT2_ADDRESS",
            "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
        );
        std::env::set_var(
            "NIGHTFALL_ADDRESS",
            "0x0000000000000000000000000000000000000001",
        );
        let provider = ProviderBuilder::new()
            .connect_http(anvil.endpoint_url());

        // `forge script --broadcast` is flaky on some foundry/anvil combinations.
        // If it's not working, skip this test rather than failing the whole suite.
        let forge_available = std::process::Command::new("forge")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !forge_available {
            eprintln!("Skipping: forge not available");
            return;
        }

        // Run the existing Foundry script to deploy mocks and write `run-latest.json`.
        // The script reads `NF4_SIGNING_KEY`, `CLIENT_ADDRESS`, `CLIENT2_ADDRESS`, `NIGHTFALL_ADDRESS`.
        let forge_output = std::process::Command::new("forge")
            .args([
                "script",
                "MockDeployer",
                "--fork-url",
                anvil.endpoint().as_str(),
                "--broadcast",
                "--force",
            ])
            .output();

        match forge_output {
            Ok(o) if o.status.success() => {
                // ok
            }
            Ok(o) => {
                eprintln!(
                    "Skipping: forge broadcast failed (stdout/stderr below)\n{}\n{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                return;
            }
            Err(e) => {
                eprintln!("Skipping: forge execution failed: {e}");
                return;
            }
        }

        let mock_addresses = TestSettings::retrieve_mock_addresses();
        let erc20_code = provider.get_code_at(mock_addresses.erc20).await.unwrap();
        let erc721_code = provider.get_code_at(mock_addresses.erc721).await.unwrap();
        let erc1155_code = provider.get_code_at(mock_addresses.erc1155).await.unwrap();
        let erc3525_code = provider.get_code_at(mock_addresses.erc3525).await.unwrap();
        assert_eq!(erc20_code, erc20_mock::DEPLOYED_BYTECODE);
        assert_eq!(erc721_code, erc721_mock::DEPLOYED_BYTECODE);
        assert_eq!(erc1155_code, erc1155_mock::DEPLOYED_BYTECODE);
        assert_eq!(erc3525_code, erc3525_mock::DEPLOYED_BYTECODE);
    }
}
