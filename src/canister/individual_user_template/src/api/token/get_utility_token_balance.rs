use ic_cdk_macros::query;

use crate::CANISTER_DATA;

#[query]
fn get_utility_token_balance() -> u64 {
    CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .my_token_balance
            .utility_token_balance
    })
}
