use std::borrow::Cow;

use candid::CandidType;
use ciborium::de;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, PartialEq, PartialOrd, Eq, Ord, CandidType)]
pub enum WasmType {
    IndividualUserWasm,
    PostCacheWasm,
}

impl Storable for WasmType {
    fn to_bytes(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut bytes = vec![];
        ciborium::ser::into_writer(self, &mut bytes).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let wasm_type: WasmType = de::from_reader(bytes.as_ref()).unwrap();
        wasm_type
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 25,
        is_fixed_size: false,
    };
}

#[derive(Serialize, Deserialize, CandidType, Clone)]
pub struct CanisterWasm {
    pub wasm_blob: Vec<u8>,
    pub version: String,
}

impl Storable for CanisterWasm {
    fn to_bytes(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut bytes = vec![];
        ciborium::ser::into_writer(self, &mut bytes).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        let canister_wasm: CanisterWasm = de::from_reader(bytes.as_ref()).unwrap();
        canister_wasm
    }

    const BOUND: Bound = Bound::Unbounded;
}
