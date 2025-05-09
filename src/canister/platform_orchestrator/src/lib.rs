use candid::{Principal, Nat};
use std::cell::RefCell;

use std::collections::HashSet;

use crate::api::generic_proposal::{
    PlatformOrchestratorGenericArgumentType, PlatformOrchestratorGenericResultType,
};
use crate::data_model::CanisterUpgradeStatus;
use data_model::CanisterData;
use ic_cdk_macros::export_candid;
use shared_utils::{
    canister_specific::platform_orchestrator::types::args::{
        PlatformOrchestratorInitArgs, UpgradeCanisterArg,
    },
    canister_specific::platform_orchestrator::types::SubnetUpgradeReport,
    canister_specific::user_index::types::UpgradeStatus,
    common::types::http::{HttpRequest, HttpResponse},
    common::types::known_principal::KnownPrincipalType,
    common::types::wasm::WasmType,
    types::creator_dao_stats::CreatorDaoTokenStats,
};

mod api;
mod data_model;
mod guard;
mod utils;

thread_local! {
    pub static CANISTER_DATA: RefCell<CanisterData> = RefCell::default();
    pub static SNAPSHOT_DATA: RefCell<Vec<u8>> = RefCell::default();
}

export_candid!();
