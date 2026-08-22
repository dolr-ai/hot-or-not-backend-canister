use candid::Principal;
use ic_cdk::api::management_canister::provisional::CanisterId;
use shared_utils::constant::GLOBAL_SUPER_ADMIN_USER_ID_V1;

pub mod v1;

pub fn get_global_super_admin_principal_id() -> Principal {
    Principal::from_text(GLOBAL_SUPER_ADMIN_USER_ID_V1).unwrap()
}

pub fn get_mock_user_alice_principal_id() -> Principal {
    Principal::self_authenticating([1])
}

pub fn get_mock_user_bob_principal_id() -> Principal {
    Principal::self_authenticating([2])
}

pub fn get_mock_user_charlie_principal_id() -> Principal {
    Principal::self_authenticating([3])
}

pub fn get_mock_user_dan_principal_id() -> Principal {
    Principal::self_authenticating([4])
}

pub fn get_mock_user_tom_principal_id() -> Principal {
    Principal::self_authenticating([5])
}

pub fn get_mock_user_lucy_principal_id() -> Principal {
    Principal::self_authenticating([6])
}

pub fn get_mock_canister_id_root() -> Principal {
    CanisterId::from_slice(&2_usize.to_ne_bytes())
}

pub fn get_mock_canister_id_topic_cache() -> Principal {
    CanisterId::from_slice(&4_usize.to_ne_bytes())
}

pub fn get_mock_canister_id_configuration() -> Principal {
    CanisterId::from_slice(&6_usize.to_ne_bytes())
}

pub fn get_mock_canister_id_data_backup() -> Principal {
    CanisterId::from_slice(&7_usize.to_ne_bytes())
}

pub fn get_mock_user_alice_canister_id() -> Principal {
    CanisterId::from_slice(&8_usize.to_ne_bytes())
}

pub fn get_mock_user_bob_canister_id() -> Principal {
    CanisterId::from_slice(&9_usize.to_ne_bytes())
}

pub fn get_mock_user_charlie_canister_id() -> Principal {
    CanisterId::from_slice(&10_usize.to_ne_bytes())
}

pub fn get_mock_user_dan_canister_id() -> Principal {
    CanisterId::from_slice(&11_usize.to_ne_bytes())
}
