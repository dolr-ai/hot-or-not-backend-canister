# AGENTS.md

## Purpose

This is the living agent guide for the `yral-backend-canister` repository. It captures current repository-specific conventions, workflow patterns, and instructions for an agent operating here.

This file is not a changelog. It should describe the active way this repository is expected to be used and updated. When repository-wide conventions change, update this file immediately so future agents can rely on it.

## Repository Overview

- This repo contains backend canisters for the HotOrNot/yral Internet Computer project.
- The root uses `dfx` and canister-based Rust crates under `src/canister/`.
- Local development and CI are driven from repository scripts, not ad hoc commands.
- `canister_ids.json` and `sns_canister_ids.json` are authoritative canister ID manifests for this repo.

## Canonical Scripts

All scripts live under `scripts/`. Use these — do not invent alternatives.

| Script | Purpose |
|--------|---------|
| `scripts/install-dependencies.sh` | Install dfx, pocket-ic, and candid-extractor (idempotent — safe to re-run) |
| `scripts/run-canister-test-suite.sh` | Full test suite |
| `scripts/generate-candid.sh` | Rebuild wasm(s) and regenerate `can.did` from the compiled output |
| `scripts/release-and-submit-proposals.sh` | Build wasms and submit SNS upgrade proposals directly from local machine |
| `scripts/upgrade_ic_repl.sh` | ic-repl script invoked by release script for user_index and individual_user_template |
| `scripts/canister_snapshot.sh` | Canister snapshot operations (take / list / load) |

### Running the test suite

```sh
bash scripts/install-dependencies.sh
bash scripts/run-canister-test-suite.sh
```

### Snapshot operations

```sh
ACTION=take_snapshot CANISTER_ID=<canister-id> bash scripts/canister_snapshot.sh
ACTION=list_snapshots CANISTER_ID=<canister-id> bash scripts/canister_snapshot.sh
ACTION=load_snapshot  CANISTER_ID=<canister-id> SNAPSHOT_ID=<snapshot-id> bash scripts/canister_snapshot.sh
```

## Testing Upgrades Locally

Before pushing canister changes to mainnet, verify the upgrade path:

1. `dfx start --clean --background`
2. `git checkout vx.y.z` — check out the last tag
3. `bash scripts/run-canister-test-suite.sh` — run suite on old tag
4. `git checkout main`
5. `bash scripts/run-canister-test-suite.sh` — run suite on new code
6. `dfx canister call <individual-canister-id> get_version` — confirm version is greater than `v1.0.0`

Also run `ic_repl_tests/all_tests.sh` to create test users and posts, then verify they are retained after upgrade.

## Mainnet Deployment

Pre-deployment checklist:
- Run the full upgrade test above.
- Ensure `actions_identity.pem` contains your SNS proposal submitter PEM key (never commit this file).

Deployment sequence:
1. Paste your PEM key into `actions_identity.pem` in the repo root.
2. Run:
   ```sh
   VERSION=v1.2.3 CHANGE_SUMMARY="your description" bash scripts/release-and-submit-proposals.sh
   ```
3. The script builds `platform_orchestrator`, `user_index`, and `individual_user_template` for mainnet, then submits SNS upgrade proposals for all three.
4. SNS neuron holders vote on the proposals.
5. On passing, `platform_orchestrator` distributes and installs the new wasms fleet-wide.

Verify after deployment:
- `dfx canister info <canister-id> --network=ic` — `Module hash` must match the hash printed by the script.
- Canister IDs: `canister_ids.json`.

## Agent Behavior Rules

- Always check `AGENTS.md` and `scripts/*` first for the current workflow.
- Avoid making arbitrary changes to canister deployment or upgrade behavior without explicit evidence from repo docs or tests.
- If a new process is introduced, document it here and keep the language prescriptive.
- Keep agent edits minimal when updating workflows: update the official script or docs, then update `AGENTS.md`.

## SQL / Database Access (sqlx)

**Hard rule: only compile-time checked queries are permitted.**

- Every interaction with SQLite (in `task_runner` and `ic_canister_snapshot`) **must** use the checked macros:
  - `sqlx::query!(...)` for statements that do not return rows (INSERT, UPDATE, DELETE, CREATE, PRAGMA, etc.)
  - `sqlx::query_scalar!(...)` for single-value SELECTs (COUNT(*), single columns, etc.)
  - `sqlx::query_as!(Type, ...)` (or the equivalent typed form) for row-to-struct mapping
- **Never** use the runtime string forms: `sqlx::query("...")`, `query_scalar("...")`, `query_as::<_, T>("...")`, or any `.bind()` on a raw query string.
- This rule applies to **all** statements, including initialisation PRAGMAs and `CREATE TABLE IF NOT EXISTS` / index DDL. There are no exceptions for "setup" or "infrastructure" queries.
- **One narrow, documented exception for SQLite PRAGMA connection tuning only**:
  - The two PRAGMA *assignment* statements executed at pool open time in `task_runner/src/db.rs:open_pool` and `ic_canister_snapshot/src/fetch.rs:open_and_init_db` (`journal_mode = WAL` and `synchronous = NORMAL`) are written with the raw runtime form `sqlx::query("PRAGMA ... = ...").execute(...)`.
  - Reason: SQLite's PRAGMA assignment syntax returns a row whose column is reported to the driver as untyped/NULL during sqlx macro expansion. No combination of `query!` / `query_scalar!` (plain or wrapped in `SELECT CAST(...)`) can be prepared against it without a syntax error or "no built-in mapping for NULL".
  - Immediately after each setter we issue the corresponding *getter* (`PRAGMA journal_mode`, `PRAGMA synchronous`) using the checked `query_scalar!` macro; those are clean typed columns and are recorded in `.sqlx/`.
  - Every other statement in the two crates — every CREATE TABLE / INDEX, every INSERT / UPDATE / SELECT / COUNT used by decommission tracking, cycle harvest (po_* tables, pending_harvests, mark_harvested, etc.), snapshot population, checkpoints, and progress — **must** be written with the checked `!` macros. The exception is strictly limited to these two initialisation lines.
- Rationale: type safety at compile time, prevention of schema drift, consistent behaviour across the two crates that share the `ic_canisters.db` file, and elimination of runtime SQL surprises. "No shortcuts. We only do queries that are type checked."

**Destructive operations and schema changes — strict prohibition**

- **Never ever drop tables without checking in with me.** This is a standing, non-negotiable instruction from the operator.
- You must **never** execute, propose, or generate any command, script, test, or one-off that performs `DROP TABLE`, `DROP TABLE IF EXISTS`, `DROP INDEX`, `DELETE FROM` (outside of narrowly scoped, versioned cleanup of transient rows), `TRUNCATE`, or any other destructive removal of tables or rows in `ic_canisters.db` (or any other SQLite file used for decommission, cycle harvest, canister snapshot, or PO-controlled canister tracking).
- All schema changes must be **additive and data-preserving**:
  - `CREATE TABLE IF NOT EXISTS`
  - `CREATE INDEX IF NOT EXISTS`
  - `ALTER TABLE ... ADD COLUMN` (only when it does not lose or invalidate existing data)
  - New tables or new versioned tables following the same patterns used for `decommissioned` / `cycle_harvested` etc.
- This rule applies even for "preparing a clean DB for sqlx prepare", "resetting for tests", "local development convenience", or "one-time migration scripts". There are no exceptions.
- This is consistent with the root repository `AGENTS.md` "Immutable Data Operations" and "1. Immutable Data Operations" principles. Treat the SQLite tracking DB with the same immutability discipline as production ClickHouse / Kafka / object storage state.
- If a table is truly obsolete, the correct path is: (1) stop writing to it, (2) stop reading from it, (3) propose removal only after explicit operator approval and after a data-preserving archival step if any historical data must be retained. Never drop first and ask later.

**Workflow when adding or changing queries:**

1. Write the new/changed query using the `!` macro form.
2. If the query references a table/column that does not yet exist in the on-disk DB used for prepare, first ensure the schema is present **using only additive, non-destructive commands**:
   - For task_runner tables (decommissioned, cycle_harvested, release_counter, etc.): run the ignored bootstrap test:
     `cargo test -p task_runner -- --ignored bootstrap_schema --nocapture`
   - For snapshot tables (canisters, controllers, progress): run the corresponding ignored populate test.
   - For tables populated by external operator scripts such as `po_controlled_canisters` (created by `scripts/snapshot-po-state.sh`): use a pure additive sqlite3 command, for example:
     ```
     sqlite3 ic_canisters.db "CREATE TABLE IF NOT EXISTS po_controlled_canisters (principal TEXT PRIMARY KEY);"
     ```
     **Never** run the snapshot-po-state.sh (or any other script) if it would DROP tables, unless you have received explicit "yes, you may drop" confirmation from the operator in this conversation for this specific operation.
3. Then populate / refresh the compile-time query cache:
   ```
   DATABASE_URL=sqlite:<absolute-path-to-ic_canisters.db> cargo sqlx prepare --workspace
   ```
4. Commit the updated `.sqlx/` directory contents along with the code change.

The `.sqlx/` cache **must** be checked into version control.

If a prepare fails with "no database rows" or "unknown column" errors, the tables were not present in the DATABASE_URL database at prepare time — re-run the appropriate bootstrap / additive create step first. Never "fix" this by writing a DROP-based reset.

**Scripts that currently contain DROP statements (e.g. `scripts/snapshot-po-state.sh`)** are operator-only tools. When an agent needs to refresh the po_* data for prepare or testing purposes, it must propose running a non-destructive subset or ask the operator for permission before invoking any DROP-containing logic. The default is to use the minimal `CREATE TABLE IF NOT EXISTS` above.

## When to Update This File

Update `AGENTS.md` whenever any of the following change:

- The canonical test or deployment script changes (name or behavior).
- The repository adds or removes a major canister or canister manifest file.
- The release/tagging/proposal process changes.
- The local reproducibility workflow changes.
- A new high-level engineering convention appears that future agents must know.

When updating, keep it terse and current. Remove obsolete patterns immediately.

## Self-Update Instructions

This file is the authoritative agent reference for this repository. If you are an agent making changes to repo-wide conventions:

- Change `AGENTS.md` as part of the same commit.
- Summarize the changed convention in a short new paragraph or bullet.
- Keep the content focused on the active repository state.
- Do not preserve old workflows as permanent content.

If a section of this repo becomes obsolete, delete it from this file instead of retaining it as historical context.

## HANDOFF.md Handling

- There is currently no `HANDOFF.md` in this repository.
- If a future agent sees a `HANDOFF.md` file:
  - Read it fully and absorb the exact resume state and next steps.
  - Migrate any relevant instructions into `AGENTS.md` if they represent ongoing repository conventions.
  - Remove `HANDOFF.md` after its context has been absorbed and the handoff is complete.

## Notes for Future Agents

- This repo is strongly centered on Internet Computer canisters and DFX tooling.
- Root-level scripts are the main integration points for developer workflows.
- If you need to experiment, prefer the documented `bash scripts/...` flows rather than creating new command conventions.
- Keep the living nature of this document in mind: it should reflect how this repository is actually used today.
