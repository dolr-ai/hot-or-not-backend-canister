use std::collections::{BTreeMap, HashMap};

use candid::{CandidType, Deserialize, Principal};
use serde::Serialize;
#[derive(Default, Serialize, Deserialize, Clone)]
pub struct FollowData {
    pub follower: FollowList,
    pub following: FollowList,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct FollowList {
    pub sorted_index: BTreeMap<FollowEntryId, FollowEntryDetail>,
    pub members: HashMap<FollowEntryDetail, FollowEntryId>,
}

impl FollowList {
    /// Returns the follow entry ID after the follow entry was added.
    pub fn add(&mut self, follow_entry_detail: FollowEntryDetail) -> FollowEntryId {
        let follow_entry_id = self.sorted_index.last_key_value().map_or(0, |(k, _)| k + 1);

        self.sorted_index
            .insert(follow_entry_id, follow_entry_detail.clone());
        self.members.insert(follow_entry_detail, follow_entry_id);

        follow_entry_id
    }

    /// Returns the follow entry ID if the follow entry was removed.
    pub fn remove(&mut self, follow_entry_detail: &FollowEntryDetail) -> Option<FollowEntryId> {
        let follow_entry_id = self.members.remove(follow_entry_detail);

        if let Some(follow_entry_id) = follow_entry_id {
            self.sorted_index.remove(&follow_entry_id);
        }

        follow_entry_id
    }

    /// Returns true if the follow entry exists.
    pub fn contains(&self, follow_entry_detail: &FollowEntryDetail) -> bool {
        self.members.contains_key(follow_entry_detail)
    }

    /// Returns the number of follow entries.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }
}

pub type FollowEntryId = u64;

#[derive(Serialize, Deserialize, Eq, PartialEq, Hash, Clone, CandidType, Debug)]
pub struct FollowEntryDetail {
    pub principal_id: Principal,
    pub canister_id: Principal,
}

#[cfg(test)]
mod test {
    use super::*;

    mod test_follow_list {
        use super::*;

        #[test]
        fn test_add() {
            let mut follow_list = FollowList::default();

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((0u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((0u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));
        }

        #[test]
        fn test_remove() {
            let mut follow_list = FollowList::default();

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((0u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((0u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));

            let follow_entry_id = follow_list.remove(&follow_entry_detail);

            assert_eq!(follow_entry_id, Some(0));
            assert_eq!(follow_list.len(), 0);
            assert!(!follow_list.contains(&follow_entry_detail));
        }

        #[test]
        fn test_add_remove() {
            let mut follow_list = FollowList::default();

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((0u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((0u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));

            let follow_entry_id = follow_list.remove(&follow_entry_detail);

            assert_eq!(follow_entry_id, Some(0));
            assert_eq!(follow_list.len(), 0);
            assert!(!follow_list.contains(&follow_entry_detail));

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));
        }

        #[test]
        fn test_contains() {
            let mut follow_list = FollowList::default();

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((0u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((0u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((1u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((1u64).to_ne_bytes()),
            };

            assert!(!follow_list.contains(&follow_entry_detail));
        }

        #[test]
        fn test_len() {
            let mut follow_list = FollowList::default();

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((0u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((0u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 0);
            assert_eq!(follow_list.len(), 1);
            assert!(follow_list.contains(&follow_entry_detail));

            let follow_entry_detail = FollowEntryDetail {
                principal_id: Principal::self_authenticating((1u64).to_ne_bytes()),
                canister_id: Principal::self_authenticating((1u64).to_ne_bytes()),
            };

            let follow_entry_id = follow_list.add(follow_entry_detail.clone());

            assert_eq!(follow_entry_id, 1);
            assert_eq!(follow_list.len(), 2);
            assert!(follow_list.contains(&follow_entry_detail));
        }
    }
}
