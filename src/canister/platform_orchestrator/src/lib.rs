use std::cell::RefCell;

use crate::api::canister_management::get_controllers_and_cycle_balance::ControlledCanisterDetails;
use crate::api::canister_management::get_decommission_status::DecommissionStatus;
use candid::Principal;
use data_model::CanisterData;
use ic_cdk_macros::export_candid;
use shared_utils::common::types::http::{HttpRequest, HttpResponse};

mod api;
mod data_model;
mod guard;
mod utils;

thread_local! {
    pub static CANISTER_DATA: RefCell<CanisterData> = RefCell::default();
}

export_candid!();
