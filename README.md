# nightfall_4_CE
Community edition of Nightfall_4

_This code is not owned by EY and EY provides no warranty and disclaims any and all liability for use of this code. Users must conduct their own diligence with respect to use for their purposes and any and all usage is on an as-is basis and at your own risk._

Nightfall_4 is a ZK rollup build around the ZK Privacy of Nightfall. It enables one to transfer ERC20, ERC721, ERC1155 and ERC3525 tokens in privacy. Full details can be found in the /doc folder of this repository.

Please note that this software should be treated as experimental. It should not be used to make significant value transactions.

## Starknet devnet (Katana) + dummy event emission

This repo includes a Dockerized Starknet devnet flow used to validate the Starknet event path end-to-end:

- `starknet-devnet`: Katana JSON-RPC at `http://localhost:5050`
- `starknet-emitter`: deploys the Cairo 1 `DummyEmitter` and emits sample events
- `client` / `proposer`: poll Starknet events and decode them into `NightfallEvent`

### Prerequisites

- Build the Cairo 1 dummy emitter artifacts locally (the emitter container mounts `starknet_assets/` and expects the compiled artifacts to exist):

```sh
cd starknet_assets/cairo1_dummy_emitter
scarb build
```

### Run Katana + auto-emitter

```sh
cd /Users/Adarsh.Ron/nightfall_4_Starknet
docker compose --profile starknet_devnet up --build starknet-devnet starknet-emitter
```

This writes the deployed address to `starknet_assets/artifacts/dummy_emitter_address.txt`
and the deployed class hash to `starknet_assets/artifacts/dummy_emitter_class_hash.txt`.

### Run client/proposer with contract filtering

```sh
export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS="$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt)"
docker compose --profile starknet_devnet up --build client proposer
```

### Starknet account management via env (EVM-style)

Use the same local private-key flow as EVM for local development by setting keys in your env file (`.env` / local env file used by compose):

```sh
export NF4_RUN_MODE=starknet_devnet
export CLIENT_SIGNING_KEY=0x...
export PROPOSER_SIGNING_KEY=0x...
export CLIENT_STARKNET_ACCOUNT_ADDRESS=0x...
export PROPOSER_STARKNET_ACCOUNT_ADDRESS=0x...
export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS="$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt)"
```

`client` and `proposer` containers map these into `NF4_SIGNING_KEY` and
`NF4_STARKNET_ACCOUNT_ADDRESS` at startup, so no code changes are needed between
EVM and Starknet dev key management patterns.

Current chain-neutral Starknet transaction conventions in `lib::chain_client`:
- `call_view(contract, calldata)` expects `calldata` encoded as 32-byte words:
	first word is the Starknet entrypoint selector, remaining words are calldata felts.
- `send_transaction(tx)` expects `tx.bytes` to contain UTF-8 JSON for the
	`invoke_transaction` object accepted by `starknet_addInvokeTransaction`.

These conventions are an interim compatibility layer until a dedicated
Starknet-native transaction type is introduced in the chain-neutral API.

### Optional: raw JSON-RPC event watcher

```sh
export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS="$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt)"
./starknet_assets/scripts/watch_events.sh
```

