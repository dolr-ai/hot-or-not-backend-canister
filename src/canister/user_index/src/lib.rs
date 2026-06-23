use std::cell::RefCell;

use candid::{Principal, Nat};
use data_model::CanisterData;
use ic_cdk::api::{
    call::CallResult,
    management_canister::main::{CanisterInstallMode, CanisterStatusResponse},
};
use ic_cdk_macros::export_candid;
use api::canister_management::get_individual_canister_details::IndividualCanisterDetails;
use data_model::bulk_individual_canister_operation_status::BulkIndividualCanisterOperationStatus;
use shared_utils::{
    canister_specific::user_index::types::{
        args::UserIndexInitArgs, BroadcastCallStatus, RecycleStatus, UpgradeStatus,
    },
    common::types::http::{HttpRequest, HttpResponse},
    common::types::known_principal::KnownPrincipalType,
    types::canister_specific::user_index::error_types::SetUniqueUsernameError,
};

mod api;
mod data_model;
mod util;

thread_local! {
    static CANISTER_DATA: RefCell<CanisterData> = RefCell::default();
    static SNAPSHOT_DATA: RefCell<Vec<u8>> = RefCell::default();
}

export_candid!();
