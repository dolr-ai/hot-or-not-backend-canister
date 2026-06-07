use candid::Principal;
use ic_cdk_macros::update;
use shared_utils::{
    canister_specific::individual_user_template::types::arg::IndividualUserTemplateInitArgs,
    common::utils::permissions::is_caller_controller,
};

use crate::{util::canister_management::recharge_and_upgrade, CANISTER_DATA};

#[update(guard = "is_caller_controller")]
async fn upgrade_specific_individual_canister_with_wasm_version(
    individual_user_canister_id: Principal,
    version: String,
    wasm_blob: Vec<u8>,
) -> Result<(), String> {
    let known_principal_ids = CANISTER_DATA
        .with_borrow(|canister_data| canister_data.configuration.known_principal_ids.clone());

    let pump_dump_onboarding_reward = CANISTER_DATA
        .with_borrow(|canister_data| canister_data.pump_dump_onboarding_reward.clone());

    recharge_and_upgrade(
        individual_user_canister_id,
        Principal::anonymous(),
        wasm_blob,
        IndividualUserTemplateInitArgs {
            known_principal_ids: Some(known_principal_ids.clone()),
            profile_owner: None,
            upgrade_version_number: None,
            url_to_send_canister_metrics_to: None,
            version,
            pump_dump_onboarding_reward: Some(pump_dump_onboarding_reward),
        },
    )
    .await
    .map_err(|e| e.1)?;

    Ok(())
}
