use candid::Principal;
use ic_cdk::{caller, caller as get_caller};
use ic_cdk_macros::update;

use super::decommission_canister_impl::decommission_canister_impl;

const AUTHORIZED_CALLER: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

#[update]
pub async fn decommission_individual_canister(canister_id: Principal) -> Result<(), String> {
    if caller() != Principal::from_text(AUTHORIZED_CALLER).unwrap() {
        return Err("Unauthorized".into());
    }
    decommission_canister_impl(canister_id).await
}
