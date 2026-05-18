#!/usr/bin/env bash
# Build canister wasms and submit SNS upgrade proposals directly from a local machine.
# Must be run from the repo root.
#
# The version counter below is auto-incremented after each successful run.
# Commit the updated script to git after each deployment so the next run
# picks up the correct version number.
#
# Usage:
#   bash scripts/release-and-submit-proposals.sh
#   CHANGE_SUMMARY="your description" bash scripts/release-and-submit-proposals.sh
#   RELEASE_SCOPE=platform_orchestrator bash scripts/release-and-submit-proposals.sh
#
# RELEASE_SCOPE controls which canisters are built and proposed:
#   platform_orchestrator            — platform_orchestrator only
#   platform_orchestrator_user_index — platform_orchestrator + user_index
#   all (default)                    — platform_orchestrator + user_index + individual_user_template
#
# Prerequisites:
#   - actions_identity.pem: paste your SNS proposal submitter PEM key into this file (gitignored)
#   - dfx installed: bash scripts/install-dependencies.sh
set -euo pipefail

# ── Version counter (auto-incremented on each successful run) ──────────────────
CURRENT_VERSION=4
# ──────────────────────────────────────────────────────────────────────────────

NEURON_ID="4de673e9cd7a1339afea6523a5f227d25e9d739ff52635ac86dbdb0447ae106a"
IDENTITY_FILE="actions_identity.pem"
RELEASE_SCOPE="${RELEASE_SCOPE:-all}"

# Determine which subnet canisters to build and propose in addition to platform_orchestrator.
# platform_orchestrator is always built and proposed via direct SNS upgrade.
# Subnet canisters (user_index, individual_user_template) are proposed via the
# platform_orchestrator's UpgradeSubnetCanisters generic function (id 4002).
case "$RELEASE_SCOPE" in
  platform_orchestrator)
    SUBNET_CANISTERS_TO_BUILD=""
    SUBNET_CANISTERS_TO_UPGRADE=""
    ;;
  platform_orchestrator_user_index)
    SUBNET_CANISTERS_TO_BUILD="user_index"
    SUBNET_CANISTERS_TO_UPGRADE="user_index"
    ;;
  all)
    SUBNET_CANISTERS_TO_BUILD="individual_user_template user_index"
    SUBNET_CANISTERS_TO_UPGRADE="user_index individual_user_template"
    ;;
  *)
    echo "Error: Unknown RELEASE_SCOPE '${RELEASE_SCOPE}'."
    echo "Valid values: platform_orchestrator, platform_orchestrator_user_index, all"
    exit 1
    ;;
esac

if [[ ! -f "$IDENTITY_FILE" ]] || ! grep -q "BEGIN" "$IDENTITY_FILE" 2>/dev/null; then
  echo "Error: $IDENTITY_FILE not found or does not contain a PEM key."
  echo "Paste your SNS proposal submitter PEM key into $IDENTITY_FILE and re-run."
  exit 1
fi

NEXT_VERSION=$((CURRENT_VERSION + 1))
VERSION="v${NEXT_VERSION}"
CHANGE_SUMMARY="${CHANGE_SUMMARY:-Upgrade canister fleet to ${VERSION}}"

echo "Deploying version:  ${VERSION}"
echo "Change summary:     ${CHANGE_SUMMARY}"
echo "Release scope:      ${RELEASE_SCOPE}"
echo ""

# Import the proposal submitter identity into dfx
dfx identity import --storage-mode=plaintext actions "$IDENTITY_FILE" --force
dfx identity use actions

# Build canisters for mainnet and print module hashes
for canister in platform_orchestrator $SUBNET_CANISTERS_TO_BUILD; do
  echo "==> Building $canister for mainnet..."
  dfx build "$canister" --network=ic
  hash=$(sha256sum < ".dfx/ic/canisters/${canister}/${canister}.wasm.gz")
  echo "    ${canister} module hash: ${hash}"
done

# Regenerate can.did files from the freshly-built wasm so the repo stays in sync.
# generate-candid.sh accepts multiple canister names as arguments.
echo "==> Regenerating Candid interfaces..."
bash scripts/generate-candid.sh platform_orchestrator $SUBNET_CANISTERS_TO_BUILD

# Download quill (macOS) if not present
if [[ ! -x ./quill ]]; then
  echo "==> Downloading quill..."
  curl -fsSL -o quill \
    https://github.com/dfinity/quill/releases/download/v0.4.2/quill-macos-x86_64
  chmod +x quill
fi

# Download ic-repl (macOS) if not present
if [[ ! -x ./ic-repl ]]; then
  echo "==> Downloading ic-repl..."
  ICREPL_VERSION=$(curl -s https://api.github.com/repos/dfinity/ic-repl/releases/latest \
    | python3 -c "import sys,json; print(json.load(sys.stdin)['tag_name'])")
  curl -fsSL -o ic-repl \
    "https://github.com/dfinity/ic-repl/releases/download/${ICREPL_VERSION}/ic-repl-macos"
  chmod +x ic-repl
fi

# Submit SNS upgrade proposal for platform_orchestrator (direct canister upgrade)
canister_name="platform_orchestrator"
canister_id="$(dfx canister id "$canister_name" --network=ic)"
mkdir -p "scripts/proposals/${canister_name}"

echo "==> Submitting SNS upgrade proposal for ${canister_name}..."
./quill sns \
  --canister-ids-file ./sns_canister_ids.json \
  --pem-file "$IDENTITY_FILE" \
  make-upgrade-canister-proposal \
  --title "Upgrade ${canister_name}" \
  --summary "# Upgrade ${canister_name}
  ${CHANGE_SUMMARY}" \
  --url 'https://yral.com' \
  --target-canister-id "$canister_id" \
  --wasm-path ".dfx/ic/canisters/${canister_name}/${canister_name}.wasm.gz" \
  --canister-upgrade-arg "(record {version=\"${VERSION}\"})" \
  "$NEURON_ID" > "scripts/proposals/${canister_name}/upgrade.json"

./quill send "scripts/proposals/${canister_name}/upgrade.json" --yes

# Submit SNS generic function proposals for subnet canisters (if any).
# These go through platform_orchestrator's UpgradeSubnetCanisters generic function (id 4002),
# which distributes the wasm to the entire canister fleet.
if [[ -n "$SUBNET_CANISTERS_TO_UPGRADE" ]]; then
  for canister_name in $SUBNET_CANISTERS_TO_UPGRADE; do
    echo "==> Submitting SNS proposal for ${canister_name}..."
    CANISTER_NAME="$canister_name" \
    CHANGE_SUMMARY="$CHANGE_SUMMARY" \
    VERSION="$VERSION" \
      ./ic-repl scripts/upgrade_ic_repl.sh -r ic
  done
fi

rm -rf scripts/proposals

# ── Increment version counter in this script ───────────────────────────────────
SCRIPT_PATH="$(cd "$(dirname "$0")" && pwd)/$(basename "$0")"
sed -i '' "s/^CURRENT_VERSION=${CURRENT_VERSION}$/CURRENT_VERSION=${NEXT_VERSION}/" "$SCRIPT_PATH"
# ──────────────────────────────────────────────────────────────────────────────

echo ""
echo "Done. All proposals submitted for version ${VERSION}."
echo "CURRENT_VERSION in this script is now ${NEXT_VERSION}."
echo "Commit scripts/release-and-submit-proposals.sh to git to persist the version counter."
