use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChainId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockNumber(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractId(pub Address);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEvent {
    pub block_number: BlockNumber,
    pub tx_hash: TxHash,
    pub contract: ContractId,
    pub keys: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventFilter {
    pub contract: Option<ContractId>,
    pub keys: Vec<[u8; 32]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct U256(pub [u8; 32]);

#[cfg(feature = "backend_evm")]
mod evm_conversions {
    use super::*;

    impl From<alloy::primitives::Address> for Address {
        fn from(value: alloy::primitives::Address) -> Self {
            let mut out = [0u8; 32];
            out[12..].copy_from_slice(value.as_slice());
            Address(out)
        }
    }

    impl From<alloy::primitives::TxHash> for TxHash {
        fn from(value: alloy::primitives::TxHash) -> Self {
            TxHash(value.into())
        }
    }

    impl From<alloy::primitives::U256> for U256 {
        fn from(value: alloy::primitives::U256) -> Self {
            U256(value.to_be_bytes())
        }
    }
}

#[cfg(feature = "backend_starknet")]
mod starknet_conversions {
    // Intentionally left minimal: Starknet types/deps will be added under the
    // `backend_starknet` feature in later stories.
}
