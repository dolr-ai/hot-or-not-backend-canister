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

    POST_UPGRADE_DEBUG.with(|s| {
        *s.borrow_mut() += " | restored OK";
    });
}
