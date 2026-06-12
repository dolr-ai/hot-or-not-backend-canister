use candid::{decode_one, CandidType, Deserialize, Encode, Nat, Principal};
use ic_cdk::api::management_canister::provisional::CanisterIdRecord;
use ic_cdk_macros::update;
use serde::Serialize;

#[derive(CandidType, Serialize, Deserialize, Debug, PartialEq)]
pub enum CanisterRunningStatus {
    Running,
    Stopping,
    Stopped,
}

// The following internal types are duplicated (with care) from the harvester's
// direct mgmt canister call definitions. This keeps the PO's get_controllers_and_cycle_balance
// independent of any ic_cdk re-exported types and ensures the decoded shape
// exactly matches the current IC management canister interface (same fields and
// Option<Vec<u8>> handling as the harvester uses for canister_status).
// This fixes the "Not a valid visitor" panic on Option<Vec<u8>> fields like module_hash.

#[derive(CandidType, Deserialize, Debug)]
enum CanisterStatus {
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "stopping")]
    Stopping,
    #[serde(rename = "stopped")]
    Stopped,
}

#[derive(CandidType, Deserialize, Debug)]
enum LogVisibility {
    #[serde(rename = "controllers")]
    Controllers,
    #[serde(rename = "public")]
    Public,
}

#[derive(CandidType, Deserialize, Debug)]
struct DefiniteCanisterSettings {
    controllers: Vec<Principal>,
    compute_allocation: Nat,
    memory_allocation: Nat,
    freezing_threshold: Nat,
    reserved_cycles_limit: Nat,
    wasm_memory_limit: Nat,
    log_visibility: LogVisibility,
}

#[derive(CandidType, Deserialize, Debug)]
struct CanisterStatusResult {
    status: CanisterStatus,
    settings: DefiniteCanisterSettings,
    module_hash: Option<Vec<u8>>,
    memory_size: Nat,
    cycles: Nat,
    idle_cycles_burned_per_day: Nat,
    reserved_cycles: Nat,
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
// We do the call manually (mimicking the harvester's direct-mgmt style with effective
// canister id semantics via the arg) and decode with the full current record shape
// so that deserialization cannot panic on Option fields or new settings.
#[update]
pub async fn get_controllers_and_cycle_balance(
    canister_id: Principal,
) -> Result<ControlledCanisterDetails, String> {
    let mgmt = Principal::from_text("aaaaa-aa").unwrap();
    let id_record = CanisterIdRecord { canister_id };

    // Encode the arg directly (same as harvester does for dfx parity and routing).
    let arg_bytes = Encode!(&id_record).map_err(|e| e.to_string())?;

    // Use call_raw so we get the response bytes and can decode with our own
    // controlled Candid types (avoiding any ic_cdk re-export type that may be stale).
    // This ensures Option<Vec<u8>> fields (module_hash etc.) decode the same way
    // the harvester's direct-mgmt code does.
    let raw_response = ic_cdk::api::call::call_raw(mgmt, "canister_status", &arg_bytes, 0)
        .await
        .map_err(|(code, msg)| format!("{}: {}", code as u8, msg))?;

    let status: CanisterStatusResult = decode_one(&raw_response)
        .map_err(|e| format!("failed to decode canister_status reply from mgmt: {}", e))?;

    let running_status = match status.status {
        CanisterStatus::Running => CanisterRunningStatus::Running,
        CanisterStatus::Stopping => CanisterRunningStatus::Stopping,
        CanisterStatus::Stopped => CanisterRunningStatus::Stopped,
    };

    Ok(ControlledCanisterDetails {
        controllers: status.settings.controllers,
        cycle_balance: u128::try_from(status.cycles.0).map_err(|e| e.to_string())?,
        reserved_cycles: u128::try_from(status.reserved_cycles.0).map_err(|e| e.to_string())?,
        status: running_status,
    })
}
