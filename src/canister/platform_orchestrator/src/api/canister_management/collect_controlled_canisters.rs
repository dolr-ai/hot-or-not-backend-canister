use candid::Principal;
use ic_cdk::call;
use ic_cdk_macros::update;
use shared_utils::common::utils::{permissions::is_caller_controller, task::run_task_concurrently};

use crate::CANISTER_DATA;

/// Queries every registered user_index for its full individual canister list
/// (assigned users + available pool) and stores all results in controlled_canisters.
/// Returns the total number of controlled canisters after collection.
#[update(guard = "is_caller_controller")]
pub async fn collect_controlled_canisters() -> u64 {
    let subnet_orchestrators: Vec<Principal> = CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .all_subnet_orchestrator_canisters_list
            .iter()
            .copied()
            .collect()
    });

    let futures = subnet_orchestrators.into_iter().map(|user_index_id| async move {
        call::<_, (Vec<Principal>,)>(user_index_id, "get_user_canister_incl_avail_list", ())
            .await
            .map(|(canisters,)| canisters)
            .map_err(|e| (user_index_id, e.1))
    });

    let mut fetched_count: u64 = 0;

    let result_callback = |res: Result<Vec<Principal>, (Principal, String)>| match res {
        Ok(canisters) => {
            fetched_count += canisters.len() as u64;
            CANISTER_DATA.with_borrow_mut(|canister_data| {
                canister_data.controlled_canisters.extend(canisters);
            });
        }
        Err((user_index_id, err)) => {
            ic_cdk::println!(
                "collect_controlled_canisters: failed for user_index {}: {}",
                user_index_id,
                err
            );
        }
    };

    run_task_concurrently(futures, 10, result_callback, || false).await;

    fetched_count
}
