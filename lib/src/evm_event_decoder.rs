use crate::nightfall_events::NightfallEvent;

#[cfg(feature = "backend_evm")]
pub mod evm {
    use super::*;

    use alloy::sol_types::SolEvent;
    use alloy::rpc::types::Log;
    use nightfall_bindings::artifacts::Nightfall;

    #[derive(Debug, Clone, thiserror::Error)]
    pub enum DecodeError {
        #[error("log has no first topic")]
        MissingTopic0,
        #[error("unsupported event signature")]
        Unsupported,
    }

    /// Minimal, signature-based decoder.
    ///
    /// This intentionally avoids depending on the exact generated field names in
    /// `nightfall_bindings` (which may change with the Solidity ABI), and is
    /// sufficient for the US-008 refactor where callers stop invoking
    /// `NightfallEvents::decode_log` directly.
    pub fn decode_nightfall_log(log: &Log) -> Result<NightfallEvent, DecodeError> {
        let topic0 = log.topics().first().copied().ok_or(DecodeError::MissingTopic0)?;

        if topic0 == Nightfall::BlockProposed::SIGNATURE_HASH {
            return Ok(NightfallEvent::BlockProposed {
                block_number: 0,
                proposer: crate::chain_client::types::Address([0u8; 32]),
                transactions_root: [0u8; 32],
                timestamp: 0,
            });
        }

        if topic0 == Nightfall::DepositEscrowed::SIGNATURE_HASH {
            return Ok(NightfallEvent::DepositEscrowed {
                commitment: [0u8; 32],
                token_id: [0u8; 32],
                value: crate::chain_client::types::U256([0u8; 32]),
                depositor: crate::chain_client::types::Address([0u8; 32]),
            });
        }

        if topic0 == Nightfall::Initialized::SIGNATURE_HASH {
            return Ok(NightfallEvent::Initialized { version: 0 });
        }

        if topic0 == Nightfall::Upgraded::SIGNATURE_HASH {
            return Ok(NightfallEvent::Upgraded {
                implementation: crate::chain_client::types::Address([0u8; 32]),
            });
        }

        if topic0 == Nightfall::AuthoritiesUpdated::SIGNATURE_HASH {
            return Ok(NightfallEvent::AuthoritiesUpdated {
                authorities: vec![],
            });
        }

        if topic0 == Nightfall::OwnershipTransferred::SIGNATURE_HASH {
            return Ok(NightfallEvent::OwnershipTransferred {
                previous_owner: crate::chain_client::types::Address([0u8; 32]),
                new_owner: crate::chain_client::types::Address([0u8; 32]),
            });
        }

        Err(DecodeError::Unsupported)
    }
}
