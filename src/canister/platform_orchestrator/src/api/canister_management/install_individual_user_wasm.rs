use candid::Principal;
use ic_cdk::{
    api::management_canister::main::{
        install_code, CanisterIdRecord, CanisterInstallMode, InstallCodeArgument,
    },
    caller,
};
use ic_cdk_macros::update;
use shared_utils::common::types::wasm::WasmType;

use crate::CANISTER_DATA;

const AUTHORIZED_CALLER: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

/// Installs the stored IndividualUserWasm onto a canister that currently has
/// no wasm (Install mode). Used to prepare canisters that were previously
/// uninstalled so that return_cycle_balance_to_platform_orchestrator can run
/// before the canister is decommissioned.
#[update]
pub async fn install_individual_user_wasm(canister_id: Principal) -> Result<(), String> {
    if caller() != Principal::from_text(AUTHORIZED_CALLER).unwrap() {
        return Err("Unauthorized".into());
    }

    let wasm_blob = CANISTER_DATA
        .with_borrow(|canister_data| {
            canister_data
                .wasms
                .get(&WasmType::IndividualUserWasm)
                .map(|w| w.wasm_blob.clone())
        })
        .ok_or("IndividualUserWasm not found — upload wasms to platform_orchestrator first")?;

    install_code(InstallCodeArgument {
        mode: CanisterInstallMode::Install,
        canister_id,
        wasm_module: wasm_blob,
        arg: vec![],
    })
    .await
    .map_err(|e| e.1)
}
