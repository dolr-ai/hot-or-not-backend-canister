use candid::Principal;
use ic_cdk::api::management_canister::main::{
    canister_status, update_settings, CanisterIdRecord, CanisterSettings, LogVisibility,
    UpdateSettingsArgument,
};
use serde::{Deserialize, Serialize};
use shared_utils::cycles::calculate_threshold_and_recharge_cycles_for_canister;

use crate::{util::canister_management::recharge_canister, CANISTER_DATA};

use super::subnet_orchestrator_operation::SubnetOrchestratorOperation;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisteredIndividualUserCanister {
    pub canister_id: Principal,
    pub profile_id: Principal,
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub(crate) struct RechargeIndividualUserCanister {
    canister_id: Principal,
}

impl RechargeIndividualUserCanister {
    fn new(canister_id: Principal) -> Result<Self, String> {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            if canister_data
                .on_going_operation
                .contains(&SubnetOrchestratorOperation::RechargeIndividualUserCanister(canister_id))
            {
                Err(format!(
                    "ongoing operation for recharging individual user canister {}",
                    canister_id,
                ))
            } else {
                canister_data.on_going_operation.insert(
                    SubnetOrchestratorOperation::RechargeIndividualUserCanister(canister_id),
                );

                Ok(Self { canister_id })
            }
        })
    }
    async fn recharge_canister(&self) -> Result<(), String> {
        let (user_canister_status,) = canister_status(CanisterIdRecord {
            canister_id: self.canister_id,
        })
        .await
        .map_err(|e| e.1)?;

        let idle_cycles_burned_in_a_day =
            u128::try_from(user_canister_status.idle_cycles_burned_per_day.0)
                .map_err(|e| e.to_string())?;
        let reserved_cycles =
            u128::try_from(user_canister_status.reserved_cycles.0).map_err(|e| e.to_string())?;
        let current_user_canister_balance =
            u128::try_from(user_canister_status.cycles.0).map_err(|e| e.to_string())?;
        let (threshold, recharge_amount) = calculate_threshold_and_recharge_cycles_for_canister(
            idle_cycles_burned_in_a_day,
            reserved_cycles,
            None,
        );

        if current_user_canister_balance <= threshold {
            return recharge_canister(&self.canister_id, recharge_amount).await;
        }

        Ok(())
    }
}

impl Drop for RechargeIndividualUserCanister {
    fn drop(&mut self) {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            canister_data.on_going_operation.remove(
                &SubnetOrchestratorOperation::RechargeIndividualUserCanister(self.canister_id),
            )
        });
    }
}

impl RegisteredIndividualUserCanister {
    pub fn new(canister_id: Principal) -> Result<Self, String> {
        let res = CANISTER_DATA.with_borrow(|canister_data| {
            canister_data
                .user_principal_id_to_canister_id_map
                .iter()
                .find(move |(_, &user_canister)| user_canister == canister_id)
                .map(|entry| (*entry.0, *entry.1))
        });

        if let Some((user_profile_id, user_canister_id)) = res {
            Ok(Self {
                canister_id: user_canister_id,
                profile_id: user_profile_id,
            })
        } else {
            Err(format!("Canister Id {canister_id} not found in the subnet"))
        }
    }

    pub async fn recharge_individual_canister(&self) -> Result<(), String> {
        let recharge_individual_canister_operation =
            RechargeIndividualUserCanister::new(self.canister_id)?;

        recharge_individual_canister_operation
            .recharge_canister()
            .await
    }

    pub async fn make_individual_canister_logs_public(&self) -> Result<(), String> {
        update_settings(UpdateSettingsArgument {
            canister_id: self.canister_id,
            settings: CanisterSettings {
                log_visibility: Some(LogVisibility::Public),
                ..Default::default()
            },
        })
        .await
        .map_err(|e| e.1)
    }

    pub async fn make_indvidual_canister_logs_private(&self) -> Result<(), String> {
        update_settings(UpdateSettingsArgument {
            canister_id: self.canister_id,
            settings: CanisterSettings {
                log_visibility: Some(LogVisibility::Controllers),
                ..Default::default()
            },
        })
        .await
        .map_err(|e| e.1)
    }

    pub async fn allot_empty_canister(&self) -> Result<Principal, String> {
        let alloted_canister_id_res = CANISTER_DATA.with_borrow_mut(|canister_data| {
            let Some(new_canister_id) = canister_data.backup_canister_pool.iter().next().copied()
            else {
                return Err("No Backup Canisters Available".into());
            };

            canister_data.backup_canister_pool.remove(&new_canister_id);

            Ok(new_canister_id)
        });

        //try to update controller of the canister
        if let Ok(canister_id) = alloted_canister_id_res {
            update_settings(UpdateSettingsArgument {
                canister_id,
                settings: CanisterSettings {
                    controllers: Some(vec![self.canister_id]),
                    ..Default::default()
                },
            })
            .await
            .inspect_err(|_| {
                CANISTER_DATA.with_borrow_mut(|canister_data| {
                    canister_data.backup_canister_pool.insert(canister_id)
                });
            })
            .map_err(|e| e.1)?;
        }

        alloted_canister_id_res
    }

    }
