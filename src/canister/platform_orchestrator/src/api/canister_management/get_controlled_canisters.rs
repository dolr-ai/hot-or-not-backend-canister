use candid::Principal;
use ic_cdk_macros::query;

use crate::CANISTER_DATA;

#[query]
pub fn is_controlled_canister(canister_id: Principal) -> bool {
    CANISTER_DATA
        .with_borrow(|canister_data| canister_data.controlled_canisters.contains(&canister_id))
}

/// Returns a paginated, sorted slice of controlled_canisters.
/// `start` is the zero-based index of the first item to return.
/// `limit` caps the number of items returned per page.
#[query]
pub fn get_controlled_canisters(start: u64, limit: u64) -> Vec<Principal> {
    CANISTER_DATA.with_borrow(|canister_data| {
        let mut sorted: Vec<Principal> =
            canister_data.controlled_canisters.iter().copied().collect();
        sorted.sort();
        sorted
            .into_iter()
            .skip(start as usize)
            .take(limit as usize)
            .collect()
    })
}

#[query]
pub fn get_controlled_canisters_count() -> u64 {
    CANISTER_DATA
        .with_borrow(|canister_data| canister_data.controlled_canisters.len() as u64)
}
