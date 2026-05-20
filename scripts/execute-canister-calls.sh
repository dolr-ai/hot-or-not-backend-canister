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
# Probe: decommission 2 random individual canisters and assert all three
# post-conditions before proceeding to any bulk operation:
#   1. controllers == [platform_orchestrator only]
#   2. wasm is uninstalled (module_hash absent)
#   3. cycles were returned (balance <= 2T — near the 100M reserve)

PROBE=2

total_raw=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_controlled_canisters_count --network=ic)

python3 - <<PYEOF
import subprocess, re, sys, random, time

DFX = ["dfx", "canister", "call", "--network=ic"]
PO  = "${PLATFORM_ORCHESTRATOR_ID}"

CYCLE_WARN = 500_000_000_000  # 0.5T — 100M reserve + small post-reinstall reserved amount

def run(cmd):
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.stdout + r.stderr

def dfx_call(canister, method, args=""):
    cmd = DFX + ([canister, method] if not args else [canister, method, args])
    return run(cmd)

def extract_principals(text):
    return re.findall(r'principal "([^"]+)"', text)

def parse_nat(text):
    m = re.search(r'(\d[\d_]*)', text)
    return int(m.group(1).replace('_', '')) if m else 0

def get_details(cid):
    """Returns (controllers, cycle_balance, reserved_cycles, canister_status)."""
    raw = dfx_call(PO, "get_controllers_and_cycle_balance", f'(principal "{cid}")')
    controllers = extract_principals(raw)
    bal  = re.search(r'cycle_balance\s*=\s*([\d_]+)', raw)
    res  = re.search(r'reserved_cycles\s*=\s*([\d_]+)', raw)
    stat = re.search(r'status\s*=\s*variant\s*\{\s*(\w+)', raw)
    return (
        controllers,
        int(bal.group(1).replace('_',''))  if bal  else -1,
        int(res.group(1).replace('_',''))  if res  else -1,
        stat.group(1)                      if stat else 'Unknown',
    )

# Pick 2 random canisters from the controlled set
total = parse_nat("""${total_raw}""")
start = random.randint(0, max(0, total - ${PROBE}))
probe_ids = extract_principals(
    dfx_call(PO, "get_controlled_canisters", f"({start} : nat64, ${PROBE} : nat64)")
)[:${PROBE}]

print("═" * 64)
print(f"Probe: decommission_individual_canister on {len(probe_ids)} random canisters")
print("═" * 64)

all_pass = True

for cid in probe_ids:
    print(f"\n── {cid}")
    failures = []

    # Snapshot BEFORE decommission
    _, bal_before, res_before, status_before = get_details(cid)
    frozen_before = status_before in ('Stopped', 'Stopping')
    print(f"  BEFORE  cycle_balance={bal_before/1e9:.0f}M  reserved_cycles={res_before/1e9:.0f}M  "
          f"total={(bal_before+res_before)/1e9:.0f}M  status={status_before}"
          + ("  ⚠ FROZEN — return_cycle_balance will be skipped" if frozen_before else ""))

    # Decommission
    result = dfx_call(PO, "decommission_individual_canister", f'(principal "{cid}")')
    if "err" in result.lower() and "variant { Ok" not in result:
        failures.append(f"decommission returned error: {result.strip()}")
        for f in failures: print(f"  ✗ {f}")
        all_pass = False
        continue
    print(f"  decommission  : {result.strip()}")

    # Snapshot immediately AFTER
    controllers, bal_imm, res_imm, status_imm = get_details(cid)
    print(f"  AFTER(imm)  cycle_balance={bal_imm/1e9:.0f}M  reserved_cycles={res_imm/1e9:.0f}M  status={status_imm}")

    # Wait 15s to test whether reserved release is immediate or delayed
    print(f"  Waiting 15s to confirm reserved release timing...")
    time.sleep(15)
    _, bal_15s, res_15s, _ = get_details(cid)
    print(f"  AFTER(15s)  cycle_balance={bal_15s/1e9:.0f}M  reserved_cycles={res_15s/1e9:.0f}M")

    if bal_imm == bal_15s and res_imm == res_15s:
        print(f"  timing        : ✓ immediate (no change after 15s)")
    else:
        delta = (bal_15s + res_15s) - (bal_imm + res_imm)
        print(f"  timing        : ⚠ balance changed after 15s (Δ={delta/1e9:.0f}M) — delayed release")

    # Cycle recovery summary
    recovered_main = max(0, bal_before - bal_imm)
    recovered_res  = max(0, res_before - res_imm)
    if frozen_before:
        print(f"  recovered     : 0M (canister was frozen — could not call return function)")
    else:
        print(f"  recovered     : {recovered_main/1e9:.0f}M from main + {recovered_res/1e9:.0f}M from reserved "
              f"= {(recovered_main+recovered_res)/1e9:.0f}M total sent to PO")

    # Assertions
    if controllers == [PO]:
        print(f"  controllers   : ✓ [PO only]")
    else:
        failures.append(f"controllers={controllers}, expected [PO only]")

    # Frozen canisters legitimately have 0 recovery — not a failure
    if frozen_before:
        print(f"  cycle_balance : ℹ {bal_imm/1e9:.0f}M (was frozen, no cycles to recover)")
    elif 0 <= bal_imm <= CYCLE_WARN:
        print(f"  cycle_balance : ✓ {bal_imm/1e9:.0f}M (within 0.5T threshold)")
    else:
        failures.append(f"cycle_balance={bal_imm/1e9:.0f}M exceeds 0.5T — return may not have run")

    info = run(["dfx", "canister", "info", cid, "--network=ic"])
    if "module hash: none" in info.lower() or "module hash:" not in info.lower():
        print(f"  wasm          : ✓ uninstalled")
    else:
        hash_line = next((l for l in info.splitlines() if "module hash" in l.lower()), "")
        failures.append(f"wasm still present: {hash_line.strip()}")

    if failures:
        for f in failures: print(f"  ✗ {f}")
        all_pass = False
    else:
        print(f"  ── all assertions passed ✓")

print()
print("═" * 64)
if all_pass:
    print("Probe PASSED — all assertions satisfied.")
    print("Run decommission_all_controlled_canisters when ready for the bulk pass.")
else:
    print("Probe FAILED — fix issues above before running bulk decommission.")
    sys.exit(1)
print("═" * 64)
PYEOF
