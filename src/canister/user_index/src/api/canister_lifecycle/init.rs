use ic_cdk_macros::init;
use shared_utils::canister_specific::user_index::types::args::UserIndexInitArgs;

use crate::{data_model::CanisterData, CANISTER_DATA};

#[init]
fn init(init_args: UserIndexInitArgs) {
    CANISTER_DATA.with(|canister_data_ref_cell| {
        let mut data = canister_data_ref_cell.borrow_mut();
        init_impl(init_args, &mut data);
    });
}

fn init_impl(init_args: UserIndexInitArgs, data: &mut CanisterData) {
    init_args
        .known_principal_ids
        .unwrap_or_default()
        .iter()
        .for_each(|(principal_belongs_to, principal_id)| {
            data.configuration
                .known_principal_ids
                .insert(*principal_belongs_to, *principal_id);
        });
    data.version_details.version = init_args.version;
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use shared_utils::{
        access_control::UserAccessRole,
        common::types::known_principal::{KnownPrincipalMap, KnownPrincipalType},
    };
    use test_utils::setup::test_constants::{
        get_global_super_admin_principal_id,
        get_mock_canister_id_user_index, get_mock_user_alice_canister_id,
    };

    use super::*;

    #[test]
    fn test_init_impl() {
        // * Add some known principals
        let mut known_principal_ids = KnownPrincipalMap::new();
        known_principal_ids.insert(
            KnownPrincipalType::UserIdGlobalSuperAdmin,
            get_global_super_admin_principal_id(),
        );
        known_principal_ids.insert(
            KnownPrincipalType::CanisterIdUserIndex,
            get_mock_canister_id_user_index(),
        );

        // * Add some access control roles
        let mut access_control_map = HashMap::new();
        access_control_map.insert(
            get_global_super_admin_principal_id(),
            vec![
                UserAccessRole::CanisterController,
                UserAccessRole::CanisterAdmin,
            ],
        );
        access_control_map.insert(
            get_mock_user_alice_canister_id(),
            vec![UserAccessRole::ProjectCanister],
        );

        // * Create the init args
        let init_args = UserIndexInitArgs {
            known_principal_ids: Some(known_principal_ids),
            access_control_map: Some(access_control_map),
            version: String::from("v1.0.0"),
        };
        let mut data = CanisterData::default();

        // * Run the init impl
        init_impl(init_args, &mut data);

        // * Check the data
        assert_eq!(
            data.configuration
                .known_principal_ids
                .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
                .unwrap(),
            &get_global_super_admin_principal_id()
        );
        assert_eq!(
            data.configuration
                .known_principal_ids
                .get(&KnownPrincipalType::CanisterIdUserIndex)
                .unwrap(),
            &get_mock_canister_id_user_index()
        );
        assert!(data.version_details.version.eq("v1.0.0"))
    }
}
