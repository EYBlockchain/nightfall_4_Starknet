#!/usr/bin/env sh
set -eu

ADDRESS_FILE="${NF4_DUMMY_EMITTER_OUT_FILE:-./starknet_assets/artifacts/dummy_emitter_address.txt}"

if [ ! -f "$ADDRESS_FILE" ]; then
  echo "Address file not found: $ADDRESS_FILE" >&2
  exit 1
fi

ADDRESS="$(cat "$ADDRESS_FILE" | tr -d '\n' | tr -d '\r')"
if [ -z "$ADDRESS" ]; then
  echo "Empty address in: $ADDRESS_FILE" >&2
  exit 1
fi

echo "export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS=$ADDRESS"
