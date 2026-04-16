#!/usr/bin/env sh
set -eu

RPC_URL="${STARKNET_RPC_URL:-${NF4_STARKNET_CLIENT_URL:-http://starknet-devnet:5050/rpc}}"
ADDRESS_FILE="${NF4_STARKNET_EVENTS_ADDRESS_FILE:-/starknet_assets/artifacts/dummy_emitter_address.txt}"

if [ ! -f "$ADDRESS_FILE" ]; then
  echo "Address file not found: $ADDRESS_FILE" >&2
  exit 1
fi

CONTRACT="$(cat "$ADDRESS_FILE" | tr -d '\n' | tr -d '\r')"
if [ -z "$CONTRACT" ]; then
  echo "Empty contract address in: $ADDRESS_FILE" >&2
  exit 1
fi

echo "RPC:      $RPC_URL"
echo "Contract: $CONTRACT"

/app/bin/nf4_starknet_emitter_tool --rpc-url "$RPC_URL" emit-block-proposed --contract "$CONTRACT" --block-number 1 --transactions-root 0x1234 --timestamp 1700000000
/app/bin/nf4_starknet_emitter_tool --rpc-url "$RPC_URL" emit-deposit-escrowed --contract "$CONTRACT" --commitment 0xbeef --token-id 0xcafe --value-low 0x1 --value-high 0x0

echo "Emitted events OK"
