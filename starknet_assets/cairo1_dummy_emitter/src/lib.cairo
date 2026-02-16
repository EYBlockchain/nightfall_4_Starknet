// Interface must live outside the contract module so the plugin can generate
// the dispatcher and the embedded impl can reference it via `super::`.
#[starknet::interface]
trait IDummyEmitter<TContractState> {
    fn emit_block_proposed(
        ref self: TContractState,
        block_number: felt252,
        transactions_root: felt252,
        timestamp: felt252,
    );

    fn emit_deposit_escrowed(
        ref self: TContractState,
        commitment: felt252,
        token_id: felt252,
        value_low: felt252,
        value_high: felt252,
    );
}

#[starknet::contract]
mod DummyEmitter {
    use starknet::get_caller_address;

    #[storage]
    struct Storage {}

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        BlockProposed: BlockProposed,
        DepositEscrowed: DepositEscrowed,
    }

    #[derive(Drop, starknet::Event)]
    struct BlockProposed {
        block_number: felt252,
        proposer: starknet::ContractAddress,
        transactions_root: felt252,
        timestamp: felt252,
    }

    #[derive(Drop, starknet::Event)]
    struct DepositEscrowed {
        commitment: felt252,
        token_id: felt252,
        value_low: felt252,
        value_high: felt252,
        depositor: starknet::ContractAddress,
    }

    #[abi(embed_v0)]
    impl DummyEmitterImpl of super::IDummyEmitter<ContractState> {
        fn emit_block_proposed(
            ref self: ContractState,
            block_number: felt252,
            transactions_root: felt252,
            timestamp: felt252,
        ) {
            let proposer = get_caller_address();
            self.emit(Event::BlockProposed(BlockProposed {
                block_number, proposer, transactions_root, timestamp,
            }));
        }

        fn emit_deposit_escrowed(
            ref self: ContractState,
            commitment: felt252,
            token_id: felt252,
            value_low: felt252,
            value_high: felt252,
        ) {
            let depositor = get_caller_address();
            self.emit(Event::DepositEscrowed(DepositEscrowed {
                commitment, token_id, value_low, value_high, depositor,
            }));
        }
    }
}
