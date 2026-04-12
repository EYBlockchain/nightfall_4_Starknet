#[starknet::interface]
pub trait IERC20Mock<TContractState> {
    fn mint(ref self: TContractState, mint_to_: starknet::ContractAddress, spender: starknet::ContractAddress, value_: u256);
}

#[starknet::contract]
pub mod ERC20Mock {
    use starknet::ContractAddress;
    use openzeppelin::token::erc20::{ERC20Component, ERC20HooksEmptyImpl};

    component!(path: ERC20Component, storage: erc20, event: ERC20Event);

    #[abi(embed_v0)]
    impl ERC20Impl = ERC20Component::ERC20Impl<ContractState>;
    
    impl ERC20InternalImpl = ERC20Component::InternalImpl<ContractState>;

    #[storage]
    struct Storage {
        #[substorage(v0)]
        erc20: ERC20Component::Storage
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        ERC20Event: ERC20Component::Event
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        initial_supply: u256,
        spender: ContractAddress,
        initial_owner: ContractAddress,
        other_client: ContractAddress
    ) {
        self.erc20.initializer("ERC20Mock", "E20");
        let initial_supply_half = initial_supply / 2;
        
        ERC20MockImpl::mint(ref self, initial_owner, spender, initial_supply_half);
        ERC20MockImpl::mint(ref self, other_client, spender, initial_supply_half);
    }
    
    #[abi(embed_v0)]
    impl ERC20MetadataImpl of openzeppelin::token::erc20::interface::IERC20Metadata<ContractState> {
        fn name(self: @ContractState) -> ByteArray {
            "ERC20Mock"
        }
        fn symbol(self: @ContractState) -> ByteArray {
            "E20"
        }
        fn decimals(self: @ContractState) -> u8 {
            9
        }
    }

    #[abi(embed_v0)]
    impl ERC20MockImpl of super::IERC20Mock<ContractState> {
        fn mint(ref self: ContractState, mint_to_: ContractAddress, spender: ContractAddress, value_: u256) {
            self.erc20._approve(mint_to_, spender, value_);
            self.erc20.mint(mint_to_, value_);
        }
    }
}
