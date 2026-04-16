use crate::chain_client::types::{Address, TxHash, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NightfallEvent {
    BlockProposed {
        tx_hash: TxHash,
        block_number: u64,
        proposer: Address,
        transactions_root: [u8; 32],
        timestamp: u64,
    },
    DepositEscrowed {
        tx_hash: TxHash,
        commitment: [u8; 32],
        token_id: [u8; 32],
        value: U256,
        depositor: Address,
    },
    Initialized {
        version: u64,
    },
    Upgraded {
        implementation: Address,
    },
    AuthoritiesUpdated {
        authorities: Vec<Address>,
    },
    OwnershipTransferred {
        previous_owner: Address,
        new_owner: Address,
    },
}
