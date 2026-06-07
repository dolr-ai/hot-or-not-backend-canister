use candid::Principal;
use ic_cdk::api::management_canister::main::CanisterInstallMode;
use ic_cdk_macros::update;
use shared_utils::{
    canister_specific::individual_user_template::types::arg::IndividualUserTemplateInitArgs,
    common::{types::wasm::WasmType, utils::permissions::is_caller_controller},
};

use crate::{
    util::canister_management::{recharge_and_install, recharge_and_upgrade},
    CANISTER_DATA,
};

#[update(guard = "is_caller_controller")]
async fn upgrade_specific_individual_user_canister_with_latest_wasm(
    user_canister_id: Principal,
    user_principal_id: Option<Principal>,
    upgrade_mode: Option<CanisterInstallMode>,
) -> Result<(), String> {
    let known_principal_ids = CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .configuration
            .known_principal_ids
            .clone()
    });

    let saved_upgrade_status = CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .last_run_upgrade_status
            .clone()
    });

    let configuration = CANISTER_DATA
        .with(|canister_data_ref_cell| canister_data_ref_cell.borrow().configuration.clone());

    let individual_canister_wasm = CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .wasms
            .get(&WasmType::IndividualUserWasm)
            .unwrap()
    });

    let pump_dump_onboarding_reward = CANISTER_DATA
        .with_borrow_mut(|canister_data| canister_data.pump_dump_onboarding_reward.clone());

    let args = IndividualUserTemplateInitArgs {
        known_principal_ids: Some(known_principal_ids.clone()),
        profile_owner: None,
        upgrade_version_number: Some(saved_upgrade_status.version_number + 1),
        url_to_send_canister_metrics_to: Some(configuration.url_to_send_canister_metrics_to),
        version: individual_canister_wasm.version,
        pump_dump_onboarding_reward: Some(pump_dump_onboarding_reward),
    };

    let result = match upgrade_mode {
        Some(CanisterInstallMode::Install) => {
            recharge_and_install(
                user_canister_id,
                user_principal_id.unwrap_or(Principal::anonymous()),
                individual_canister_wasm.wasm_blob,
                args,
            )
            .await
        }
        _ => {
            recharge_and_upgrade(
                user_canister_id,
                user_principal_id.unwrap_or(Principal::anonymous()),
                individual_canister_wasm.wasm_blob,
                args,
            )
            .await
        }
    };

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.1),
    }
}
