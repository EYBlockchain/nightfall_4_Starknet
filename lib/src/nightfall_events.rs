use serde::{Deserialize, Serialize};

use crate::chain_client::types::{Address, U256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NightfallEvent {
    BlockProposed {
        block_number: u64,
        proposer: Address,
        transactions_root: [u8; 32],
        timestamp: u64,
    },
    DepositEscrowed {
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
