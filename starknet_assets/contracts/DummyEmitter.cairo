// DummyEmitter.cairo
// Minimal event emitter contract for NF4 Starknet ingestion bring-up.

%lang starknet

from starkware.cairo.common.uint256 import Uint256
from starkware.starknet.common.syscalls import get_caller_address

@event
func BlockProposed(block_number: felt, proposer: felt, transactions_root: felt, timestamp: felt):
end

@event
func DepositEscrowed(commitment: felt, token_id: felt, value: Uint256, depositor: felt):
end

@external
func emit_block_proposed(block_number: felt, transactions_root: felt, timestamp: felt):
    let (caller) = get_caller_address()
    BlockProposed.emit(block_number, caller, transactions_root, timestamp)
    return ()
end

@external
func emit_deposit_escrowed(commitment: felt, token_id: felt, value: Uint256):
    let (caller) = get_caller_address()
    DepositEscrowed.emit(commitment, token_id, value, caller)
    return ()
end
