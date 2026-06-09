use std::borrow::Cow;

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_stable_structures::{storable::Bound, Storable};

#[derive(CandidType, Deserialize, Ord, PartialOrd, Eq, PartialEq, Clone)]
pub struct StorablePrincipal(pub Principal);

impl Storable for StorablePrincipal {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 38,
        is_fixed_size: true,
    };
}
