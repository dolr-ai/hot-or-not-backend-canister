use std::collections::HashMap;

use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;

#[derive(CandidType, Deserialize, PartialEq, Eq, Hash, Serialize, Copy, Clone)]
pub enum KnownPrincipalType {
    UserIdGlobalSuperAdmin,
    CanisterIdConfiguration,
    CanisterIdDataBackup,
    CanisterIdPostCache,
    CanisterIdProjectMemberIndex,
    CanisterIdRootCanister,
    CanisterIdSNSController,
    CanisterIdTopicCacheIndex,
    CanisterIdSnsGovernance,
    CanisterIdPlatformOrchestrator,
    CanisterIdSnsWasm,
    CanisterIdSnsLedger,
}

pub type KnownPrincipalMap = HashMap<KnownPrincipalType, Principal>;
