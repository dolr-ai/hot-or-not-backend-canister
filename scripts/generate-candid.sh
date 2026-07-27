#!/usr/bin/env bash
# Builds every Rust canister wasm and regenerates its can.did from the compiled output.
# Run this after any change to a canister's public API (#[query] / #[update] functions).
#
# Usage:
#   bash scripts/generate-candid.sh                  # regenerate all canisters
#   bash scripts/generate-candid.sh user_info_service # regenerate one canister
#
# NOTE: This script is also invoked directly by Rust integration tests
# (task_runner tests) via Command::new("bash").args(["scripts/generate-candid.sh", ...]).
# It must remain as a file — do not inline into mise.toml only.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

ALL_CANISTERS=(
  user_post_service
  user_info_service
  canister_to_harvest
)

if [ $# -gt 0 ]; then
  CANISTERS=("$@")
else
  CANISTERS=("${ALL_CANISTERS[@]}")
fi

for canister in "${CANISTERS[@]}"; do
  echo "==> $canister: building wasm..."
  cargo build -p "$canister" --target wasm32-unknown-unknown --release -q

  wasm="$REPO_ROOT/target/wasm32-unknown-unknown/release/${canister}.wasm"
  did="$REPO_ROOT/src/canister/${canister}/can.did"

  echo "==> $canister: extracting candid..."
  candid-extractor "$wasm" > "$did"
  echo "    wrote $did"
done

echo "Done."