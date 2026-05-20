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

USER_INDEX="m4e67-viaaa-aaaal-ad73q-cai"

# These 5 individual canisters had no wasm when the upgrade ran (previously
# uninstalled by another path). Install the IndividualUserWasm via PO so that
# return_cycle_balance_to_platform_orchestrator can run, then decommission.
FAILED_CANISTERS=(
  "ledwa-6iaaa-aaaal-ahilq-cai"
  "hqqr4-ziaaa-aaaal-alcfq-cai"
  "d7gmn-kyaaa-aaaal-aib4q-cai"
  "vmbhj-yiaaa-aaaal-agx2q-cai"
  "jq2am-jyaaa-aaaal-aiagq-cai"
)

for canister_id in "${FAILED_CANISTERS[@]}"; do
  echo "==> install_individual_user_wasm: ${canister_id}"
  dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
    install_individual_user_wasm \
    "(principal \"${canister_id}\")" \
    --network=ic

  echo "==> decommission_individual_canister: ${canister_id}"
  dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
    decommission_individual_canister \
    "(principal \"${canister_id}\")" \
    --network=ic
  echo ""
done

# ── Verify that both assigned and available individual canisters from a sample of
# user_indexes are present in platform_orchestrator's controlled_canisters set.

SAMPLE_PER_UI=5   # available canisters to check per user_index

orchestrators_raw=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_all_subnet_orchestrators --network=ic)

python3 - <<PYEOF
import subprocess, re, sys, random
from datetime import datetime

DFX = ["dfx", "canister", "call", "--network=ic"]
PO  = "${PLATFORM_ORCHESTRATOR_ID}"
SAMPLE = ${SAMPLE_PER_UI}

def dfx_call(canister, method, args=""):
    cmd = DFX + ([canister, method] if not args else [canister, method, args])
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.stdout + r.stderr

def extract_principals(text):
    return re.findall(r'principal "([^"]+)"', text)

def is_controlled(canister_id):
    out = dfx_call(PO, "is_controlled_canister", f'(principal "{canister_id}")')
    return "true" in out.lower()

orchestrators = extract_principals("""${orchestrators_raw}""")
print(f"Checking available canisters across {len(orchestrators)} user_indexes "
      f"({SAMPLE} samples each)\n")

total_checked = 0
total_failures = 0

for ui in orchestrators:
    available_raw = dfx_call(ui, "get_list_of_available_canisters")
    available = extract_principals(available_raw)

    if not available:
        print(f"  {ui}: no available canisters, skipping")
        continue

    sample = random.sample(available, min(SAMPLE, len(available)))

    ui_failures = 0
    for cid in sample:
        total_checked += 1
        if is_controlled(cid):
            print(f"  ✓  {ui}  {cid}")
        else:
            print(f"  ✗  {ui}  {cid}  — NOT in controlled_canisters")
            ui_failures += 1
            total_failures += 1

    if ui_failures:
        print(f"    ^ {ui_failures} failures on {ui}")

print()
print("════════════════════════════════════════════════════════════════")
print(f"  Checked : {total_checked}  Failures : {total_failures}")
if total_failures == 0:
    print("  Result  : ALL PASS ✓")
else:
    print("  Result  : FAILURES — available canisters missing from controlled_canisters ✗")
    sys.exit(1)
print("════════════════════════════════════════════════════════════════")
PYEOF
