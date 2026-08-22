#!/usr/bin/env bash
set -euo pipefail

ACTION="${ACTION:-take_snapshot}"
CANISTER_ID="${CANISTER_ID:-}"
SNAPSHOT_ID="${SNAPSHOT_ID:-}"
MAX_SNAPSHOTS="${MAX_SNAPSHOTS:-10}"
DFX="$(command -v dfx)"

if [[ -z "$CANISTER_ID" ]]; then
  echo "CANISTER_ID is required"
  exit 1
fi

case "$ACTION" in
  list_snapshots)
    "$DFX" canister --network ic snapshot list "$CANISTER_ID"
    ;;
  take_snapshot)
    echo "Checking existing snapshots..."
    snapshots=$("$DFX" canister --network ic snapshot list "$CANISTER_ID" 2>/dev/null || true)
    snapshot_ids=($(echo "$snapshots" | grep -oP '(?<=snapshot_id: )[a-f0-9]+' || true))
    snapshot_count=${#snapshot_ids[@]}

    if [[ "$snapshot_count" -ge "$MAX_SNAPSHOTS" ]]; then
      oldest_snapshot="${snapshot_ids[0]}"
      echo "At max capacity. Deleting oldest snapshot: $oldest_snapshot"
      "$DFX" canister --network ic snapshot delete "$CANISTER_ID" "$oldest_snapshot"
    fi

    "$DFX" canister --network ic stop "$CANISTER_ID"
    "$DFX" canister --network ic snapshot create "$CANISTER_ID"
    "$DFX" canister --network ic start "$CANISTER_ID"
    "$DFX" canister --network ic snapshot list "$CANISTER_ID"
    ;;
  load_snapshot)
    if [[ -z "$SNAPSHOT_ID" ]]; then
      echo "SNAPSHOT_ID is required when ACTION=load_snapshot"
      exit 1
    fi

    "$DFX" canister --network ic stop "$CANISTER_ID"
    "$DFX" canister --network ic snapshot load "$CANISTER_ID" "$SNAPSHOT_ID"
    "$DFX" canister --network ic start "$CANISTER_ID"
    ;;
  *)
    echo "Unsupported ACTION: $ACTION"
    exit 1
    ;;
esac
