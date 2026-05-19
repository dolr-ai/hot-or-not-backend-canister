use candid::Principal;
use ic_cdk::api::{
    canister_balance128,
    management_canister::{main::deposit_cycles, provisional::CanisterIdRecord},
};
use ic_cdk_macros::{export_candid, update};

const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

// Cycles kept back to cover this call's own execution overhead.
// deposit_cycles to the management canister costs ~260K cycles base;
// 100M is a generous safety margin.
const CYCLE_RESERVE: u128 = 100_000_000;

/// Sends all cycles above CYCLE_RESERVE to the platform_orchestrator.
/// Must be called before uninstalling this canister's wasm, since
/// deposit_cycles requires this wasm to be running.
/// Returns the number of cycles transferred.
#[update]
async fn return_cycle_balance_to_platform_orchestrator() -> Result<u128, String> {
    let current_balance = canister_balance128();

    if current_balance <= CYCLE_RESERVE {
        return Ok(0);
    }

    let amount_to_send = current_balance - CYCLE_RESERVE;
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
