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

This writes the deployed address to `starknet_assets/artifacts/dummy_emitter_address.txt`.

### Run client/proposer with contract filtering

```sh
export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS="$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt)"
docker compose --profile starknet_devnet up --build client proposer
```

### Optional: raw JSON-RPC event watcher

```sh
export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS="$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt)"
./starknet_assets/scripts/watch_events.sh
```

