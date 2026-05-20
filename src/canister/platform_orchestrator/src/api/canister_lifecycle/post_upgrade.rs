use ciborium::de;
use ic_cdk_macros::post_upgrade;
use ic_stable_structures::Memory;
use shared_utils::{
    canister_specific::platform_orchestrator::types::args::PlatformOrchestratorInitArgs,
    common::utils::system_time,
};

use crate::{data_model::memory, CANISTER_DATA, POST_UPGRADE_DEBUG};

#[post_upgrade]
pub fn post_upgrade() {
    restore_data_from_stable_memory();
    update_version_from_args();
}

fn restore_data_from_stable_memory() {
    let heap_data = memory::get_upgrades_memory();
    let mut heap_data_len_bytes = [0; 4];
    heap_data.read(0, &mut heap_data_len_bytes);
    let heap_data_len = u32::from_le_bytes(heap_data_len_bytes) as usize;

    POST_UPGRADE_DEBUG.with(|s| {
        *s.borrow_mut() = format!("restore_stable_memory: heap_data_len={}", heap_data_len);
    });

    let mut canister_data_bytes = vec![0; heap_data_len];
    heap_data.read(4, &mut canister_data_bytes);
    let canister_data =
        de::from_reader(&*canister_data_bytes).expect("Failed to deserialize heap data");
    CANISTER_DATA.with_borrow_mut(|cd| {
        *cd = canister_data;
    });
}

fn update_version_from_args() {
    // Read raw bytes first; if empty or malformed, keep the existing version rather
    // than panicking (SNS governance may encode the upgrade arg differently).
    let raw = ic_cdk::api::call::arg_data_raw();
    let raw_preview: Vec<u8> = raw.iter().take(64).copied().collect();

    if raw.is_empty() {
        POST_UPGRADE_DEBUG.with(|s| {
            *s.borrow_mut() = "arg_data_raw: EMPTY — version not updated".to_string();
        });
        return;
    }

    match candid::decode_args::<(PlatformOrchestratorInitArgs,)>(&raw) {
        Ok((upgrade_args,)) => {
            POST_UPGRADE_DEBUG.with(|s| {
                *s.borrow_mut() = format!(
                    "arg_data_raw len={} preview={:?} | decoded version={:?}",
                    raw.len(),
                    raw_preview,
                    upgrade_args.version
                );
            });
            if !upgrade_args.version.is_empty() {
                CANISTER_DATA.with_borrow_mut(|canister_data| {
                    canister_data.version_detail.version = upgrade_args.version;
                    canister_data.version_detail.last_update_on =
                        system_time::get_current_system_time();
                });
            }
        }
        Err(e) => {
            POST_UPGRADE_DEBUG.with(|s| {
                *s.borrow_mut() = format!(
                    "arg_data_raw len={} preview={:?} | decode_args FAILED: {}",
                    raw.len(),
                    raw_preview,
                    e
                );
            });
        }
    }
}
