use std::error::Error;

use candid::{utils::ArgumentEncoder, CandidType, Deserialize, Principal};
use pocket_ic::{PocketIc, WasmResult};

pub fn query<A, T>(
    pocket_ic: &PocketIc,
    canister_id: Principal,
    sender: Principal,
    method_name: &str,
    args: A,
) -> Result<T, Box<dyn Error>>
where
    A: CandidType + ArgumentEncoder,
    T: CandidType + for<'de> Deserialize<'de>,
{
    let encoded_args = candid::encode_args(args)?;
    let wasm_result = pocket_ic
        .query_call(canister_id, sender, method_name, encoded_args)
        .map_err(|e| std::convert::Into::<Box<dyn Error>>::into(e.to_string()))?;

    match wasm_result {
        WasmResult::Reply(payload) => {
            let decoded: T = candid::decode_one(&payload)?;
            Ok(decoded)
        }
        WasmResult::Reject(e) => Err(e.into()),
    }
}

pub fn update<A, T>(
    pocket_ic: &PocketIc,
    canister_id: Principal,
    sender: Principal,
    method_name: &str,
    args: A,
) -> Result<T, Box<dyn Error>>
where
    A: CandidType + ArgumentEncoder,
    T: CandidType + for<'de> Deserialize<'de>,
{
    let encoded_args = candid::encode_args(args)?;
    let wasm_result = pocket_ic
        .update_call(canister_id, sender, method_name, encoded_args)
        .map_err(|e| std::convert::Into::<Box<dyn Error>>::into(e.to_string()))?;

    match wasm_result {
        WasmResult::Reply(payload) => {
            let decoded: T = candid::decode_one(&payload)?;
            Ok(decoded)
        }
        WasmResult::Reject(e) => Err(e.into()),
    }
}
