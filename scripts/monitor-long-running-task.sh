#!/usr/bin/env bash
# Monitor a long-running canister operation on mainnet as the actions identity.
# Edit the Operations section below to target whichever operation is currently running.
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

def parse_nat(text, field):
    """Extract a numeric field value from Candid output, stripping _ separators."""
    m = re.search(rf'{re.escape(field)}\s*=\s*([\d_]+)', text)
    return int(m.group(1).replace('_', '')) if m else 0

def parse_version(text):
    m = re.search(r'version\s*=\s*"([^"]+)"', text)
    return m.group(1) if m else '?'

def parse_failed_count(text):
    """Count entries in failed_canister_ids = vec { record{p;p;msg}; ... }.
    Uses brace-counting to correctly extract the full vec body (which contains
    nested record{} entries), then counts 'record {' occurrences — one per failure."""
    m = re.search(r'failed_canister_ids\s*=\s*vec\s*\{', text, re.DOTALL)
    if not m:
        return 0
    start = m.end()
    depth = 1
    for i, ch in enumerate(text[start:], start):
        if ch == '{':
            depth += 1
        elif ch == '}':
            depth -= 1
            if depth == 0:
                section = text[start:i]
                return section.count('record {')
    return 0

orchestrators = extract_principals("""${orchestrators_raw}""")
total_orchestrators = len(orchestrators)
print(f"    Found {total_orchestrators} subnet orchestrators.\n")

# ── Phase 1: fetch expected totals for each user_index ───────────────────────
print("==> Fetching expected canister totals per user_index...")
totals = {}
for ui in orchestrators:
    assigned  = parse_nat(dfx_call(ui, "get_user_index_canister_count"), "")
    available = parse_nat(dfx_call(ui, "get_subnet_available_capacity"),  "")
    # get_user_index_canister_count returns a bare nat, not a named record field
    assigned_raw  = dfx_call(ui, "get_user_index_canister_count")
    available_raw = dfx_call(ui, "get_subnet_available_capacity")
    assigned  = int(re.search(r'(\d[\d_]*)', assigned_raw).group(1).replace('_',''))
    available = int(re.search(r'(\d[\d_]*)', available_raw).group(1).replace('_',''))
    totals[ui] = assigned + available
    print(f"    {ui}  assigned={assigned}  available={available}  total={totals[ui]}")

print()

# ── Phase 2: round-robin poll until every user_index is accounted for ────────
# Completion: successful + failed == total  (all canisters processed)
# Stuck:      processed count unchanged AND processed < total
#             → user_index updated what it could; remainder are unresolvable failures
prev_processed = {ui: None for ui in orchestrators}
pending = list(orchestrators)
stuck   = {}   # ui -> final processed count when declared stuck

while pending:
    sweep_time = datetime.now().strftime("%H:%M:%S")
    still_pending = []

    for ui in pending:
        raw        = dfx_call(ui, "get_index_details_last_upgrade_status")
        successful = parse_nat(raw, "successful_upgrade_count")
        failed     = parse_failed_count(raw)
        version    = parse_version(raw)
        processed  = successful + failed
        expected   = totals[ui]
        prev       = prev_processed[ui]

        if processed >= expected:
            print(f"  ✓ [{sweep_time}] {ui}  "
                  f"upgraded={successful}  failed={failed}  total={expected}  version={version}")
        elif prev is not None and processed == prev:
            # No progress since last poll → stuck
            gap = expected - processed
            print(f"  ⚠ [{sweep_time}] {ui}  STUCK — "
                  f"upgraded={successful}  failed={failed}  unaccounted={gap}  total={expected}  version={version}")
            stuck[ui] = (successful, failed, gap, version)
        else:
            print(f"    [{sweep_time}] {ui}  "
                  f"upgraded={successful}  failed={failed}  remaining≈{expected - processed}  version={version}")
            still_pending.append(ui)

        prev_processed[ui] = processed

    pending = still_pending
    sys.stdout.flush()

    if pending:
        print(f"  --- {len(pending)} user_index(es) still running, "
              f"next poll in {POLL_INTERVAL}s ---\n")
        time.sleep(POLL_INTERVAL)

print()
print("════════════════════════════════════════════════════════════════")
if stuck:
    print(f"  {total_orchestrators - len(stuck)}/{total_orchestrators} user_indexes completed cleanly.")
    print(f"  {len(stuck)} user_index(es) got stuck — investigate failures:")
    for ui, (ok, fail, gap, ver) in stuck.items():
        print(f"    {ui}  upgraded={ok}  failed={fail}  unaccounted={gap}  version={ver}")
else:
    print(f"  All {total_orchestrators} user_indexes finished upgrading. ✓")
print("════════════════════════════════════════════════════════════════")
PYEOF
