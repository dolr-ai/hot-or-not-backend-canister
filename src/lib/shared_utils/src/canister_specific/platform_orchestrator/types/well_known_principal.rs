use std::collections::HashMap;

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

use crate::common::types::known_principal::{KnownPrincipalMap, KnownPrincipalType};

#[derive(Serialize, Deserialize, CandidType, Default)]
pub struct PlatformOrchestratorKnownPrincipal {
    pub global_known_principals : KnownPrincipalMap,
    pub subnet_orchestrator_known_principals_map: HashMap<Principal, KnownPrincipalMap>
}

impl PlatformOrchestratorKnownPrincipal {
    pub fn add_global_known_principal(&mut self, principal_type:KnownPrincipalType, value: Principal) {
        self.global_known_principals.insert(principal_type, value);
    }

    pub fn add_subnet_orchestrator_known_principal(&mut self, subnet_orchestrator: Principal, principal_type: KnownPrincipalType, value: Principal) {
        self.subnet_orchestrator_known_principals_map
        .entry(subnet_orchestrator)
        .or_default()
        .insert(principal_type, value);
    }
    
    pub fn get_subnet_known_principal(&self, subnet_orchestrator_id: &Principal, known_principal_type: &KnownPrincipalType) -> Principal {
        self.subnet_orchestrator_known_principals_map
        .get(subnet_orchestrator_id)
        .expect("Subnet Orchestrator not found")
        .get(known_principal_type)
        .expect("Known Principal type not found in the subnet")
        .clone()
    }

    pub fn get_global_known_principal(&self, known_principal_type: &KnownPrincipalType) -> Principal {
        self.global_known_principals
        .get(known_principal_type)
        .expect("known principal type not found")
        .clone()
    }
    
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_add_subnet_orchestrator_known_principal() {
        let mut platform_known_principal = PlatformOrchestratorKnownPrincipal::default();

        let subnet_principal = Principal::from_text("rpf7h-oyaaa-aaaag-qiu2a-cai").unwrap();
        platform_known_principal.add_subnet_orchestrator_known_principal(subnet_principal, KnownPrincipalType::CanisterIdPlatformOrchestrator, Principal::anonymous());
        let retrieved_governance_canister = platform_known_principal.subnet_orchestrator_known_principals_map.get(&subnet_principal).unwrap().get(&KnownPrincipalType::CanisterIdPlatformOrchestrator).unwrap();
        assert_eq!(*retrieved_governance_canister, Principal::anonymous()); 

    }
}