use shared_utils::canister_specific::individual_user_template::types::session::SessionType;
use test_utils::canister_calls::{query, update};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned;
use test_utils::setup::test_constants::{
    get_mock_user_alice_principal_id, get_mock_user_charlie_principal_id,
};

#[test]
fn test_check_session_type_for_user_info_service() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let user_info_service_canister = service_canisters.user_info_service_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();

    // First, register the user
    let registration_result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "register_new_user",
        (),
    )
    .expect("Failed to register new user");

    assert!(
        registration_result.is_ok(),
        "User registration failed: {:?}",
        registration_result
    );

    // Check session type for the registered user
    let session_type_result = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "get_user_session_type",
        (charlie_principal_id,),
    )
    .expect("Failed to get session type");

    assert!(
        session_type_result.is_ok(),
        "Failed to get session type: {:?}",
        session_type_result
    );

    let session_type = session_type_result.unwrap();

    // New users should start with AnonymousSession
    assert_eq!(session_type, SessionType::AnonymousSession);

    // Try to get session type for non-existent user (should fail)
    let non_existent_user = get_mock_user_alice_principal_id();
    let non_existent_result = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        non_existent_user,
        "get_user_session_type",
        (non_existent_user,),
    )
    .expect("Failed to call get_session_type for non-existent user");

    assert!(non_existent_result.is_err());
    assert!(non_existent_result.unwrap_err().contains("User not found"));
}
