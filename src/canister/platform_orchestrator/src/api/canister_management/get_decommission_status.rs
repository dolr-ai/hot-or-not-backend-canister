use candid::{CandidType, Principal};
use ic_cdk_macros::query;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Stateless decommission status — always empty since no heap state is retained.
#[derive(CandidType, Serialize, Deserialize, Default, Clone)]
pub struct DecommissionStatus {
    pub canisters_remaining: HashSet<Principal>,
    pub completed_count: u64,
    pub failed_canisters: Vec<(Principal, String)>,
}

#[query]
pub fn get_decommission_status() -> DecommissionStatus {
    DecommissionStatus::default()
}
