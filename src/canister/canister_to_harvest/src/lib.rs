/// Minimal canister installed on a target canister during cycle harvesting.
///
/// Provides exactly the two update methods the harvest flow needs:
///   - `return_cycle_balance_to_platform_orchestrator(reserve)` — sends
///     (balance - reserve) cycles back to the platform_orchestrator via the
///     management canister's `deposit_cycles`.
///   - `add_controllers(target_canister_id)` — adds PO + the actions principal
///     as controllers of a child canister (used by parent-recovery).
///
/// The wasm is built with `dfx build canister_to_harvest --network=ic` and
/// installed (Reinstall mode) on the canister being harvested so the cycle
/// transfer can execute before the final uninstall.
use candid::Principal;
use ic_cdk::api::{
    canister_balance128,
    management_canister::{main::deposit_cycles, provisional::CanisterIdRecord},
};
use ic_cdk_macros::{export_candid, update};

const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";
const ACTIONS_PRINCIPAL_ID: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";
const MANAGEMENT_CANISTER_ID: &str = "aaaaa-aa";

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

#[derive(candid::CandidType, candid::Deserialize, Debug)]
struct CanisterStatusResult {
    settings: DefiniteCanisterSettings,
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
enum LogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
}

/// Sends (current_balance - reserve) cycles to the platform_orchestrator via deposit_cycles.
/// The reserve (passed by the caller) is left behind to cover execution + call overhead.
///
/// Must be called before uninstalling this canister's wasm, since deposit_cycles requires
/// this wasm to be running.
///
/// Returns the number of cycles transferred (or 0 on early exit).
#[update]
async fn return_cycle_balance_to_platform_orchestrator(reserve: u128) -> Result<u128, String> {
    let current_balance = canister_balance128();

    if current_balance <= reserve {
        return Ok(0);
    }

    let amount_to_send = current_balance - reserve;
    let platform_orchestrator =
        Principal::from_text(PLATFORM_ORCHESTRATOR_ID).map_err(|e| e.to_string())?;

    deposit_cycles(
        CanisterIdRecord {
            canister_id: platform_orchestrator,
        },
        amount_to_send,
    )
    .await
    .map_err(|e| e.1)?;

    Ok(amount_to_send)
}

/// Adds the platform orchestrator and actions principal as controllers to a target canister.
/// This is used during harvest recovery for canisters that don't have PO as a controller.
/// The calling canister must be a controller of the target canister for this to succeed.
#[update]
async fn add_controllers(target_canister_id: Principal) -> Result<(), String> {
    let management_canister =
        Principal::from_text(MANAGEMENT_CANISTER_ID).map_err(|e| e.to_string())?;
    let platform_orchestrator =
        Principal::from_text(PLATFORM_ORCHESTRATOR_ID).map_err(|e| e.to_string())?;
    let actions_principal =
        Principal::from_text(ACTIONS_PRINCIPAL_ID).map_err(|e| e.to_string())?;

    // First, get current controllers from the target canister
    let status_result: (CanisterStatusResult,) = ic_cdk::api::call::call(
        management_canister,
        "canister_status",
        (CanisterIdRecord {
            canister_id: target_canister_id,
        },),
    )
    .await
    .map_err(|e| format!("Failed to get canister status: {}", e.1))?;

    let mut current_controllers = status_result.0.settings.controllers;

    // Add PO and actions principal if not already present
    if !current_controllers.contains(&platform_orchestrator) {
        current_controllers.push(platform_orchestrator);
    }
    if !current_controllers.contains(&actions_principal) {
        current_controllers.push(actions_principal);
    }

    // Update the target canister's settings with the new controller list
    ic_cdk::api::call::call::<_, ()>(
        management_canister,
        "update_settings",
        (UpdateSettingsArgument {
            canister_id: target_canister_id,
            settings: CanisterSettings {
                controllers: Some(current_controllers),
                ..Default::default()
            },
        },),
    )
    .await
    .map_err(|e| format!("Failed to update canister settings: {}", e.1))?;

    Ok(())
}

export_candid!();