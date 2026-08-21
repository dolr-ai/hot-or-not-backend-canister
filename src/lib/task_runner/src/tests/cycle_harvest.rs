/// Cycle harvesting via task_runner — reclaims cycles from a single PO-controlled
/// canister and transfers them back to platform_orchestrator.
///
/// Only single-canister harvesting is supported. The canister list lives in
/// `src/canister/platform_orchestrator/principal.csv` (one principal per line).
///
/// Flow per canister:
///   1. Get pre-state (balance, reserved, status, has_wasm)
///   2. Add actions_identity as controller via PO endpoint; *immediately* re-query the
///      management canister directly (canister_status) using the actions-signed agent
///      and assert the actions principal is now in the returned controllers list
///      (explicit post-Ok check, no polling loop).
///   3. Uninstall wasm (if installed) → releases reserved cycles to main balance
///   4. Check post-uninstall balance; top up with 0.5T if < 1.0 TC
///   5. Install canister_to_harvest wasm
///   6. Call return_cycle_balance_to_platform_orchestrator → sends everything back
///   7. Final uninstall
///   8. Set controllers to [PO] only via management canister
///   9. Validate final state
///
/// Run:
///   HARVEST_CANISTER_ID=z7bpd-waaaa-aaaag-acogq-cai \
///     cargo test -p task_runner -- --ignored harvest_single_canister --nocapture
///
/// If HARVEST_CANISTER_ID is not set, the first canister from principal.csv is used.
use anyhow::{Context, Result};
use candid::{encode_args, Encode, Principal};
use ic_agent::Identity;
use std::collections::HashSet;

use crate::{
    agent::{agent_from_pem, workspace_root},
    canister_list::{read_canisters, PRINCIPAL_CSV},
    sns_types::PLATFORM_ORCHESTRATOR_ID,
};

/// IC management canister.
const MANAGEMENT_CANISTER: &str = "aaaaa-aa";

// Minimum balance (cycles) required after uninstall to proceed with reinstall + transfer.
// Must be high enough to cover:
// - wasm install cost (can be ~0.2 TC for template)
// - execution + deposit_cycles to PO inside return_cycle...
// - final uninstall + update_settings
// - canister-specific freezing thresholds and memory allocations
// If below this after uninstall (i.e. for very low-balance PO-controlled canisters),
// we top up via PO's deposit_cycles_to_canister (recover most via the return call).
const MIN_REINSTALL_BALANCE: u128 = 1_000_000_000_000; // 1.0 TC (raised to safely cover freezing thresholds + install costs)

/// Top-up amount when a canister's balance is too low to reinstall.
/// We recover most of this back via return_cycle_balance, so still net positive.
const TOP_UP_AMOUNT: u128 = 500_000_000_000; // 0.5T

/// Pre-harvest deposit to ensure out-of-cycles canisters can be reached.
/// This small amount covers the management canister call overhead so we can
/// query status and set controllers. We recover it during the harvest itself.
const PRE_HARVEST_DEPOSIT: u128 = 100_000_000_000; // 100B cycles (~0.1 TC)

/// Path to the canister_to_harvest wasm (built by cargo for wasm32).
/// This minimal canister is installed on the target during harvest so it can
/// call return_cycle_balance_to_platform_orchestrator to send cycles back to the PO.
/// Built by `cargo build --target wasm32-unknown-unknown --release` (via generate-candid.sh).
const HARVEST_WASM_PATH: &str =
    "target/wasm32-unknown-unknown/release/canister_to_harvest.wasm";

/// Check if a principal is a user principal (long string) vs a canister principal (short string).
/// User principals cannot be harvested — they're not canisters.
/// Canister principals are always 27 chars (e.g. `gq4rc-paaaa-aaaai-agpaq-cai`).
/// User principals are ~57 chars (e.g. `7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae`).
fn is_user_principal(p: &Principal) -> bool {
    p.to_text().len() > 30
}

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

/// Deposit a small amount of cycles into a canister via the PO.
/// This ensures out-of-cycles canisters can be reached for status queries
/// and controller mutations. We recover the deposit during the harvest itself.
async fn ensure_canister_has_cycles(
    agent: &ic_agent::Agent,
    po: Principal,
    canister_id: Principal,
) -> Result<()> {
    let deposit_arg = encode_args((canister_id, PRE_HARVEST_DEPOSIT))?;
    agent
        .update(&po, "deposit_cycles_to_canister")
        .with_arg(deposit_arg)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("pre-harvest deposit_cycles_to_canister failed: {}", e))?;
    Ok(())
}

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

    // Handle the case where a prior successful harvest left the canister with *only* the
    // Platform Orchestrator as controller. The early direct management canister_status
    // (our existence + pre-state gate) will then fail with an IC0512 "Only the controllers"
    // rejection. In that situation we ask the PO (which is the sole controller) to add our
    // actions principal as a co-controller, then immediately retry the same direct
    // canister_status. After that we fall through to the normal "Step 2" add + verify,
    // which will be a harmless idempotent second add.
    fn is_controller_only_error(s: &str) -> bool {
        s.contains("Only the controllers of the canister")
            || s.contains("can control it")
            || s.contains("IC0512")
            || s.contains("add the current caller as a controller")
    }

    let exists_response = match exists_res {
        Ok(resp) => resp,
        Err(e) => {
            println!("  [debug] raw canister_status error (Debug): {:?}", e);
            let estr = e.to_string();
            println!("  [debug] raw canister_status error (to_string): {}", estr);

            if is_controller_only_error(&estr) {
                println!(
                    "  \u{26a0} early canister_status rejected with 'only controllers' error \
                     (PO is the sole controller from a prior harvest). Calling platform_orchestrator \
                     add_our_identity_as_controller to grant ourselves co-controller rights, then \
                     retrying the management canister_status..."
                );
                let add_ctrl_arg = encode_args((canister_id,))?;
                agent
                    .update(&po, "add_our_identity_as_controller")
                    .with_arg(add_ctrl_arg)
                    .call_and_wait()
                    .await
                    .map_err(|ae| {
                        anyhow::anyhow!("recovery add_our_identity_as_controller failed: {}", ae)
                    })?;
                println!(
                    "  \u{2713} recovery add_our_identity_as_controller succeeded; \
                     re-issuing early canister_status (now as co-controller)"
                );

                // Retry the exact same direct management call that just failed.
                let retry_arg = Encode!(&id_record)?;
                agent
                    .update(
                        &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
                        "canister_status",
                    )
                    .with_arg(retry_arg)
                    .with_effective_canister_id(canister_id)
                    .call_and_wait()
                    .await
                    .map_err(|ae| {
                        anyhow::anyhow!(
                            "early canister_status retry after recovery add failed for {}: {}",
                            canister_id,
                            ae
                        )
                    })?
            } else {
                // Print the exact replica error instead of replacing with our user-friendly message
                return Err(anyhow::anyhow!(
                    "early canister_status on management canister failed for {}: {}",
                    canister_id,
                    estr
                ));
            }
        }
    };

    // The early (or recovery-retried) management canister_status call succeeded.
    // Decode it now to get the pre-state (balance, reserved_cycles, status).
    // This replaces the previous redundant PO get_controllers_and_cycle_balance call for pre-state.
    // We already paid the roundtrip for the existence gate; no need to call PO again just for data.
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

    // has_wasm must be based on module_hash (the authoritative field from canister_status),
    // not merely the high-level status (running/stopped). A canister can report Running
    // or Stopped while having module_hash = None (no code installed). We uninstall only
    // when code is present so we can release any reserved cycles tied to the wasm/stable memory.
    let has_wasm = status.module_hash.is_some();

    let pre_module = if status.module_hash.is_some() {
        "present"
    } else {
        "absent"
    };
    println!(
        "  pre_balance: {} TC, pre_reserved: {} TC, status: {:?}, module_hash: {}",
        format_cycles(pre_balance),
        format_cycles(pre_reserved),
        status.status,
        pre_module
    );

    // Step 2: Add actions_identity as controller via PO endpoint.
    let add_ctrl_arg = encode_args((canister_id,))?;
    agent
        .update(&po, "add_our_identity_as_controller")
        .with_arg(add_ctrl_arg)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("add_our_identity_as_controller failed: {}", e))?;
    println!("  \u{2713} actions_identity added as controller");
    // Immediately after the add_our update call returns Ok, try to re-query the management
    // canister directly using the actions-signed agent and assert the actions principal
    // is now present.
    //
    // We are tolerant of "canister_not_found" here: the principal.csv list may contain
    // canisters that have since been deleted (uninstalled, decommissioned, or self-removed).
    // If the canister disappeared between the PO add and this verify, we log and continue —
    // later direct calls (uninstall etc.) will surface the same error.
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
                    "  \u{26a0} uninstall_code failed (canister may have no wasm): {}",
                    e
                );
            }
        }

        // Immediately re-query management canister_status to *confirm* the uninstall took effect.
        // The definitive signal is module_hash == None. The high-level "status" (running/stopped)
        // alone is not reliable for "has code or not".
        let post_uninst_id = CanisterIdRecord { canister_id };
        let post_uninst_arg = Encode!(&post_uninst_id)?;
        let post_uninst_res = agent
            .update(
                &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
                "canister_status",
            )
            .with_arg(post_uninst_arg)
            .with_effective_canister_id(canister_id)
            .call_and_wait()
            .await;

        match post_uninst_res {
            Ok(resp) => {
                let s: CanisterStatusResult = candid::decode_one(&resp).map_err(|e| {
                    anyhow::anyhow!("failed to decode post-uninstall canister_status: {}", e)
                })?;
                let module_state = if s.module_hash.is_some() {
                    "still present"
                } else {
                    "absent (confirming uninstall)"
                };
                println!(
                    "  post-uninstall probe: status={:?}, module_hash: {}, cycles now ~{} TC",
                    s.status,
                    module_state,
                    format_cycles(u128::try_from(s.cycles.0.clone()).unwrap_or(0))
                );
            }
            Err(e) => {
                println!(
                    "  \u{26a0} post-uninstall canister_status probe failed (may be ok if canister is being drained): {}",
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
            "  \u{26A0} balance too low ({} TC < {} TC), topping up with 0.5T...",
            format_cycles(post_uninstall_balance),
            format_cycles(MIN_REINSTALL_BALANCE)
        );

        // Top up via PO's deposit_cycles_to_canister (PO itself funds it from its pool).
        // This is recovered (mostly) when the freshly-installed template calls
        // return_cycle_balance_to_platform_orchestrator.
        let deposit_arg = encode_args((canister_id, TOP_UP_AMOUNT))?;
        agent
            .update(&po, "deposit_cycles_to_canister")
            .with_arg(deposit_arg)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("deposit_cycles_to_canister failed: {}", e))?;

        topped_up = TOP_UP_AMOUNT;
        println!("  \u{2713} topped up with 0.5T");
    }

    // Unconditionally ensure the canister is empty right before we install the template.
    // This is the robust fix for IC0514 "canister not empty" (the early has_wasm
    // decision or a prior partial run can leave code on the canister). We ignore any
    // error because "no module" (already empty) is the happy case. Reinstall would
    // also work for non-empty canisters, but an explicit uninstall + Install is
    // simple, matches dfx force-restore patterns, and guarantees a clean stable memory.
    let _ = agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "uninstall_code",
        )
        .with_arg(encode_args((CanisterIdRecord { canister_id },))?)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await;

    // Step 5: Install canister_to_harvest wasm.
    // Always use Reinstall mode + unconditional pre-uninstall (see above) so this is
    // robust against IC0514 "canister not empty" and partial prior runs.
    // This matches the behavior dfx uses for force-restore/install scenarios.
    let install_arg = encode_args((InstallCodeArgs {
        mode: CanisterInstallMode::Reinstall,
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
    println!("  \u{2713} wasm installed");

    // Immediately after install, fetch status directly from management canister
    // to see the exact cycle consumption of the install itself.
    // This lets us validate how much the install actually burned.
    let post_install_id_record = CanisterIdRecord { canister_id };
    let post_install_arg = Encode!(&post_install_id_record)?;
    let post_install_res = agent
        .update(
            &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
            "canister_status",
        )
        .with_arg(post_install_arg)
        .with_effective_canister_id(canister_id)
        .call_and_wait()
        .await
        .map_err(|e| anyhow::anyhow!("post-install canister_status failed: {}", e))?;

    let post_install_status: CanisterStatusResult = candid::decode_one(&post_install_res)
        .map_err(|e| anyhow::anyhow!("failed to decode post-install canister_status: {}", e))?;

    let cycles_big = post_install_status.cycles.0.clone();
    let post_install_balance: u128 = cycles_big.clone().try_into().unwrap_or_else(|_| {
        cycles_big
            .iter_u64_digits()
            .fold(0u128, |acc, d| (acc << 64) | u128::from(d))
    });

    let reserved_big = post_install_status.reserved_cycles.0.clone();
    let post_install_reserved: u128 = reserved_big.clone().try_into().unwrap_or_else(|_| {
        reserved_big
            .iter_u64_digits()
            .fold(0u128, |acc, d| (acc << 64) | u128::from(d))
    });

    let post_mod = if post_install_status.module_hash.is_some() {
        let h = &post_install_status.module_hash.as_ref().unwrap();
        format!("present ({:x?}...)", &h[..std::cmp::min(4, h.len())])
    } else {
        "absent".to_string()
    };
    println!(
        "  post_install_balance: {} TC, post_install_reserved: {} TC, status: {:?}, module_hash: {}",
        format_cycles(post_install_balance),
        format_cycles(post_install_reserved),
        post_install_status.status,
        post_mod
    );

    // Ensure the canister is running before attempting the transfer. If the canister
    // is Stopped, `return_cycle_balance_to_platform_orchestrator` will fail with IC0508
    // because a stopped canister cannot execute code. `start_canister` is idempotent for
    // already-running canisters, but we only call it when actually needed to avoid
    // unnecessary round-trips.
    if matches!(post_install_status.status, CanisterStatus::Stopped) {
        println!("  canister is stopped; starting it before transfer...");
        agent
            .update(
                &Principal::from_text(MANAGEMENT_CANISTER).unwrap(),
                "start_canister",
            )
            .with_arg(encode_args((CanisterIdRecord { canister_id },))?)
            .with_effective_canister_id(canister_id)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("start_canister failed: {}", e))?;
        println!("  \u{2713} canister started");
    }

    // Step 6: Transfer cycles to PO, passing an explicit reserve (cycles to leave behind).
    // Start at 50B (batch runs showed 30B consistently insufficient after a fresh
    // canister_to_harvest install). Step +10B on "Couldn't send message" / out-of-cycles
    // up to 100B max. If it still fails at 100B, accept 0 transferred and proceed to
    // final uninstall + PO-only lockdown (the transfer is best-effort).
    let mut cycles_transferred: u128 = 0;
    let mut reserve = 50_000_000_000u128; // bumped from 30B after fleet batch observed consistent failures
    let max_reserve: u128 = 100_000_000_000; // 100B

    while reserve <= max_reserve {
        println!(
            "  attempting return_cycle_balance_to_platform_orchestrator (reserve={}B)...",
            reserve / 1_000_000_000
        );

        let transfer_result = agent
            .update(
                &canister_id,
                "return_cycle_balance_to_platform_orchestrator",
            )
            .with_arg(encode_args((reserve,))?)
            .call_and_wait()
            .await;

        let (succeeded, transferred, should_retry_higher) = match transfer_result {
            Ok(response) => {
                let result: Result<u128, String> = candid::decode_one(&response)
                    .map_err(|e| anyhow::anyhow!("failed to decode transfer result: {}", e))?;
                match result {
                    Ok(amount) => (true, amount, false),
                    Err(e) => {
                        let is_ooce = is_out_of_cycles_error(&e);
                        if is_ooce {
                            println!(
                                "  \u{26A0} return returned out-of-cycles with reserve {}B: {}",
                                reserve / 1_000_000_000,
                                e
                            );
                            (false, 0, true)
                        } else {
                            println!("  \u{26A0} return_cycle_balance returned error: {}", e);
                            (false, 0, false)
                        }
                    }
                }
            }
            Err(e) => {
                let estr = e.to_string();
                let is_ooce = is_out_of_cycles_error(&estr);
                if is_ooce {
                    println!(
                        "  \u{26A0} return_cycle_balance call failed (out of cycles) with reserve {}B: {}",
                        reserve / 1_000_000_000,
                        e
                    );
                    (false, 0, true)
                } else {
                    println!("  \u{26A0} return_cycle_balance call failed: {}", e);
                    (false, 0, false)
                }
            }
        };

        if succeeded {
            cycles_transferred = transferred;
            println!(
                "  \u{2713} transferred {} TC to platform_orchestrator (reserve={}B)",
                format_cycles(cycles_transferred),
                reserve / 1_000_000_000
            );
            break;
        }

        if !should_retry_higher {
            cycles_transferred = transferred; // 0
            break;
        }

        reserve += 10_000_000_000;
        if reserve > max_reserve {
            println!(
                "  \u{26A0} exhausted reserve retries up to 100B; 0 transferred (will proceed to uninstall/lockdown)"
            );
            break;
        }
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
    println!("  \u{2713} final uninstall done");

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
    println!("  \u{2713} controllers set to [PO] only");

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
        anyhow::bail!(
            "controllers not set to [PO] only after update_settings: {:?}. \
             The canister was NOT fully harvested.",
            details.controllers
        );
    } else {
        println!("  \u{2713} validated: controllers = [PO]");
    }

    println!(
        "  \u{2713} final balance: {} TC",
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

fn is_out_of_cycles_error(s: &str) -> bool {
    let s = s.to_lowercase();
    s.contains("out of cycles")
        || s.contains("ic0504")
        || (s.contains("cycles") && s.contains("out"))
        || s.contains("couldn't send message")
}

/// Prints a `dfx canister status` for the Platform Orchestrator so we can
/// observe whether its cycle balance is increasing as a result of harvest
/// transfers (the `return_cycle_balance_to_platform_orchestrator` calls deposit
/// cycles into the PO).
fn print_po_cycle_balance(label: &str, root: &std::path::Path) {
    println!(
        "
=== Platform Orchestrator cycle balance ({}) ===",
        label
    );
    let _ = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args([
            "canister",
            "status",
            PLATFORM_ORCHESTRATOR_ID,
            "--network=ic",
        ])
        .current_dir(root)
        .status();
    println!("=== end PO status ({}) ===\n", label);
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

// ── Test entry point ──────────────────────────────────────────────────────────

/// Harvest cycles from a single canister.
///
/// To target a specific canister ID:
///   HARVEST_CANISTER_ID=z7bpd-waaaa-aaaag-acogq-cai \
///     cargo test -p task_runner -- --ignored harvest_single_canister --nocapture
///
/// If HARVEST_CANISTER_ID is not set, the first canister from principal.csv is used.
#[tokio::test]
#[ignore = "harvests cycles on mainnet — run explicitly"]
async fn harvest_single_canister() -> Result<()> {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");

    // Allow forcing an exact canister (e.g. the one from a previous partial run)
    // instead of picking the first from the principal.csv list.
    let canister_id: Principal = if let Ok(id_str) = std::env::var("HARVEST_CANISTER_ID") {
        Principal::from_text(&id_str)
            .with_context(|| format!("invalid HARVEST_CANISTER_ID: {}", id_str))?
    } else {
        let csv_path = root.join(PRINCIPAL_CSV);
        let pending = read_canisters(&csv_path, 1)?;
        if pending.is_empty() {
            println!("No canisters found in {}.", csv_path.display());
            return Ok(());
        }
        pending[0]
    };
    println!("Harvesting: {}", canister_id);

    // Build canister_to_harvest wasm and auto-regenerate its Candid interface
    // (so the .did is always produced from the exact compiled export_candid!() in this build).
    // generate-candid.sh runs `cargo build --target wasm32-unknown-unknown --release` internally,
    // producing the wasm at target/wasm32-unknown-unknown/release/canister_to_harvest.wasm.
    println!("Building canister_to_harvest + regenerating Candid via generate-candid.sh...");
    let build_status = std::process::Command::new("bash")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["scripts/generate-candid.sh", "canister_to_harvest"])
        .current_dir(&root)
        .status()
        .context("bash or scripts/generate-candid.sh not found")?;
    anyhow::ensure!(
        build_status.success(),
        "scripts/generate-candid.sh canister_to_harvest failed"
    );

    let wasm_path = root.join(HARVEST_WASM_PATH);
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

    // Pre-harvest: deposit a small amount of cycles to ensure the canister
    // isn't out-of-cycles (IC0207). This also ensures controller principals
    // are set consistently with the rest of the fleet.
    println!(
        "Depositing {} cycles into {}...",
        format_cycles(PRE_HARVEST_DEPOSIT),
        canister_id
    );
    if let Err(e) = ensure_canister_has_cycles(&agent, po, canister_id).await {
        println!("  \u{26a0} pre-harvest deposit failed (continuing anyway): {}", e);
    } else {
        println!("  \u{2713} pre-harvest deposit done");
    }

    print_po_cycle_balance("before harvest", &root);

    match harvest_canister(&agent, actions_principal, po, canister_id, &wasm_blob).await {
        Ok((_pre_balance, _pre_reserved, _post_uninstall, transferred, topped_up)) => {
            println!(
                "\n\u{2713} Harvest complete for {} (transferred {} TC{})",
                canister_id,
                format_cycles(transferred),
                if topped_up > 0 { " (topped up)" } else { "" }
            );
        }
        Err(e) => {
            let reason = e.to_string();

            // If the failure looks like a controller/ownership error,
            // try parent recovery as a fallback (additive, non-destructive).
            if is_controller_ownership_error(&reason) {
                println!(
                    "\n\u{26a0} Normal harvest failed with controller error, \
                     attempting parent recovery for {}...",
                    canister_id
                );

                match harvest_with_parent_recovery(
                    &agent,
                    actions_principal,
                    po,
                    canister_id,
                    &wasm_blob,
                    &root,
                )
                .await
                {
                    Ok(transferred) => {
                        println!(
                            "\n\u{2713} Harvest complete for {} via parent recovery, transferred {} TC",
                            canister_id,
                            format_cycles(transferred)
                        );
                    }
                    Err(parent_err) => {
                        println!(
                            "\n\u{2717} Parent recovery also failed for {}: {}",
                            canister_id,
                            parent_err
                        );
                    }
                }
            } else {
                println!("\n\u{2717} Harvest failed for {}: {}", canister_id, reason);
            }
        }
    }

    print_po_cycle_balance("after harvest", &root);

    Ok(())
}

// ── Parent recovery fallback ──────────────────────────────────────────────────

/// Check if an error message indicates a controller/ownership problem.
/// These are the cases where parent recovery is worth attempting.
fn is_controller_ownership_error(reason: &str) -> bool {
    reason.contains("Only the controllers of the canister")
        || reason.contains("can control it")
        || reason.contains("IC0512")
        || reason.contains("add the current caller as a controller")
        || reason.contains("add_our_identity_as_controller failed")
        || reason.contains("not a controller")
        || reason.contains("controller") && reason.contains("failed")
}

/// Get controllers of a canister by running `dfx canister info`.
fn get_canister_controllers(
    canister_id: Principal,
    root: &std::path::Path,
) -> Result<Vec<Principal>> {
    let output = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["canister", "info", &canister_id.to_text(), "--network=ic"])
        .current_dir(root)
        .output()
        .context("failed to execute `dfx canister info`")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!(
            "`dfx canister info` failed: {}",
            stderr.trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let controllers_line = stdout
        .lines()
        .find(|line| line.starts_with("Controllers:"))
        .context("`dfx canister info` output missing 'Controllers:' line")?;

    let controller_principals: Vec<Principal> = controllers_line
        .strip_prefix("Controllers:")
        .unwrap()
        .split_whitespace()
        .filter_map(|s| Principal::from_text(s).ok())
        .collect();

    Ok(controller_principals)
}

/// Maximum recursion depth for parent recovery.
/// Each level tries to harvest a parent canister that may itself need parent recovery.
/// 5 is generous — real chains are typically 1-2 levels deep.
const MAX_PARENT_RECOVERY_DEPTH: usize = 5;

/// Harvest a canister where PO is not a controller, using parent recovery.
async fn harvest_with_parent_recovery(
    agent: &ic_agent::Agent,
    actions_principal: Principal,
    po: Principal,
    child_id: Principal,
    wasm_blob: &[u8],
    root: &std::path::Path,
) -> Result<u128> {
    let mut visited = HashSet::new();
    visited.insert(child_id);
    harvest_with_parent_recovery_inner(
        agent,
        actions_principal,
        po,
        child_id,
        wasm_blob,
        root,
        0,
        &mut visited,
    )
    .await
}

async fn harvest_with_parent_recovery_inner(
    agent: &ic_agent::Agent,
    actions_principal: Principal,
    po: Principal,
    child_id: Principal,
    wasm_blob: &[u8],
    root: &std::path::Path,
    depth: usize,
    visited: &mut HashSet<Principal>,
) -> Result<u128> {
    if depth >= MAX_PARENT_RECOVERY_DEPTH {
        anyhow::bail!(
            "parent recovery depth limit ({}) reached for {}, giving up",
            MAX_PARENT_RECOVERY_DEPTH,
            child_id
        );
    }

    let management_canister = Principal::from_text(MANAGEMENT_CANISTER)?;

    // Get the parent controller from the child canister via dfx
    let controllers = get_canister_controllers(child_id, root)?;

    if controllers.is_empty() {
        anyhow::bail!("child canister {} has no controllers", child_id);
    }

    // Check if our actions principal is already a controller — if so, just run
    // normal harvest directly (no parent recovery needed).
    if controllers.contains(&actions_principal) {
        println!(
            "  Child {} already has actions principal as controller, running normal harvest...",
            child_id
        );
        let (_pre_balance, _pre_reserved, _post_uninstall, transferred, _topped_up) =
            harvest_canister(agent, actions_principal, po, child_id, wasm_blob).await?;
        return Ok(transferred);
    }

    // Find all valid canister parents (skip user principals — they're not harvestable).
    // Also skip canisters we've already visited in this recovery chain to break cycles.
    let canister_parents: Vec<Principal> = controllers
        .iter()
        .filter(|c| !is_user_principal(c))
        .filter(|c| !visited.contains(c))
        .cloned()
        .collect();

    if canister_parents.is_empty() {
        anyhow::bail!(
            "child {} has no unvisited canister parents (all are user principals or already visited)",
            child_id
        );
    }

    // Try each canister parent in order. The first one that works wins.
    let mut last_error = None;
    for parent_id in &canister_parents {
        println!("  Child {} trying parent {}", child_id, parent_id);

        // Mark this parent as visited to prevent cycles.
        visited.insert(*parent_id);

        // Attempt normal harvest on parent first.
        if let Err(e) = ensure_canister_has_cycles(agent, po, *parent_id).await {
            println!("  \u{26a0} pre-harvest deposit for parent failed: {}", e);
        }

        match harvest_canister(agent, actions_principal, po, *parent_id, wasm_blob).await {
            Ok((_pre_balance, _pre_reserved, _post_uninstall, transferred, _topped_up)) => {
                println!(
                    "  \u{2713} parent harvested first, transferred {} TC",
                    format_cycles(transferred)
                );
            }
            Err(e) => {
                let reason = e.to_string();
                // If parent also has controller issues, recurse into parent recovery.
                if is_controller_ownership_error(&reason) {
                    println!(
                        "  \u{26a0} parent harvest failed with controller error, \
                         recursing into parent recovery for {}...",
                        parent_id
                    );
                    match Box::pin(harvest_with_parent_recovery_inner(
                        agent,
                        actions_principal,
                        po,
                        *parent_id,
                        wasm_blob,
                        root,
                        depth + 1,
                        visited,
                    ))
                    .await
                    {
                        Ok(_) => {
                            println!("  \u{2713} parent recovered via recursive parent recovery");
                        }
                        Err(recurse_err) => {
                            println!(
                                "  \u{26a0} parent {} recursive recovery failed: {}, trying next parent...",
                                parent_id, recurse_err
                            );
                            last_error = Some(recurse_err);
                            continue;
                        }
                    }
                } else {
                    println!(
                        "  \u{26a0} parent {} harvest failed (non-controller error): {}, trying next parent...",
                        parent_id, reason
                    );
                    last_error = Some(anyhow::anyhow!(
                        "parent {} harvest failed (non-controller error): {}",
                        parent_id,
                        reason
                    ));
                    continue;
                }
            }
        }

        // --- Parent is ready. Now use it for recovery. ---

        // Add actions_identity as controller of parent via PO
        println!(
            "  Adding actions_identity as controller of parent {}...",
            parent_id
        );
        let add_ctrl_arg = encode_args((parent_id,))?;
        agent
            .update(&po, "add_our_identity_as_controller")
            .with_arg(add_ctrl_arg)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("add_our_identity_as_controller on parent failed: {}", e))?;
        println!("  \u{2713} actions_identity added as controller of parent");

        // Top up parent with cycles so it can afford wasm install
        let deposit_arg = encode_args((parent_id, TOP_UP_AMOUNT))?;
        agent
            .update(&po, "deposit_cycles_to_canister")
            .with_arg(deposit_arg)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("deposit_cycles_to_canister on parent failed: {}", e))?;
        println!("  \u{2713} topped up parent with 0.5T");

        // Install wasm on parent (now we have access as co-controller)
        println!("  Installing wasm on parent {}...", parent_id);
        let install_arg = encode_args((InstallCodeArgs {
            mode: CanisterInstallMode::Reinstall,
            canister_id: *parent_id,
            wasm_module: wasm_blob.to_vec(),
            arg: vec![],
            sender_canister_version: None,
        },))?;

        agent
            .update(&management_canister, "install_code")
            .with_arg(install_arg)
            .with_effective_canister_id(*parent_id)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("install_code on parent failed: {}", e))?;

        println!("  \u{2713} wasm installed on parent");

        // Call add_controllers(child_id) on parent
        println!("  Calling add_controllers({}) on parent...", child_id);
        let add_ctrl_arg = encode_args((child_id,))?;
        let add_ctrl_response = agent
            .update(parent_id, "add_controllers")
            .with_arg(add_ctrl_arg)
            .call_and_wait()
            .await
            .map_err(|e| anyhow::anyhow!("add_controllers call failed: {}", e))?;

        let _: Result<(), String> = candid::decode_one(&add_ctrl_response)
            .map_err(|e| anyhow::anyhow!("failed to decode add_controllers response: {}", e))?;

        println!(
            "  \u{2713} add_controllers succeeded, PO + actions principal now controllers of child"
        );

        // Harvest child using the normal, tested flow
        println!("  Harvesting child {}...", child_id);
        let (
            _child_pre_balance,
            _child_pre_reserved,
            _child_post_uninstall,
            child_transferred,
            _child_topped_up,
        ) = harvest_canister(agent, actions_principal, po, child_id, wasm_blob).await?;
        println!(
            "  \u{2713} child harvested, transferred {} TC",
            format_cycles(child_transferred)
        );

        // Re-harvest parent using the normal flow (wasm reinstall means it has reserved cycles again)
        println!("  Re-harvesting parent {}...", parent_id);
        let (
            _parent_pre_balance,
            _parent_pre_reserved,
            _parent_post_uninstall,
            parent_transferred,
            _parent_topped_up,
        ) = harvest_canister(agent, actions_principal, po, *parent_id, wasm_blob).await?;
        println!(
            "  \u{2713} parent re-harvested, transferred {} TC",
            format_cycles(parent_transferred)
        );

        return Ok(child_transferred + parent_transferred);
    }

    // All parents failed — return the last error (or a generic message if none).
    Err(last_error.unwrap_or_else(|| {
        anyhow::anyhow!("all canister parents failed for child {}", child_id)
    }))
}