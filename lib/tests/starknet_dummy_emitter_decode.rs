#![cfg(feature = "backend_starknet")]

use lib::chain_client::types::{BlockHash, BlockNumber, ContractId, RawEvent, TxHash};
use lib::chain_client::types::{Address, U256};

fn selector(name: &str) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(name.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out[0] &= 0x03;
    out
}

fn felt_u64(v: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&v.to_be_bytes());
    out
}

fn push_felt(data: &mut Vec<u8>, felt: [u8; 32]) {
    data.extend_from_slice(&felt);
}

#[test]
fn decode_block_proposed() {
    let mut data = Vec::new();
    push_felt(&mut data, felt_u64(7));

    let proposer = Address::from_hex_str("0x1234").unwrap();
    push_felt(&mut data, proposer.0);

    let mut root = [0u8; 32];
    root[31] = 0xAA;
    push_felt(&mut data, root);

    push_felt(&mut data, felt_u64(1000));

    let raw = RawEvent {
        block_number: BlockNumber(7),
        block_hash: BlockHash([1u8; 32]),
        tx_hash: TxHash([0u8; 32]),
        contract: ContractId(Address([0u8; 32])),
        keys: vec![selector("BlockProposed")],
        data,
    };

    let decoded = lib::starknet_event_decoder::starknet::default_registry().decode(&raw).unwrap();

    assert_eq!(
        decoded,
        lib::nightfall_events::NightfallEvent::BlockProposed {
            tx_hash: TxHash([0u8; 32]),
            block_number: 7,
            proposer,
            transactions_root: root,
            timestamp: 1000,
        }
    );
}

#[test]
fn decode_deposit_escrowed() {
    let mut data = Vec::new();

    let mut commitment = [0u8; 32];
    commitment[31] = 0x01;
    push_felt(&mut data, commitment);

    let mut token_id = [0u8; 32];
    token_id[31] = 0x02;
    push_felt(&mut data, token_id);

    let mut low = [0u8; 32];
    low[31] = 0x10;
    push_felt(&mut data, low);

    let mut high = [0u8; 32];
    high[31] = 0x20;
    push_felt(&mut data, high);

    let depositor = Address::from_hex_str("0x7777").unwrap();
    push_felt(&mut data, depositor.0);

    let mut value_bytes = [0u8; 32];
    value_bytes[16..].copy_from_slice(&low[16..]);
    value_bytes[..16].copy_from_slice(&high[16..]);

    let raw = RawEvent {
        block_number: BlockNumber(1),
        block_hash: BlockHash([2u8; 32]),
        tx_hash: TxHash([0u8; 32]),
        contract: ContractId(Address([0u8; 32])),
        keys: vec![selector("DepositEscrowed")],
        data,
    };

    let decoded = lib::starknet_event_decoder::starknet::default_registry().decode(&raw).unwrap();

    assert_eq!(
        decoded,
        lib::nightfall_events::NightfallEvent::DepositEscrowed {
            tx_hash: TxHash([0u8; 32]),
            commitment,
            token_id,
            value: U256(value_bytes),
            depositor,
        }
    );
}
