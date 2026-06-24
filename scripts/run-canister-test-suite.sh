#!/usr/bin/env bash
set -euo pipefail

export POCKET_IC_BIN="${POCKET_IC_BIN:-$PWD/pocket-ic}"

echo "POCKET_IC_BIN=$POCKET_IC_BIN"

dfx identity list

dfx start --background
cleanup() {
  dfx stop || true
}
trap cleanup EXIT

for canister in \
  user_post_service \
  user_info_service \
  rate_limits
  do
  dfx canister create --no-wallet "$canister"
done

for canister in \
  user_post_service \
  user_info_service \
  rate_limits
  do
  dfx build "$canister"
  gzip -f -1 "./target/wasm32-unknown-unknown/release/${canister}.wasm"
done

cargo test
