use std::collections::{BTreeMap, HashSet};

use candid::{Deserialize, Nat, Principal};
use ic_stable_structures::StableBTreeMap;
use serde::Serialize;
use shared_utils::canister_specific::user_index::types::{
    BroadcastCallStatus, RecycleStatus, UpgradeStatus,
};
use shared_utils::common::types::known_principal::KnownPrincipalType;
use shared_utils::common::types::version_details::VersionDetails;
use shared_utils::common::types::wasm::{CanisterWasm, WasmType};
use shared_utils::common::utils::default_pump_dump_onboarding_reward;

use crate::util::types::subnet_orchestrator_operation::SubnetOrchestratorOperation;
use crate::CANISTER_DATA;

use self::memory::get_wasm_memory;
use self::{configuration::Configuration, memory::Memory};

pub mod configuration;
pub mod memory;

const fn _default_true() -> bool {
    return true;
}

fn _default_vec_principal() -> HashSet<Principal> {
    return HashSet::new();
}

#[derive(Serialize, Deserialize)]
pub struct CanisterData {
    pub configuration: Configuration,
    pub last_run_upgrade_status: UpgradeStatus,
    pub allow_upgrades_for_individual_canisters: bool,
    pub available_canisters: HashSet<Principal>,
    #[serde(default)]
    pub backup_canister_pool: HashSet<Principal>,
    pub user_principal_id_to_canister_id_map: BTreeMap<Principal, Principal>,
    pub unique_user_name_to_user_principal_id_map: BTreeMap<String, Principal>,
    #[serde(skip, default = "_empty_wasms")]
    pub wasms: StableBTreeMap<WasmType, CanisterWasm, Memory>,
    #[serde(default)]
    pub recycle_status: RecycleStatus,
    #[serde(default)]
    pub last_broadcast_call_status: BroadcastCallStatus,
    #[serde(default)]
    pub version_details: VersionDetails,
    #[serde(default = "default_pump_dump_onboarding_reward")]
    pub pump_dump_onboarding_reward: Nat,
    #[serde(default)]
    pub on_going_operation: HashSet<SubnetOrchestratorOperation>,
}

impl Default for CanisterData {
    fn default() -> Self {
        Self {
            configuration: Default::default(),
            last_run_upgrade_status: Default::default(),
            allow_upgrades_for_individual_canisters: Default::default(),
            available_canisters: Default::default(),
            user_principal_id_to_canister_id_map: Default::default(),
            unique_user_name_to_user_principal_id_map: Default::default(),
            wasms: _empty_wasms(),
            backup_canister_pool: Default::default(),
            recycle_status: Default::default(),
            last_broadcast_call_status: Default::default(),
            version_details: VersionDetails::default(),
            pump_dump_onboarding_reward: default_pump_dump_onboarding_reward(),
            on_going_operation: HashSet::new(),
        }
    }
}

fn _empty_wasms() -> StableBTreeMap<WasmType, CanisterWasm, Memory> {
    StableBTreeMap::init(get_wasm_memory())
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
