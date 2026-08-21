# AGENTS.md

## Purpose

This is the living agent guide for the `yral-backend-canister` repository. It captures current repository-specific conventions, workflow patterns, and instructions for an agent operating here.

This file is not a changelog. It should describe the active way this repository is expected to be used and updated. When repository-wide conventions change, update this file immediately so future agents can rely on it.

## Repository Overview

- This repo contains backend canisters for the HotOrNot/yral Internet Computer project.
- The root uses `dfx` and canister-based Rust crates under `src/canister/`.
- Local development and CI are driven from repository scripts, not ad hoc commands.
- `canister_ids.json` and `sns_canister_ids.json` are authoritative canister ID manifests for this repo.

## Canonical Workflows (task_runner)

**All deployment, upgrade, local PO setup, and cycle harvest / reclaim workflows live as versioned, ignored `cargo test` entry points inside `src/lib/task_runner/src/tests/`.**

Run them with:
```sh
cargo test -p task_runner -- --ignored <test-name> --nocapture
```

Key entry points (see the `#[ignore]` tests and their module docs for exact commands and env vars):
- `setup_local_po_and_validate_harvest_methods` — clean local dfx replica + post-cleanup PO deploy + controller wiring + validation that the three harvest methods (`get_version`, `get_controllers_and_cycle_balance`, `add_our_identity_as_controller`) are callable. This is the required local gate and replaces the old `scripts/deploy-local.sh`.
- `upgrade_po_directly` (and `upgrade_po_and_ui_directly`, `upgrade_all_directly`) — direct controller upgrade of platform_orchestrator (and optionally the fleets) via `dfx canister install --mode=upgrade`. This is the current path because `actions_identity` is a direct controller of the live PO. No SNS proposal is submitted for PO itself.
- `harvest_single_canister` (supports `HARVEST_CANISTER_ID=...` to target a specific canister) — the 10-step cycle reclaim flow for a single PO-controlled canister. The canister list is read from `src/canister/platform_orchestrator/principal.csv`.
- `deregister_po` / `reinstall_po_directly` / `reregister_po` — legacy one-time bridge (SNS deregister → direct management `install_code(Reinstall)` → SNS reregister) used to acquire direct controller rights. Do not use for routine upgrades.
- `reinstall_po` and `reinstall_via_generic` — legacy pure-SNS paths (UpgradeSnsControlledCanister or registered generic nervous system function). Retired for PO now that we are controllers.

The `task_runner` tests are the single source of truth for these operations. They are executable runbooks and co-located with the code they exercise.

### Utility scripts (still under `scripts/`)

These are retained for supporting tasks (Candid, snapshots). They are **not** the deployment/upgrade path.

| Script | Purpose |
|--------|--------|
| `scripts/generate-candid.sh` | Rebuild wasm(s) and regenerate `can.did` (still invoked from task_runner tests via `Command::new("bash")`) — must remain as a file. Also wrapped as `mise run canister-generate-candid` from the repo root. |
| `scripts/canister_snapshot.sh` | Canister snapshot operations (take / list / load) |

**Note:** `install-dependencies.sh` and `run-canister-test-suite.sh` have been inlined into mise.toml tasks (`canister-bootstrap`, `canister-test`). Use `mise run canister-bootstrap` and `mise run canister-test` from the repo root instead.

Other scripts (`release-and-submit-proposals.sh`, `deploy-local.sh`, `upgrade_ic_repl.sh`, etc.) are legacy for PO lifecycle and should not be used for new deployments or upgrades. They are kept in-tree only for reference / audit.

### Running the test suite

From the repo root:
```sh
mise run canister-bootstrap   # install dfx, pocket-ic, candid-extractor
mise run canister-test          # run full (non-ignored) test suite
```

### Snapshot operations

```sh
ACTION=take_snapshot CANISTER_ID=<canister-id> bash scripts/canister_snapshot.sh
ACTION=list_snapshots CANISTER_ID=<canister-id> bash scripts/canister_snapshot.sh
ACTION=load_snapshot  CANISTER_ID=<canister-id> SNAPSHOT_ID=<snapshot-id> bash scripts/canister_snapshot.sh
```

## Testing Upgrades Locally

**The canonical local validation is the ignored cargo test `setup_local_po_and_validate_harvest_methods`.**

This test:
- Starts a clean local dfx replica.
- Deploys the current `platform_orchestrator` (post-cleanup minimal surface).
- Wires the actions principal as controller of PO and PO as controller of a test target canister.
- Exercises the three harvest-critical methods (`get_version`, `get_controllers_and_cycle_balance`, `add_our_identity_as_controller`).
- Asserts success and prints the local PO/target IDs plus a ✅ banner.

Run (from the yral-backend-canister directory):
```sh
cargo test -p task_runner -- --ignored setup_local_po_and_validate_harvest_methods --nocapture
```

This replaces the old `bash scripts/deploy-local.sh` flow (which called removed methods like `upload_wasms` and would fail after the PO cleanup).

For full non-ignored unit/integration tests (the regular suite):
```sh
mise run canister-test
```

For a complete local upgrade rehearsal (old tag → new code), the spirit of the old checklist still applies:
1. Check out the last tag, run the regular test suite + the local PO setup test.
2. Switch to main / your branch.
3. Re-run the regular suite + the local PO setup test (and, if changing PO or harvest logic, the direct upgrade tests against a local PO if extended).
4. Verify `get_version` (or the harvest methods) report the expected new version on a local PO instance.

## Mainnet Deployment

**We are direct controllers of `platform_orchestrator`. PO upgrades no longer go through SNS proposals.**

Pre-deployment checklist:
- Run the local PO setup test (`setup_local_po_and_validate_harvest_methods`) and confirm it passes with the current code. This is the local gate.
- Ensure `actions_identity.pem` in the repo root contains the PEM for the principal that is a controller of the live PO (never commit this file).
- For PO itself: use the direct `dfx canister install --mode=upgrade` path (exposed via the `upgrade_po_directly` ignored test).
- The old user_index / individual_user_template fleet upgrade paths have been removed. The PO is the only canister we upgrade directly.

**DFX mainnet plaintext identity warning suppression:**

When the `actions_identity.pem` (plaintext storage) is the selected/imported identity and dfx targets `--network=ic`, dfx refuses with "The actions identity is not stored securely."

All `Command::new("dfx")` (and the bash wrapper for candid regen) inside the task_runner mainnet tests (`direct_upgrade.rs`, `cycle_harvest.rs`, `reinstall_po.rs`, etc.) now automatically inject:

```rust
.env("DFX_WARNING", "-mainnet_plaintext_identity")
```

This is the canonical place to handle it — no need to export in the shell when running the cargo tests. If you ever invoke raw `dfx ... --network=ic` manually with the actions identity, export `DFX_WARNING=-mainnet_plaintext_identity` in that shell first.

Deployment / upgrade sequence (PO and fleets):

1. Paste the controller PEM into `actions_identity.pem`.
2. For a PO-only upgrade (the common case now):
   ```sh
   cargo test -p task_runner -- --ignored upgrade_po_directly --nocapture
   ```
   This does `dfx build platform_orchestrator --network=ic` followed by `dfx canister install platform_orchestrator --mode=upgrade --network ic ...` using the actions identity (which is a controller).

3. The test will print module hashes before/after and the new version. Confirm via `dfx canister info platform_orchestrator --network ic` (module hash changed) and/or calling `get_version` on the live PO.

Cycle reclaim / harvest (after a successful PO upgrade that includes the harvest endpoints):

- To harvest a specific canister (e.g. the singular one that failed a prior run because the PO lacked the methods):
  ```sh
  HARVEST_CANISTER_ID=z7bpd-waaaa-aaaag-acogq-cai \
    cargo test -p task_runner -- --ignored harvest_single_canister --nocapture
  ```
- For a batch (resumable, uses the DB tracking tables):
  ```sh
  cargo test -p task_runner -- --ignored harvest_cycles_batch --nocapture
  ```

Legacy paths (do not use for routine PO work):
- `bash scripts/release-and-submit-proposals.sh` (SNS proposal submission for PO) — retired.
- The 3-step deregister/reinstall/reregister or pure SNS reinstall tests — were used to acquire controller status; now historical.

After any mainnet PO change, re-run the singular-canister reclaim for any previously failing canister (with the `HARVEST_CANISTER_ID` override) and confirm it passes before proceeding with other work.
5. On passing, `platform_orchestrator` distributes and installs the new wasms fleet-wide.

Verify after deployment:
- `dfx canister info <canister-id> --network=ic` — `Module hash` must match the hash printed by the script.
- Canister IDs: `canister_ids.json`.

## Agent Behavior Rules

- Always check `AGENTS.md` and the ignored tests under `src/lib/task_runner/src/tests/` first for the current workflow. The task_runner tests are the executable, versioned source of truth for PO lifecycle, direct upgrades, local setup, and cycle harvest/reclaim.
- The scripts under `scripts/` are now secondary (utility / reference only for deployment paths). Do not treat `release-and-submit-proposals.sh`, `deploy-local.sh`, or `upgrade_ic_repl.sh` as the canonical deployment mechanism for platform_orchestrator.
- Avoid making arbitrary changes to canister deployment or upgrade behavior without explicit evidence from repo docs or tests (prefer the task_runner tests over ad-hoc dfx or bash).
- If a new process is introduced, document it here and keep the language prescriptive.
- Keep agent edits minimal when updating workflows: update the relevant task_runner test (and its module-level docs), then update `AGENTS.md`.

## Canister List (principal.csv)

The list of PO-controlled canisters for cycle harvesting lives in
`src/canister/platform_orchestrator/principal.csv` (one principal per line, with
a `principal` header row). The `task_runner` crate reads it via
`canister_list::read_po_controlled_canisters`. No SQLite database or sqlx is
used — the CSV is the single source of truth.

## When to Update This File

Update `AGENTS.md` whenever any of the following change:

- The canonical workflow moves (e.g., from a bash script in `scripts/` to an ignored `cargo test` in `task_runner`, or vice versa).
- A task_runner ignored test (local_po_setup, direct_upgrade, cycle_harvest, etc.) changes name, behavior, or becomes the new preferred entry point.
- The controller model for platform_orchestrator changes (SNS-governed vs. direct controller) — this affects which upgrade path is used.
- The repository adds or removes a major canister or canister manifest file.
- The local reproducibility workflow or DB safety contract changes.
- A new high-level engineering convention appears that future agents must know (e.g., "all PO lifecycle lives in task_runner").

When updating, keep it terse and current. Remove or clearly mark obsolete patterns (e.g., retired SNS-for-PO or script-based deploy flows) immediately.

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
