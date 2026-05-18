#!/usr/bin/env bash
# Build and deploy the full canister suite locally.
# Mirrors the mainnet release flow as closely as possible:
#   1. Build + regenerate Candid interfaces
#   2. Deploy platform_orchestrator
#   3. Upload real wasm blobs to platform_orchestrator via ic-repl (binary Candid,
#      avoids the 4 MB HTTP text-encoding limit that dfx canister call hits)
#   4. Create user_index with platform_orchestrator as controller (CMC substitute)
#   5. platform_orchestrator installs the user_index wasm (mirrors governance upgrade)
#   6. Register user_index with platform_orchestrator
#   7. Provision a pool of individual user canisters via user_index
#
# Note: provision_subnet_orchestrator_canister uses the Cycles Minting Canister (CMC)
# which is not available on a local replica. Step 4 replicates what CMC does (canister
# creation only). Every step after that is driven by platform_orchestrator functions,
# matching mainnet exactly.
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

# ── Build ─────────────────────────────────────────────────────────────────────
echo "==> Building canisters and regenerating Candid interfaces..."
bash scripts/generate-candid.sh platform_orchestrator user_index individual_user_template

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

# ── Upload real wasm blobs to platform_orchestrator ───────────────────────────
# ic-repl sends binary Candid so the wasm bytes pass through as-is (~2-3 MB),
# well within the local replica's 4 MB HTTP body limit. dfx canister call
# text-encodes blobs as \XX hex sequences (4× expansion) and exceeds that limit.
echo "==> Uploading wasms to platform_orchestrator via ic-repl..."
cat > /tmp/upload_wasms_local.sh << ICREPL
#!/usr/bin/ic-repl -o
identity deployer "${DFX_IDENTITY_PEM}";
import po = "${PLATFORM_ORCHESTRATOR_ID}" as "${REPO_ROOT}/src/canister/platform_orchestrator/can.did";
call po.upload_wasms(variant {SubnetOrchestratorWasm}, file("${REPO_ROOT}/target/wasm32-unknown-unknown/release/user_index.wasm"));
call po.upload_wasms(variant {IndividualUserWasm}, file("${REPO_ROOT}/target/wasm32-unknown-unknown/release/individual_user_template.wasm"));
ICREPL
./ic-repl /tmp/upload_wasms_local.sh -r "${LOCAL_REPLICA}"
rm -f /tmp/upload_wasms_local.sh

# ── Create user_index canister (CMC substitute) ───────────────────────────────
# On mainnet, platform_orchestrator calls provision_subnet_orchestrator_canister
# which creates the canister via CMC on a specific subnet, installs the stored
# SubnetOrchestratorWasm, and provisions the individual canister pool. CMC is not
# available locally, so we replicate only the canister-creation step with dfx.
# Every subsequent step is a real platform_orchestrator function call.
echo "==> Creating user_index canister (CMC substitute — local only)..."
dfx canister create user_index
dfx canister update-settings user_index --add-controller "${PLATFORM_ORCHESTRATOR_ID}"

USER_INDEX_ID="$(dfx canister id user_index)"
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

# Mirrors what happens when a user_index governance upgrade proposal is approved:
# governance → platform_orchestrator_generic_function(UpgradeSubnetCanisters) →
# upgrade_canisters_in_network → recharge_and_upgrade_subnet_orchestrator.
# Here we drive the single-canister variant directly.
echo "==> platform_orchestrator upgrading user_index with stored wasm (mirrors governance proposal)..."
dfx canister call platform_orchestrator upgrade_subnet_orchestrator_canister_with_latest_wasm \
  "(principal \"${USER_INDEX_ID}\")"

# ── Provision individual canister pool via user_index ─────────────────────────
# On mainnet, provision_subnet_orchestrator_canister calls
# create_pool_of_individual_user_available_canisters on user_index after installing it.
# We replicate that call here using ic-repl (binary Candid, same size reason as above).
echo "==> Provisioning individual canister pool via user_index..."
cat > /tmp/provision_pool_local.sh << ICREPL
#!/usr/bin/ic-repl -o
identity deployer "${DFX_IDENTITY_PEM}";
import ui = "${USER_INDEX_ID}" as "${REPO_ROOT}/src/canister/user_index/can.did";
call ui.create_pool_of_individual_user_available_canisters("${VERSION}", file("${REPO_ROOT}/target/wasm32-unknown-unknown/release/individual_user_template.wasm"));
ICREPL
./ic-repl /tmp/provision_pool_local.sh -r "${LOCAL_REPLICA}"
rm -f /tmp/provision_pool_local.sh

echo ""
echo "════════════════════════════════════════════════════════════════"
echo " Deployment complete."
echo "  platform_orchestrator : ${PLATFORM_ORCHESTRATOR_ID}"
echo "  user_index            : ${USER_INDEX_ID}"
echo ""
echo " Validate with:"
echo "   dfx canister call user_index get_bulk_operation_status"
echo "   dfx canister call user_index add_platform_orchestrator_as_controller_to_all_canisters"
echo "   dfx canister call user_index add_platform_orchestrator_as_controller_to_specific_canister '(principal \"${USER_INDEX_ID}\")'"
echo "════════════════════════════════════════════════════════════════"
