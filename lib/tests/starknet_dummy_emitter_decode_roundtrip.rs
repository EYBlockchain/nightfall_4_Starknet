#![cfg(feature = "backend_starknet")]

use lib::chain_client::types::RawEvent;
use lib::starknet_event_decoder::starknet::decode_dummy_emitter_event;

fn u64_felt(v: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    out[24..].copy_from_slice(&v.to_be_bytes());
    out
}

fn selector(name: &str) -> [u8; 32] {
    use sha3::{Digest, Keccak256};
    let mut hasher = Keccak256::new();
    hasher.update(name.as_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out[0] &= 0x03;
    out
}

#[test]
fn decode_block_proposed_from_raw_event() {
    let mut data = Vec::new();
    data.extend_from_slice(&u64_felt(1)); // block_number
    data.extend_from_slice(&[0x01u8; 32]); // proposer
    data.extend_from_slice(&[0x02u8; 32]); // transactions_root
    data.extend_from_slice(&u64_felt(1_700_000_000)); // timestamp

    let raw = RawEvent {
        block_number: lib::chain_client::types::BlockNumber(0),
        tx_hash: lib::chain_client::types::TxHash([0u8; 32]),
        contract: lib::chain_client::types::ContractId(lib::chain_client::types::Address([0u8; 32])),
        keys: vec![selector("BlockProposed")],
        data,
    };

    let decoded = decode_dummy_emitter_event(&raw).expect("decode");
    match decoded {
        lib::nightfall_events::NightfallEvent::BlockProposed { block_number, timestamp, .. } => {
            assert_eq!(block_number, 1);
            assert_eq!(timestamp, 1_700_000_000);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}

#[test]
fn decode_deposit_escrowed_from_raw_event() {
    let mut data = Vec::new();
    data.extend_from_slice(&[0x11u8; 32]); // commitment
    data.extend_from_slice(&[0x22u8; 32]); // token_id
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&[0x33u8; 16]); // value_low (lower 128 in last 16 bytes)
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&[0x44u8; 16]); // value_high (upper 128 in last 16 bytes)
    data.extend_from_slice(&[0x55u8; 32]); // depositor

    let raw = RawEvent {
        block_number: lib::chain_client::types::BlockNumber(0),
        tx_hash: lib::chain_client::types::TxHash([0u8; 32]),
        contract: lib::chain_client::types::ContractId(lib::chain_client::types::Address([0u8; 32])),
        keys: vec![selector("DepositEscrowed")],
        data,
    };

    let decoded = decode_dummy_emitter_event(&raw).expect("decode");
    match decoded {
        lib::nightfall_events::NightfallEvent::DepositEscrowed { value, .. } => {
            // value should be [0x44.. (16 bytes)] || [0x33.. (16 bytes)]
            assert_eq!(value.0[..16], [0x44u8; 16]);
            assert_eq!(value.0[16..], [0x33u8; 16]);
        }
        other => panic!("unexpected event: {other:?}"),
    }
}
