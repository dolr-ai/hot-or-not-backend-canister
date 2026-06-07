use std::pin::Pin;

use futures::Future;
use ic_cdk::{api::is_controller, caller};
use ic_cdk_macros::update;
use shared_utils::{
    common::{
        types::wasm::{CanisterWasm, WasmType},
        utils::task::run_task_concurrently,
    },
    constant::{
        get_backup_individual_user_canister_batch_size,
        get_individual_user_canister_subnet_batch_size,
    },
};

use crate::{
    util::canister_management::{create_empty_user_canister, create_users_canister},
    CANISTER_DATA,
};

enum CanisterCodeState {
    Empty,
    WasmInstalled,
}

#[update]
pub fn create_pool_of_individual_user_available_canisters(
    version: String,
    individual_user_wasm: Vec<u8>,
) -> Result<String, String> {
    if !is_controller(&caller()) {
        return Err("Unauthorized".into());
    }

    //store wasm internally
    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data.wasms.insert(
            WasmType::IndividualUserWasm,
            CanisterWasm {
                version: version.clone(),
                wasm_blob: individual_user_wasm.clone(),
            },
        )
    });

    ic_cdk::spawn(impl_create_pool_of_individual_user_available_canisters(
        version,
        individual_user_wasm,
    ));
    Ok("Success".into())
}

pub async fn impl_create_pool_of_individual_user_available_canisters(
    version: String,
    individual_user_wasm: Vec<u8>,
) {
    let backup_individual_user_canister_batch_size =
        get_backup_individual_user_canister_batch_size();

    //empty canister for backup
    let create_empty_canister_futures = (0..backup_individual_user_canister_batch_size).map(|_| {
        Box::pin(async {
            let canister_id = create_empty_user_canister().await;
            (canister_id, CanisterCodeState::Empty)
        }) as Pin<Box<dyn Future<Output = _>>>
    });

    //canisters with installed wasm for available pool
    let individual_user_canister_subnet_batch_size =
        get_individual_user_canister_subnet_batch_size();
    let create_canister_with_wasm_futures =
        (0..individual_user_canister_subnet_batch_size).map(|_| {
            Box::pin(async {
                let canister_id =
                    create_users_canister(None, version.clone(), individual_user_wasm.clone())
                        .await;
                (canister_id, CanisterCodeState::WasmInstalled)
            }) as Pin<Box<dyn Future<Output = _>>>
        });

    let combined_create_canister_futures =
        create_canister_with_wasm_futures.chain(create_empty_canister_futures);

    run_task_concurrently(
        combined_create_canister_futures,
        10,
        |(canister_id, canister_code_state)| {
            CANISTER_DATA.with_borrow_mut(|canister_data| {
                match canister_code_state {
                    CanisterCodeState::Empty => {
                        canister_data.backup_canister_pool.insert(canister_id)
                    }
                    CanisterCodeState::WasmInstalled => {
                        canister_data.available_canisters.insert(canister_id)
                    }
                };
            });
        },
        || false,
    )
    .await;
}
