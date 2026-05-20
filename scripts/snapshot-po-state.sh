#!/usr/bin/env bash
# Snapshot platform_orchestrator on-chain state into ic_canister_snapshot/ic_canisters.db.
#
# Adds/replaces the following tables in the existing DB:
#   po_metadata            — version, counts, snapshot timestamp
#   po_subnet_orchestrators — all registered user_index canisters
#   po_controlled_canisters — all canisters tracked in controlled_canisters
#   po_decommission_failed  — canister IDs that failed the decommission flow
#
# Usage:
#   bash scripts/snapshot-po-state.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

PO="74zq4-iqaaa-aaaam-ab53a-cai"
DB="src/lib/ic_canister_snapshot/ic_canisters.db"

PREVIOUS_IDENTITY="$(dfx identity whoami)"
restore_identity() { dfx identity use "$PREVIOUS_IDENTITY" 2>/dev/null || true; }
trap restore_identity EXIT

dfx identity import --storage-mode=plaintext actions actions_identity.pem --force
dfx identity use actions
export DFX_WARNING=-mainnet_plaintext_identity

echo "==> Snapshotting platform_orchestrator state into ${DB}"

python3 - <<PYEOF
import subprocess, re, sys, sqlite3
from datetime import datetime, timezone

DB  = "${DB}"
PO  = "${PO}"
DFX = ["dfx", "canister", "call", "--network=ic", PO]

def call(method, args=""):
    cmd = DFX + ([method] if not args else [method, args])
    r = subprocess.run(cmd, capture_output=True, text=True)
    if r.returncode != 0:
        print(f"  WARN: {method} failed: {r.stderr.strip()}", file=sys.stderr)
        return ""
    return r.stdout.strip()

def principals(text):
    return re.findall(r'principal "([^"]+)"', text)

def nat(text):
    m = re.search(r'(\d[\d_]*)', text)
    return int(m.group(1).replace('_', '')) if m else 0

con = sqlite3.connect(DB)
cur = con.cursor()

import atexit
def _close():
    try:
        con.execute("PRAGMA wal_checkpoint(TRUNCATE)")
        con.close()
    except Exception:
        pass
atexit.register(_close)

# ── Schema ────────────────────────────────────────────────────────────────────
cur.executescript("""
DROP TABLE IF EXISTS po_metadata;
DROP TABLE IF EXISTS po_subnet_orchestrators;
DROP TABLE IF EXISTS po_controlled_canisters;
DROP TABLE IF EXISTS po_decommission_failed;

CREATE TABLE po_metadata (
    key   TEXT PRIMARY KEY,
    value TEXT
);
CREATE TABLE po_subnet_orchestrators (
    principal TEXT PRIMARY KEY
);
CREATE TABLE po_controlled_canisters (
    principal TEXT PRIMARY KEY
);
CREATE TABLE po_decommission_failed (
    principal TEXT,
    reason    TEXT
);
""")

# ── Metadata ──────────────────────────────────────────────────────────────────
print("  fetching version and counts...")
version_raw = call("get_version")
version = (re.search(r'"([^"]*)"', version_raw) or re.search(r"'([^']*)'", version_raw))
version = version.group(1) if version else version_raw

ctrl_count = nat(call("get_controlled_canisters_count"))

cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("snapshot_time", datetime.now(timezone.utc).isoformat()))
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("version", version))
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("controlled_canisters_count_on_chain", str(ctrl_count)))
print(f"  version={version}  controlled_canisters={ctrl_count:,}")

# ── Subnet orchestrators ──────────────────────────────────────────────────────
print("  fetching subnet orchestrators...")
uis = principals(call("get_all_subnet_orchestrators"))
cur.executemany("INSERT OR IGNORE INTO po_subnet_orchestrators VALUES (?)", [(p,) for p in uis])
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("subnet_orchestrators_count", str(len(uis))))
print(f"  {len(uis)} subnet orchestrators")

# ── Controlled canisters (paginated 10k at a time) ────────────────────────────
print(f"  fetching controlled canisters ({ctrl_count:,} total)...")
PAGE    = 10_000
fetched = 0
start   = 0
while start <= ctrl_count:
    raw   = call("get_controlled_canisters", f"({start} : nat64, {PAGE} : nat64)")
    batch = principals(raw)
    if not batch:
        break
    cur.executemany("INSERT OR IGNORE INTO po_controlled_canisters VALUES (?)", [(p,) for p in batch])
    fetched += len(batch)
    start   += len(batch)
    print(f"  {fetched:,}/{ctrl_count:,}", end="\r", flush=True)
    if len(batch) < PAGE:
        break
print(f"  fetched {fetched:,} controlled canisters          ")
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("controlled_canisters_fetched", str(fetched)))

# ── Decommission status ────────────────────────────────────────────────────────
print("  fetching decommission status...")
raw       = call("get_decommission_status")
completed = nat(re.search(r'completed_count\s*=\s*([\d_]+)', raw).group(0) if re.search(r'completed_count', raw) else "0")
# failed_canisters are vec of (Principal, Text) tuples
fail_matches = re.findall(r'principal "([^"]+)"[;\s]*"([^"]*)"', raw)
for pid, reason in fail_matches:
    cur.execute("INSERT INTO po_decommission_failed VALUES (?,?)", (pid, reason))
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("decommission_completed_count", str(completed)))
cur.execute("INSERT INTO po_metadata VALUES (?,?)", ("decommission_failed_count", str(len(fail_matches))))
print(f"  decommission completed={completed:,}  failed={len(fail_matches)}")

con.commit()

print()
print("=" * 60)
print(f"Done. Tables written to {DB}:")
print(f"  po_subnet_orchestrators : {len(uis)}")
print(f"  po_controlled_canisters : {fetched:,}")
print(f"  po_decommission_failed  : {len(fail_matches)}")
print("=" * 60)
PYEOF
