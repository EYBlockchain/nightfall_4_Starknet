use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChainId(pub [u8; 32]);

impl ChainId {
    pub fn from_evm_u64(value: u64) -> Self {
        let mut out = [0u8; 32];
        out[24..].copy_from_slice(&value.to_be_bytes());
        ChainId(out)
    }

    pub fn from_hex_str(hex: &str) -> Result<Self, &'static str> {
        let s = hex.strip_prefix("0x").unwrap_or(hex);
        if s.is_empty() || s.len() > 64 {
            return Err("invalid hex length");
        }

        let mut decoded = Vec::with_capacity(s.len().div_ceil(2));
        let mut i = 0usize;

        if s.len() % 2 == 1 {
            let b = u8::from_str_radix(&format!("0{}", &s[0..1]), 16).map_err(|_| "invalid hex")?;
            decoded.push(b);
            i = 1;
        }
        while i < s.len() {
            let b = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid hex")?;
            decoded.push(b);
            i += 2;
        }

        if decoded.len() > 32 {
            return Err("invalid hex length");
        }

        let mut out = [0u8; 32];
        out[32 - decoded.len()..].copy_from_slice(&decoded);
        Ok(ChainId(out))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BlockNumber(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TxHash(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockHash(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContractId(pub Address);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

impl Address {
    pub fn from_hex_str(hex: &str) -> Result<Self, &'static str> {
        let s = hex.trim();
        let s = s.strip_prefix("0x").unwrap_or(s);
        if s.is_empty() || s.len() > 64 {
            return Err("invalid hex length");
        }

        let mut decoded = Vec::with_capacity(s.len().div_ceil(2));
        let mut i = 0usize;
        if s.len() % 2 == 1 {
            let b = u8::from_str_radix(&format!("0{}", &s[0..1]), 16).map_err(|_| "invalid hex")?;
            decoded.push(b);
            i = 1;
        }
        while i < s.len() {
            let b = u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "invalid hex")?;
            decoded.push(b);
            i += 2;
        }
        if decoded.len() > 32 {
            return Err("invalid hex length");
        }

        let mut out = [0u8; 32];
        out[32 - decoded.len()..].copy_from_slice(&decoded);
        Ok(Address(out))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawEvent {
    pub block_number: BlockNumber,
    pub block_hash: BlockHash,
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

#[cfg(test)]
mod tests {
    use super::ChainId;

    #[test]
    fn chain_id_from_evm_u64_is_big_endian_padded() {
        let id = ChainId::from_evm_u64(1);
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(id.0, expected);
    }

    #[test]
    fn chain_id_from_hex_str_left_pads() {
        let id = ChainId::from_hex_str("0x1").unwrap();
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(id.0, expected);
    }

    #[test]
    fn chain_id_from_hex_str_parses_full_len() {
        let hex = "0x534e5f5345504f4c4941"; // "SN_SEPOLIA" in ASCII hex
        let id = ChainId::from_hex_str(hex).unwrap();
        let bytes = b"SN_SEPOLIA";
        let mut expected = [0u8; 32];
        expected[32 - bytes.len()..].copy_from_slice(bytes);
        assert_eq!(id.0, expected);
    }
}
