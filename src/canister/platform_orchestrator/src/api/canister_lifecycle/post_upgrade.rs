use ciborium::de;
use ic_cdk_macros::post_upgrade;
use ic_stable_structures::Memory;

use crate::{data_model::memory, CANISTER_DATA, POST_UPGRADE_DEBUG};

#[post_upgrade]
pub fn post_upgrade() {
    restore_data_from_stable_memory();
}

fn restore_data_from_stable_memory() {
    let heap_data = memory::get_upgrades_memory();
    let mut heap_data_len_bytes = [0; 4];
    heap_data.read(0, &mut heap_data_len_bytes);
    let heap_data_len = u32::from_le_bytes(heap_data_len_bytes) as usize;

    // Guard: empty stable memory means first upgrade after initial install — use defaults.
    if heap_data_len == 0 {
        POST_UPGRADE_DEBUG.with(|s| {
            *s.borrow_mut() = "heap_data_len=0: using default state".to_string();
        });
        return;
    }

    let mut canister_data_bytes = vec![0; heap_data_len];
    heap_data.read(4, &mut canister_data_bytes);

    match de::from_reader::<crate::data_model::CanisterData, _>(&*canister_data_bytes) {
        Ok(canister_data) => {
            CANISTER_DATA.with_borrow_mut(|cd| {
                *cd = canister_data;
            });
            POST_UPGRADE_DEBUG.with(|s| {
                *s.borrow_mut() = format!("heap_data_len={} | restored OK", heap_data_len);
            });
        }
        Err(e) => {
            // Log the error without panicking so we can query it via get_post_upgrade_debug.
            // State will be default (empty) — recover manually after diagnosing.
            POST_UPGRADE_DEBUG.with(|s| {
                *s.borrow_mut() = format!(
                    "heap_data_len={} | DESER FAILED: {:?}",
                    heap_data_len, e
                );
            });
        }
    }
}
