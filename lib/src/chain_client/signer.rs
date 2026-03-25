use async_trait::async_trait;

use super::types::Address;
use super::ChainClientError;

#[async_trait]
pub trait ChainSigner: Send + Sync {
    async fn address(&self) -> Result<Address, ChainClientError>;

    async fn sign_message_hash(&self, message_hash: [u8; 32]) -> Result<Vec<u8>, ChainClientError>;
}

#[cfg(feature = "backend_evm")]
pub mod evm {
    use super::*;

    use alloy::signers::Signer;
    use crate::wallets::WalletType;

    pub struct EvmChainSigner {
        wallet: WalletType,
    }

    impl EvmChainSigner {
        pub fn new(wallet: WalletType) -> Self {
            Self { wallet }
        }
    }

    #[async_trait]
    impl ChainSigner for EvmChainSigner {
        async fn address(&self) -> Result<Address, ChainClientError> {
            let evm_address = match &self.wallet {
                WalletType::Local(signer) => signer.address(),
                WalletType::Azure(wallet) => wallet.address(),
            };

            Ok(Address::from(evm_address))
        }

        async fn sign_message_hash(
            &self,
            message_hash: [u8; 32],
        ) -> Result<Vec<u8>, ChainClientError> {
            let signature = match &self.wallet {
                WalletType::Local(signer) => signer
                    .sign_message(&message_hash)
                    .await
                    .map_err(|e| ChainClientError::Rpc(e.to_string()))?,
                WalletType::Azure(wallet) => wallet
                    .sign(&message_hash)
                    .await
                    .map_err(|e| ChainClientError::Rpc(e.to_string()))?,
            };

            Ok(signature.as_bytes().to_vec())
        }
    }
}

#[cfg(feature = "backend_starknet")]
pub mod starknet {
    use async_trait::async_trait;

    use super::{Address, ChainClientError, ChainSigner};
    use ::starknet::core::types::Felt;
    use ::starknet::signers::{LocalWallet, Signer, SigningKey};

    pub struct StarknetSigner {
        wallet: LocalWallet,
        account_address: Address,
    }

    impl StarknetSigner {
        pub fn from_hex_key(
            private_key_hex: &str,
            account_address_hex: &str,
        ) -> Result<Self, ChainClientError> {
            let private_felt = Felt::from_hex(private_key_hex)
                .map_err(|e| ChainClientError::Rpc(format!("invalid starknet private key: {e}")))?;
            let wallet = LocalWallet::from(SigningKey::from_secret_scalar(private_felt));

            let account_address = Address::from_hex_str(account_address_hex)
                .map_err(|e| ChainClientError::Rpc(format!("invalid starknet account address: {e}")))?;

            Ok(Self {
                wallet,
                account_address,
            })
        }
    }

    #[async_trait]
    impl ChainSigner for StarknetSigner {
        async fn address(&self) -> Result<Address, ChainClientError> {
            Ok(self.account_address)
        }

        async fn sign_message_hash(
            &self,
            message_hash: [u8; 32],
        ) -> Result<Vec<u8>, ChainClientError> {
            let hash_felt = Felt::from_bytes_be(&message_hash);
            let signature = self
                .wallet
                .sign_hash(&hash_felt)
                .await
                .map_err(|e| ChainClientError::Rpc(format!("starknet signing failed: {e}")))?;

            let mut out = Vec::with_capacity(64);
            out.extend_from_slice(&signature.r.to_bytes_be());
            out.extend_from_slice(&signature.s.to_bytes_be());
            Ok(out)
        }
    }
}
