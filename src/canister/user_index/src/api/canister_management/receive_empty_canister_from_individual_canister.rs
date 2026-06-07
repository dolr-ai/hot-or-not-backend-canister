use candid::Principal;
use futures::{stream::FuturesUnordered, TryStreamExt};
use ic_cdk::{
    api::management_canister::main::{canister_info, CanisterInfoRequest},
    id,
};
use ic_cdk_macros::update;

use crate::CANISTER_DATA;

#[update]
pub async fn receive_empty_canister_from_individual_canister(
    canister_ids: Vec<Principal>,
) -> Result<(), String> {
    canister_ids
        .iter()
        .map(|canister_id| async {
            let (canister_info_response,) = canister_info(CanisterInfoRequest {
                canister_id: *canister_id,
                num_requested_changes: None,
            })
            .await
            .map_err(|e| e.1)?;

            if !canister_info_response.controllers.contains(&id()) {
                return Err("Controller not set as subnet orchestrator".to_owned());
            }

            if canister_info_response.module_hash.is_some() {
                return Err("Canister is not Empty".to_owned());
            }

            Ok(())
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<()>()
        .await?;

    CANISTER_DATA.with_borrow_mut(|canister_data| {
        canister_data.backup_canister_pool.extend(canister_ids);
    });

    Ok(())
}
