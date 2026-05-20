use candid::Principal;
use ic_cdk::caller;
use ic_cdk_macros::update;
use shared_utils::common::utils::task::run_task_concurrently;

use super::decommission_canister_impl::decommission_canister_impl;

const AUTHORIZED_CALLER: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

const MAX_CONCURRENCY: usize = 5;

#[update]
pub fn decommission_all_controlled_canisters(
    canister_ids: Vec<Principal>,
    individual_user_wasm: Option<Vec<u8>>,
) -> Result<String, String> {
    if caller() != Principal::from_text(AUTHORIZED_CALLER).unwrap() {
        return Err("Unauthorized".into());
    }

    ic_cdk::spawn(async move {
        let futures = canister_ids.into_iter().map(|canister_id| {
            let wasm = individual_user_wasm.clone();
            async move {
                let result = decommission_canister_impl(canister_id, wasm).await;
                (canister_id, result)
            }
        });

        let result_callback = |(_canister_id, _result): (Principal, Result<(), String>)| {
            // Stateless: results not tracked in heap state.
        };

        run_task_concurrently(futures, MAX_CONCURRENCY, result_callback, || false).await;
    });

    Ok("Started".to_string())
}
