use candid::Principal;
use ic_cdk::{
    api::{self, canister_balance128},
    call,
};
use shared_utils::{
    common::types::known_principal::KnownPrincipalType,
    constant::SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD,
};

use crate::CANISTER_DATA;

pub async fn check_and_request_cycles_from_platform_orchestrator() -> Result<(), String> {
    let current_cycle_balance = canister_balance128();

    if current_cycle_balance < SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD {
        let platform_orchestrator = CANISTER_DATA.with_borrow(|canister_data| {
            canister_data
                .configuration
                .known_principal_ids
                .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
                .cloned()
        });

        let platform_orchestrator_canister_id = platform_orchestrator
            .ok_or(String::from("Platform orchestrator canister id not found"))?;

        let (res,): (Result<(), String>,) = call(
            platform_orchestrator_canister_id,
            "recharge_subnet_orchestrator",
            (),
        )
        .await
        .map_err(|err| err.1)?;

        return res;
    }

    Ok(())
}

pub async fn set_controller_with_platform_orchestrator(
    canister_id_being_updated: Principal,
    platform_orchestrator: Principal,
) -> Result<(), String> {
    ic_cdk::api::management_canister::main::update_settings(
        ic_cdk::api::management_canister::main::UpdateSettingsArgument {
            canister_id: canister_id_being_updated,
            settings: ic_cdk::api::management_canister::main::CanisterSettings {
                controllers: Some(vec![api::id(), platform_orchestrator]),
                ..Default::default()
            },
        },
    )
    .await
    .map_err(|e| e.1)
}
