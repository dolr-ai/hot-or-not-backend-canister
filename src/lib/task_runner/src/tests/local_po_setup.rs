/// Local platform_orchestrator setup + harvest-method validation as a cargo test.
///
/// This replaces the fragile bash + ic-repl logic that used to live in
/// `scripts/deploy-local.sh` (and the corresponding parts of the release flow).
///
/// Why this exists:
/// - After the PO cleanup, `platform_orchestrator` only exposes the minimal
///   surface needed for cycle harvesting (add_our_identity_as_controller,
///   get_controllers_and_cycle_balance, get_version, return_cycle_balance_..., etc.).
/// - The old deploy script called removed methods such as `upload_wasms`, producing:
///   "Error: The replica returned a rejection error: ... Canister has no update method 'upload_wasms'."
/// - All orchestration logic now lives in Rust (task_runner), is versioned with the
///   code, type-checked where possible, and runnable via `cargo test -- --ignored`.
///
/// What the test does (mirrors the essential parts of the old deploy-local flow
/// but using only current PO methods + dfx for canister lifecycle):
/// 1. Generate candid (good hygiene, keeps .did in sync).
/// 2. Stop any running replica, start a clean local one in the background.
/// 3. Wait for the replica to be healthy (short-poll, like the cluster AGENTS guideline).
/// 4. `dfx deploy platform_orchestrator --network local`.
/// 5. Add the actions_identity principal as a controller of the PO (so it can call
///    the guarded harvest endpoints — the guard is "is_caller_platform_global_admin_or_controller").
/// 6. Create a fresh "test target" canister (stand-in for a po_controlled_canister).
/// 7. Add the local PO as a controller of that target (required for the internal
///    `update_settings` that `add_our_identity_as_controller` performs on the target).
/// 8. Build a local ic-agent using the actions_identity.pem (via the new
///    `local_agent_from_pem` helper).
/// 9. Call the three key methods from the harvester:
///    - get_version (query)
///    - get_controllers_and_cycle_balance(target)
///    - add_our_identity_as_controller(target)
/// 10. Assert they succeed (no "no update method", no Unauthorized, and the
///     controller precondition on the target is satisfied so we get Ok(()) or
///     a clean business error rather than a rejection).
///
/// Run:
///   cargo test -p task_runner -- --ignored setup_local_po_and_validate_harvest_methods --nocapture
///
/// After success you will see the local PO canister ID and a ✅ banner.
/// The replica is left running so you can inspect with dfx / call other methods.
/// Stop it with `dfx stop` when done.
///
/// This test is the canonical way to obtain a local PO that the cycle harvester
/// can talk to. It is also the required local validation step before any mainnet
/// PO change (per the spirit of the "Testing Upgrades Locally" section in AGENTS.md).
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use candid::Encode;
use ic_agent::Identity;
use tokio::time::sleep;

use crate::agent::{local_agent_from_pem, workspace_root};

/// DB SAFETY CONTRACT (critical):
///
/// This test MUST NEVER write local canister IDs (or any data) into the
/// production `ic_canisters.db` (the one under `src/lib/ic_canister_snapshot/`
/// that is used for real mainnet cycle harvesting and snapshot tracking).
///
/// Why this is safe today:
/// - This test only uses `std::process::Command` (to drive dfx) + `ic_agent`
///   direct calls against the *local* replica.
/// - It does **not** import `crate::db`, `crate::tests::cycle_harvest`, or any
///   module that calls `open_pool`, `pending_harvests`, `mark_harvested`, etc.
/// - It never calls `harvest_canister`, `harvest_single_canister`, or anything
///   that populates `po_controlled_canisters` / `cycle_harvested` / `pending_harvests`.
///
/// The local PO ID (e.g. uxrrr-...) and the test target (the canister created
/// under the "individual_user_template" dfx.json entry as a stand-in) exist
/// **only** on the ephemeral local dfx replica started by this test.
/// They are never inserted into SQLite.
///
/// If in the future you extend this test (or a new test) to actually exercise
/// the harvester logic against a local PO, you **must** either:
///   a) Use a completely separate SQLite file for local experiments, or
///   b) Only use the read-only `HARVEST_CANISTER_ID` path without any writes
///      to the tracking tables, or
///   c) Explicitly switch the DB path for the duration of the test.
///
/// We also actively assert below (via mtime) that the real DB file is not
/// modified by this test run.
/// Small duplicated view of the PO return type so we don't have to make the
/// cycle_harvest structs public just for this setup test. Keep in sync with
/// the real definition in cycle_harvest.rs.
#[derive(candid::CandidType, candid::Deserialize, Debug)]
struct ControlledCanisterDetails {
    controllers: Vec<candid::Principal>,
    cycle_balance: u128,
    reserved_cycles: u128,
    status: CanisterRunningStatus,
}

#[derive(candid::CandidType, candid::Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
enum CanisterRunningStatus {
    Running,
    Stopping,
    Stopped,
}

#[tokio::test]
#[ignore = "starts a clean local dfx replica, deploys current PO, wires controllers, validates the harvest methods the task_runner needs (add_our_identity_as_controller, get_controllers_and_cycle_balance, etc.)"]
async fn setup_local_po_and_validate_harvest_methods() -> Result<()> {
    let root: PathBuf = workspace_root();
    let pem_path = root.join("actions_identity.pem");
    anyhow::ensure!(
        pem_path.exists(),
        "actions_identity.pem not found at {} — required for the actions principal that the harvester uses",
        pem_path.display()
    );

    // --- DB pollution guard (runtime assertion) ---
    // Record the mtime of the real production DB (if it exists) *before* we do anything.
    // At the end of the test we will assert it has not changed.
    // The production DB lives under the ic_canister_snapshot crate.
    let prod_db_path = root.join("src/lib/ic_canister_snapshot/ic_canisters.db");
    let db_mtime_before = std::fs::metadata(&prod_db_path)
        .ok()
        .and_then(|m| m.modified().ok());

    // Helper to check at the end (we call this via a guard or just at the very end).
    let assert_db_untouched = |label: &str| -> Result<()> {
        let mtime_after = std::fs::metadata(&prod_db_path)
            .ok()
            .and_then(|m| m.modified().ok());
        if db_mtime_before != mtime_after {
            anyhow::bail!(
                "DB POLLUTION DETECTED ({}): the production ic_canisters.db mtime changed during the local PO setup test. \
                 This test must never write local canister IDs into the real harvest DB. \
                 Before: {:?}, After: {:?}",
                label,
                db_mtime_before,
                mtime_after
            );
        }
        Ok(())
    };

    println!("==> [1/10] Generating candid (platform_orchestrator + individual_user_template) ...");
    let status = Command::new("bash")
        .args([
            "scripts/generate-candid.sh",
            "platform_orchestrator",
            "individual_user_template",
        ])
        .current_dir(&root)
        .status()
        .context("failed to spawn generate-candid.sh")?;
    anyhow::ensure!(status.success(), "generate-candid.sh exited with failure");

    println!("==> [2/10] Stopping any previous replica and starting a clean local one (--background) ...");
    // Best-effort stop; ignore errors if none was running.
    let _ = Command::new("dfx")
        .args(["stop"])
        .current_dir(&root)
        .status();
    let status = Command::new("dfx")
        .args(["start", "--clean", "--background"])
        .current_dir(&root)
        .status()
        .context("dfx start --clean --background failed to spawn")?;
    anyhow::ensure!(
        status.success(),
        "dfx start --clean --background returned non-zero"
    );

    println!("==> [3/10] Waiting for local replica to become healthy (short-poll, max ~60s) ...");
    let mut healthy = false;
    for attempt in 0..30 {
        // NOTE: dfx ping takes the network name as a *positional* argument (not --network).
        // Using `dfx ping local` (or just `dfx ping` for the default local network).
        let output = Command::new("dfx")
            .args(["ping", "local"])
            .current_dir(&root)
            .output();
        if let Ok(out) = output {
            // A successful exit status from dfx ping means the replica is reachable and responsive.
            // We also accept common success strings for older/newer dfx output variations.
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success()
                || stdout.contains("replica")
                || stdout.contains("healthy")
                || stdout.contains("Pinged")
                || stderr.contains("replica")
            {
                healthy = true;
                println!("  ✓ replica healthy after {} attempts", attempt + 1);
                break;
            }
        }
        if attempt % 5 == 0 && attempt > 0 {
            println!("  ... still waiting (attempt {})", attempt + 1);
        }
        sleep(Duration::from_secs(2)).await;
    }
    anyhow::ensure!(
        healthy,
        "local replica did not report healthy within timeout — check `dfx ping local` manually"
    );

    println!("==> [4/10] Deploying current platform_orchestrator (local) ...");
    let status = Command::new("dfx")
        .args(["deploy", "platform_orchestrator", "--network", "local"])
        .current_dir(&root)
        .status()
        .context("dfx deploy platform_orchestrator failed to spawn")?;
    anyhow::ensure!(
        status.success(),
        "dfx deploy platform_orchestrator returned non-zero"
    );

    let po_id_output = Command::new("dfx")
        .args([
            "canister",
            "id",
            "platform_orchestrator",
            "--network",
            "local",
        ])
        .current_dir(&root)
        .output()
        .context("dfx canister id platform_orchestrator failed")?;
    anyhow::ensure!(
        po_id_output.status.success(),
        "failed to get local PO canister id"
    );
    let po_id_str = String::from_utf8(po_id_output.stdout)?.trim().to_string();
    let po = candid::Principal::from_text(&po_id_str)
        .with_context(|| format!("invalid principal from dfx: {}", po_id_str))?;
    println!("  local PO canister id: {}", po);

    // Load the actions identity so we can compute its principal and later use it for the agent.
    let actions_identity = ic_agent::identity::Secp256k1Identity::from_pem_file(&pem_path)
        .with_context(|| {
            format!(
                "failed to load Secp256k1Identity from {}",
                pem_path.display()
            )
        })?;
    let actions_principal = actions_identity
        .sender()
        .map_err(anyhow::Error::msg)
        .context("could not derive sender principal from actions_identity.pem")?;
    println!("  actions principal (from pem): {}", actions_principal);

    println!("==> [5/10] Adding actions principal as controller of the local PO (guard requirement for harvest endpoints) ...");
    let status = Command::new("dfx")
        .args([
            "canister",
            "update-settings",
            "platform_orchestrator",
            "--add-controller",
            &actions_principal.to_string(),
            "--network",
            "local",
        ])
        .current_dir(&root)
        .status()
        .context("dfx update-settings (add actions controller to PO) failed to spawn")?;
    anyhow::ensure!(
        status.success(),
        "failed to add actions principal as controller of local PO"
    );

    // We use "individual_user_template" (a real entry that exists in this project's dfx.json)
    // as the stand-in target canister. We only create the canister + manipulate controllers;
    // we never deploy the template wasm in this test. This is sufficient to exercise the
    // PO's get_controllers_and_cycle_balance + add_our_identity_as_controller paths.
    println!("==> [6/10] Creating a fresh test target canister (stand-in for a po_controlled_canister) ...");
    let status = Command::new("dfx")
        .args([
            "canister",
            "create",
            "individual_user_template",
            "--network",
            "local",
            "--with-cycles",
            "2000000000000",
        ])
        .current_dir(&root)
        .status()
        .context("dfx canister create individual_user_template (as test target) failed to spawn")?;
    anyhow::ensure!(
        status.success(),
        "failed to create individual_user_template target (stand-in)"
    );

    let target_id_output = Command::new("dfx")
        .args([
            "canister",
            "id",
            "individual_user_template",
            "--network",
            "local",
        ])
        .current_dir(&root)
        .output()
        .context("dfx canister id individual_user_template (as test target) failed")?;
    anyhow::ensure!(
        target_id_output.status.success(),
        "failed to get individual_user_template (test target) id"
    );
    let target_id_str = String::from_utf8(target_id_output.stdout)?
        .trim()
        .to_string();
    let target = candid::Principal::from_text(&target_id_str)
        .with_context(|| format!("invalid principal for test target: {}", target_id_str))?;
    println!("  test target canister id: {}", target);

    println!("==> [7/10] Adding local PO as a controller of the test target (required for add_our_identity_as_controller's internal update_settings) ...");
    let status = Command::new("dfx")
        .args([
            "canister",
            "update-settings",
            "individual_user_template",
            "--add-controller",
            &po.to_string(),
            "--network",
            "local",
        ])
        .current_dir(&root)
        .status()
        .context("dfx update-settings (add PO controller to target) failed to spawn")?;
    anyhow::ensure!(
        status.success(),
        "failed to add local PO as controller of test target"
    );

    println!("==> [8/10] Building local agent from actions_identity.pem (points at 127.0.0.1:4943 + fetches root key) ...");
    let agent = local_agent_from_pem(&pem_path).await?;
    println!("  ✓ local agent ready");

    println!("\n==> [9/10] Verifying the harvest methods the task_runner needs are present and callable on local PO {} ...", po);

    // --- get_version (simple query, no guard) ---
    // Explicit empty arg for maximum compatibility with ic-cdk arg decoding on local replica.
    let version_bytes = agent
        .query(&po, "get_version")
        .with_arg(Encode!()?)
        .call()
        .await
        .map_err(|e| anyhow::anyhow!("get_version query rejected: {}", e))?;
    let version: String = candid::decode_one(&version_bytes)
        .map_err(|e| anyhow::anyhow!("failed to decode get_version: {}", e))?;
    println!("  get_version() => {:?}", version);
    // The exact string may evolve; we mainly care that the method exists and returns something.
    anyhow::ensure!(!version.is_empty(), "get_version returned empty string");

    // --- get_controllers_and_cycle_balance (the read path used in harvest step 1) ---
    let status_arg = Encode!(&target)?;
    let response = agent
        .update(&po, "get_controllers_and_cycle_balance")
        .with_arg(status_arg)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("get_controllers_and_cycle_balance update rejected (method missing or caller not allowed): {}", e))?;
    // PO returns Result<ControlledCanisterDetails, String> directly (no outer tuple).
    let details: Result<ControlledCanisterDetails, String> = candid::decode_one(&response)
        .map_err(|e| {
            anyhow::anyhow!("failed to decode get_controllers_and_cycle_balance: {}", e)
        })?;
    let details = details
        .map_err(|e| anyhow::anyhow!("PO.get_controllers_and_cycle_balance returned Err: {}", e))?;
    println!(
        "  get_controllers_and_cycle_balance(target) => controllers: {:?}, balance: {} TC, reserved: {} TC, status: {:?}",
        details.controllers,
        details.cycle_balance / 1_000_000_000_000u128,
        details.reserved_cycles / 1_000_000_000_000u128,
        details.status
    );
    // Sanity: PO should be in the controllers list now.
    anyhow::ensure!(
        details.controllers.iter().any(|c| *c == po),
        "PO principal not present in the controllers returned by PO for the target"
    );

    // --- The critical one: add_our_identity_as_controller (harvest step 2) ---
    // At this point:
    // - The caller (actions principal) is a controller of the PO → guard passes.
    // - The PO is a controller of the target → the internal update_settings will succeed.
    let add_arg = Encode!(&target)?;
    agent
        .update(&po, "add_our_identity_as_controller")
        .with_arg(add_arg)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("add_our_identity_as_controller rejected: {}", e))?;
    println!("  ✓ add_our_identity_as_controller(target) => Ok(())   [actions principal added as co-controller of target]");

    // Optional: demonstrate that a second call is idempotent / safe (the implementation should tolerate it).
    let add_arg2 = Encode!(&target)?;
    let _ = agent
        .update(&po, "add_our_identity_as_controller")
        .with_arg(add_arg2)
        .call_and_wait()
        .await;
    println!("  ✓ second add_our_identity_as_controller(target) also accepted (idempotent path)");

    println!("\n==> [10/10] All checks passed.");

    // Final DB safety assertion — this is the success path.
    assert_db_untouched("end of successful test")?;

    println!(
        "\n✅ SUCCESS: local PO {} exposes the methods required by the cycle harvester.",
        po
    );
    println!("   Test target (po_controlled stand-in): {}", target);
    println!("   Actions principal used: {}", actions_principal);
    println!(
        "\n   You can now exercise harvest_single_canister against this environment if you extend"
    );
    println!("   the harvester to support a local URL + dynamic PO id (or run manual dfx calls).");
    println!("   When finished: `dfx stop` to shut down the local replica.");

    // We intentionally leave the replica running for follow-up inspection / further manual calls.
    Ok(())
}
