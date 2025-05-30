use std::collections::HashSet;

use candid::{Nat, Principal};
use ic_cdk::{
    api::management_canister::main::{
        canister_status, deposit_cycles, start_canister, update_settings, CanisterIdRecord,
        CanisterSettings, LogVisibility, UpdateSettingsArgument,
    },
    notify,
};
use shared_utils::{
    common::types::wasm::WasmType, constant::SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD,
};

use crate::CANISTER_DATA;

pub struct OngoingRequestForCyclesFromSubnetOrchestratorGuard {
    subnet_orchestrator_canister_id: Principal,
}

impl OngoingRequestForCyclesFromSubnetOrchestratorGuard {
    fn new(subnet_orchestrator_canister_id: Principal) -> Result<Self, String> {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            match canister_data
                .state_guard
                .ongoing_request_for_cycles_from_subnet_orchestrator
                .contains(&subnet_orchestrator_canister_id)
            {
                true => Err(format!(
                    "Already Processing a request for Cycles from subnet orchestrator {}",
                    subnet_orchestrator_canister_id
                )),
                false => {
                    canister_data
                        .state_guard
                        .ongoing_request_for_cycles_from_subnet_orchestrator
                        .insert(subnet_orchestrator_canister_id);
                    Ok(Self {
                        subnet_orchestrator_canister_id,
                    })
                }
            }
        })
    }
}

impl Drop for OngoingRequestForCyclesFromSubnetOrchestratorGuard {
    fn drop(&mut self) {
        CANISTER_DATA.with_borrow_mut(|canister_data| {
            canister_data
                .state_guard
                .ongoing_request_for_cycles_from_subnet_orchestrator
                .remove(&self.subnet_orchestrator_canister_id)
        });
    }
}

pub struct RegisteredSubnetOrchestrator {
    canister_id: Principal,
}

impl RegisteredSubnetOrchestrator {
    pub fn new(canister_id: Principal) -> Result<RegisteredSubnetOrchestrator, String> {
        let contains = CANISTER_DATA.with_borrow(|canister_data| {
            canister_data
                .all_subnet_orchestrator_canisters_list
                .contains(&canister_id)
        });

        if contains {
            Ok(RegisteredSubnetOrchestrator { canister_id })
        } else {
            Err("Canister Id is not found in platform orchestrator".into())
        }
    }

    pub fn get_canister_id(&self) -> Principal {
        self.canister_id
    }

    pub async fn start_canister(&self) -> Result<(), String> {
        start_canister(CanisterIdRecord {
            canister_id: self.canister_id,
        })
        .await
        .map_err(|e| e.1)
    }

    pub async fn deposit_cycles(&self) -> Result<(), String> {
        let _guard = OngoingRequestForCyclesFromSubnetOrchestratorGuard::new(self.canister_id)?;

        let (subnet_orchestrator_status_res,) = canister_status(CanisterIdRecord {
            canister_id: self.canister_id,
        })
        .await
        .map_err(|e| e.1)?;

        let subnet_orchestrator_cycles = subnet_orchestrator_status_res.cycles;

        if subnet_orchestrator_cycles > SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD {
            return Ok(());
        }

        deposit_cycles(
            CanisterIdRecord {
                canister_id: self.canister_id,
            },
            SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD,
        )
        .await
        .map_err(|e| e.1)
    }

    pub async fn make_logs_public(&self) -> Result<(), String> {
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

    pub async fn make_logs_private(&self) -> Result<(), String> {
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

    pub async fn make_individual_canister_logs_public(
        &self,
        individual_canister_id: Principal,
    ) -> Result<(), String> {
        let (res,) = ic_cdk::call(
            self.canister_id,
            "make_individual_canister_logs_public",
            (individual_canister_id,),
        )
        .await
        .map_err(|e| e.1)?;

        res
    }

    pub async fn fixup_individual_cansiters_mapping(&self) -> Result<(), String> {
        ic_cdk::call::<_, ()>(self.canister_id, "fixup_individual_canisters_mapping", ())
            .await
            .map_err(|e| e.1)
    }

    pub async fn make_individual_canister_logs_private(
        &self,
        individual_canister_id: Principal,
    ) -> Result<(), String> {
        let (res,) = ic_cdk::call(
            self.canister_id,
            "make_individual_canister_logs_private",
            (individual_canister_id,),
        )
        .await
        .map_err(|e| e.1)?;

        res
    }

    pub async fn delete_all_sns_creator_token_for_an_individual_canister(
        &self,
        individual_canister_id: Principal,
    ) -> Result<(), String> {
        ic_cdk::call::<_, (Result<(), String>,)>(
            self.canister_id,
            "delete_all_sns_creator_token_of_an_individual_canister",
            (individual_canister_id,),
        )
        .await
        .map_err(|e| e.1)?
        .0
    }

    pub async fn delete_all_sns_creator_token_in_the_network(&self) -> Result<(), String> {
        ic_cdk::call::<_, ()>(
            self.canister_id,
            "delete_all_sns_creator_token_in_the_network",
            (),
        )
        .await
        .map_err(|e| e.1)
    }

    pub async fn provision_empty_canisters(&self, number_of_canisters: u64) -> Result<(), String> {
        ic_cdk::call::<_, ()>(
            self.canister_id,
            "provision_empty_canisters",
            (number_of_canisters,),
        )
        .await
        .map_err(|e| e.1)
    }

    pub async fn upgrade_individual_canisters_in_subnet_with_latest_wasm(
        &self,
    ) -> Result<(), String> {
        let individual_canister_wasm = CANISTER_DATA
            .with_borrow(|canister_data| canister_data.wasms.get(&WasmType::IndividualUserWasm))
            .unwrap();
        self.deposit_cycles().await?;

        let res: Result<(String,), String> = ic_cdk::call(
            self.canister_id,
            "start_upgrades_for_individual_canisters",
            (
                individual_canister_wasm.version.clone(),
                individual_canister_wasm.wasm_blob.clone(),
            ),
        )
        .await
        .map_err(|e| {
            format!(
                "Failed to start upgrades on {}. Error {}",
                self.canister_id, e.1
            )
        });

        match res {
            Ok((_str,)) => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub async fn set_reserved_cycle_limit(&self, amount: u128) -> Result<(), String> {
        update_settings(UpdateSettingsArgument {
            canister_id: self.canister_id,
            settings: CanisterSettings {
                reserved_cycles_limit: Some(Nat::from(amount)),
                ..Default::default()
            },
        })
        .await
        .map_err(|e| e.1)
    }

    pub fn notify_specific_individual_canister_to_upgrade_creator_dao_governance_canisters(
        &self,
        individual_canister_id: Principal,
        wasm_module: Vec<u8>,
    ) -> Result<(), String> {
        notify(
            self.canister_id,
            "notify_specific_individual_canister_to_upgrade_creator_dao_governance_canisters",
            (individual_canister_id, wasm_module),
        )
        .map_err(|e| format!("Notify to subnet orchestrator failed {:?}", e))
    }

    pub async fn upgrade_specific_individual_canister_with_wasm_version(
        &self,
        individual_canister_id: Principal,
        version: String,
        wasm_module: Vec<u8>,
    ) -> Result<(), String> {
        ic_cdk::call::<_, (Result<(), String>,)>(
            self.canister_id,
            "upgrade_specific_individual_canister_with_wasm_version",
            (individual_canister_id, version, wasm_module),
        )
        .await
        .map_err(|e| format!("{:?} {}", e.0, e.1))?
        .0
    }
}
