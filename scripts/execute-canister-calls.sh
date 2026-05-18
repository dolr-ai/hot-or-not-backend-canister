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

POLL_INTERVAL=10   # seconds between status polls
POLL_TIMEOUT=6000   # max seconds to wait per user_index (10 minutes)

echo "==> Fetching all subnet orchestrators from platform_orchestrator..."
orchestrators=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_all_subnet_orchestrators --network=ic | extract_principals)

total=$(echo "$orchestrators" | grep -c .) || total=0
echo "    Found ${total} subnet orchestrators."
echo ""

index=0
for user_index_id in $orchestrators; do
  index=$((index + 1))
  echo "── [${index}/${total}] user_index: ${user_index_id} ──────────────────────────"

  echo "    Calling add_platform_orchestrator_as_controller_to_all_canisters..."
  dfx canister call "${user_index_id}" \
    add_platform_orchestrator_as_controller_to_all_canisters --network=ic

  echo "    Polling get_bulk_operation_status (timeout: ${POLL_TIMEOUT}s)..."
  elapsed=0
  while true; do
    status_output=$(dfx canister call "${user_index_id}" \
      get_bulk_operation_status --network=ic)

    remaining=$(echo "$status_output" | remaining_count)
    completed=$(echo "$status_output" | completed_count)
    failed=$(echo "$status_output"   | failed_count)

    echo "    [${elapsed}s] remaining=${remaining}  completed=${completed}  failed=${failed}"

    if [[ "$remaining" -eq 0 ]]; then
      echo "    Done. completed=${completed} failed=${failed}"
      break
    fi

    if [[ $elapsed -ge $POLL_TIMEOUT ]]; then
      echo "    Timeout after ${POLL_TIMEOUT}s — ${remaining} canisters still remaining."
      break
    fi

    sleep "${POLL_INTERVAL}"
    elapsed=$((elapsed + POLL_INTERVAL))
  done
  echo ""
done

echo "════════════════════════════════════════════════════════════════"
echo " All ${total} subnet orchestrators processed."
echo "════════════════════════════════════════════════════════════════"
