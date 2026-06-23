use candid::Principal;
use futures::StreamExt;
use ic_cdk::{
    api::{
        call::CallResult,
        is_controller,
        management_canister::{
            main::{canister_status, CanisterStatusResponse},
            provisional::CanisterIdRecord,
        },
    },
    caller,
};
use ic_cdk_macros::{query, update};
use shared_utils::{
    canister_specific::{
        individual_user_template::types::session::SessionType, user_index::types::RecycleStatus,
    },
    common::{
        types::{
            known_principal::KnownPrincipalType,
            wasm::{CanisterWasm, WasmType},
        },
        utils::{permissions::is_reclaim_canister_id, system_time::get_current_system_time},
    },
};

use crate::{
    util::canister_management::{self, reinstall_canister_wasm},
    CANISTER_DATA,
};

pub mod add_platform_orchestrator_as_controller_to_all_canisters;
pub mod add_platform_orchestrator_as_controller_to_specific_canister;
pub mod allot_empty_canister;
pub mod create_pool_of_available_canisters;
pub mod fixup_individual_canisters_mappings;
pub mod get_backup_canister_sample;
pub mod get_bulk_operation_status;
pub mod get_individual_canister_details;
pub mod get_last_broadcast_call_status;
pub mod get_subnet_available_capacity;
pub mod get_subnet_backup_capacity;
pub mod make_individual_canister_logs_private;
pub mod make_individual_canister_logs_public;
pub mod provision_empty_canisters;
pub mod receive_empty_canister_from_individual_canister;
pub mod recharge_individual_user_canister;
pub mod recycle_canisters;
pub mod request_cycles;
pub mod reset_user_canister_ml_feed_cache;
pub mod start_upgrades_for_individual_canisters;
pub mod uninstall_individual_user_canister;
pub mod update_user_canister_restart_timers;

#[update]
pub async fn get_user_canister_status(
    canister_id: Principal,
) -> CallResult<(CanisterStatusResponse,)> {
    canister_status(CanisterIdRecord { canister_id }).await
}

#[update]
pub async fn set_permission_to_upgrade_individual_canisters(flag: bool) -> String {
    if !is_controller(&caller()) {
        return "Unauthorized Access".to_string();
    }

    CANISTER_DATA.with(|canister_data_ref| {
        canister_data_ref
            .borrow_mut()
            .allow_upgrades_for_individual_canisters = flag;
    });
    "Success".to_string()
}

#[query]
pub fn get_list_of_available_canisters() -> Vec<Principal> {
    CANISTER_DATA.with(|canister_data_ref| {
        canister_data_ref
            .borrow()
            .available_canisters
            .clone()
            .into_iter()
            .collect()
    })
}

#[query]
pub fn validate_reset_user_individual_canisters(
    _canisters: Vec<Principal>,
) -> Result<String, String> {
    let caller_id = caller();
    let governance_canister_id = CANISTER_DATA
        .with(|canister_data_ref| {
            canister_data_ref
                .borrow()
                .configuration
                .known_principal_ids
                .get(&KnownPrincipalType::CanisterIdSnsGovernance)
                .cloned()
        })
        .ok_or("Governance Canister Id not found")?;

    if caller_id != governance_canister_id {
        return Err("This Proposal can only be executed through DAO".to_string());
    };

    Ok("Success".to_string())
}

#[update(guard = "is_reclaim_canister_id")]
pub async fn reset_user_individual_canisters(canisters: Vec<Principal>) -> Result<String, String> {
    // TODO: remove this after hotornot to yral migration
    // return if principal id is `rimrc-piaaa-aaaao-aaljq-cai`
    // for a secondary measure to prevent accidental recycling of hotornot canisters
    if ic_cdk::id() == Principal::from_text("rimrc-piaaa-aaaao-aaljq-cai").unwrap() {
        return Err("This method can not be executed on this subnet".to_string());
    }

    ic_cdk::spawn(reset_canisters_impl(canisters));

    Ok("Started".to_string())
}

pub async fn reset_canisters_impl(canister_ids: Vec<Principal>) {
    let start = get_current_system_time();

    let individual_user_template_canister_wasm = CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .wasms
            .get(&WasmType::IndividualUserWasm)
            .unwrap()
            .clone()
    });

    let futures = canister_ids.iter().map(|canister_id| {
        reset_canister(*canister_id, individual_user_template_canister_wasm.clone())
    });

    let stream = futures::stream::iter(futures).boxed().buffer_unordered(25);

    let results = stream
        .collect::<Vec<Result<Principal, (Principal, String)>>>()
        .await;

    // update recycle_status

    let success_canisters = results
        .iter()
        .filter_map(|r| r.as_ref().ok().cloned())
        .collect::<Vec<Principal>>();

    let num_success = success_canisters.len();

    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data
            .available_canisters
            .extend(success_canisters.clone());

        // remove the canisters that are recycled from user_principal_id_to_canister_id_map and unique_user_name_to_user_principal_id_map
        // canister_id is the value in the map
        for canister_id in success_canisters.clone() {
            canister_data
                .user_principal_id_to_canister_id_map
                .retain(|_, v| *v != canister_id);
            canister_data
                .unique_user_name_to_user_principal_id_map
                .retain(|_, v| *v != canister_id);
        }
    });

    let failed_list = results
        .iter()
        .filter(|r| r.is_err())
        .map(|r| r.as_ref().unwrap_err().clone())
        .collect::<Vec<(Principal, String)>>();

    let end = get_current_system_time();

    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data.recycle_status = RecycleStatus {
            success_canisters: success_canisters.iter().map(|c| c.to_string()).collect(),
            last_recycled_at: Some(end),
            last_recycled_duration: Some(end.duration_since(start).unwrap().as_secs()),
            num_last_recycled_canisters: num_success as u64,
            failed_recycling: failed_list,
        };
    });
}

pub async fn reset_canister(
    canister_id: Principal,
    individual_user_template_canister_wasm: CanisterWasm,
) -> Result<Principal, (Principal, String)> {
    // secondary check for not allowing registered canisters to be recycled
    let (session_type_res,): (Result<SessionType, String>,) =
        match ic_cdk::call(canister_id, "get_session_type", ()).await {
            Ok(r) => r,
            Err(e) => return Err((canister_id, e.1)),
        };
    if let Err(e) = session_type_res {
        return Err((canister_id, e));
    }
    if session_type_res.unwrap() != SessionType::AnonymousSession {
        return Err((
            canister_id,
            "Canister is not AnonymousSession. Can not recycle".to_string(),
        ));
    }

    canister_management::recharge_canister_for_installing_wasm(canister_id)
        .await
        .map_err(|e| (canister_id, e))?;

    // reinstall wasm
    let _ = match reinstall_canister_wasm(
        canister_id,
        None,
        individual_user_template_canister_wasm.version.clone(),
        individual_user_template_canister_wasm.wasm_blob.clone(),
    )
    .await
    {
        Ok(_) => canister_id,
        Err(e) => return Err((canister_id, e)),
    };

    // return extra cycles
    let (_,): ((),) =
        match ic_cdk::call(canister_id, "return_cycles_to_user_index_canister", ()).await {
            Ok(r) => r,
            Err(e) => return Err((canister_id, e.1)),
        };

    Ok(canister_id)
}
