use candid::Principal;
use ic_cdk::{
    api::management_canister::main::{update_settings, CanisterSettings, UpdateSettingsArgument},
    caller, update,
};

/// Principal of the actions identity that runs task_runner tests.
const ACTIONS_PRINCIPAL: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

/// Platform orchestrator canister ID.
const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

/// Adds our actions identity as a co-controller on the target canister.
/// Sets controllers to [PO, ACTIONS_PRINCIPAL] — no canister_status fetch needed
/// since we know these are PO-controlled canisters.
#[update]
pub async fn add_our_identity_as_controller(canister_id: Principal) -> Result<(), String> {
    if caller() != Principal::from_text(ACTIONS_PRINCIPAL).map_err(|e| e.to_string())? {
        return Err("Unauthorized".into());
    }

    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).map_err(|e| e.to_string())?;
    let actions = Principal::from_text(ACTIONS_PRINCIPAL).map_err(|e| e.to_string())?;

    update_settings(UpdateSettingsArgument {
        canister_id,
        settings: CanisterSettings {
            controllers: Some(vec![po, actions]),
            ..Default::default()
        },
    })
    .await
    .map_err(|e| e.1)?;

    Ok(())
}
