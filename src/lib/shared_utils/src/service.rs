use std::{borrow::Cow, cell::RefCell, thread::LocalKey};

use candid::CandidType;
use ciborium::{de, ser};
use ic_stable_structures::{writer::Writer, GrowFailed, Memory};
use serde::{de::DeserializeOwned, Serialize};

#[derive(CandidType, candid::Deserialize, Clone)]
pub struct ServiceInitArgs {
    pub version: String,
}

pub trait SetVersion {
    fn set_version(&mut self, version: &str);
}

pub trait GetVersion {
    fn get_version(&'_ self) -> Cow<'_, str>;
}

impl SetVersion for ServiceInitArgs {
    fn set_version(&mut self, version: &str) {
        self.version = version.into();
    }
}

impl GetVersion for ServiceInitArgs {
    fn get_version(&'_ self) -> Cow<'_, str> {
        self.version.as_str().into()
    }
}

pub fn update_version_from_args<Args>(state: &'static LocalKey<RefCell<impl SetVersion>>)
where
    for<'a> Args: GetVersion + candid::CandidType + candid::Deserialize<'a>,
{
    let (args,) =
        ic_cdk::api::call::arg_data::<(Args,)>(ic_cdk::api::call::ArgDecoderConfig::default());
    let version = args.get_version();

    state.with_borrow_mut(|state| {
        state.set_version(&version);
    });
}

#[derive(Debug, thiserror::Error)]
pub enum StableStateError {
    #[error("serde error: {0}")]
    Cbor(String),
    #[error("write error: {0:?}")]
    Write(#[from] GrowFailed),
}

pub struct StableState;

impl StableState {
    pub fn save<State>(
        state: &'static LocalKey<RefCell<State>>,
        memory: &mut impl Memory,
    ) -> Result<(), StableStateError>
    where
        State: Serialize,
    {
        let mut serialized_state = vec![];
        state
            .with_borrow(|s| ser::into_writer(&s, &mut serialized_state))
            .map_err(|err| StableStateError::Cbor(err.to_string()))?;

        let len = serialized_state.len() as u32;
        let mut writer = Writer::new(memory, 0);

        writer.write(&len.to_le_bytes())?;
        writer.write(&serialized_state)?;

        Ok(())
    }

    pub fn restore<State>(
        state: &'static LocalKey<RefCell<State>>,
        memory: &mut impl Memory,
    ) -> Result<(), StableStateError>
    where
        State: DeserializeOwned,
    {
        // the length is stored as u32, which is 4bytes
        let mut serialized_state_bytes_len = [0; 4];

        memory.read(0, &mut serialized_state_bytes_len);

        let serialized_state_bytes_len = u32::from_le_bytes(serialized_state_bytes_len) as usize;

        let mut serialized_state = vec![0; serialized_state_bytes_len];

        // the serialized data starts right after the length data
        memory.read(4, &mut serialized_state);

        let state_data = de::from_reader::<State, _>(&*serialized_state)
            .map_err(|err| StableStateError::Cbor(err.to_string()))?;

        state.with_borrow_mut(|state| *state = state_data);

        Ok(())
    }
}
