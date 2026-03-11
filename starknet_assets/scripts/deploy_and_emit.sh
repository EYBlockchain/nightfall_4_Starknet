#!/usr/bin/env zsh
set -euo pipefail

RPC_URL="${NF4_STARKNET_CLIENT_URL:-http://localhost:5050}"
CONTRACT_PATH="${NF4_DUMMY_EMITTER_CONTRACT_PATH:-$(cd "$(dirname "$0")/.." && pwd)/cairo1_dummy_emitter/target/dev/cairo1_dummy_emitter_DummyEmitter.contract_class.json}"
OUT_FILE="${NF4_DUMMY_EMITTER_OUT_FILE:-$(cd "$(dirname "$0")/.." && pwd)/artifacts/dummy_emitter_address.txt}"
CLASS_HASH_OUT_FILE="${NF4_DUMMY_EMITTER_CLASS_HASH_OUT_FILE:-$(cd "$(dirname "$0")/.." && pwd)/artifacts/dummy_emitter_class_hash.txt}"

# Katana prints prefunded accounts (address + private key) at startup.
# If you don't set these, starkli will prompt/fail.
ACCOUNT="${NF4_STARKNET_ACCOUNT:-${STARKNET_ACCOUNT:-}}"
PRIVATE_KEY="${NF4_STARKNET_PRIVATE_KEY:-${STARKNET_PRIVATE_KEY:-}}"

if [[ -z "${ACCOUNT}" || -z "${PRIVATE_KEY}" ]]; then
  echo "Missing Starknet account credentials for starkli." >&2
  echo "Set one of the following (recommended for local Katana):" >&2
  echo "  export NF4_STARKNET_ACCOUNT=0x..." >&2
  echo "  export NF4_STARKNET_PRIVATE_KEY=0x..." >&2
  echo "You can copy these from the 'PREFUNDED ACCOUNTS' section in the Katana logs." >&2
  exit 2
fi

if ! command -v starkli >/dev/null 2>&1; then
  echo "starkli not found. Install it first." >&2
  echo "See: https://github.com/xJonathanLEI/starkli" >&2
  exit 1
fi

echo "RPC: ${RPC_URL}"
echo "Contract artifact: ${CONTRACT_PATH}"
echo "Account: ${ACCOUNT}"

# Notes:
# - We intentionally keep this script basic; local account / key management varies.
# - Users should set up starkli with an account (e.g., starkli account fetch / env vars).

echo "\n==> Declaring contract (if needed)"
# Declare returns a class hash and may fail if already declared.
set +e
if [[ ! -f "$CONTRACT_PATH" ]]; then
  echo "Contract artifact not found. Build it first:" >&2
  echo "  (cd $(cd "$(dirname "$0")/.." && pwd)/cairo1_dummy_emitter && scarb build)" >&2
  exit 2
fi

DECLARE_OUT=$(starkli declare --rpc "$RPC_URL" --account "$ACCOUNT" --private-key "$PRIVATE_KEY" "$CONTRACT_PATH" 2>&1)
DECLARE_RC=$?
set -e
if [[ $DECLARE_RC -ne 0 ]]; then
  echo "$DECLARE_OUT" | grep -qi "already declared" && echo "Already declared; continuing" || {
    echo "$DECLARE_OUT" >&2
    exit $DECLARE_RC
  }
else
  echo "$DECLARE_OUT"
fi

echo "\n==> Deploying"
# `starkli deploy` expects a class hash. We'll extract it from the declare output.
CLASS_HASH=$(echo "$DECLARE_OUT" | grep -Eo '0x[0-9a-fA-F]+' | head -n 1)
if [[ -z "${CLASS_HASH}" ]]; then
  echo "Could not parse class hash from declare output:" >&2
  echo "$DECLARE_OUT" >&2
  exit 1
fi

DEPLOY_OUT=$(starkli deploy --rpc "$RPC_URL" --account "$ACCOUNT" --private-key "$PRIVATE_KEY" "$CLASS_HASH")
echo "$DEPLOY_OUT"

# Try to extract address from output (starkli formats can vary).
ADDRESS=$(echo "$DEPLOY_OUT" | grep -Eo '0x[0-9a-fA-F]+' | tail -n 1)
if [[ -z "${ADDRESS}" ]]; then
  echo "Could not parse deployed contract address." >&2
  exit 1
fi

echo "Deployed at: $ADDRESS"

mkdir -p "$(dirname "$OUT_FILE")"
echo -n "$ADDRESS" > "$OUT_FILE"
echo "Wrote address to: $OUT_FILE"

echo "\nExport this to enable NF4 filtering:"
echo "export NF4_STARKNET_EVENTS_CONTRACT_ADDRESS=$ADDRESS"

echo "\n==> Fetching deployed class hash"
ON_CHAIN_CLASS_HASH=$(curl -sS "$RPC_URL" -H 'content-type: application/json' -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"starknet_getClassHashAt\",\"params\":[\"latest\",\"$ADDRESS\"]}" \
  | python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("result",""))')

if [[ -z "${ON_CHAIN_CLASS_HASH}" ]]; then
  echo "Could not fetch class hash from RPC; falling back to parsed declare hash: ${CLASS_HASH}" >&2
  ON_CHAIN_CLASS_HASH="$CLASS_HASH"
fi

echo -n "$ON_CHAIN_CLASS_HASH" > "$CLASS_HASH_OUT_FILE"
echo "Wrote class hash to: $CLASS_HASH_OUT_FILE"
echo "Export this to enable startup verification:"
echo "export NF4_STARKNET_EXPECTED_CLASS_HASH=$ON_CHAIN_CLASS_HASH"

echo "\n==> Emitting sample events"
# Use a deterministic block_number and root.
starkli invoke --rpc "$RPC_URL" --account "$ACCOUNT" --private-key "$PRIVATE_KEY" "$ADDRESS" emit_block_proposed 1 0x1234 1700000000
starkli invoke --rpc "$RPC_URL" --account "$ACCOUNT" --private-key "$PRIVATE_KEY" "$ADDRESS" emit_deposit_escrowed 0xBEEF 0xCAFE 1 0

echo "\nDone. Contract address: $ADDRESS"
