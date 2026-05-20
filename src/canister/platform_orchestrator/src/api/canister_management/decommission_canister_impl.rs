use candid::Principal;
use ic_cdk::{
    api::management_canister::main::{
        canister_status, uninstall_code, update_settings, CanisterIdRecord, CanisterSettings,
        UpdateSettingsArgument,
    },
    call,
};

const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

/// Core decommission logic, shared by the single and bulk functions.
///
/// Idempotency: reads canister_status first. If the wasm is already uninstalled
/// AND the only controller is already the platform_orchestrator, the function
/// is a no-op and returns Ok(()) immediately.
///
/// Steps (when work is needed):
///   1. Best-effort: call return_cycle_balance_to_platform_orchestrator on the
///      individual canister (sends remaining cycles here; ignored if it fails
///      because the wasm may already be gone or cycles already sent).
///   2. uninstall_code — removes the wasm and clears heap + stable memory.
///   3. update_settings — sets controllers to [platform_orchestrator] only,
///      removing any residual user_index controller entry.
pub async fn decommission_canister_impl(canister_id: Principal) -> Result<(), String> {
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();

    let (status,) = canister_status(CanisterIdRecord { canister_id })
        .await
        .map_err(|e| e.1)?;

    let is_installed = status.module_hash.is_some();
    let controllers = status.settings.controllers;
    let po_is_only_controller = controllers.len() == 1 && controllers.first() == Some(&po);

    if !is_installed && po_is_only_controller {
        return Ok(());
    }

    if is_installed {
        // Best-effort cycle return — proceed even if this fails (e.g. the
        // individual canister was already partially decommissioned).
        let _ = call::<_, (Result<u128, String>,)>(
            canister_id,
            "return_cycle_balance_to_platform_orchestrator",
            (),
        )
        .await;

        uninstall_code(CanisterIdRecord { canister_id })
            .await
            .map_err(|e| e.1)?;
    }

    // Always enforce: controllers = [platform_orchestrator] only.
    update_settings(UpdateSettingsArgument {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![po]),
            ..Default::default()
        },
    })
    .await
    .map_err(|e| e.1)
}
