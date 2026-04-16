# `nf4_starknet_emitter_tool`

Minimal Starknet emitter CLI for local devnet workflows.

## Commands

- `ping` calls `starknet_chainId` on the configured RPC endpoint.
- `deploy` is scaffolded for future Sierra + CASM deployment flow.
- `emit-block-proposed` is scaffolded for future event emission.
- `emit-deposit-escrowed` is scaffolded for future event emission.

## Quick start

```bash
cargo run -- --help
cargo run -- --rpc-url http://localhost:5050/rpc ping
```

## Global flags

- `--rpc-url` defaults to `http://localhost:5050/rpc`
- `--private-key` is accepted now for future deploy and invoke flows
