use candid::{CandidType, Deserialize, Nat, Principal};
use ic_cdk::api::management_canister::provisional::CanisterIdRecord;
use ic_cdk_macros::update;
use serde::Serialize;

#[derive(CandidType, Serialize, Deserialize, Debug, PartialEq)]
pub enum CanisterRunningStatus {
    Running,
    Stopping,
    Stopped,
}

#[derive(CandidType, Serialize, Deserialize)]
pub struct ControlledCanisterDetails {
    pub controllers: Vec<Principal>,
    pub cycle_balance: u128,
    /// Cycles held in the reserved balance for future storage payments.
    /// Released back to the main balance when memory is freed (e.g. after uninstall_code).
    pub reserved_cycles: u128,
    /// Whether the canister is Running, Stopping, or Stopped. Stopped means it is
    /// frozen (ran out of cycles below the freezing threshold) and cannot process calls.
    pub status: CanisterRunningStatus,
}

// #[query] is not possible: canister_status is a management canister update call.
#[update]
pub async fn get_controllers_and_cycle_balance(
    canister_id: Principal,
) -> Result<ControlledCanisterDetails, String> {
    let (status,) = canister_status(CanisterIdRecord { canister_id })
        .await
        .map_err(|e| e.1)?;

    let running_status = match status.status {
        CanisterStatusType::Running => CanisterRunningStatus::Running,
        CanisterStatusType::Stopping => CanisterRunningStatus::Stopping,
        CanisterStatusType::Stopped => CanisterRunningStatus::Stopped,
    };

    Ok(ControlledCanisterDetails {
        controllers: status.settings.controllers,
        cycle_balance: u128::try_from(status.cycles.0).map_err(|e| e.to_string())?,
        reserved_cycles: u128::try_from(status.reserved_cycles.0).map_err(|e| e.to_string())?,
        status: running_status,
    })
}
