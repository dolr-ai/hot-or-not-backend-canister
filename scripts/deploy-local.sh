#!/usr/bin/env bash
# Build and deploy platform_orchestrator locally.
#
# Usage:
#   bash scripts/deploy-local.sh
#
# Prerequisites:
#   - dfx installed: bash scripts/install-dependencies.sh
#   - candid-extractor on PATH (installed by install-dependencies.sh)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

LOCAL_REPLICA="http://localhost:4943"
VERSION="v0.0.1-local"

# ── Build: Candid interfaces + raw wasms ──────────────────────────────────────
echo "==> Building canisters and regenerating Candid interfaces..."
bash scripts/generate-candid.sh platform_orchestrator

# ── Start local replica ───────────────────────────────────────────────────────
echo "==> Starting local dfx replica (clean)..."
dfx stop 2>/dev/null || true
dfx start --clean --background
sleep 4

# ── Deploy platform_orchestrator ──────────────────────────────────────────────
echo "==> Deploying platform_orchestrator..."
dfx deploy platform_orchestrator \
  --argument "(record { version = \"${VERSION}\" })" \
  --no-wallet

PLATFORM_ORCHESTRATOR_ID="$(dfx canister id platform_orchestrator)"
echo "    platform_orchestrator: ${PLATFORM_ORCHESTRATOR_ID}"

# Top up cycles — the upgrade flow makes inter-canister calls that burn cycles.
# On local dfx, fabricate-cycles works without ICP balance.
echo "==> Topping up cycles on platform_orchestrator..."
dfx ledger fabricate-cycles --canister "${PLATFORM_ORCHESTRATOR_ID}" --t 2000

echo ""
echo "════════════════════════════════════════════════════════════════"
echo " Deployment complete."
echo "  platform_orchestrator : ${PLATFORM_ORCHESTRATOR_ID}"
echo "════════════════════════════════════════════════════════════════"
