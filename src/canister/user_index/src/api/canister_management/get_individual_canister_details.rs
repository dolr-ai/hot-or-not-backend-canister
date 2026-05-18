use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::management_canister::{main::canister_status, provisional::CanisterIdRecord};
use ic_cdk_macros::update;
use serde::Serialize;

#[derive(CandidType, Serialize, Deserialize)]
pub struct IndividualCanisterDetails {
    pub controllers: Vec<Principal>,
    pub cycle_balance: u128,
}

// #[query] is not possible here: canister_status is a management canister update
// call (certified state read), so this must be an #[update].
#[update]
pub async fn get_individual_canister_details(
    canister_id: Principal,
) -> Result<IndividualCanisterDetails, String> {
    let (status,) = canister_status(CanisterIdRecord { canister_id })
        .await
        .map_err(|e| e.1)?;

    Ok(IndividualCanisterDetails {
        controllers: status.settings.controllers,
        cycle_balance: u128::try_from(status.cycles.0).map_err(|e| e.to_string())?,
    })
}
