use pocket_ic::PocketIc;
use shared_utils::canister_specific::individual_user_template::types::profile::UserProfileDetailsForFrontendV5;
use shared_utils::canister_specific::user_info_service::types::{
    SubscriptionPlan, YralProSubscription,
};
use test_utils::canister_calls::{query, update};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned;
use test_utils::setup::test_constants::{
    get_global_super_admin_principal_id, get_mock_user_alice_principal_id,
    get_mock_user_bob_principal_id, get_mock_user_charlie_principal_id,
};

fn setup_test_user(
    pocket_ic: &PocketIc,
    user_service_canister: candid::Principal,
    user_principal: candid::Principal,
) {
    // Register the user first
    let result = update::<_, Result<(), String>>(
        pocket_ic,
        user_service_canister,
        user_principal,
        "register_new_user",
        (),
    )
    .expect("Failed to register new user");
    assert!(result.is_ok(), "User registration failed: {:?}", result);
}

fn get_user_subscription_plan(
    pocket_ic: &PocketIc,
    user_service_canister: candid::Principal,
    caller: candid::Principal,
    user_principal: candid::Principal,
) -> SubscriptionPlan {
    let profile_details = query::<_, Result<UserProfileDetailsForFrontendV5, String>>(
        pocket_ic,
        user_service_canister,
        caller,
        "get_user_profile_details_v5",
        (user_principal,),
    )
    .unwrap()
    .unwrap();

    profile_details.subscription_plan
}

#[test]
fn test_change_subscription_plan_from_free_to_pro() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_alice_principal_id();

    // Setup user with default Free plan
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    // Verify user starts with Free plan
    let initial_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    assert!(matches!(initial_plan, SubscriptionPlan::Free));

    // Change to Pro plan
    let new_pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, new_pro_plan),
    )
    .expect("Failed to change subscription plan");

    assert!(
        result.is_ok(),
        "Subscription plan change failed: {:?}",
        result
    );

    // Verify the plan was changed
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_change_subscription_plan_from_pro_to_free() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_bob_principal_id();

    // Setup user and change to Pro plan first
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 30,
        total_video_credits_alloted: 30,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set initial Pro plan");

    // Now change back to Free plan
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, SubscriptionPlan::Free),
    )
    .expect("Failed to change subscription plan");

    assert!(
        result.is_ok(),
        "Subscription plan change failed: {:?}",
        result
    );

    // Verify the plan was changed to Free
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    assert!(matches!(updated_plan, SubscriptionPlan::Free));
}

#[test]
fn test_change_subscripton_for_bot_account() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();

    let user_service_canister = service_canisters.user_info_service_canister_id;

    let admin_principal = get_global_super_admin_principal_id();
    let parent_account = get_mock_user_charlie_principal_id(); // Don't register this user
    let bot_account = get_mock_user_bob_principal_id();

    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_account,
        bot_account,
        admin_principal,
    );

    let new_pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (bot_account, new_pro_plan),
    )
    .expect("Failed to call change_subscription_plan");

    assert!(
        result.is_ok(),
        "Subscription plan change failed: {:?}",
        result
    );

    // Verify credits were added (10 + 20 = 30)
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_account,
        parent_account,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_change_subscription_plan_for_non_existent_user() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let non_existent_user = get_mock_user_charlie_principal_id(); // Don't register this user

    let new_pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (non_existent_user, new_pro_plan),
    )
    .expect("Failed to call change_subscription_plan");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User not found");
}

#[test]
fn test_add_pro_plan_free_video_credits_success() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_alice_principal_id();

    // Setup user and change to Pro plan
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 10,
        total_video_credits_alloted: 30,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set Pro plan");

    // Add 20 credits
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (user_principal, 20u32),
    )
    .expect("Failed to add video credits");

    assert!(result.is_ok(), "Adding video credits failed: {:?}", result);

    // Verify credits were added (10 + 20 = 30)
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 30);
            assert_eq!(pro_sub.total_video_credits_alloted, 30);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_add_pro_plan_free_video_credits_for_free_user() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_bob_principal_id();

    // Setup user with Free plan (default)
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    // Try to add credits to Free plan user
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (user_principal, 10u32),
    )
    .expect("Failed to call add_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User is on Free plan");
}

#[test]
fn test_add_pro_plan_free_video_credits_for_non_existent_user() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let non_existent_user = get_mock_user_charlie_principal_id(); // Don't register this user

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (non_existent_user, 10u32),
    )
    .expect("Failed to call add_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User not found");
}

#[test]
fn test_remove_pro_plan_free_video_credits_success() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_alice_principal_id();

    // Setup user and change to Pro plan with credits
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set Pro plan");

    // Remove 20 credits
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (user_principal, 20u32),
    )
    .expect("Failed to remove video credits");

    assert!(
        result.is_ok(),
        "Removing video credits failed: {:?}",
        result
    );

    // Verify credits were removed (50 - 20 = 30)
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 30);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_remove_pro_plan_free_video_credits_insufficient_credits() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_bob_principal_id();

    // Setup user and change to Pro plan with limited credits
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 5,
        total_video_credits_alloted: 5,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set Pro plan");

    // Try to remove more credits than available
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (user_principal, 10u32),
    )
    .expect("Failed to call remove_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Not enough free video credits");

    // Verify credits remain unchanged
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 5);
            assert_eq!(pro_sub.total_video_credits_alloted, 5);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_remove_pro_plan_free_video_credits_for_free_user() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_charlie_principal_id();

    // Setup user with Free plan (default)
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    // Try to remove credits from Free plan user
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (user_principal, 5u32),
    )
    .expect("Failed to call remove_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User is on Free plan");
}

#[test]
fn test_remove_pro_plan_free_video_credits_for_non_existent_user() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let non_existent_user = candid::Principal::anonymous(); // Use a different non-existent user

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (non_existent_user, 5u32),
    )
    .expect("Failed to call remove_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User not found");
}

#[test]
fn test_remove_all_pro_plan_free_video_credits() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_alice_principal_id();

    // Setup user and change to Pro plan with credits
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 25,
        total_video_credits_alloted: 25,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set Pro plan");

    // Remove exactly all credits
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (user_principal, 25u32),
    )
    .expect("Failed to remove video credits");

    assert!(
        result.is_ok(),
        "Removing all video credits failed: {:?}",
        result
    );

    // Verify all credits were removed
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 0);
            assert_eq!(pro_sub.total_video_credits_alloted, 25);
        }
        _ => panic!("Expected Pro plan, got: {:?}", updated_plan),
    }
}

#[test]
fn test_video_credits_never_exceed_allotted_limit() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let user_principal = get_mock_user_alice_principal_id();

    // Setup user and change to Pro plan with a specific allotment
    setup_test_user(&pocket_ic, user_service_canister, user_principal);

    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 20,
        total_video_credits_alloted: 50,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (user_principal, pro_plan),
    )
    .expect("Failed to set Pro plan");

    // Verify initial state
    let plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 20);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
            assert!(pro_sub.free_video_credits_left <= pro_sub.total_video_credits_alloted);
        }
        _ => panic!("Expected Pro plan"),
    }

    // Add credits that would reach the limit exactly
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (user_principal, 30u32),
    )
    .expect("Failed to add video credits");

    assert!(result.is_ok(), "Adding video credits failed: {:?}", result);

    // Verify credits are now at limit
    let updated_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match updated_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
            assert!(pro_sub.free_video_credits_left <= pro_sub.total_video_credits_alloted);
        }
        _ => panic!("Expected Pro plan"),
    }

    // Try to add more credits beyond the limit - this should fail
    let result_overflow = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (user_principal, 10u32),
    )
    .expect("Failed to call add_pro_plan_free_video_credits");

    assert!(result_overflow.is_err());
    assert!(result_overflow
        .unwrap_err()
        .contains("would exceed allotted limit"));

    // Verify that credits remain at the limit and weren't changed
    let final_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        user_principal,
        user_principal,
    );
    match final_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            // Credits should remain at the allotted limit
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
            assert!(
                pro_sub.free_video_credits_left <= pro_sub.total_video_credits_alloted,
                "Credits {} should never exceed allotted limit {}",
                pro_sub.free_video_credits_left,
                pro_sub.total_video_credits_alloted
            );
        }
        _ => panic!("Expected Pro plan"),
    }
}

// ============================================================================
// Bot Account Subscription Tests
// ============================================================================

/// Helper to setup a bot account with its parent
fn setup_bot_with_parent(
    pocket_ic: &PocketIc,
    user_service_canister: candid::Principal,
    parent_principal: candid::Principal,
    bot_principal: candid::Principal,
    admin_principal: candid::Principal,
) {
    // Register parent as main account
    setup_test_user(pocket_ic, user_service_canister, parent_principal);

    // Register bot linked to parent using accept_new_user_registration_v2
    let result = update::<_, Result<(), String>>(
        pocket_ic,
        user_service_canister,
        admin_principal,
        "accept_new_user_registration_v2",
        (bot_principal, true, Some(parent_principal)),
    )
    .expect("Failed to register bot");
    assert!(result.is_ok(), "Bot registration failed: {:?}", result);
}

#[test]
fn test_bot_inherits_parent_subscription_plan() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot_principal = get_mock_user_bob_principal_id();

    // Setup parent and bot accounts
    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        bot_principal,
        admin_principal,
    );

    // Verify both start with Free plan
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    assert!(matches!(parent_plan, SubscriptionPlan::Free));

    let bot_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot_principal,
        bot_principal,
    );
    assert!(matches!(bot_plan, SubscriptionPlan::Free));

    // Upgrade parent to Pro plan
    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 100,
        total_video_credits_alloted: 100,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (parent_principal, pro_plan),
    )
    .expect("Failed to change parent subscription plan");

    // Verify parent has Pro plan
    let parent_updated = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_updated {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 100);
            assert_eq!(pro_sub.total_video_credits_alloted, 100);
        }
        _ => panic!("Expected Pro plan for parent"),
    }

    // Verify bot also sees Pro plan (inherits from parent)
    let bot_updated = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot_principal,
        bot_principal,
    );
    println!("Bot subscription plan: {:?}", bot_updated);
    match bot_updated {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 100);
            assert_eq!(pro_sub.total_video_credits_alloted, 100);
        }
        _ => panic!(
            "Expected Pro plan for bot (inherited from parent), got: {:?}",
            bot_updated
        ),
    }
}

#[test]
fn test_change_subscription_plan_for_bot_updates_parent() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot_principal = get_mock_user_bob_principal_id();

    // Setup parent and bot accounts
    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        bot_principal,
        admin_principal,
    );

    // Change subscription plan using bot principal (should update parent)
    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (bot_principal, pro_plan),
    )
    .expect("Failed to change bot subscription plan");

    assert!(
        result.is_ok(),
        "Changing bot subscription plan failed: {:?}",
        result
    );

    // Verify parent's subscription was updated
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for parent after bot subscription change"),
    }

    // Verify bot sees the same plan
    let bot_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot_principal,
        bot_principal,
    );
    match bot_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 50);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for bot"),
    }
}

#[test]
fn test_add_credits_to_bot_updates_parent() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot_principal = get_mock_user_bob_principal_id();

    // Setup parent and bot accounts
    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        bot_principal,
        admin_principal,
    );

    // Set parent to Pro plan
    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 10,
        total_video_credits_alloted: 50,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (parent_principal, pro_plan),
    )
    .expect("Failed to set parent Pro plan");

    // Add credits using bot principal
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (bot_principal, 20u32),
    )
    .expect("Failed to add credits to bot");

    assert!(result.is_ok(), "Adding credits to bot failed: {:?}", result);

    // Verify parent's credits were updated (10 + 20 = 30)
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 30);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for parent"),
    }

    // Verify bot sees the updated credits
    let bot_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot_principal,
        bot_principal,
    );
    match bot_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 30);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for bot"),
    }
}

#[test]
fn test_remove_credits_from_bot_updates_parent() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot_principal = get_mock_user_bob_principal_id();

    // Setup parent and bot accounts
    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        bot_principal,
        admin_principal,
    );

    // Set parent to Pro plan with credits
    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 50,
        total_video_credits_alloted: 50,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (parent_principal, pro_plan),
    )
    .expect("Failed to set parent Pro plan");

    // Remove credits using bot principal
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (bot_principal, 15u32),
    )
    .expect("Failed to remove credits from bot");

    assert!(
        result.is_ok(),
        "Removing credits from bot failed: {:?}",
        result
    );

    // Verify parent's credits were updated (50 - 15 = 35)
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 35);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for parent"),
    }

    // Verify bot sees the updated credits
    let bot_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot_principal,
        bot_principal,
    );
    match bot_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 35);
            assert_eq!(pro_sub.total_video_credits_alloted, 50);
        }
        _ => panic!("Expected Pro plan for bot"),
    }
}

#[test]
fn test_bot_credits_shared_with_parent() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot1_principal = get_mock_user_bob_principal_id();
    let bot2_principal = get_mock_user_charlie_principal_id();

    // Setup parent with two bots
    setup_test_user(&pocket_ic, user_service_canister, parent_principal);

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "accept_new_user_registration_v2",
        (bot1_principal, true, Some(parent_principal)),
    )
    .expect("Failed to register bot1");

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "accept_new_user_registration_v2",
        (bot2_principal, true, Some(parent_principal)),
    )
    .expect("Failed to register bot2");

    // Set parent to Pro plan
    let pro_plan = SubscriptionPlan::Pro(YralProSubscription {
        free_video_credits_left: 100,
        total_video_credits_alloted: 100,
    });

    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "change_subscription_plan",
        (parent_principal, pro_plan),
    )
    .expect("Failed to set parent Pro plan");

    // Remove credits via bot1
    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (bot1_principal, 30u32),
    )
    .expect("Failed to remove credits via bot1");

    // Verify all accounts see the same updated credits (100 - 30 = 70)
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 70);
        }
        _ => panic!("Expected Pro plan for parent"),
    }

    let bot1_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot1_principal,
        bot1_principal,
    );
    match bot1_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 70);
        }
        _ => panic!("Expected Pro plan for bot1"),
    }

    let bot2_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot2_principal,
        bot2_principal,
    );
    match bot2_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 70);
        }
        _ => panic!("Expected Pro plan for bot2"),
    }

    // Add credits via bot2
    let _ = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (bot2_principal, 20u32),
    )
    .expect("Failed to add credits via bot2");

    // Verify all accounts see the new total (70 + 20 = 90)
    let parent_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        parent_principal,
    );
    match parent_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 90);
        }
        _ => panic!("Expected Pro plan for parent"),
    }

    let bot1_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot1_principal,
        bot1_principal,
    );
    match bot1_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 90);
        }
        _ => panic!("Expected Pro plan for bot1"),
    }

    let bot2_plan = get_user_subscription_plan(
        &pocket_ic,
        user_service_canister,
        bot2_principal,
        bot2_principal,
    );
    match bot2_plan {
        SubscriptionPlan::Pro(pro_sub) => {
            assert_eq!(pro_sub.free_video_credits_left, 90);
        }
        _ => panic!("Expected Pro plan for bot2"),
    }
}

#[test]
fn test_bot_cannot_operate_with_free_plan() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let parent_principal = get_mock_user_alice_principal_id();
    let bot_principal = get_mock_user_bob_principal_id();

    // Setup parent and bot accounts (both start with Free plan)
    setup_bot_with_parent(
        &pocket_ic,
        user_service_canister,
        parent_principal,
        bot_principal,
        admin_principal,
    );

    // Try to add credits to bot with Free plan (should fail)
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "add_pro_plan_free_video_credits",
        (bot_principal, 10u32),
    )
    .expect("Failed to call add_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User is on Free plan");

    // Try to remove credits from bot with Free plan (should fail)
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "remove_pro_plan_free_video_credits",
        (bot_principal, 5u32),
    )
    .expect("Failed to call remove_pro_plan_free_video_credits");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "User is on Free plan");
}

#[test]
fn test_bot_subscription_operations_fail_when_owner_not_found() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let non_existent_parent = candid::Principal::from_text("aaaaa-aa").unwrap();
    let bot_principal = get_mock_user_bob_principal_id();

    // Try to register bot with non-existent parent (should fail)
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        admin_principal,
        "accept_new_user_registration_v2",
        (bot_principal, true, Some(non_existent_parent)),
    )
    .expect("Failed to call accept_new_user_registration_v2");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Owner not found");
}
