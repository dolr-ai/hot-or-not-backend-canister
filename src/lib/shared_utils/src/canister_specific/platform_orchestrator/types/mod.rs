use std::collections::HashMap;

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

pub mod args;
pub mod well_known_principal;

#[derive(Default, Clone, CandidType, Serialize, Deserialize)]
pub struct UpgradeStatus {
    pub version: String,
    pub successful_upgrades: u64,
    pub failed_upgrades: u64,
}

#[derive(Default, Clone, CandidType, Serialize, Deserialize)]
pub struct SubnetUpgradeReport {
    pub subnet_wise_report: HashMap<Principal, UpgradeStatus>,
}
