use std::cell::RefCell;

use candid::{Principal, Nat};
use data_model::CanisterData;
use ic_cdk::api::{
    call::CallResult,
    management_canister::main::CanisterStatusResponse,
};
use ic_cdk_macros::export_candid;
use shared_utils::{
    canister_specific::user_index::types::{args::UserIndexInitArgs, BroadcastCallStatus},
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
