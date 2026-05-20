use std::collections::HashSet;

use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;

#[derive(CandidType, Serialize, Deserialize, Default, Clone)]
pub struct LongRunningTaskStatus {
    pub canisters_remaining: HashSet<Principal>,
    pub completed_count: u64,
    pub failed_canisters: Vec<(Principal, String)>,
}
