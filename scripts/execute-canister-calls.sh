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

PAGE_SIZE=5       # canisters per random sample
SAMPLES=5         # random offsets per iteration
ITERATIONS=20     # total verification rounds

echo "==> Fetching total controlled_canisters count..."
total_raw=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
  get_controlled_canisters_count --network=ic)
total=$(echo "$total_raw" | python3 -c "
import sys, re
m = re.search(r'(\d[\d_]*)', sys.stdin.read())
print(m.group(1).replace('_','') if m else '0')
")
echo "    Total: ${total}"
echo ""

if [[ "${total}" -eq 0 ]]; then
  echo "No controlled canisters found. Run collect_controlled_canisters first."
  exit 1
fi

max_start=$(( total - PAGE_SIZE ))
total_checked=0
total_failures=0

for iteration in $(seq 1 "${ITERATIONS}"); do
  echo "── Iteration ${iteration}/${ITERATIONS} ────────────────────────────────────"

  iter_failures=0
  for sample in $(seq 1 "${SAMPLES}"); do
    start=$(python3 -c "import random; print(random.randint(0, ${max_start}))")

    canister_ids=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
      get_controlled_canisters "(${start} : nat64, ${PAGE_SIZE} : nat64)" --network=ic \
      | extract_principals)

    for canister_id in $canister_ids; do
      total_checked=$(( total_checked + 1 ))

      result=$(dfx canister call "${PLATFORM_ORCHESTRATOR_ID}" \
        get_controllers_and_cycle_balance "(principal \"${canister_id}\")" --network=ic)

      if echo "${result}" | grep -q "\"${PLATFORM_ORCHESTRATOR_ID}\""; then
        echo "    ✓  [s${sample} off=${start}] ${canister_id}"
      else
        echo "    ✗  [s${sample} off=${start}] ${canister_id} — PO NOT a controller"
        echo "       raw: ${result}"
        iter_failures=$(( iter_failures + 1 ))
        total_failures=$(( total_failures + 1 ))
      fi
    done
  done

  echo "    Iteration ${iteration}: ${iter_failures} failures"
  echo ""
done

echo "════════════════════════════════════════════════════════════════"
echo " Verification complete."
echo "  Canisters checked : ${total_checked}  (${ITERATIONS} iterations × ${SAMPLES} samples × ${PAGE_SIZE})"
echo "  Failures          : ${total_failures}"
if [[ "${total_failures}" -eq 0 ]]; then
  echo "  Result: ALL PASS ✓"
else
  echo "  Result: FAILURES DETECTED ✗"
  exit 1
fi
echo "════════════════════════════════════════════════════════════════"
