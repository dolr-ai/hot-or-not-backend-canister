use super::common::{GlobalRateLimitConfig, RateLimitResult, RateLimitStatus};
use test_utils::canister_calls::{query, update};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned;
use test_utils::setup::test_constants::get_mock_user_charlie_principal_id;

#[test]
fn test_increment_request_count_v1_basic() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let rate_limits_canister = service_canisters.rate_limits_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();

    // Check the default rate limit config
    let default_config = query::<_, GlobalRateLimitConfig>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "get_default_rate_limit_config",
        (),
    )
    .expect("Failed to get default rate limit config");

    println!(
        "Default config - registered: {}, unregistered: {}, window: {}",
        default_config.max_requests_per_window_registered,
        default_config.max_requests_per_window_unregistered,
        default_config.window_duration_seconds
    );

    // Set a custom rate limit for this user that allows multiple requests
    let admin_principal = test_utils::setup::test_constants::get_global_super_admin_principal_id();
    let set_result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        admin_principal,
        "set_principal_rate_limit",
        (charlie_principal_id, "default".to_string(), 10u64, 86400u64), // Allow 10 requests per day
    )
    .expect("Failed to set principal rate limit");

    match set_result {
        RateLimitResult::Ok(msg) => println!("Set rate limit: {}", msg),
        RateLimitResult::Err(e) => panic!("Failed to set rate limit: {}", e),
    }

    // Increment request count using v1 without payment
    let result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "increment_request_count_v1",
        (
            charlie_principal_id,
            "default".to_string(),
            false,
            false,
            None::<String>,
        ),
    )
    .expect("Failed to increment request count v1");

    match result {
        RateLimitResult::Ok(msg) => assert!(msg.contains("Request count incremented")),
        RateLimitResult::Err(e) => panic!("Failed to increment request count v1: {}", e),
    }

    // Check the status to verify increment
    let status = update::<_, Option<RateLimitStatus>>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "get_rate_limit_status",
        (charlie_principal_id, "default".to_string(), false),
    )
    .expect("Failed to get rate limit status")
    .expect("Expected status after increment");

    assert_eq!(status.request_count, 1);

    // Increment again
    let result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "increment_request_count_v1",
        (
            charlie_principal_id,
            "default".to_string(),
            false,
            false,
            None::<String>,
        ),
    )
    .expect("Failed to increment request count v1");

    match result {
        RateLimitResult::Ok(_msg) => {}
        RateLimitResult::Err(e) => panic!("Second increment failed: {}", e),
    }

    // Check count increased
    let status = update::<_, Option<RateLimitStatus>>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "get_rate_limit_status",
        (charlie_principal_id, "default".to_string(), false),
    )
    .expect("Failed to get rate limit status")
    .expect("Expected status after second increment");

    assert_eq!(status.request_count, 2);
}

#[test]
fn test_increment_request_count_v1_with_payment() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let rate_limits_canister = service_canisters.rate_limits_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();

    // Set a custom rate limit
    let admin_principal = test_utils::setup::test_constants::get_global_super_admin_principal_id();
    let set_result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        admin_principal,
        "set_principal_rate_limit",
        (charlie_principal_id, "default".to_string(), 5u64, 86400u64), // Allow 5 requests per day
    )
    .expect("Failed to set principal rate limit");

    match set_result {
        RateLimitResult::Ok(msg) => println!("Set rate limit: {}", msg),
        RateLimitResult::Err(e) => panic!("Failed to set rate limit: {}", e),
    }

    // Increment with payment
    let result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "increment_request_count_v1",
        (
            charlie_principal_id,
            "default".to_string(),
            false,
            true,
            Some("100".to_string()),
        ),
    )
    .expect("Failed to increment request count v1 with payment");

    match result {
        RateLimitResult::Ok(msg) => {
            println!("Increment with payment successful: {}", msg);
            assert!(msg.contains("Paid request processed"));
        }
        RateLimitResult::Err(e) => panic!("Failed to increment with payment: {}", e),
    }

    // Check the status - paid requests don't increment user's request count
    let status = update::<_, Option<RateLimitStatus>>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "get_rate_limit_status",
        (charlie_principal_id, "default".to_string(), false),
    )
    .expect("Failed to get rate limit status");

    // Paid requests don't increment user count, so status might be None or count is 0
    if let Some(status) = status {
        assert_eq!(
            status.request_count, 0,
            "Paid requests should not increment user count"
        );
    }

    // Try another paid request with different amount
    let result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "increment_request_count_v1",
        (
            charlie_principal_id,
            "default".to_string(),
            false,
            true,
            Some("200".to_string()),
        ),
    )
    .expect("Failed to increment request count v1 with payment");

    match result {
        RateLimitResult::Ok(_msg) => {}
        RateLimitResult::Err(e) => panic!("Second paid increment failed: {}", e),
    }

    // Check count - paid requests don't increment user count
    let status = update::<_, Option<RateLimitStatus>>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "get_rate_limit_status",
        (charlie_principal_id, "default".to_string(), false),
    )
    .expect("Failed to get rate limit status");

    // User count should still be 0 as both were paid requests
    if let Some(status) = status {
        assert_eq!(
            status.request_count, 0,
            "Paid requests should not increment user count"
        );
    }
}

#[test]
fn test_increment_request_count_v1_mixed_paid_unpaid() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let rate_limits_canister = service_canisters.rate_limits_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();

    // Set a custom rate limit
    let admin_principal = test_utils::setup::test_constants::get_global_super_admin_principal_id();
    let set_result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        admin_principal,
        "set_principal_rate_limit",
        (charlie_principal_id, "default".to_string(), 8u64, 86400u64),
    )
    .expect("Failed to set principal rate limit");

    match set_result {
        RateLimitResult::Ok(msg) => println!("Set rate limit: {}", msg),
        RateLimitResult::Err(e) => panic!("Failed to set rate limit: {}", e),
    }

    // Mix of paid and unpaid requests
    let test_cases = [
        (false, None, "unpaid request 1", 1), // Should increment count to 1
        (true, Some("50".to_string()), "paid request 1", 1), // Paid, count stays 1
        (false, None, "unpaid request 2", 2), // Should increment count to 2
        (true, Some("100".to_string()), "paid request 2", 2), // Paid, count stays 2
        (false, None, "unpaid request 3", 3), // Should increment count to 3
    ];

    for (is_paid, payment_amount, description, expected_count) in test_cases.iter() {
        let result = update::<_, RateLimitResult>(
            &pocket_ic,
            rate_limits_canister,
            charlie_principal_id,
            "increment_request_count_v1",
            (
                charlie_principal_id,
                "default".to_string(),
                false,
                *is_paid,
                payment_amount.clone(),
            ),
        )
        .unwrap_or_else(|_| panic!("Failed to increment for {}", description));

        match result {
            RateLimitResult::Ok(msg) => {
                println!("{}: {}", description, msg);
                // Check for both possible success messages
                assert!(
                    msg.contains("Request count incremented")
                        || msg.contains("Paid request processed"),
                    "Expected success message, got: {}",
                    msg
                );
            }
            RateLimitResult::Err(e) => panic!("{} failed: {}", description, e),
        }

        // Verify count
        let status = update::<_, Option<RateLimitStatus>>(
            &pocket_ic,
            rate_limits_canister,
            charlie_principal_id,
            "get_rate_limit_status",
            (charlie_principal_id, "default".to_string(), false),
        )
        .expect("Failed to get rate limit status")
        .expect("Expected status");

        assert_eq!(
            status.request_count, *expected_count as u64,
            "Expected count {} after {}",
            expected_count, description
        );
    }
}

#[test]
fn test_increment_request_count_v1_different_payment_amounts() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let rate_limits_canister = service_canisters.rate_limits_canister_id;
    let charlie_principal_id = get_mock_user_charlie_principal_id();

    // Set a custom rate limit
    let admin_principal = test_utils::setup::test_constants::get_global_super_admin_principal_id();
    let set_result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        admin_principal,
        "set_principal_rate_limit",
        (charlie_principal_id, "default".to_string(), 10u64, 86400u64),
    )
    .expect("Failed to set principal rate limit");

    match set_result {
        RateLimitResult::Ok(msg) => println!("Set rate limit: {}", msg),
        RateLimitResult::Err(e) => panic!("Failed to set rate limit: {}", e),
    }

    // Test different payment amounts - all paid requests
    let payment_amounts = ["10", "50", "100", "500", "1000"];

    for amount in payment_amounts.iter() {
        let result = update::<_, RateLimitResult>(
            &pocket_ic,
            rate_limits_canister,
            charlie_principal_id,
            "increment_request_count_v1",
            (
                charlie_principal_id,
                "default".to_string(),
                false,
                true,
                Some(amount.to_string()),
            ),
        )
        .unwrap_or_else(|_| panic!("Failed to increment with payment amount {}", amount));

        match result {
            RateLimitResult::Ok(msg) => {
                println!("Payment amount {}: {}", amount, msg);
                assert!(msg.contains("Paid request processed"));
            }
            RateLimitResult::Err(e) => panic!("Failed with payment amount {}: {}", amount, e),
        }

        // Verify count - paid requests don't increment user count
        let status = update::<_, Option<RateLimitStatus>>(
            &pocket_ic,
            rate_limits_canister,
            charlie_principal_id,
            "get_rate_limit_status",
            (charlie_principal_id, "default".to_string(), false),
        )
        .expect("Failed to get rate limit status");

        // All requests are paid, so user count should remain 0
        if let Some(status) = status {
            assert_eq!(
                status.request_count, 0,
                "Paid requests should not increment user count (amount: {})",
                amount
            );
        }
    }

    // Also test with is_paid=true but no amount
    let result = update::<_, RateLimitResult>(
        &pocket_ic,
        rate_limits_canister,
        charlie_principal_id,
        "increment_request_count_v1",
        (
            charlie_principal_id,
            "default".to_string(),
            false,
            true,
            None::<String>,
        ),
    )
    .expect("Failed to increment with is_paid=true but no amount");

    match result {
        RateLimitResult::Ok(msg) => {
            println!("Paid but no amount: {}", msg);
        }
        RateLimitResult::Err(e) => {
            println!("Expected behavior - paid but no amount error: {}", e);
        }
    }
}
