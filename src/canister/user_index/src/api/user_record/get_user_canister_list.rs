use candid::Principal;
use ic_cdk_macros::query;

use crate::CANISTER_DATA;

#[query]
fn get_user_canister_list() -> Vec<Principal> {
    CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .user_principal_id_to_canister_id_map
            .values()
            .cloned()
            .collect()
    })
}

#[query]
fn get_user_id_and_canister_list() -> Vec<(Principal, Principal)> {
    CANISTER_DATA.with(|canister_data_ref_cell| {
        canister_data_ref_cell
            .borrow()
            .user_principal_id_to_canister_id_map
            .iter()
            .map(|(principal_id, canister_id)| (*principal_id, *canister_id))
            .collect()
    })
}

#[query]
fn get_user_canister_incl_avail_list() -> Vec<Principal> {
    CANISTER_DATA.with(|canister_data_ref_cell| {
        let mut canister_list = canister_data_ref_cell
            .borrow()
            .user_principal_id_to_canister_id_map
            .values()
            .cloned()
            .collect::<Vec<Principal>>();

        let available_canister_list = canister_data_ref_cell.borrow().available_canisters.clone();

        canister_list.extend(available_canister_list);

        canister_list
    })
}

#[cfg(test)]
mod test {
    use test_utils::setup::test_constants::{
        get_mock_user_alice_canister_id, get_mock_user_alice_principal_id,
        get_mock_user_bob_canister_id, get_mock_user_bob_principal_id,
        get_mock_user_dan_canister_id,
    };

    use super::*;

    #[test]
    fn test_get_user_canister_list() {
        CANISTER_DATA.with(|canister_data_ref_cell| {
            let mut canister_data = canister_data_ref_cell.borrow_mut();

            canister_data.user_principal_id_to_canister_id_map.insert(
                get_mock_user_alice_principal_id(),
                get_mock_user_alice_canister_id(),
            );

            canister_data.user_principal_id_to_canister_id_map.insert(
                get_mock_user_bob_principal_id(),
                get_mock_user_bob_canister_id(),
            );

            canister_data
                .available_canisters
                .insert(get_mock_user_dan_canister_id());
        });

        let user_canister_list = get_user_canister_list();
        assert_eq!(user_canister_list.len(), 2);
        assert!(user_canister_list.contains(&get_mock_user_alice_canister_id()));
        assert!(user_canister_list.contains(&get_mock_user_bob_canister_id()));
    }

    #[test]
    fn test_get_user_canister_incl_avail_list() {
        CANISTER_DATA.with(|canister_data_ref_cell| {
            let mut canister_data = canister_data_ref_cell.borrow_mut();

            canister_data.user_principal_id_to_canister_id_map.insert(
                get_mock_user_alice_principal_id(),
                get_mock_user_alice_canister_id(),
            );

            canister_data.user_principal_id_to_canister_id_map.insert(
                get_mock_user_bob_principal_id(),
                get_mock_user_bob_canister_id(),
            );

            canister_data
                .available_canisters
                .insert(get_mock_user_dan_canister_id());
        });

        let user_canister_list = get_user_canister_incl_avail_list();
        assert_eq!(user_canister_list.len(), 3);
        assert!(user_canister_list.contains(&get_mock_user_alice_canister_id()));
        assert!(user_canister_list.contains(&get_mock_user_bob_canister_id()));
        assert!(user_canister_list.contains(&get_mock_user_dan_canister_id()));
    }
}
