use ic_cdk_macros::query;

pub mod decommission_all_controlled_canisters;
mod decommission_canister_impl;
pub mod decommission_individual_canister;
pub mod install_individual_user_wasm;
pub mod get_controllers_and_cycle_balance;
pub mod get_decommission_status;

#[query]
pub fn get_version() -> String {
    "v20".to_string()
}
