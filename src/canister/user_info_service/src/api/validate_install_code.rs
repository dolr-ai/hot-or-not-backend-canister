use candid::Principal;
use ic_cdk::caller;
use ic_cdk_macros::query;

const SNS_GOVERNANCE_CANISTER_ID: &str = "6wcax-haaaa-aaaaq-aaava-cai";

/// SNS validator for the generic install_code nervous system function.
/// Accepts any payload from the SNS governance canister and returns Ok so the
/// SNS can pass raw install_code arguments through without Candid type-checking.
#[query]
pub fn validate_install_code(_payload: Vec<u8>) -> Result<String, String> {
    if caller()
        != Principal::from_text(SNS_GOVERNANCE_CANISTER_ID)
            .expect("invalid SNS governance principal")
    {
        return Err("Only callable by SNS governance".to_string());
    }
    Ok("Success".to_string())
}
