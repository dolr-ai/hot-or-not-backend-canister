#!/usr/bin/env bash
# Build and deploy the full canister suite locally.
# Mirrors the mainnet release flow as closely as possible:
#   1. Build + regenerate Candid interfaces
#   2. Deploy platform_orchestrator
#   3. Build user_index via dfx (wasm-opt + gzip)
#   4. Upload optimized wasm blobs to platform_orchestrator via ic-repl
#   5. Create user_index with platform_orchestrator as controller (CMC substitute)
#   6. platform_orchestrator installs the user_index wasm (mirrors governance upgrade)
#   7. Register user_index with platform_orchestrator
#
# Wasm upload uses ic-repl (binary Candid) so the payload stays small.
# dfx build applies wasm-opt (-Os) + gzip: user_index 2.1 MB → 617 KB.
#
# Note: provision_subnet_orchestrator_canister calls the Cycles Minting Canister
# (CMC) to create canisters on specific subnets — CMC is not available locally.
# Step 5 replicates what CMC does (canister creation only). Every step after
# that is a real platform_orchestrator function call, matching mainnet exactly.
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
bash scripts/generate-candid.sh platform_orchestrator user_index

# ── Download ic-repl if not present ──────────────────────────────────────────
if [[ ! -x ./ic-repl ]]; then
  echo "==> Downloading ic-repl..."
  ICREPL_VERSION=$(curl -s https://api.github.com/repos/dfinity/ic-repl/releases/latest \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
  curl -fsSL -o ic-repl \
    "https://github.com/dfinity/ic-repl/releases/download/${ICREPL_VERSION}/ic-repl-macos"
  chmod +x ic-repl
fi

# ── Start local replica ───────────────────────────────────────────────────────
echo "==> Starting local dfx replica (clean)..."
dfx stop 2>/dev/null || true
dfx start --clean --background
sleep 4

# ── Locate the dfx identity PEM (used by ic-repl for authenticated calls) ────
DFX_IDENTITY_NAME="$(dfx identity whoami)"
DFX_CONFIG_ROOT="$(dirname "$(dfx info config-json-path)")"
DFX_IDENTITY_PEM="${DFX_CONFIG_ROOT}/identity/${DFX_IDENTITY_NAME}/identity.pem"

# ── Deploy platform_orchestrator ──────────────────────────────────────────────
echo "==> Deploying platform_orchestrator..."
dfx deploy platform_orchestrator \
  --argument "(record { version = \"${VERSION}\" })" \
  --no-wallet

PLATFORM_ORCHESTRATOR_ID="$(dfx canister id platform_orchestrator)"
echo "    platform_orchestrator: ${PLATFORM_ORCHESTRATOR_ID}"

# ── Build user_index via dfx ────────────────────────────────────────────────
# dfx build applies wasm-opt (-Os) + gzip (per dfx.json: optimize=size, gzip=true).
# user_index: 2.1 MB raw → 617 KB.
# This keeps the ic-repl upload payloads well within the 4 MB HTTP body limit.
echo "==> Building user_index (wasm-opt + gzip)..."
dfx canister create user_index
dfx build user_index
USER_INDEX_WASM=".dfx/local/canisters/user_index/user_index.wasm.gz"

# ── Upload optimized wasm blobs to platform_orchestrator ─────────────────────
echo "==> Uploading wasms to platform_orchestrator via ic-repl..."
cat > /tmp/upload_wasms_local.sh << ICREPL
#!/usr/bin/ic-repl -o
identity deployer "${DFX_IDENTITY_PEM}";
import po = "${PLATFORM_ORCHESTRATOR_ID}" as "${REPO_ROOT}/src/canister/platform_orchestrator/can.did";
call po.upload_wasms(variant {SubnetOrchestratorWasm}, file("${REPO_ROOT}/${USER_INDEX_WASM}"));
ICREPL
./ic-repl /tmp/upload_wasms_local.sh -r "${LOCAL_REPLICA}"
rm -f /tmp/upload_wasms_local.sh

# ── Create user_index canister (CMC substitute) ───────────────────────────────
# The canister ID already exists from the dfx build step above.
# Add platform_orchestrator as a controller to mirror mainnet.
echo "==> Adding platform_orchestrator as controller of user_index..."
USER_INDEX_ID="$(dfx canister id user_index)"
dfx canister update-settings user_index --add-controller "${PLATFORM_ORCHESTRATOR_ID}"
echo "    user_index: ${USER_INDEX_ID}"

# Initial wasm install — on mainnet platform_orchestrator does this via
# install_code(Install, ...) inside provision_subnet_orchestrator_canister.
# upgrade_subnet_orchestrator_canister_with_latest_wasm uses Upgrade mode and
# requires a wasm to already be present, so we must do this initial install first.
echo "==> Initial install of user_index wasm (replicates PO install_code call)..."
dfx canister install user_index \
  --argument "(record { known_principal_ids = null; access_control_map = null; version = \"${VERSION}\" })"

# ── platform_orchestrator registers and manages user_index ────────────────────
# From here every step is a real platform_orchestrator function call, matching mainnet.
echo "==> Registering user_index with platform_orchestrator..."
dfx canister call platform_orchestrator register_new_subnet_orchestrator \
  "(principal \"${USER_INDEX_ID}\", true)"

# Top up cycles on both canisters — the upgrade flow makes inter-canister calls
# that burn cycles. On local dfx, fabricate-cycles works without ICP balance.
# SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD = 1_000T cycles.
# user_index must exceed that so PO skips the recharge-before-upgrade step.
# PO needs enough to run install_code after the threshold check passes.
echo "==> Topping up cycles on platform_orchestrator and user_index..."
dfx ledger fabricate-cycles --canister "${PLATFORM_ORCHESTRATOR_ID}" --t 2000
dfx ledger fabricate-cycles --canister "${USER_INDEX_ID}" --t 2000

# Mirrors what happens when a user_index governance upgrade proposal is approved:
# governance → platform_orchestrator_generic_function(UpgradeSubnetCanisters) →
# upgrade_canisters_in_network → recharge_and_upgrade_subnet_orchestrator.
echo "==> platform_orchestrator upgrading user_index with stored wasm (mirrors governance proposal)..."
dfx canister call platform_orchestrator upgrade_subnet_orchestrator_canister_with_latest_wasm \
  "(principal \"${USER_INDEX_ID}\")"

echo ""
echo "════════════════════════════════════════════════════════════════"
echo " Deployment complete."
echo "  platform_orchestrator : ${PLATFORM_ORCHESTRATOR_ID}"
echo "  user_index            : ${USER_INDEX_ID}"
echo "════════════════════════════════════════════════════════════════"
