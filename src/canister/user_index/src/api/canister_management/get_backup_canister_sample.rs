use candid::Principal;
use ic_cdk_macros::query;

use crate::CANISTER_DATA;

/// Returns up to `limit` canister IDs from the backup_canister_pool.
/// Useful for spot-checking controller state on backup pool canisters.
#[query]
pub fn get_backup_canister_sample(limit: u64) -> Vec<Principal> {
    CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .backup_canister_pool
            .iter()
            .take(limit as usize)
            .copied()
            .collect()
    })
}
