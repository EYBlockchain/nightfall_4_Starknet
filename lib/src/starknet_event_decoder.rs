use crate::chain_client::types::{Address, RawEvent, U256};
use crate::nightfall_events::NightfallEvent;

#[cfg(feature = "backend_starknet")]
pub mod starknet {
    use super::*;
    use crate::chain_client::types::ContractId;
    use sha3::{Digest, Keccak256};
    use std::collections::HashMap;
    use std::sync::Arc;

    type DecoderFn = Arc<dyn Fn(&RawEvent) -> Result<NightfallEvent, DecodeError> + Send + Sync + 'static>;

    #[derive(Debug, Clone, thiserror::Error)]
    pub enum DecodeError {
        #[error("event has no first key")]
        MissingKey0,
        #[error("unsupported event selector")]
        Unsupported,
        #[error("invalid data length (expected multiple of 32 bytes)")]
        InvalidDataLength,
        #[error("missing data field #{0}")]
        MissingField(usize),
        #[error("value does not fit u64")]
        U64Overflow,
    }

    #[derive(Default, Clone)]
    pub struct StarknetEventDecoderRegistry {
        decoders: HashMap<(Option<ContractId>, [u8; 32]), DecoderFn>,
    }

    impl StarknetEventDecoderRegistry {
        pub fn new() -> Self {
            Self {
                decoders: HashMap::new(),
            }
        }

        pub fn register_global<F>(&mut self, selector: [u8; 32], decoder_fn: F)
        where
            F: Fn(&RawEvent) -> Result<NightfallEvent, DecodeError> + Send + Sync + 'static,
        {
            self.decoders.insert((None, selector), Arc::new(decoder_fn));
        }

        pub fn register_for_contract<F>(
            &mut self,
            contract: ContractId,
            selector: [u8; 32],
            decoder_fn: F,
        ) where
            F: Fn(&RawEvent) -> Result<NightfallEvent, DecodeError> + Send + Sync + 'static,
        {
            self.decoders
                .insert((Some(contract), selector), Arc::new(decoder_fn));
        }

        pub fn decode(&self, raw: &RawEvent) -> Result<NightfallEvent, DecodeError> {
            let selector = raw.keys.first().copied().ok_or(DecodeError::MissingKey0)?;

            if let Some(decoder) = self.decoders.get(&(Some(raw.contract), selector)) {
                return decoder(raw);
            }

            if let Some(decoder) = self.decoders.get(&(None, selector)) {
                return decoder(raw);
            }

            Err(DecodeError::Unsupported)
        }
    }

    pub fn default_registry() -> StarknetEventDecoderRegistry {
        let mut registry = StarknetEventDecoderRegistry::new();
        registry.register_global(event_selector("BlockProposed"), decode_block_proposed);
        registry.register_global(
            event_selector("DepositEscrowed"),
            decode_deposit_escrowed,
        );
        registry
    }

    pub fn dummy_emitter_selectors() -> Vec<[u8; 32]> {
        vec![
            event_selector("BlockProposed"),
            event_selector("DepositEscrowed"),
        ]
    }

    pub fn decode_dummy_emitter_event(raw: &RawEvent) -> Result<NightfallEvent, DecodeError> {
        default_registry().decode(raw)
    }

    fn decode_block_proposed(raw: &RawEvent) -> Result<NightfallEvent, DecodeError> {
        validate_data_len(raw)?;
        let block_number = felt_u64(&felt(raw, 0)?)?;
        let proposer = Address(felt(raw, 1)?);
        let transactions_root = felt(raw, 2)?;
        let timestamp = felt_u64(&felt(raw, 3)?)?;
        Ok(NightfallEvent::BlockProposed {
            tx_hash: raw.tx_hash,
            block_number,
            proposer,
            transactions_root,
            timestamp,
        })
    }

    fn decode_deposit_escrowed(raw: &RawEvent) -> Result<NightfallEvent, DecodeError> {
        validate_data_len(raw)?;
        let commitment = felt(raw, 0)?;
        let token_id = felt(raw, 1)?;
        let value_low = felt(raw, 2)?;
        let value_high = felt(raw, 3)?;
        let depositor = Address(felt(raw, 4)?);
        let value = u256_from_low_high(value_low, value_high);

        Ok(NightfallEvent::DepositEscrowed {
            tx_hash: raw.tx_hash,
            commitment,
            token_id,
            value,
            depositor,
        })
    }

    fn validate_data_len(raw: &RawEvent) -> Result<(), DecodeError> {
        if raw.data.len() % 32 != 0 {
            return Err(DecodeError::InvalidDataLength);
        }
        Ok(())
    }

    fn felt(raw: &RawEvent, index: usize) -> Result<[u8; 32], DecodeError> {
        let start = index
            .checked_mul(32)
            .ok_or(DecodeError::MissingField(index))?;
        let end = start + 32;
        if end > raw.data.len() {
            return Err(DecodeError::MissingField(index));
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&raw.data[start..end]);
        Ok(out)
    }

    fn felt_u64(word: &[u8; 32]) -> Result<u64, DecodeError> {
        if word[..24].iter().any(|b| *b != 0) {
            return Err(DecodeError::U64Overflow);
        }
        let mut tail = [0u8; 8];
        tail.copy_from_slice(&word[24..]);
        Ok(u64::from_be_bytes(tail))
    }

    fn u256_from_low_high(low: [u8; 32], high: [u8; 32]) -> U256 {
        // Both low/high are 252-bit values encoded as 32 bytes.
        // Combine into 256-bit as: value = low + (high << 128)
        // We model U256 as 32 big-endian bytes.
        let mut out = [0u8; 32];

        // Take low 128 bits from low (last 16 bytes)
        out[16..].copy_from_slice(&low[16..]);

        // Add high 128 bits from high (last 16 bytes)
        out[..16].copy_from_slice(&high[16..]);

        U256(out)
    }

    fn event_selector(name: &str) -> [u8; 32] {
        // Starknet selector: keccak256(name) with top 6 bits cleared (250-bit)
        let mut hasher = Keccak256::new();
        hasher.update(name.as_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out[0] &= 0x03;
        out
    }
}
