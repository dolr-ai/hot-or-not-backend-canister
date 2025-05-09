use std::collections::{HashMap, HashSet};

use candid::{CandidType, Principal};
use serde::{Deserialize, Serialize};

type IndividualProfileId = Principal;
type RootCanisterId = Principal;

#[derive(Default, Debug, Serialize, Deserialize, CandidType, Clone)]
pub struct CreatorDaoTokenStats {
    creator_dao_token_sns_canisters: HashMap<IndividualProfileId, IndividualUserCreatorDaoEntry>,
    pub total_number_of_creator_dao_tokens: u64,
}

#[derive(CandidType, Serialize, Deserialize, Debug, Clone)]
pub struct IndividualUserCreatorDaoEntry {
    pub individual_profile_id: IndividualProfileId,
    pub deployed_canisters: HashSet<RootCanisterId>,
}

impl CreatorDaoTokenStats {
    pub fn insert_new_entry(
        &mut self,
        individual_user_profile_id: IndividualProfileId,
        root_canister_id: RootCanisterId,
    ) {
        let individual_user_creator_dao_entry = self
            .creator_dao_token_sns_canisters
            .get_mut(&individual_user_profile_id);

        if let Some(individual_user_creator_dao_entry) = individual_user_creator_dao_entry {
            if individual_user_creator_dao_entry
                .deployed_canisters
                .insert(root_canister_id)
            {
                self.total_number_of_creator_dao_tokens += 1;
            }
        } else {
            self.creator_dao_token_sns_canisters.insert(
                individual_user_profile_id,
                IndividualUserCreatorDaoEntry {
                    individual_profile_id: individual_user_profile_id,
                    deployed_canisters: vec![root_canister_id].into_iter().collect(),
                },
            );

            self.total_number_of_creator_dao_tokens += 1;
        }
    }
}
