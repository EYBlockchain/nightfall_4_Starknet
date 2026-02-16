# Starknet Assets

This folder contains Starknet-only tooling to stand up a small contract that emits events.

## Layout

- `contracts/`: Cairo contracts (dummy emitter)
- `artifacts/`: build outputs (ignored or generated)
- `scripts/`: deploy + invoke helpers for a running devnet

## Requirements

- A running Starknet JSON-RPC devnet (Katana)
- `starkli` installed and available on PATH (optional; currently recommended)

## Quickstart (Katana in docker-compose)

1. Start the stack:

```sh
docker compose --profile starknet_devnet up -d --build
```

2. Deploy the dummy emitter + emit a sample event:

```sh
./starknet_assets/scripts/deploy_and_emit.sh
```

If `starkli` is not working with your Katana version, you can still validate
RPC connectivity via the Rust helper:

```sh
cargo run --manifest-path starknet_assets/rust/Cargo.toml -- --rpc-url http://localhost:5050
```

If you need to override the RPC URL:

```sh
NF4_STARKNET_CLIENT_URL=http://localhost:5050 ./starknet_assets/scripts/deploy_and_emit.sh
```
