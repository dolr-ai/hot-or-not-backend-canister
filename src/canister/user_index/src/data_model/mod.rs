use std::collections::{BTreeMap, HashSet};

use candid::{Deserialize, Nat, Principal};
use serde::Serialize;
use shared_utils::canister_specific::user_index::types::BroadcastCallStatus;
use shared_utils::common::types::known_principal::KnownPrincipalType;
use shared_utils::common::types::version_details::VersionDetails;
use shared_utils::common::utils::default_pump_dump_onboarding_reward;

use crate::CANISTER_DATA;

use self::configuration::Configuration;

pub mod bulk_individual_canister_operation_status;
pub mod configuration;
pub mod memory;

use self::bulk_individual_canister_operation_status::BulkIndividualCanisterOperationStatus;

#[derive(Serialize, Deserialize)]
pub struct CanisterData {
    pub configuration: Configuration,
    pub available_canisters: HashSet<Principal>,
    #[serde(default)]
    pub backup_canister_pool: HashSet<Principal>,
    pub user_principal_id_to_canister_id_map: BTreeMap<Principal, Principal>,
    pub unique_user_name_to_user_principal_id_map: BTreeMap<String, Principal>,
    #[serde(default)]
    pub last_broadcast_call_status: BroadcastCallStatus,
    #[serde(default)]
    pub version_details: VersionDetails,
    #[serde(default = "default_pump_dump_onboarding_reward")]
    pub pump_dump_onboarding_reward: Nat,
    #[serde(default)]
    pub bulk_operation_status: BulkIndividualCanisterOperationStatus,
}

impl Default for CanisterData {
    fn default() -> Self {
        Self {
            configuration: Default::default(),
            available_canisters: Default::default(),
            user_principal_id_to_canister_id_map: Default::default(),
            unique_user_name_to_user_principal_id_map: Default::default(),
            last_broadcast_call_status: Default::default(),
            version_details: VersionDetails::default(),
            pump_dump_onboarding_reward: default_pump_dump_onboarding_reward(),
            bulk_operation_status: Default::default(),
            backup_canister_pool: Default::default(),
        }
    }
}

pub fn get_sns_ledger() -> Option<Principal> {
    let ledger = CANISTER_DATA.with_borrow(|cdata| {
        cdata
            .configuration
            .known_principal_ids
            .get(&KnownPrincipalType::CanisterIdSnsLedger)
            .copied()
    });

    ledger
}
