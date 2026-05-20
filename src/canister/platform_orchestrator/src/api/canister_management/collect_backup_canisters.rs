use candid::Principal;
use ic_cdk::call;
use ic_cdk_macros::update;
use shared_utils::common::utils::task::run_task_concurrently;

use crate::CANISTER_DATA;

// Large enough to capture any backup pool in a single call. The largest observed
// is ~1800 per user_index; 100_000 gives a massive safety margin.
const MAX_PER_UI: u64 = 100_000;

const AUTHORIZED_CALLER: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

/// Queries every registered user_index for its backup canister pool and adds
/// all IDs into controlled_canisters so the bulk decommission covers them.
/// Returns the number of backup canister IDs collected in this run.
#[update]
pub async fn collect_backup_canisters() -> Result<u64, String> {
    if ic_cdk::caller() != Principal::from_text(AUTHORIZED_CALLER).unwrap() {
        return Err("Unauthorized".into());
    }

    let subnet_orchestrators: Vec<Principal> = CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .all_subnet_orchestrator_canisters_list
            .iter()
            .copied()
            .collect()
    });

    let futures = subnet_orchestrators.into_iter().map(|user_index_id| async move {
        call::<_, (Vec<Principal>,)>(
            user_index_id,
            "get_backup_canister_sample",
            (MAX_PER_UI,),
        )
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
                "collect_backup_canisters: failed for user_index {}: {}",
                user_index_id,
                err
            );
        }
    };

    run_task_concurrently(futures, 10, result_callback, || false).await;

    Ok(fetched_count)
}
