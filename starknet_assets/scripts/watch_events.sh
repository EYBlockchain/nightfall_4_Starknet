#!/usr/bin/env zsh
set -euo pipefail

RPC_URL="${NF4_STARKNET_CLIENT_URL:-http://localhost:5050}"
ADDRESS="${NF4_STARKNET_EVENTS_CONTRACT_ADDRESS:-}"

if [[ -z "${ADDRESS}" ]]; then
  echo "NF4_STARKNET_EVENTS_CONTRACT_ADDRESS is not set." >&2
  echo "Tip: export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS=\"$(cat ./starknet_assets/artifacts/dummy_emitter_address.txt 2>/dev/null || true)\"" >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl not found" >&2
  exit 1
fi

echo "RPC:     ${RPC_URL}"
echo "Address: ${ADDRESS}"

block_number() {
  curl -sS -H 'Content-Type: application/json' -X POST \
    --data '{"jsonrpc":"2.0","id":1,"method":"starknet_blockNumber","params":[]}' \
    "${RPC_URL}" | sed -n 's/.*"result"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p'
}

get_events() {
  local from_block="$1"
  local to_block="$2"
  curl -sS -H 'Content-Type: application/json' -X POST \
    --data "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"starknet_getEvents\",\"params\":[{\"filter\":{\"from_block\":{\"block_number\":${from_block}},\"to_block\":{\"block_number\":${to_block}},\"address\":\"${ADDRESS}\",\"keys\":[]},\"chunk_size\":100}]}" \
    "${RPC_URL}"
}

last=$(block_number)
if [[ -z "${last}" ]]; then
  echo "Failed to query starknet_blockNumber" >&2
  exit 1
fi

echo "Starting at block ${last}"

while true; do
  cur=$(block_number)
  if [[ -z "${cur}" ]]; then
    echo "blockNumber failed; retrying" >&2
    sleep 1
    continue
  fi

  if (( cur >= last )); then
    res=$(get_events "$last" "$cur")
    echo "${res}" | grep -q '"error"' && {
      echo "RPC error: ${res}" >&2
    }
    # Print only the interesting bits to keep logs readable.
    echo "${res}" | tr -d '\n' | sed 's/\\u0000//g' | sed 's/},{/},\n{/g'
    last=$((cur + 1))
  fi

  sleep 2
done
