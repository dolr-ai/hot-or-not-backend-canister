#!/usr/bin/env bash
# Execute canister calls on mainnet as the actions identity.
# Add new operations below as needed — the VSCode task just runs this file.
#
# Usage:
#   bash scripts/execute-canister-calls.sh
#
# Prerequisites:
#   - actions_identity.pem must exist in the repo root (gitignored)
#     Paste your SNS proposal submitter PEM key into that file before running.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

IDENTITY_FILE="actions_identity.pem"
PLATFORM_ORCHESTRATOR_ID="74zq4-iqaaa-aaaam-ab53a-cai"

if [[ ! -f "$IDENTITY_FILE" ]] || ! grep -q "BEGIN" "$IDENTITY_FILE" 2>/dev/null; then
  echo "Error: $IDENTITY_FILE not found or does not contain a PEM key."
  echo "Paste your SNS proposal submitter PEM key into $IDENTITY_FILE and re-run."
  exit 1
fi

PREVIOUS_IDENTITY="$(dfx identity whoami)"
restore_identity() {
  dfx identity use "$PREVIOUS_IDENTITY" 2>/dev/null || true
}
trap restore_identity EXIT

dfx identity import --storage-mode=plaintext actions "$IDENTITY_FILE" --force
dfx identity use actions

# ── Operations ────────────────────────────────────────────────────────────────

echo "==> Calling collect_controlled_canisters on platform_orchestrator (${PLATFORM_ORCHESTRATOR_ID})..."
result=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  collect_controlled_canisters --network=ic)
echo "    Fetched this run: ${result}"

echo ""
echo "==> Querying controlled_canisters total count..."
count=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_controlled_canisters_count --network=ic)
echo "    Total stored: ${count}"
