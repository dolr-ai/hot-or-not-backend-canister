#!/usr/bin/env bash
# Ad-hoc canister calls on mainnet as the actions identity.
# Edit the Operations section to run whatever one-off calls are needed.
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

# Suppress dfx's "plaintext identity on mainnet" warning.
export DFX_WARNING=-mainnet_plaintext_identity

# ── Operations ────────────────────────────────────────────────────────────────

# Example: query how many controlled canisters are stored on platform_orchestrator
echo "==> get_controlled_canisters_count"
dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" get_controlled_canisters_count --network=ic

# Example: query the last subnet upgrade status
echo "==> get_subnet_last_upgrade_status"
dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" get_subnet_last_upgrade_status --network=ic

# Example: query the decommission operation status
echo "==> get_decommission_status"
dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" get_decommission_status --network=ic
