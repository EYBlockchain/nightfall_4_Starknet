#[starknet::interface]
pub trait IERC721Mock<TContractState> {
    fn mint(ref self: TContractState, owner: starknet::ContractAddress, spender: starknet::ContractAddress, token_id: u256);
}

#[starknet::contract]
pub mod ERC721Mock {
    use starknet::ContractAddress;
    use openzeppelin::token::erc721::{ERC721Component, ERC721HooksEmptyImpl};
    use openzeppelin::introspection::src5::SRC5Component;

    component!(path: SRC5Component, storage: src5, event: SRC5Event);
    component!(path: ERC721Component, storage: erc721, event: ERC721Event);

    #[abi(embed_v0)]
    impl SRC5Impl = SRC5Component::SRC5Impl<ContractState>;
    #[abi(embed_v0)]
    impl ERC721Impl = ERC721Component::ERC721Impl<ContractState>;
    #[abi(embed_v0)]
    impl ERC721MetadataImpl = ERC721Component::ERC721MetadataImpl<ContractState>;

    impl SRC5InternalImpl = SRC5Component::InternalImpl<ContractState>;
    impl ERC721InternalImpl = ERC721Component::InternalImpl<ContractState>;

    #[storage]
    struct Storage {
        #[substorage(v0)]
        src5: SRC5Component::Storage,
        #[substorage(v0)]
        erc721: ERC721Component::Storage
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    enum Event {
        #[flat]
        SRC5Event: SRC5Component::Event,
        #[flat]
        ERC721Event: ERC721Component::Event
    }

    #[constructor]
    fn constructor(
        ref self: ContractState,
        initial_id: u256,
        initial_owner: ContractAddress,
        spender: ContractAddress
    ) {
        self.erc721.initializer("ERC721Mock", "E721", "");
        ERC721MockImpl::mint(ref self, initial_owner, spender, initial_id);
    }

    #[abi(embed_v0)]
    impl ERC721MockImpl of super::IERC721Mock<ContractState> {
        fn mint(ref self: ContractState, owner: ContractAddress, spender: ContractAddress, token_id: u256) {
            self.erc721._set_approval_for_all(owner, spender, true);
            self.erc721._mint(owner, token_id);
        }
    }
}
