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

POLL_INTERVAL=30  # seconds between decommission status polls

total_raw=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_controlled_canisters_count --network=ic)

python3 - <<PYEOF
import subprocess, re, sys, random, time
from datetime import datetime

DFX    = ["dfx", "canister", "call", "--network=ic"]
PO     = "${PLATFORM_ORCHESTRATOR_ID}"
POLL   = ${POLL_INTERVAL}
PROBE  = 2   # canisters to test individually before bulk run

def dfx_call(canister, method, args=""):
    cmd = DFX + ([canister, method] if not args else [canister, method, args])
    r = subprocess.run(cmd, capture_output=True, text=True)
    return r.stdout + r.stderr

def extract_principals(text):
    return re.findall(r'principal "([^"]+)"', text)

def parse_nat(text, field=""):
    pattern = rf'{re.escape(field)}\s*=\s*([\d_]+)' if field else r'(\d[\d_]*)'
    m = re.search(pattern, text)
    return int(m.group(1).replace('_','')) if m else 0

def parse_failed_count(text):
    m = re.search(r'failed_canisters\s*=\s*vec\s*\{', text, re.DOTALL)
    if not m:
        return 0
    start, depth = m.end(), 1
    for i, ch in enumerate(text[start:], start):
        if ch == '{': depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                return text[start:i].count('record {')
    return 0

def decommission_status():
    raw = dfx_call(PO, "get_decommission_status")
    remaining = parse_nat(raw, "canisters_remaining")
    completed = parse_nat(raw, "completed_count")
    failed    = parse_failed_count(raw)
    return remaining, completed, failed

# ── Pass 1: probe a couple of random canisters individually ──────────────────
print("═" * 64)
print(f"Pass 1: decommission_individual_canister on {PROBE} random canisters")
print("═" * 64)

total = parse_nat("""${total_raw}""")
probe_ids = [
    p for raw in [
        dfx_call(PO, "get_controlled_canisters", f"({random.randint(0, max(0, total - PROBE))} : nat64, {PROBE} : nat64)")
    ]
    for p in extract_principals(raw)
][:PROBE]

probe_ok = True
for cid in probe_ids:
    print(f"\n  ==> decommission_individual_canister: {cid}")
    result = dfx_call(PO, "decommission_individual_canister", f'(principal "{cid}")')
    print(f"      {result.strip()}")
    if "ok" not in result.lower() and "err" not in result.lower():
        print("  ✗ Unexpected response — aborting before bulk run.")
        probe_ok = False
        break
    if '"Err"' in result or "err =" in result.lower():
        print("  ✗ Error returned — aborting before bulk run.")
        probe_ok = False
        break

    # Assert the only controller is now the platform_orchestrator
    details = dfx_call(PO, "get_controllers_and_cycle_balance", f'(principal "{cid}")')
    controllers = extract_principals(details)
    if controllers == [PO]:
        print(f"  ✓ controllers=[PO only]  cycle_balance present")
    else:
        print(f"  ✗ Unexpected controllers: {controllers} — aborting before bulk run.")
        probe_ok = False
        break

if not probe_ok:
    sys.exit(1)

print(f"\nProbe passed. Proceeding to bulk decommission of {total} canisters.\n")

# ── Pass 2: bulk decommission ─────────────────────────────────────────────────
print("═" * 64)
print("Pass 2: decommission_all_controlled_canisters")
print("═" * 64)

result = dfx_call(PO, "decommission_all_controlled_canisters")
print(f"  {result.strip()}\n")

# ── Monitor until complete ────────────────────────────────────────────────────
print(f"Polling get_decommission_status every {POLL}s...\n")

while True:
    remaining, completed, failed = decommission_status()
    ts = datetime.now().strftime("%H:%M:%S")
    print(f"  [{ts}]  remaining={remaining}  completed={completed}  failed={failed}")
    sys.stdout.flush()

    if remaining == 0:
        print()
        print("═" * 64)
        print(f"  Decommission complete.")
        print(f"  completed={completed}  failed={failed}")
        if failed:
            print(f"  ⚠  {failed} canister(s) failed — check get_decommission_status for details.")
        else:
            print("  Result: ALL PASS ✓")
        print("═" * 64)
        break

    time.sleep(POLL)
PYEOF
