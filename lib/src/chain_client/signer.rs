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
