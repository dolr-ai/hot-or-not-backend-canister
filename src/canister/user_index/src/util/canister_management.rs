use candid::{CandidType, Principal};
use futures::{future::BoxFuture, FutureExt};
use ic_cdk::{
    api::{
        self,
        call::RejectionCode,
        canister_balance128,
        management_canister::{
            main::{
                self, canister_info, canister_status, CanisterInfoRequest, CanisterInstallMode,
                CreateCanisterArgument, InstallCodeArgument, WasmModule,
            },
            provisional::{CanisterIdRecord, CanisterSettings},
        },
    },
    call,
};
use serde::{Deserialize, Serialize};
use shared_utils::{
    canister_specific::individual_user_template::types::arg::IndividualUserTemplateInitArgs,
    common::{
        types::known_principal::KnownPrincipalType,
        utils::{task::run_task_concurrently, upgrade_canister::upgrade_canister_util},
    },
    constant::{
        BASE_INDIVIDUAL_USER_CANISTER_RECHARGE_AMOUNT, EMPTY_CANISTER_RECHARGE_AMOUNT,
        SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD,
    },
    cycles::calculate_required_cycles_for_upgrading,
};

use crate::{data_model::configuration::Configuration, CANISTER_DATA};

#[derive(
    CandidType, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone,
)]
struct CustomInstallCodeArgument {
    /// See [CanisterInstallMode].
    pub mode: CanisterInstallMode,
    /// Principle of the canister.
    pub canister_id: Principal,
    /// Code to be installed.
    pub wasm_module: WasmModule,
    /// The argument to be passed to `canister_init` or `canister_post_upgrade`.
    pub arg: Vec<u8>,
    /// sender_canister_version must be set to ic_cdk::api::canister_version()
    pub sender_canister_version: Option<u64>,
    /// drop stable memory after install/upgrade execution.
    pub unsafe_drop_stable_memory: Option<bool>,
}

pub async fn create_users_canister(
    profile_owner: Option<Principal>,
    version: String,
    individual_user_wasm: Vec<u8>,
) -> Principal {
    let canister_id = create_empty_user_canister().await;
    recharge_canister_for_installing_wasm(canister_id).await;
    install_canister_wasm(canister_id, profile_owner, version, individual_user_wasm).await;
    canister_id
}

pub async fn create_empty_user_canister() -> Principal {
    // * config for provisioning canister
    let arg = CreateCanisterArgument {
        settings: Some(CanisterSettings {
            controllers: Some(vec![
                // * this subnet_orchestrator canister
                api::id(),
            ]),
            ..Default::default()
        }),
    };

    let _ = check_and_request_cycles_from_platform_orchestrator().await;

    // * provisioned canister
    let canister_id: Principal = main::create_canister(arg, EMPTY_CANISTER_RECHARGE_AMOUNT)
        .await
        .unwrap()
        .0
        .canister_id;

    canister_id
}

pub async fn provision_number_of_empty_canisters(
    number_of_canisters: u64,
    breaking_condition: impl Fn() -> bool,
) {
    let create_canister_futures = (0..number_of_canisters).map(|_| {
        let future = create_empty_user_canister();
        future
    });

    let result_callback = |canister_id: Principal| {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            canister_data.backup_canister_pool.insert(canister_id)
        });
    };

    run_task_concurrently(
        create_canister_futures.into_iter(),
        10,
        result_callback,
        breaking_condition,
    )
    .await;
}

pub async fn install_canister_wasm(
    canister_id: Principal,
    profile_owner: Option<Principal>,
    version: String,
    wasm: Vec<u8>,
) -> Result<Principal, (Principal, String)> {
    let configuration = CANISTER_DATA
        .with(|canister_data_ref_cell| canister_data_ref_cell.borrow().configuration.clone());

    let pump_dump_onboarding_reward =
        Some(CANISTER_DATA.with_borrow(|cdata| cdata.pump_dump_onboarding_reward.clone()));

    let individual_user_tempalate_init_args = IndividualUserTemplateInitArgs {
        profile_owner,
        known_principal_ids: Some(CANISTER_DATA.with(|canister_data_ref_cell| {
            canister_data_ref_cell
                .borrow()
                .configuration
                .known_principal_ids
                .clone()
        })),
        upgrade_version_number: Some(0),
        version,
        url_to_send_canister_metrics_to: Some(configuration.url_to_send_canister_metrics_to),
        pump_dump_onboarding_reward,
    };

    // * encode argument for user canister init lifecycle method
    let arg = candid::encode_one(individual_user_tempalate_init_args)
        .expect("Failed to serialize the install argument.");

    // * install wasm to provisioned canister
    main::install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Install,
        canister_id,
        wasm_module: wasm,
        arg,
    })
    .await
    .map_err(|e| (canister_id, e.1))?;

    Ok(canister_id)
}

pub async fn reinstall_canister_wasm(
    canister_id: Principal,
    profile_owner: Option<Principal>,
    version: String,
    wasm: Vec<u8>,
) -> Result<Principal, String> {
    let configuration = CANISTER_DATA
        .with(|canister_data_ref_cell| canister_data_ref_cell.borrow().configuration.clone());

    let pump_dump_onboarding_reward =
        Some(CANISTER_DATA.with_borrow(|cdata| cdata.pump_dump_onboarding_reward.clone()));

    let individual_user_tempalate_init_args = IndividualUserTemplateInitArgs {
        profile_owner,
        known_principal_ids: Some(CANISTER_DATA.with(|canister_data_ref_cell| {
            canister_data_ref_cell
                .borrow()
                .configuration
                .known_principal_ids
                .clone()
        })),
        upgrade_version_number: Some(0),
        version,
        url_to_send_canister_metrics_to: Some(configuration.url_to_send_canister_metrics_to),
        pump_dump_onboarding_reward,
    };

    // * encode argument for user canister init lifecycle method
    let arg = match candid::encode_one(individual_user_tempalate_init_args) {
        Ok(arg) => arg,
        Err(err) => return Err(err.to_string()),
    };

    // * install wasm to provisioned canister
    match main::install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Reinstall,
        canister_id,
        wasm_module: wasm,
        arg,
    })
    .await
    {
        Ok(_) => Ok(canister_id),
        Err(err) => Err(err.1),
    }
}

pub async fn check_and_request_cycles_from_platform_orchestrator() -> Result<(), String> {
    let current_cycle_balance = canister_balance128();

    if current_cycle_balance < SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD {
        let platform_orchestrator = CANISTER_DATA.with_borrow(|canister_data| {
            canister_data
                .configuration
                .known_principal_ids
                .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
                .cloned()
        });

        let platform_orchestrator_canister_id = platform_orchestrator
            .ok_or(String::from("Platform orchestrator canister id not found"))?;

        let (res,): (Result<(), String>,) = call(
            platform_orchestrator_canister_id,
            "recharge_subnet_orchestrator",
            (),
        )
        .await
        .map_err(|err| err.1)?;

        return res;
    }

    Ok(())
}

pub async fn recharge_canister(
    canister_id: &Principal,
    recharge_amount: u128,
) -> Result<(), String> {
    let result = main::deposit_cycles(
        CanisterIdRecord {
            canister_id: *canister_id,
        },
        recharge_amount,
    )
    .await
    .map_err(|e| e.1);

    let _ = check_and_request_cycles_from_platform_orchestrator().await;

    result
}

pub(crate) async fn recharge_canister_for_installing_wasm(
    canister_id: Principal,
) -> Result<(), String> {
    recharge_canister_for_installing_wasm_with_retries(canister_id, None, 1).await
}

fn recharge_canister_for_installing_wasm_with_retries(
    canister_id: Principal,
    retry_attempt: Option<u32>,
    max_retry_attempts: u32,
) -> BoxFuture<'static, Result<(), String>> {
    async move {
        let canister_status_res = canister_status(CanisterIdRecord { canister_id }).await;
        match canister_status_res {
            Ok((canister_status,)) => {
                let idle_cycles_burned_per_day =
                    u128::try_from(canister_status.idle_cycles_burned_per_day.0)
                        .map_err(|e| e.to_string())?;
                let amount_required_to_upgrade_canister =
                    calculate_required_cycles_for_upgrading(idle_cycles_burned_per_day, None);

                if canister_status.cycles < amount_required_to_upgrade_canister {
                    recharge_canister(&canister_id, amount_required_to_upgrade_canister).await?;
                }
                Ok(())
            }
            Err(e) => {
                if let Some(retry_attempt) = retry_attempt {
                    if retry_attempt >= max_retry_attempts {
                        return Err(e.1);
                    }
                }

                //canister might be out of cycles recharge with base amount and retry.
                recharge_canister(&canister_id, BASE_INDIVIDUAL_USER_CANISTER_RECHARGE_AMOUNT)
                    .await?;
                let retry_attempt = retry_attempt.unwrap_or(0) + 1;
                Box::pin(recharge_canister_for_installing_wasm_with_retries(
                    canister_id,
                    Some(retry_attempt),
                    max_retry_attempts,
                ))
                .await
            }
        }
    }
    .boxed()
}

async fn upgrade_individual_user_canister(
    canister_id: Principal,
    install_mode: CanisterInstallMode,
    arg: IndividualUserTemplateInitArgs,
    individual_user_wasm: Vec<u8>,
) -> Result<(), (RejectionCode, String)> {
    let serialized_arg =
        candid::encode_args((arg,)).expect("Failed to serialize the install argument.");

    let install_code_argument = InstallCodeArgument {
        mode: install_mode,
        canister_id,
        wasm_module: individual_user_wasm,
        arg: serialized_arg,
    };

    upgrade_canister_util(install_code_argument).await
}

pub(crate) async fn recharge_and_upgrade(
    user_canister_id: Principal,
    user_principal_id: Principal,
    individual_user_wasm: Vec<u8>,
    individual_user_tempalte_args: IndividualUserTemplateInitArgs,
) -> Result<(Principal, Principal), ((Principal, Principal), String)> {
    check_controller_and_update_controller(user_canister_id)
        .await
        .map_err(|e| ((user_principal_id, user_canister_id), e))?;

    recharge_canister_for_installing_wasm(user_canister_id)
        .await
        .map_err(|e| ((user_principal_id, user_canister_id), e))?;

    upgrade_user_canister(
        user_canister_id,
        individual_user_tempalte_args,
        individual_user_wasm,
    )
    .await
    .map_err(|s| ((user_principal_id, user_canister_id), s))?;

    Ok((user_principal_id, user_canister_id))
}

async fn check_controller_and_update_controller(canister_id: Principal) -> Result<(), String> {
    let (canister_info,) = canister_info(CanisterInfoRequest {
        canister_id,
        num_requested_changes: None,
    })
    .await
    .map_err(|e| e.1)?;

    if canister_info.controllers.contains(&ic_cdk::id()) {
        return Ok(());
    }

    let (_canister_version,) = call::<_, (String,)>(canister_id, "get_version", ())
        .await
        .map_err(|e| e.1)?;

    call::<_, ()>(
        canister_info.controllers[0],
        "set_controller_as_subnet_orchestrator",
        (canister_id,),
    )
    .await
    .map_err(|e| e.1)
}

async fn upgrade_user_canister(
    canister_id: Principal,
    individual_user_tempalte_args: IndividualUserTemplateInitArgs,
    individual_user_wasm: Vec<u8>,
) -> Result<(), String> {
    upgrade_individual_user_canister(
        canister_id,
        CanisterInstallMode::Upgrade(None),
        individual_user_tempalte_args,
        individual_user_wasm,
    )
    .await
    .map_err(|e| e.1)
}

/// Like recharge_and_upgrade but uses Install mode — for canisters that have
/// no wasm installed and therefore cannot be upgraded in place.
pub(crate) async fn recharge_and_install(
    user_canister_id: Principal,
    user_principal_id: Principal,
    individual_user_wasm: Vec<u8>,
    individual_user_template_args: IndividualUserTemplateInitArgs,
) -> Result<(Principal, Principal), ((Principal, Principal), String)> {
    recharge_canister_for_installing_wasm(user_canister_id)
        .await
        .map_err(|e| ((user_principal_id, user_canister_id), e))?;

    upgrade_individual_user_canister(
        user_canister_id,
        CanisterInstallMode::Install,
        individual_user_template_args,
        individual_user_wasm,
    )
    .await
    .map_err(|e| ((user_principal_id, user_canister_id), e.1))?;

    Ok((user_principal_id, user_canister_id))
}

pub async fn set_controller_with_platform_orchestrator(
    canister_id_being_updated: Principal,
    platform_orchestrator: Principal,
) -> Result<(), String> {
    main::update_settings(main::UpdateSettingsArgument {
        canister_id: canister_id_being_updated,
        settings: main::CanisterSettings {
            controllers: Some(vec![api::id(), platform_orchestrator]),
            ..Default::default()
        },
    })
    .await
    .map_err(|e| e.1)
}
