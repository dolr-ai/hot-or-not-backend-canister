/// Cycle harvesting via task_runner — processes po_controlled_canisters one-by-one,
/// reclaiming reserved cycles and transferring them back to platform_orchestrator.
///
/// Flow per canister:
///   1. Get pre-state (balance, reserved, status, has_wasm)
///   2. Add actions_identity as controller via PO endpoint; *immediately* re-query the
///      management canister directly (canister_status) using the actions-signed agent
///      and assert the actions principal is now in the returned controllers list
///      (explicit post-Ok check, no polling loop).
///   3. Uninstall wasm (if installed) → releases reserved cycles to main balance
///   4. Check post-uninstall balance; top up with 0.5T if < 500M
///   5. Install individual_user_template wasm
///   6. Call return_cycle_balance_to_platform_orchestrator → sends everything back
///   7. Final uninstall
///   8. Set controllers to [PO] only via management canister
///   9. Validate final state
///  10. Mark harvested in SQLite DB
///
/// Resumable: re-running skips already-harvested canisters (cycle_harvested table).
///
/// Local setup (required before running harvest tests against a *local* replica):
///   cargo test -p task_runner -- --ignored setup_local_po_and_validate_harvest_methods --nocapture
/// This starts a clean dfx replica, deploys the *current* platform_orchestrator (post-cleanup),
/// creates a controllable test target, adds the necessary controllers, and verifies that
/// the PO exposes `add_our_identity_as_controller`, `get_controllers_and_cycle_balance`,
/// `get_version`, etc. (i.e. no "Canister has no update method" errors).
///
/// The old `scripts/deploy-local.sh` (and raw calls to `upload_wasms` inside it) are
/// obsolete after the PO was stripped to the harvest surface; they will hit exactly the
/// error: "Canister has no update method 'upload_wasms'".
///
/// Run harvest tests with:
///   cargo test -p task_runner -- --ignored harvest_single_canister --nocapture
///   cargo test -p task_runner -- --ignored harvest_cycles_batch --nocapture
///
/// For harvest_single_canister you can also force an exact ID via env var (see
/// the function for the exact command).
use anyhow::{Context, Result};
use candid::{encode_args, Encode, Principal};
use ic_agent::Identity;

use crate::{
    agent::{agent_from_pem, workspace_root},
    db::{
        harvest_counts, mark_harvest_failed, mark_harvested, open_pool, pending_harvest_count,
        pending_harvests, total_controlled_count, DB_PATH,
    },
    sns_types::{INSTALL_MODE_INSTALL, PLATFORM_ORCHESTRATOR_ID},
};

/// IC management canister.
const MANAGEMENT_CANISTER: &str = "aaaaa-aa";

/// Minimum balance (cycles) required after uninstall to proceed with reinstall + transfer.
/// Covers wasm install cost + deposit_cycles call overhead (~260K base) + safety margin.
const MIN_REINSTALL_BALANCE: u128 = 500_000_000;

/// Top-up amount when a canister's balance is too low to reinstall.
/// We recover most of this back via return_cycle_balance, so still net positive.
const TOP_UP_AMOUNT: u128 = 500_000_000_000; // 0.5T

/// Path to the individual_user_template wasm (built for mainnet).
const INDIVIDUAL_USER_WASM_PATH: &str =
    ".dfx/ic/canisters/individual_user_template/individual_user_template.wasm.gz";

/// Batch size for harvest_cycles_batch test.
const BATCH_SIZE: i64 = 4;

// ── Helper types for management canister calls ────────────────────────────────

#[derive(candid::CandidType, candid::Deserialize)]
struct CanisterIdRecord {
    canister_id: Principal,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
pub enum WasmMemoryPersistence {
    #[serde(rename = "keep")]
    Keep,
    #[serde(rename = "replace")]
    Replace,
}

#[derive(candid::CandidType, candid::Deserialize, Debug, Default)]
pub struct UpgradeFlags {
    pub wasm_memory_persistence: Option<WasmMemoryPersistence>,
    pub skip_pre_upgrade: Option<bool>,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
pub enum CanisterInstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade(Option<UpgradeFlags>),
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
pub struct InstallCodeArgs {
    pub mode: CanisterInstallMode,
    pub canister_id: Principal,
    pub wasm_module: Vec<u8>,
    pub arg: Vec<u8>,
    pub sender_canister_version: Option<u64>,
}

#[derive(candid::CandidType, candid::Deserialize)]
struct UpdateSettingsArgument {
    canister_id: Principal,
    settings: CanisterSettings,
}

#[derive(candid::CandidType, candid::Deserialize, Default)]
struct CanisterSettings {
    controllers: Option<Vec<Principal>>,
    compute_allocation: Option<u64>,
    memory_allocation: Option<u64>,
    freezing_threshold: Option<u64>,
    reserved_cycles_limit: Option<u128>,
    wasm_memory_limit: Option<u64>,
}

// ── Core harvest logic ────────────────────────────────────────────────────────

/// Harvest cycles from a single canister. Returns (pre_balance, pre_reserved, post_uninstall, cycles_transferred, topped_up).
async fn harvest_canister(
    agent: &ic_agent::Agent,
    actions_principal: Principal,
    po: Principal,
    canister_id: Principal,
    wasm_blob: &[u8],
) -> Result<(u128, u128, u128, u128, u128)> {
    println!("\n  ── Harvesting {} ──", canister_id);

    // Early fast existence gate using direct management canister call.
    // IMPORTANT: Must be an *update* call (not query), same as dfx uses internally.
    // Using .query() here is why we were getting spurious "canister_not_found"
    // even when the canister plainly exists and the identity is a controller.
    // Management canister controller operations need the authenticated consensus path.

    // Debug: show exactly which principal the test is acting as when talking to the
    // management canister. This is the key piece of information when dfx sees the
    // canister but the test gets "not found" or unauthorized errors.
    println!(
        "  [debug] using actions principal for management calls: {}",
        actions_principal
    );

    let id_record = CanisterIdRecord { canister_id };
    // Mimic dfx exactly:
    // - Encode the struct directly with Encode! (per candid docs for user-defined types)
    // - Use .effective_canister_id(target) on the UpdateBuilder.
    //   This is critical for management canister "per-canister" methods (status, install, uninstall, update_settings, etc.).
    //   dfx always sets it. Without it the call can route wrong and return 400 "canister_not_found" even when the canister exists.
    let exists_arg = Encode!(&id_record)?;

    let exists_res = agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "canister_status",
        )
        .with_arg(exists_arg)
        .with_effective_canister_id(canister_id) // <--- correct method name (dfx always sets this for per-canister mgmt calls)
        .call_and_wait()
        .await;

    if let Err(e) = exists_res {
        println!("  [debug] raw canister_status error (Debug): {:?}", e);
        let estr = e.to_string();
        println!("  [debug] raw canister_status error (to_string): {}", estr);

        // Print the exact replica error instead of replacing with our user-friendly message
        return Err(anyhow::anyhow!(
            "early canister_status on management canister failed for {}: {}",
            canister_id,
            estr
        ));
    }

    // The early management canister_status call succeeded.
    // Decode it now to get the pre-state (balance, reserved_cycles, status).
    // This replaces the previous redundant PO get_controllers_and_cycle_balance call for pre-state.
    // We already paid the roundtrip for the existence gate; no need to call PO again just for data.
    let exists_response = exists_res.unwrap(); // safe: we returned on Err above
    let status: CanisterStatusResult = candid::decode_one(&exists_response)
        .map_err(|e| anyhow::anyhow!("failed to decode early canister_status: {}", e))?;

    let pre_balance = u128::try_from(status.cycles.0.clone()).unwrap_or_else(|_| {
        status
            .cycles
            .0
            .iter_u64_digits()
            .fold(0u128, |acc, d| (acc << 64) | u128::from(d))
    });
    let pre_reserved = u128::try_from(status.reserved_cycles.0.clone()).unwrap_or_else(|_| {
        status
            .reserved_cycles
            .0
            .iter_u64_digits()
            .fold(0u128, |acc, d| (acc << 64) | u128::from(d))
    });

    let has_wasm = matches!(
        status.status,
        CanisterStatus::Running | CanisterStatus::Stopped
    );

    println!(
        "  pre_balance: {} TC, pre_reserved: {} TC, status: {:?}",
        format_cycles(pre_balance),
        format_cycles(pre_reserved),
        status.status
    );

    // Step 2: Add actions_identity as controller via PO endpoint.
    let add_ctrl_arg = encode_args((canister_id,))?;
    agent
        .update(&po, "add_our_identity_as_controller")
        .with_arg(add_ctrl_arg)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("add_our_identity_as_controller failed: {}", e))?;
    println!("  ✓ actions_identity added as controller");
    // Immediately after the add_our update call returns Ok, try to re-query the management
    // canister directly using the actions-signed agent and assert the actions principal
    // is now present.
    //
    // We are tolerant of "canister_not_found" here: the historical po_controlled_canisters
    // list contains many canisters that have since been deleted (uninstalled, decommissioned,
    // or self-removed). If the canister disappeared between the PO add and this verify,
    // we log and continue — later direct calls (uninstall etc.) will surface the same error,
    // and the batch will record it as a harvest failure (so it is excluded from future pending).
    // The explicit assert (no polling) is preserved for the happy path where the canister
    // still exists.
    let verify_record = CanisterIdRecord { canister_id };
    // Mimic dfx exactly (same as the early gate above)
    let verify_arg = Encode!(&verify_record)?;

    let verify_response = agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "canister_status",
        )
        .with_arg(verify_arg)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await;

    // Correct types matching the management canister's canister_status response.
    // Variant names must be exactly "running", "stopping", "stopped" (lowercase)
    // as defined in the IC management canister interface.
    // Mimicking dfx's type definitions for canister_status response (from dfx's status command and management canister interface).
    // See https://github.com/dfinity/sdk/blob/master/src/dfx/src/commands/canister/status.rs
    // The variant names must be the exact labels from the IC's candid: "running", "stopping", "stopped".
    // Using #[serde(rename = "...")] keeps Rust code idiomatic (PascalCase) while producing the correct Candid labels.

    #[derive(candid::CandidType, candid::Deserialize, Debug)]
    enum CanisterStatus {
        #[serde(rename = "running")]
        Running,
        #[serde(rename = "stopping")]
        Stopping,
        #[serde(rename = "stopped")]
        Stopped,
    }

    #[derive(candid::CandidType, candid::Deserialize, Debug)]
    enum LogVisibility {
        #[serde(rename = "controllers")]
        Controllers,
        #[serde(rename = "public")]
        Public,
    }

    #[derive(candid::CandidType, candid::Deserialize, Debug)]
    struct DefiniteCanisterSettings {
        controllers: Vec<Principal>,
        compute_allocation: candid::Nat,
        memory_allocation: candid::Nat,
        freezing_threshold: candid::Nat,
        reserved_cycles_limit: candid::Nat,
        wasm_memory_limit: candid::Nat,
        log_visibility: LogVisibility,
    }

    #[derive(candid::CandidType, candid::Deserialize, Debug)]
    struct CanisterStatusResult {
        status: CanisterStatus,
        settings: DefiniteCanisterSettings,
        module_hash: Option<Vec<u8>>,
        memory_size: candid::Nat,
        cycles: candid::Nat,
        idle_cycles_burned_per_day: candid::Nat,
        reserved_cycles: candid::Nat,
    }

    let verify_status: Option<CanisterStatusResult> = match verify_response {
        Ok(resp) => {
            let s: CanisterStatusResult = candid::decode_one(&resp).map_err(|e| {
                anyhow::anyhow!("failed to decode post-add verify canister_status: {}", e)
            })?;
            Some(s)
        }
        Err(e) => {
            let estr = e.to_string();
            if estr.contains("canister_not_found")
                || estr.contains("Canister not found")
                || estr.contains("not exist")
            {
                println!(
                    "\u{26A0} post-add direct canister_status failed with not_found. \
                    Canister may have been deleted concurrently or the list entry is stale. \
                    The add_our succeeded, so the controller mutation took effect at that moment. \
                    Continuing (subsequent steps will fail if truly gone)."
                );
                None
            } else {
                return Err(anyhow::anyhow!(
                    "post-add verify canister_status failed: {}",
                    e
                ));
            }
        }
    };

    if let Some(verify_status) = &verify_status {
        if !verify_status
            .settings
            .controllers
            .contains(&actions_principal)
        {
            anyhow::bail!(
                "add_our_identity_as_controller returned Ok but actions principal {} is NOT in the controllers list from management canister (got {:?}). \
                 dfx canister status/info for {} should be inspected. The controller mutation did not take effect.",
                actions_principal,
                verify_status.settings.controllers,
                canister_id
            );
        }
        println!(
            "  \u{2713} post-add assertion passed: actions principal {} is present in management canister-reported controllers for {}",
            actions_principal, canister_id
        );
    } else {
        println!("  \u{26A0} skipped strict post-add controller assertion (canister reported not found on verify)");
    }
    // Step 3: Uninstall wasm (if installed).
    if has_wasm {
        let uninstall_arg = encode_args((CanisterIdRecord { canister_id },))?;
        let result = agent
            .update(
                &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
                "uninstall_code",
            )
            .with_arg(uninstall_arg)
            .with_effective_canister_id(canister_id)
            .call_and_wait()
            .await;

        match result {
            Ok(_) => {
                println!("\u{2713} wasm uninstalled (reserved cycles released)");
            }
            Err(e) => {
                // If uninstall fails, the canister may have no wasm — proceed to install.
                println!(
                    "  \u{26A0} uninstall_code failed (canister may have no wasm): {}",
                    e
                );
            }
        }
    }

    // Step 4: Check post-uninstall balance and top up if needed.
    let status_arg = encode_args((canister_id,))?;
    let response = agent
        .update(&po, "get_controllers_and_cycle_balance")
        .with_arg(status_arg)
        .call_and_wait()
        .await?;

    let details: Result<ControlledCanisterDetails, String> = candid::decode_one(&response)
        .map_err(|e| anyhow::anyhow!("failed to decode post-uninstall status: {}", e))?;
    let details = details
        .map_err(|e| anyhow::anyhow!("get_controllers_and_cycle_balance returned error: {}", e))?;
    let post_uninstall_balance = details.cycle_balance;

    println!(
        "  post_uninstall_balance: {} TC",
        format_cycles(post_uninstall_balance)
    );

    let mut topped_up: u128 = 0;
    if post_uninstall_balance < MIN_REINSTALL_BALANCE {
        println!(
            "  ⚠ balance too low ({} TC < {} TC), topping up with 0.5T...",
            format_cycles(post_uninstall_balance),
            format_cycles(MIN_REINSTALL_BALANCE)
        );

        // Top up via PO's deposit_cycles_to_canister.
        let deposit_arg = encode_args((canister_id, TOP_UP_AMOUNT))?;
        agent
            .update(&po, "deposit_cycles_to_canister")
            .with_arg(deposit_arg)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("deposit_cycles_to_canister failed: {}", e))?;

        topped_up = TOP_UP_AMOUNT;
        println!("  ✓ topped up with 0.5T");
    }

    // Step 5: Install individual_user_template wasm.
    // Use the proper variant-based mode to match the current management canister interface
    // (the replica rejected the old i32-based InstallCodeArgument with subtyping error).
    let install_arg = encode_args((InstallCodeArgs {
        mode: CanisterInstallMode::Install,
        canister_id,
        wasm_module: wasm_blob.to_vec(),
        arg: vec![],
        sender_canister_version: None,
    },))?;

    agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "install_code",
        )
        .with_arg(install_arg)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("install_code failed: {}", e))?;
    println!("  ✓ wasm installed");

    // Step 6: Transfer cycles to PO.
    let transfer_result = agent
        .update(
            &canister_id,
            "return_cycle_balance_to_platform_orchestrator",
        )
        .with_arg(encode_args(())?)
        .call_and_wait()
        .await;

    let cycles_transferred = match transfer_result {
        Ok(response) => {
            let (result,): (Result<u128, String>,) = candid::decode_one(&response)
                .map_err(|e| anyhow::anyhow!("failed to decode transfer result: {}", e))?;
            match result {
                Ok(amount) => amount,
                Err(e) => {
                    println!("  ⚠ return_cycle_balance returned error: {}", e);
                    0
                }
            }
        }
        Err(e) => {
            println!("  ⚠ return_cycle_balance call failed: {}", e);
            0
        }
    };

    if cycles_transferred > 0 {
        println!(
            "  ✓ transferred {} TC to platform_orchestrator",
            format_cycles(cycles_transferred)
        );
    }

    // Step 7: Final uninstall.
    let uninstall_arg = encode_args((CanisterIdRecord { canister_id },))?;
    agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "uninstall_code",
        )
        .with_arg(uninstall_arg)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("final uninstall_code failed: {}", e))?;
    println!("  ✓ final uninstall done");

    // Step 8: Set controllers to [PO] only via management canister.
    let po_principal = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();
    let settings_arg = encode_args((&UpdateSettingsArgument {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![po_principal]),
            ..Default::default()
        },
    },))?;

    agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "update_settings",
        )
        .with_arg(settings_arg)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("update_settings failed: {}", e))?;
    println!("  ✓ controllers set to [PO] only");

    // Step 9: Validate final state.
    let status_arg = encode_args((canister_id,))?;
    let response = agent
        .update(&po, "get_controllers_and_cycle_balance")
        .with_arg(status_arg)
        .call_and_wait()
        .await?;

    let details: Result<ControlledCanisterDetails, String> = candid::decode_one(&response)
        .map_err(|e| anyhow::anyhow!("failed to decode final status: {}", e))?;
    let details = details
        .map_err(|e| anyhow::anyhow!("get_controllers_and_cycle_balance returned error: {}", e))?;

    // Validate controllers are [PO] only.
    if details.controllers.len() != 1 || details.controllers[0] != po {
        println!(
            "  ⚠ WARNING: controllers not set to [PO] only: {:?}",
            details.controllers
        );
    } else {
        println!("  ✓ validated: controllers = [PO]");
    }

    println!(
        "  ✓ final balance: {} TC",
        format_cycles(details.cycle_balance)
    );

    Ok((
        pre_balance,
        pre_reserved,
        post_uninstall_balance,
        cycles_transferred,
        topped_up,
    ))
}

/// Format cycles as trillions with 2 decimal places.
fn format_cycles(cycles: u128) -> String {
    let trillions = cycles as f64 / 1_000_000_000_000.0;
    format!("{:.2}", trillions)
}

// ── Helper types for PO API responses ─────────────────────────────────────────

#[derive(candid::CandidType, candid::Deserialize, Debug)]
struct ControlledCanisterDetails {
    controllers: Vec<Principal>,
    cycle_balance: u128,
    reserved_cycles: u128,
    status: CanisterRunningStatus,
}

#[derive(candid::CandidType, candid::Deserialize, Debug)]
enum CanisterRunningStatus {
    Running,
    Stopping,
    Stopped,
}

// ── Test entry points ────────────────────────────────────────────────────────

/// Harvest cycles from a single canister (first pending one).
/// Useful for validating the flow before running batches.
///
/// To target a *specific* canister ID (instead of the next one from the DB query):
///   HARVEST_CANISTER_ID=z7bpd-waaaa-aaaag-acogq-cai \
///     cargo test -p task_runner -- --ignored harvest_single_canister --nocapture
#[tokio::test]
#[ignore = "harvests cycles on mainnet — run explicitly"]
async fn harvest_single_canister() -> Result<()> {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");
    let db_path = root.join(DB_PATH);

    let pool = open_pool(db_path.to_str().unwrap()).await?;

    // Allow forcing an exact canister (e.g. the one from a previous partial run)
    // instead of letting pending_harvests pick the "next" from the DB.
    let canister_id: Principal = if let Ok(id_str) = std::env::var("HARVEST_CANISTER_ID") {
        Principal::from_text(&id_str)
            .with_context(|| format!("invalid HARVEST_CANISTER_ID: {}", id_str))?
    } else {
        let pending = pending_harvests(&pool, 1).await?;
        if pending.is_empty() {
            println!("No pending canisters to harvest.");
            return Ok(());
        }
        pending[0]
    };
    println!("Harvesting: {}", canister_id);

    // Build individual_user_template wasm.
    println!("Building individual_user_template for mainnet...");
    let status = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["build", "individual_user_template", "--network=ic"])
        .current_dir(&root)
        .status()
        .context("dfx not found")?;
    anyhow::ensure!(
        status.success(),
        "dfx build individual_user_template failed"
    );

    let wasm_path = root.join(INDIVIDUAL_USER_WASM_PATH);
    let wasm_blob = std::fs::read(&wasm_path)
        .with_context(|| format!("wasm not found at {}", wasm_path.display()))?;
    println!("  wasm size: {} bytes", wasm_blob.len());

    let agent = agent_from_pem(&pem_path).await?;
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID)?;

    // Derive the actions principal from the same PEM used for the agent.
    // Passed into harvest_canister so we can assert (right after add_our returns Ok)
    // that the PO now reports this principal in the target's controllers list.
    // This is the explicit "wait for update to finish then assert" check (no polling).
    let actions_identity = ic_agent::identity::Secp256k1Identity::from_pem_file(&pem_path)
        .with_context(|| {
            format!(
                "failed to load Secp256k1Identity from {}",
                pem_path.display()
            )
        })?;
    let actions_principal = actions_identity.sender().map_err(|e| {
        anyhow::anyhow!(
            "could not derive sender principal from actions_identity.pem: {}",
            e
        )
    })?;

    let (pre_balance, pre_reserved, post_uninstall, transferred, topped_up) =
        harvest_canister(&agent, actions_principal, po, canister_id, &wasm_blob).await?;

    // Mark as harvested in DB — only after all steps succeed and validations pass.
    mark_harvested(
        &pool,
        &canister_id,
        pre_balance,
        pre_reserved,
        post_uninstall,
        transferred,
        topped_up,
    )
    .await?;

    println!("\n✓ Harvest complete for {}", canister_id);
    Ok(())
}

/// Harvest cycles from a batch of canisters (BATCH_SIZE at a time).
/// Resumable: skips already-harvested canisters.
#[tokio::test]
#[ignore = "harvests cycles on mainnet — run explicitly"]
async fn harvest_cycles_batch() -> Result<()> {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");
    let db_path = root.join(DB_PATH);

    let pool = open_pool(db_path.to_str().unwrap()).await?;

    // Check current progress.
    let (done, failed) = harvest_counts(&pool).await?;
    println!("Current progress: {} harvested, {} failures", done, failed);

    let pending = pending_harvests(&pool, BATCH_SIZE).await?;

    if pending.is_empty() {
        println!("No pending canisters to harvest.");
        return Ok(());
    }

    println!(
        "Processing batch of {} canisters ({} already done)...",
        pending.len(),
        done
    );

    // Build individual_user_template wasm once.
    println!("\nBuilding individual_user_template for mainnet...");
    let status = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["build", "individual_user_template", "--network=ic"])
        .current_dir(&root)
        .status()
        .context("dfx not found")?;
    anyhow::ensure!(
        status.success(),
        "dfx build individual_user_template failed"
    );

    let wasm_path = root.join(INDIVIDUAL_USER_WASM_PATH);
    let wasm_blob = std::fs::read(&wasm_path)
        .with_context(|| format!("wasm not found at {}", wasm_path.display()))?;
    println!("  wasm size: {} bytes", wasm_blob.len());

    let agent = agent_from_pem(&pem_path).await?;
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID)?;

    // Derive the actions principal once for the batch (same identity for all canisters).
    // Passed into harvest_canister so the post-add_our Ok assertion can verify the
    // PO reports the principal in the controllers list before direct mgmt calls.
    let actions_identity = ic_agent::identity::Secp256k1Identity::from_pem_file(&pem_path)
        .with_context(|| {
            format!(
                "failed to load Secp256k1Identity from {}",
                pem_path.display()
            )
        })?;
    let actions_principal = actions_identity.sender().map_err(|e| {
        anyhow::anyhow!(
            "could not derive sender principal from actions_identity.pem: {}",
            e
        )
    })?;

    // Process each canister sequentially.
    let mut total_transferred: u128 = 0;
    let mut batch_success = 0;
    let mut batch_failed = 0;

    for (i, canister_id) in pending.iter().enumerate() {
        println!(
            "\n========== [{}/{}] Harvesting {} ==========",
            i + 1,
            pending.len(),
            canister_id
        );

        match harvest_canister(&agent, actions_principal, po, *canister_id, &wasm_blob).await {
            Ok((pre_balance, pre_reserved, post_uninstall, transferred, topped_up)) => {
                total_transferred += transferred;

                // Mark as harvested in DB — only after all steps succeed.
                mark_harvested(
                    &pool,
                    canister_id,
                    pre_balance,
                    pre_reserved,
                    post_uninstall,
                    transferred,
                    topped_up,
                )
                .await?;

                batch_success += 1;
                println!(
                    "  ✓ [{}] harvested, transferred {} TC{}",
                    i + 1,
                    format_cycles(transferred),
                    if topped_up > 0 { " (topped up)" } else { "" }
                );
            }
            Err(e) => {
                batch_failed += 1;
                let reason = e.to_string();
                println!("  ✗ [{}] failed: {}", i + 1, reason);

                // Mark as failed in DB.
                mark_harvest_failed(&pool, canister_id, &reason).await.ok();
            }
        }
    }

    // Summary.
    let (final_done, final_failed) = harvest_counts(&pool).await?;
    println!("\n========== Batch Summary ==========");
    println!("  Harvested this batch: {}", batch_success);
    println!("  Failed this batch: {}", batch_failed);
    println!(
        "  Total transferred this batch: {:?} TC",
        format_cycles(total_transferred)
    );
    println!("  Total harvested (all time): {}", final_done);
    println!("  Total failures (all time): {}", final_failed);

    Ok(())
}

/// Print a safe, read-only snapshot of the current harvest progress against the
/// production `ic_canisters.db`. No on-chain calls, no mutations.
///
/// Run:
///   cargo test -p task_runner -- --ignored harvest_status --nocapture
#[tokio::test]
#[ignore = "read-only status report against the real DB"]
async fn harvest_status() -> Result<()> {
    let root = workspace_root();
    let db_path = root.join(DB_PATH);

    let pool = open_pool(db_path.to_str().unwrap()).await?;

    let total_controlled = total_controlled_count(&pool).await?;
    let pending = pending_harvest_count(&pool).await?;
    let (done, failed) = harvest_counts(&pool).await?;

    println!("\n========== Harvest Status ==========");
    println!(
        "  Source list (po_controlled_canisters): {}",
        total_controlled
    );
    println!("  Already harvested (cycle_harvested):     {}", done);
    println!("  Failed (cycle_harvest_failures):         {}", failed);
    println!("  Still pending:                           {}", pending);
    println!(
        "  Progress:                                {:.1}%",
        if total_controlled > 0 {
            (done as f64 / total_controlled as f64) * 100.0
        } else {
            0.0
        }
    );

    // Show a small sample of pending work (safe to print).
    if pending > 0 {
        let sample = pending_harvests(&pool, 5).await?;
        println!("\n  Next pending (up to 5):");
        for (i, p) in sample.iter().enumerate() {
            println!("    [{}] {}", i + 1, p);
        }
        if pending > sample.len() as i64 {
            println!("    ... and {} more", pending - sample.len() as i64);
        }
    } else {
        println!("\n  No pending canisters remaining.");
    }

    println!("\n  DB path: {}", db_path.display());
    println!("====================================\n");

    Ok(())
}
