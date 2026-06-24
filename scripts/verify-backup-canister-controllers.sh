#!/usr/bin/env bash
# Collect backup pool canisters into platform_orchestrator's controlled_canisters,
# then verify that the total count matches assigned + available + backup.
# Also spot-checks that PO is a controller on sampled backup canisters.
#
# NOTE: The old user_index-based backup verification was removed when user_index
# was decommissioned. This script is retained for future adaptation to the
# post-user_index canister architecture.
#
# Usage:
#   bash scripts/verify-backup-canister-controllers.sh
#
# Prerequisites:
#   - actions_identity.pem must exist in the repo root (gitignored)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

IDENTITY_FILE="actions_identity.pem"
PLATFORM_ORCHESTRATOR_ID="74zq4-iqaaa-aaaam-ab53a-cai"

if [[ ! -f "$IDENTITY_FILE" ]] || ! grep -q "BEGIN" "$IDENTITY_FILE" 2>/dev/null; then
  echo "Error: $IDENTITY_FILE not found or does not contain a PEM key."
  exit 1
fi

PREVIOUS_IDENTITY="$(dfx identity whoami)"
restore_identity() { dfx identity use "$PREVIOUS_IDENTITY" 2>/dev/null || true; }
trap restore_identity EXIT

dfx identity import --storage-mode=plaintext actions "$IDENTITY_FILE" --force
dfx identity use actions
export DFX_WARNING=-mainnet_plaintext_identity

echo "==> This script's user_index-based logic was removed when user_index was decommissioned."
echo "    Edit this script to add backup verification for the current canister architecture."
