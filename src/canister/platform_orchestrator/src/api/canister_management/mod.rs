use ic_cdk_macros::query;

pub mod add_our_identity_as_controller;
pub mod get_controllers_and_cycle_balance;

#[query]
pub fn get_version() -> String {
    "v21".to_string()
}
