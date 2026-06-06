use shared_utils::canister_specific::individual_user_template::types::session::SessionType;
use test_utils::canister_calls::{query, update};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned;
use test_utils::setup::test_constants::{
    get_global_super_admin_principal_id, get_mock_user_alice_principal_id,
    get_mock_user_charlie_principal_id,
};

#[test]
fn test_update_session_type_for_user_info_service() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let user_info_service_canister = service_canisters.user_info_service_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();
    let alice_principal_id = get_mock_user_alice_principal_id();
    let global_admin = get_global_super_admin_principal_id();

    // First, register Charlie as a user
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

    // Check initial session type (should be AnonymousSession)
    let initial_session_type = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "get_user_session_type",
        (charlie_principal_id,),
    )
    .unwrap()
    .unwrap();

    assert_eq!(initial_session_type, SessionType::AnonymousSession);

    // Try to update session type as regular user (should fail due to admin guard)
    let user_update_result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "update_session_type",
        (charlie_principal_id, SessionType::RegisteredSession),
    );

    // Should fail because regular user doesn't have admin permissions
    assert!(user_update_result.is_err());

    // Session type should still be AnonymousSession after failed user attempt
    let session_type_after_user_attempt = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "get_user_session_type",
        (charlie_principal_id,),
    )
    .unwrap()
    .unwrap();

    assert_eq!(
        session_type_after_user_attempt,
        SessionType::AnonymousSession
    );

    // Update session type as global admin (should succeed)
    let admin_update_result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_info_service_canister,
        global_admin,
        "update_session_type",
        (charlie_principal_id, SessionType::RegisteredSession),
    )
    .expect("Failed to update session type as admin");

    assert!(
        admin_update_result.is_ok(),
        "Admin update failed: {:?}",
        admin_update_result
    );

    // Verify session type has been updated
    let updated_session_type = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "get_user_session_type",
        (charlie_principal_id,),
    )
    .unwrap()
    .unwrap();

    assert_eq!(updated_session_type, SessionType::RegisteredSession);

    // Try to update back to AnonymousSession (should fail due to business logic)
    let _downgrade_result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_info_service_canister,
        global_admin,
        "update_session_type",
        (charlie_principal_id, SessionType::AnonymousSession),
    )
    .expect("Failed to call update_session_type for downgrade");

    // Final verification - session type should still be RegisteredSession
    let final_session_type = query::<_, Result<SessionType, String>>(
        &pocket_ic,
        user_info_service_canister,
        charlie_principal_id,
        "get_user_session_type",
        (charlie_principal_id,),
    )
    .unwrap()
    .unwrap();

    assert_eq!(final_session_type, SessionType::RegisteredSession);

    // Test updating session type for non-existent user (should fail)
    let non_existent_user_result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_info_service_canister,
        global_admin,
        "update_session_type",
        (alice_principal_id, SessionType::RegisteredSession),
    )
    .expect("Failed to call update_session_type for non-existent user");

    assert!(non_existent_user_result.is_err());
    assert!(non_existent_user_result
        .unwrap_err()
        .contains("User not found"));
}
