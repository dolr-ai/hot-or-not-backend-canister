use shared_utils::{
    canister_specific::individual_user_template::types::post::PostStatus,
    common::types::known_principal::KnownPrincipalType,
};

use crate::CANISTER_DATA;

#[ic_cdk::update]
#[candid::candid_method(update)]
fn update_post_as_ready_to_view(id: u64) {
    let api_caller = ic_cdk::caller();

    let global_super_admin_principal_id = CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .known_principal_ids
            .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
            .cloned()
            .unwrap()
    });

    if api_caller != global_super_admin_principal_id {
        return;
    }

    CANISTER_DATA.with(|canister_data_ref_cell| {
        let mut post_to_update = canister_data_ref_cell
            .borrow_mut()
            .all_created_posts
            .get(&id)
            .unwrap()
            .clone();

        post_to_update.update_status(PostStatus::ReadyToView);

        canister_data_ref_cell
            .borrow_mut()
            .all_created_posts
            .insert(id, post_to_update);
    });
}
