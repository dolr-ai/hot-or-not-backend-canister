use candid::Principal;
use ic_cdk::call;
use ic_cdk_macros::update;
use shared_utils::{
    canister_specific::user_index::types::BroadcastCallStatus,
    common::{
        types::known_principal::KnownPrincipalType,
        utils::{
            permissions::is_caller_controller, system_time::get_current_system_time,
            task::run_task_concurrently,
        },
    },
};

use crate::CANISTER_DATA;

#[update(guard = "is_caller_controller")]
fn update_well_known_principal(known_principal_type: KnownPrincipalType, value: Principal) {
    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data
            .configuration
            .known_principal_ids
            .insert(known_principal_type, value);
    });

    ic_cdk::spawn(issue_update_known_principal_for_individual_canisters(
        known_principal_type,
        value,
    ));
}

async fn issue_update_known_principal_for_individual_canisters(
    known_principal_type: KnownPrincipalType,
    value: Principal,
) {
    let all_canisters = CANISTER_DATA.with_borrow(|canister_data| {
        let mut all_canisters: Vec<Principal> = canister_data
            .user_principal_id_to_canister_id_map
            .values()
            .copied()
            .collect();
        let mut available_canisters: Vec<Principal> =
            canister_data.available_canisters.iter().copied().collect();
        all_canisters.append(&mut available_canisters);
        all_canisters
    });

    let futures = all_canisters.iter().map(|individual_canister| async {
        let res = call::<_, ()>(
            *individual_canister,
            "update_well_known_principal",
            (known_principal_type, value),
        )
        .await;

        match res {
            Ok(()) => Ok(*individual_canister),
            Err(e) => Err((*individual_canister, e.1)),
        }
    });

    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data.last_broadcast_call_status = BroadcastCallStatus {
            method_name: "update_well_known_principal".into(),
            timestamp: get_current_system_time(),
            ..Default::default()
        }
    });

    let result_callback = |res| {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            match res {
                Ok(canister_id) => {
                    canister_data
                        .last_broadcast_call_status
                        .successful_canister_ids
                        .push(canister_id);

                    canister_data
                        .last_broadcast_call_status
                        .successful_canisters_count += 1;
                }
                Err(e) => {
                    canister_data
                        .last_broadcast_call_status
                        .failed_canister_ids
                        .push(e);
                    canister_data
                        .last_broadcast_call_status
                        .failed_canisters_count += 1;
                }
            }
            canister_data.last_broadcast_call_status.total_canisters += 1;
        })
    };

    run_task_concurrently(futures, 10, result_callback, || false).await;
}
