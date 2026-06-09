use candid::Principal;
use ic_cdk::api::{
    canister_balance128,
    management_canister::{main::deposit_cycles, provisional::CanisterIdRecord},
};
use ic_cdk_macros::{export_candid, update};

const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

/// Sends (current_balance - reserve) cycles to the platform_orchestrator via deposit_cycles.
/// The reserve (passed by the caller) is left behind to cover execution + call overhead.
/// This matches dfx's approach for canister delete/withdrawal (30B base WITHDRAWAL_COST + retry increases).
///
/// Must be called before uninstalling this canister's wasm, since deposit_cycles requires
/// this wasm to be running.
///
/// Returns the number of cycles transferred (or 0 on early exit).
#[update]
async fn return_cycle_balance_to_platform_orchestrator(reserve: u128) -> Result<u128, String> {
    let current_balance = canister_balance128();

    if current_balance <= reserve {
        return Ok(0);
    }

    let amount_to_send = current_balance - reserve;
    let platform_orchestrator =
        Principal::from_text(PLATFORM_ORCHESTRATOR_ID).map_err(|e| e.to_string())?;

    deposit_cycles(
        CanisterIdRecord {
            canister_id: platform_orchestrator,
        },
        amount_to_send,
    )
    .await
    .map_err(|e| e.1)?;

    Ok(amount_to_send)
}

export_candid!();
