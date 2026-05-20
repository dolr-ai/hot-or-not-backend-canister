use candid::Principal;
use ic_cdk::{caller};
use ic_cdk_macros::update;
use shared_utils::common::utils::task::run_task_concurrently;

use super::decommission_canister_impl::decommission_canister_impl;
use crate::CANISTER_DATA;

const AUTHORIZED_CALLER: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

const MAX_CONCURRENCY: usize = 5;

#[update]
pub fn decommission_all_controlled_canisters() -> Result<String, String> {
    if caller() != Principal::from_text(AUTHORIZED_CALLER).unwrap() {
        return Err("Unauthorized".into());
    }

    let canister_ids: Vec<Principal> = CANISTER_DATA.with_borrow(|canister_data| {
        canister_data.controlled_canisters.iter().copied().collect()
    });

    // Initialise status — reset on every invocation so the query reflects this run.
    CANISTER_DATA.with_borrow_mut(|canister_data| {
        let status = &mut canister_data.decommission_status;
        status.canisters_remaining = canister_ids.iter().copied().collect();
        status.completed_count = 0;
        status.failed_canisters = Vec::new();
    });

    ic_cdk::spawn(async move {
        let futures = canister_ids.into_iter().map(|canister_id| async move {
            let result = decommission_canister_impl(canister_id).await;
            (canister_id, result)
        });

        let result_callback = |(canister_id, result): (Principal, Result<(), String>)| {
            CANISTER_DATA.with_borrow_mut(|canister_data| {
                let status = &mut canister_data.decommission_status;
                status.canisters_remaining.remove(&canister_id);
                match result {
                    Ok(()) => status.completed_count += 1,
                    Err(reason) => status.failed_canisters.push((canister_id, reason)),
                }
            });
        };

        run_task_concurrently(futures, MAX_CONCURRENCY, result_callback, || false).await;
    });

    Ok("Started".to_string())
}
