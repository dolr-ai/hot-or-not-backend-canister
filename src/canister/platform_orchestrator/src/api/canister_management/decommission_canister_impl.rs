use candid::Principal;
use ic_cdk::{
    api::management_canister::main::{
        canister_status, install_code, uninstall_code, update_settings, CanisterIdRecord,
        CanisterInstallMode, CanisterSettings, InstallCodeArgument, UpdateSettingsArgument,
    },
    call,
};

const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

/// Core decommission logic, shared by the single and bulk functions.
///
/// For installed canisters (double-pass to recover reserved cycles):
///   1. return_cycle_balance_to_platform_orchestrator — sends (balance − 100M reserve) to PO
///   2. uninstall_code — frees heap + stable memory; reserved_cycles come back to main balance
///   3. install_code(Install) — reinstall so the cycle-return function is callable again
///   4. return_cycle_balance_to_platform_orchestrator — sends the freed reserved cycles to PO
///   5. uninstall_code — final removal; leaves only the tiny post-install reserved amount
///   6. update_settings([PO]) — set controllers to platform_orchestrator only
///
/// For canisters with no wasm (e.g. backup pool): skip cycle recovery entirely and
/// just call update_settings to set controllers to PO only.
///
/// `individual_user_wasm` — the IndividualUserWasm blob to reinstall for cycle recovery.
///   Pass None if not available; in that case no-wasm canisters skip cycle recovery.
pub async fn decommission_canister_impl(
    canister_id: Principal,
    individual_user_wasm: Option<Vec<u8>>,
) -> Result<(), String> {
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();

    let (status,) = canister_status(CanisterIdRecord { canister_id })
        .await
        .map_err(|e| e.1)?;

    let is_installed = status.module_hash.is_some();
    let reserved_cycles = u128::try_from(status.reserved_cycles.0).unwrap_or(0);

    if is_installed {
        // Pass 1: return cycles accumulated during normal operation.
        let _ = call::<_, (Result<u128, String>,)>(
            canister_id,
            "return_cycle_balance_to_platform_orchestrator",
            (),
        )
        .await;

        // Uninstall: clears heap + stable memory → reserved_cycles released to main balance.
        uninstall_code(CanisterIdRecord { canister_id })
            .await
            .map_err(|e| e.1)?;

        // Pass 2 (only if there were reserved cycles worth recovering): reinstall the wasm
        // so return_cycle_balance_to_platform_orchestrator can send the freed reserved
        // cycles to PO, then do a final uninstall. Skipped when reserved_cycles == 0 since
        // the reinstall/uninstall overhead would cost more than there is to recover.
        if reserved_cycles > 0 {
            if let Some(wasm_blob) = individual_user_wasm {
                install_code(InstallCodeArgument {
                    mode: CanisterInstallMode::Install,
                    canister_id,
                    wasm_module: wasm_blob,
                    arg: vec![],
                })
                .await
                .map_err(|e| e.1)?;

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
        }
    }
    // For no-wasm canisters (backup pool): skip cycle recovery, fall through to update_settings.

    update_settings(UpdateSettingsArgument {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![po]),
            ..Default::default()
        },
    })
    .await
    .map_err(|e| e.1)?;

    Ok(())
}
