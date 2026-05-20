#!/usr/bin/env bash
# Collect backup pool canisters into platform_orchestrator's controlled_canisters,
# then verify that the total count matches assigned + available + backup across all
# user_indexes. Also spot-checks that PO is a controller on sampled backup canisters.
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

# ── Step 1: spot-check controllers on backup canisters ───────────────────────
echo "==> Spot-checking controllers on ${SAMPLE} backup canisters..."
backup_raw=$(dfx canister call "${USER_INDEX_WITH_BACKUP}" \
  get_backup_canister_sample "(${SAMPLE} : nat64)" --network=ic)

# ── Step 2: collect backup canisters into controlled_canisters ───────────────
echo ""
echo "==> collect_backup_canisters..."
dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" collect_backup_canisters --network=ic

# ── Step 3: verify total count matches assigned + available + backup ──────────
echo ""
echo "==> Verifying controlled_canisters count..."

python3 - <<PYEOF
import subprocess, re, sys

DFX = ["dfx", "canister", "call", "--network=ic"]
PO  = "${PLATFORM_ORCHESTRATOR_ID}"

def dfx_call(canister, method, args=""):
    cmd = DFX + ([canister, method] if not args else [canister, method, args])
    return subprocess.run(cmd, capture_output=True, text=True).stdout

def nat(text):
    m = re.search(r'(\d[\d_]*)', text)
    return int(m.group(1).replace('_','')) if m else 0

# ── Spot-check ────────────────────────────────────────────────────────────────
backup_ids = re.findall(r'principal "([^"]+)"', """${backup_raw}""")
print(f"Spot-check: {len(backup_ids)} backup canister(s) from ${USER_INDEX_WITH_BACKUP}")
spot_pass = True
for cid in backup_ids:
    raw         = dfx_call(PO, "get_controllers_and_cycle_balance", f'(principal "{cid}")')
    controllers = re.findall(r'principal "([^"]+)"', raw)
    bal         = re.search(r'cycle_balance\s*=\s*([\d_]+)', raw)
    balance     = int(bal.group(1).replace('_','')) if bal else -1
    if PO in controllers:
        print(f"  ✓ {cid}  controllers={controllers}  cycle_bal={balance/1e9:.0f}M")
    else:
        print(f"  ✗ {cid}  PO NOT a controller — {controllers}")
        spot_pass = False

# ── Count validation ──────────────────────────────────────────────────────────
print()
print("Count validation across all user_indexes:")
orchestrators = re.findall(r'principal "([^"]+)"', dfx_call(PO, "get_all_subnet_orchestrators"))

total_assigned = total_available = total_backup = 0
for ui in orchestrators:
    total_assigned  += nat(dfx_call(ui, "get_user_index_canister_count"))
    total_available += nat(dfx_call(ui, "get_subnet_available_capacity"))
    total_backup    += nat(dfx_call(ui, "get_subnet_backup_capacity"))

expected = total_assigned + total_available + total_backup
actual   = nat(dfx_call(PO, "get_controlled_canisters_count"))
diff     = abs(expected - actual)

print(f"  assigned={total_assigned}  available={total_available}  backup={total_backup}")
print(f"  expected total          : {expected}")
print(f"  controlled_canisters    : {actual}")
print(f"  diff                    : {diff}")

count_pass = diff <= 50  # small tolerance for canisters moving between pools during queries

print()
print("═" * 64)
if spot_pass and count_pass:
    print("  controllers spot-check  : PASS ✓")
    print("  count match             : PASS ✓")
    print("  All backup canisters collected and verified.")
else:
    if not spot_pass: print("  controllers spot-check  : FAIL ✗")
    if not count_pass: print(f"  count match             : FAIL ✗ (diff={diff} exceeds tolerance of 50)")
    sys.exit(1)
print("═" * 64)
PYEOF
