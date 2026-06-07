use std::time::SystemTime;

use candid::Principal;
use ic_cdk::call;

use shared_utils::{
    canister_specific::{
        individual_user_template::types::arg::IndividualUserTemplateInitArgs,
        user_index::types::UpgradeStatus,
    },
    common::{
        types::known_principal::KnownPrincipalType,
        utils::{system_time, task},
    },
};

use crate::{
    data_model::CanisterData, util::canister_management::recharge_and_upgrade, CANISTER_DATA,
};

const MAX_CONCURRENCY: usize = 11;

pub async fn upgrade_user_canisters_with_latest_wasm(
    _version: String,
    individual_user_wasm: Vec<u8>,
) {
    let mut upgrade_count = 0;
    let mut failed_canister_ids = Vec::new();

    let mut user_principal_id_to_canister_id_vec: Vec<(Principal, Principal)> =
        CANISTER_DATA.with(|canister_data_ref_cell| {
            canister_data_ref_cell
                .borrow()
                .user_principal_id_to_canister_id_map
                .clone()
                .into_iter()
                .collect()
        });

    CANISTER_DATA.with_borrow(|canister_data| {
        canister_data
            .available_canisters
            .iter()
            .for_each(|canister_id| {
                user_principal_id_to_canister_id_vec.push((Principal::anonymous(), *canister_id));
            })
    });

    let saved_upgrade_status = CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .last_run_upgrade_status
            .clone()
    });

    let configuration = CANISTER_DATA
        .with(|canister_data_ref_cell| canister_data_ref_cell.borrow().configuration.clone());

    let token_bet_game_onboarding_reward = CANISTER_DATA
        .with_borrow_mut(|canister_data| canister_data.pump_dump_onboarding_reward.clone());

    let individual_user_template_upgrade_args = IndividualUserTemplateInitArgs {
        known_principal_ids: Some(configuration.known_principal_ids.clone()),
        profile_owner: None,
        upgrade_version_number: Some(saved_upgrade_status.version_number + 1),
        url_to_send_canister_metrics_to: Some(configuration.url_to_send_canister_metrics_to),
        version: saved_upgrade_status.version.clone(),
        pump_dump_onboarding_reward: Some(token_bet_game_onboarding_reward),
    };

    let upgrade_individual_canister_futures =
        user_principal_id_to_canister_id_vec
            .iter()
            .map(|(user_principal_id, user_canister_id)| {
                recharge_and_upgrade(
                    *user_canister_id,
                    *user_principal_id,
                    individual_user_wasm.clone(),
                    individual_user_template_upgrade_args.clone(),
                )
            });

    let result_callback =
        |upgrade_result: Result<(Principal, Principal), ((Principal, Principal), String)>| {
            if upgrade_result.is_err() {
                let ((done_user_principal_id, done_user_canister_id), err) =
                    upgrade_result.err().unwrap();
                ic_cdk::print(format!(
                    "Failed to upgrade canister: {:?} with error: {:?}",
                    done_user_canister_id.to_text(),
                    err
                ));
                failed_canister_ids.push((done_user_principal_id, done_user_canister_id, err));
            }

            upgrade_count += 1;
            CANISTER_DATA.with(|canister_data_ref_cell| {
                update_upgrade_status(
                    &mut canister_data_ref_cell.borrow_mut(),
                    upgrade_count,
                    &failed_canister_ids,
                    None,
                    None,
                );
            });
        };

    let breaking_condition = || {
        !CANISTER_DATA.with(|canister_data_ref| {
            canister_data_ref
                .borrow()
                .allow_upgrades_for_individual_canisters
        })
    };

    task::run_task_concurrently(
        upgrade_individual_canister_futures,
        MAX_CONCURRENCY,
        result_callback,
        breaking_condition,
    )
    .await;

    CANISTER_DATA.with(|canister_data_ref_cell| {
        update_upgrade_status(
            &mut canister_data_ref_cell.borrow_mut(),
            upgrade_count,
            &failed_canister_ids,
            Some(saved_upgrade_status.version_number + 1),
            Some(system_time::get_current_system_time_from_ic()),
        );
    });

    let upgrade_status =
        CANISTER_DATA.with_borrow(|canister_data| canister_data.last_run_upgrade_status.clone());

    send_upgrade_report_to_platform_orchestrator(upgrade_status).await;
}

async fn send_upgrade_report_to_platform_orchestrator(subnet_upgrade_status: UpgradeStatus) {
    let platform_orchestrator_canister_id = CANISTER_DATA
        .with_borrow(|canister_data| {
            canister_data
                .configuration
                .known_principal_ids
                .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
                .cloned()
        })
        .expect("Platform Orchestrator Canister Id to be Present");

    let _ = call::<_, (Result<(), String>,)>(
        platform_orchestrator_canister_id,
        "report_subnet_upgrade_status",
        (subnet_upgrade_status,),
    )
    .await;
}

fn update_upgrade_status(
    canister_data: &mut CanisterData,
    upgrade_count: u32,
    failed_canister_ids: &[(Principal, Principal, String)],
    version_number: Option<u64>,
    last_run_on: Option<SystemTime>,
) {
    let mut last_run_upgrade_status = canister_data.last_run_upgrade_status.clone();

    last_run_upgrade_status.successful_upgrade_count = upgrade_count;
    last_run_upgrade_status.failed_canister_ids = failed_canister_ids.to_owned();
    last_run_upgrade_status.version_number =
        version_number.unwrap_or(canister_data.last_run_upgrade_status.version_number);
    last_run_upgrade_status.last_run_on =
        last_run_on.unwrap_or(canister_data.last_run_upgrade_status.last_run_on);

    canister_data.last_run_upgrade_status = last_run_upgrade_status;
}
