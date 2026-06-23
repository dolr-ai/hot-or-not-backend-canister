use candid::Principal;
use ic_cdk::{
    api::{
        call::CallResult,
        management_canister::{
            main::{canister_status, CanisterStatusResponse},
            provisional::CanisterIdRecord,
        },
    },
};
use ic_cdk_macros::update;

pub mod add_platform_orchestrator_as_controller_to_all_canisters;
pub mod add_platform_orchestrator_as_controller_to_specific_canister;
pub mod get_backup_canister_sample;
pub mod get_bulk_operation_status;
pub mod get_last_broadcast_call_status;
pub mod get_subnet_available_capacity;
pub mod get_subnet_backup_capacity;
pub mod request_cycles;

#[update]
pub async fn get_user_canister_status(
    canister_id: Principal,
) -> CallResult<(CanisterStatusResponse,)> {
    canister_status(CanisterIdRecord { canister_id }).await
}
