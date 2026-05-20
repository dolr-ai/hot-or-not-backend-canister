#!/usr/bin/env bash
# Execute canister calls on mainnet as the actions identity.
# Add new operations below as needed — the VSCode task just runs this file.
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

# Suppress dfx's "plaintext identity on mainnet" warning — we accept the risk
# since this identity is only used for governance proposals and canister calls,
# not for holding cycles or ICP balances.
export DFX_WARNING=-mainnet_plaintext_identity

# ── Helpers ───────────────────────────────────────────────────────────────────

# Parse "principal X" entries from dfx Candid output and print one per line.
extract_principals() {
  python3 -c "
import sys, re
text = sys.stdin.read()
for p in re.findall(r'principal \"([^\"]+)\"', text):
    print(p)
"
}

# From get_bulk_operation_status output, return the count of canisters_remaining.
remaining_count() {
  python3 -c "
import sys, re
text = sys.stdin.read()
m = re.search(r'canisters_remaining\s*=\s*vec\s*\{([^}]*)\}', text, re.DOTALL)
section = m.group(1) if m else ''
print(len(re.findall(r'principal', section)))
"
}

# From get_bulk_operation_status output, extract completed_count.
completed_count() {
  python3 -c "
import sys, re
text = sys.stdin.read()
m = re.search(r'completed_count\s*=\s*([\d_]+)', text)
print(m.group(1).replace('_', '') if m else '0')
"
}

# From get_bulk_operation_status output, return the count of failed_canisters.
failed_count() {
  python3 -c "
import sys, re
text = sys.stdin.read()
m = re.search(r'failed_canisters\s*=\s*vec\s*\{([^}]*)\}', text, re.DOTALL)
section = m.group(1) if m else ''
print(len(re.findall(r'principal', section)))
"
}

# ── Operations ────────────────────────────────────────────────────────────────

POLL_INTERVAL=10  # seconds between full round-robin sweeps

echo "==> Fetching all subnet orchestrators from platform_orchestrator..."
orchestrators_raw=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_all_subnet_orchestrators --network=ic)

# Hand off to Python for all state-tracking logic.
# The shell is only responsible for making dfx calls; Python drives the loop.
python3 - <<PYEOF
import subprocess, re, sys, time
from datetime import datetime

POLL_INTERVAL = ${POLL_INTERVAL}
DFX = ["dfx", "canister", "call", "--network=ic"]

def dfx_call(canister_id, method, args=""):
    cmd = DFX + ([canister_id, method] if not args else [canister_id, method, args])
    result = subprocess.run(cmd, capture_output=True, text=True)
    return result.stdout + result.stderr

def extract_principals(text):
    return re.findall(r'principal "([^"]+)"', text)

def parse_upgrade_count(text):
    m = re.search(r'successful_upgrade_count\s*=\s*([\d_]+)', text)
    return int(m.group(1).replace('_', '')) if m else 0

def parse_version(text):
    m = re.search(r'version\s*=\s*"([^"]+)"', text)
    return m.group(1) if m else '?'

orchestrators = extract_principals("""${orchestrators_raw}""")
total = len(orchestrators)
print(f"    Found {total} subnet orchestrators.\n")

# Track last-seen successful_upgrade_count per user_index.
# A user_index is done when its count is unchanged from the previous poll
# (the upgrade has fully drained and stabilised).
prev_counts = {ui: None for ui in orchestrators}
pending = list(orchestrators)

while pending:
    sweep_time = datetime.now().strftime("%H:%M:%S")
    still_pending = []

    for ui in pending:
        raw = dfx_call(ui, "get_index_details_last_upgrade_status")
        current = parse_upgrade_count(raw)
        version  = parse_version(raw)
        prev     = prev_counts[ui]

        if prev is not None and current == prev:
            print(f"  ✓ [{sweep_time}] {ui}  upgraded={current}  version={version}")
        else:
            print(f"    [{sweep_time}] {ui}  upgraded={current}  version={version}")
            still_pending.append(ui)

        prev_counts[ui] = current

    pending = still_pending
    sys.stdout.flush()

    if pending:
        print(f"  --- {len(pending)} user_index(es) still upgrading, "
              f"next poll in {POLL_INTERVAL}s ---\n")
        time.sleep(POLL_INTERVAL)

print()
print("════════════════════════════════════════════════════════════════")
print(f" All {total} user_indexes have finished upgrading individual canisters.")
print("════════════════════════════════════════════════════════════════")
PYEOF
