#!/usr/bin/env bash
# Verify that backup pool canisters have the platform_orchestrator as a controller.
# Samples 3 backup canister IDs from a user_index that has a non-empty backup pool
# and checks their controllers via platform_orchestrator's get_controllers_and_cycle_balance.
#
# Usage:
#   bash scripts/verify-backup-canister-controllers.sh
#
# Prerequisites:
#   - actions_identity.pem must exist in the repo root (gitignored)
#   - user_index must be upgraded with get_backup_canister_sample
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

IDENTITY_FILE="actions_identity.pem"
PLATFORM_ORCHESTRATOR_ID="74zq4-iqaaa-aaaam-ab53a-cai"
USER_INDEX_WITH_BACKUP="457xo-jaaaa-aaaap-accqa-cai"  # assigned=20104 backup=490
SAMPLE=3

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

echo "==> Fetching ${SAMPLE} backup canister IDs from ${USER_INDEX_WITH_BACKUP}..."
backup_raw=$(dfx canister call "${USER_INDEX_WITH_BACKUP}" \
  get_backup_canister_sample "(${SAMPLE} : nat64)" --network=ic)

python3 - <<PYEOF
import subprocess, re, sys

DFX = ["dfx", "canister", "call", "--network=ic"]
PO  = "${PLATFORM_ORCHESTRATOR_ID}"

def dfx_call(canister, method, args=""):
    cmd = DFX + ([canister, method] if not args else [canister, method, args])
    return subprocess.run(cmd, capture_output=True, text=True).stdout

backup_ids = re.findall(r'principal "([^"]+)"', """${backup_raw}""")
print(f"  Sampled {len(backup_ids)} backup canister ID(s)\n")

all_pass = True
for cid in backup_ids:
    raw         = dfx_call(PO, "get_controllers_and_cycle_balance", f'(principal "{cid}")')
    controllers = re.findall(r'principal "([^"]+)"', raw)
    stat        = re.search(r'status\s*=\s*variant\s*\{\s*(\w+)', raw)
    bal         = re.search(r'cycle_balance\s*=\s*([\d_]+)', raw)
    status      = stat.group(1) if stat else 'Unknown'
    balance     = int(bal.group(1).replace('_','')) if bal else -1

    if PO in controllers:
        print(f"  ✓ {cid}")
        print(f"    controllers : {controllers}")
        print(f"    status      : {status}")
        print(f"    cycle_bal   : {balance / 1e9:.0f}M cycles")
    else:
        print(f"  ✗ {cid}  PO NOT a controller — controllers={controllers}")
        all_pass = False
    print()

print("═" * 64)
if all_pass:
    print("  Result: PO is a controller on all sampled backup canisters ✓")
else:
    print("  Result: UNEXPECTED — some backup canisters missing PO as controller ✗")
    sys.exit(1)
print("═" * 64)
PYEOF
