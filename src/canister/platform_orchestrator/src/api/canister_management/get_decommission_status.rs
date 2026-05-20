use ic_cdk_macros::query;

use crate::{
    data_model::long_running_task_status::LongRunningTaskStatus, CANISTER_DATA,
};

#[query]
pub fn get_decommission_status() -> LongRunningTaskStatus {
    CANISTER_DATA.with_borrow(|canister_data| canister_data.decommission_status.clone())
}
