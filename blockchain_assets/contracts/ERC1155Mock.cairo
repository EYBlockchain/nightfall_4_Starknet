#[starknet::interface]
pub trait IERC1155Mock<TContractState> {
    fn mint(ref self: TContractState, owner: starknet::ContractAddress, spender: starknet::ContractAddress, token_id: u256, value: u256);
}

#[starknet::contract]
pub mod ERC1155Mock {
    use starknet::ContractAddress;
    use openzeppelin::token::erc1155::{ERC1155Component, ERC1155HooksEmptyImpl};
    use openzeppelin::introspection::src5::SRC5Component;

    component!(path: SRC5Component, storage: src5, event: SRC5Event);
    component!(path: ERC1155Component, storage: erc1155, event: ERC1155Event);

    #[abi(embed_v0)]
    impl SRC5Impl = SRC5Component::SRC5Impl<ContractState>;
    #[abi(embed_v0)]
    impl ERC1155Impl = ERC1155Component::ERC1155Impl<ContractState>;
    #[abi(embed_v0)]
    impl ERC1155MetadataURIImpl = ERC1155Component::ERC1155MetadataURIImpl<ContractState>;

    impl SRC5InternalImpl = SRC5Component::InternalImpl<ContractState>;
    impl ERC1155InternalImpl = ERC1155Component::InternalImpl<ContractState>;

    #[storage]
    struct Storage {
        #[substorage(v0)]
        src5: SRC5Component::Storage,
        #[substorage(v0)]
        erc1155: ERC1155Component::Storage
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        SRC5Event: SRC5Component::Event,
        #[flat]
        ERC1155Event: ERC1155Component::Event
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        spender: ContractAddress,
        initial_id: u256,
        value: u256,
        initial_id_two: u256,
        value_two: u256,
        initial_owner: ContractAddress
    ) {
        self.erc1155.initializer("ERC1155Mock");
        self.mint(initial_owner, spender, initial_id, value);
        self.mint(initial_owner, spender, initial_id_two, value_two);
    }

    #[abi(embed_v0)]
    impl ERC1155MockImpl of super::IERC1155Mock<ContractState> {
        fn mint(ref self: ContractState, owner: ContractAddress, spender: ContractAddress, token_id: u256, value: u256) {
            self.erc1155._set_approval_for_all(owner, spender, true);
            let data = array![];
            self.erc1155.mint(owner, token_id, value, data.span());
        }
    }
}
