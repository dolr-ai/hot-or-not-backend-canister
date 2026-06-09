use candid::Principal;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq)]
pub(crate) enum SubnetOrchestratorOperation {
    RechargeIndividualUserCanister(Principal),
}
