#!/usr/bin/env bash
# Monitor a long-running canister operation on mainnet as the actions identity.
# Edit the Operations section below to target whichever operation is currently running.
#
# NOTE: The old subnet orchestrator / user_index polling logic was removed when
# user_index was decommissioned.
#
# Usage:
#   bash scripts/monitor-long-running-task.sh
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

# Suppress dfx's "plaintext identity on mainnet" warning — we accept the risk
# since this identity is only used for governance proposals and canister calls,
# not for holding cycles or ICP balances.
export DFX_WARNING=-mainnet_plaintext_identity

echo "==> No active long-running task monitoring configured."
echo "    The old user_index polling logic was removed when user_index was decommissioned."
echo "    Edit this script to add monitoring for the current canister architecture."
