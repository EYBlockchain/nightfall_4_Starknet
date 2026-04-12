#[starknet::interface]
pub trait IERC3525Mock<TContractState> {
    fn mint(ref self: TContractState, spender: starknet::ContractAddress, mint_to: starknet::ContractAddress, token_id: u256, slot: u256, value: u256);
    fn mint_value(ref self: TContractState, token_id: u256, value: u256);
    fn burn(ref self: TContractState, token_id: u256);
    fn burn_value(ref self: TContractState, token_id: u256, burn_value: u256);
    
    fn owner_of(self: @TContractState, token_id: u256) -> starknet::ContractAddress;
    fn get_value(self: @TContractState, token_id: u256) -> u256;
    fn get_slot(self: @TContractState, token_id: u256) -> u256;
}

#[starknet::contract]
pub mod ERC3525Mock {
    use starknet::{ContractAddress, get_caller_address};

    #[storage]
    struct Storage {
        owners: LegacyMap<u256, ContractAddress>,
        values: LegacyMap<u256, u256>,
        slots: LegacyMap<u256, u256>,
        operator_approvals: LegacyMap<(ContractAddress, ContractAddress), bool>,
        token_approvals: LegacyMap<u256, ContractAddress>,
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        spender: ContractAddress,
        initial_id: u256,
        value: u256,
        initial_id_two: u256,
        value_two: u256,
        slot: u256,
        initial_owner: ContractAddress
    ) {
        self.mint(spender, initial_owner, initial_id, slot, value);
        self.mint(spender, initial_owner, initial_id_two, slot, value_two);
    }
    
    #[generate_trait]
    impl InternalImpl of InternalTrait {
        fn _is_approved_or_owner(self: @ContractState, spender: ContractAddress, token_id: u256) -> bool {
            let owner = self.owners.read(token_id);
            if owner == spender {
                return true;
            }
            if self.token_approvals.read(token_id) == spender {
                return true;
            }
            self.operator_approvals.read((owner, spender))
        }
    }

    #[abi(embed_v0)]
    impl ERC3525MockImpl of super::IERC3525Mock<ContractState> {
        fn mint(ref self: ContractState, spender: ContractAddress, mint_to: ContractAddress, token_id: u256, slot: u256, value: u256) {
            self.operator_approvals.write((mint_to, spender), true);
            self.owners.write(token_id, mint_to);
            self.slots.write(token_id, slot);
            self.values.write(token_id, value);
        }
        
        fn mint_value(ref self: ContractState, token_id: u256, value: u256) {
            let current_value = self.values.read(token_id);
            self.values.write(token_id, current_value + value);
        }
        
        fn burn(ref self: ContractState, token_id: u256) {
            assert(self._is_approved_or_owner(get_caller_address(), token_id), 'Not approved or owner');
            
            self.token_approvals.write(token_id, starknet::contract_address_const::<0>());
            self.owners.write(token_id, starknet::contract_address_const::<0>());
            self.values.write(token_id, 0);
            self.slots.write(token_id, 0);
        }
        
        fn burn_value(ref self: ContractState, token_id: u256, burn_value: u256) {
            assert(self._is_approved_or_owner(get_caller_address(), token_id), 'Not approved or owner');
            let current_value = self.values.read(token_id);
            assert(current_value >= burn_value, 'Insufficient value');
            self.values.write(token_id, current_value - burn_value);
        }
        
        fn owner_of(self: @ContractState, token_id: u256) -> ContractAddress {
            self.owners.read(token_id)
        }
        
        fn get_value(self: @ContractState, token_id: u256) -> u256 {
            self.values.read(token_id)
        }
        
        fn get_slot(self: @ContractState, token_id: u256) -> u256 {
            self.slots.read(token_id)
        }
    }
}
