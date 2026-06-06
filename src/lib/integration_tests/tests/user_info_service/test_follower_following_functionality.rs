use candid::Principal;
use shared_utils::canister_specific::user_info_service::types::{
    FollowersResponse, FollowingResponse, ProfileUpdateDetails,
};
use test_utils::canister_calls::{query, update};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned;
use test_utils::setup::test_constants::{
    get_mock_user_alice_principal_id, get_mock_user_bob_principal_id,
    get_mock_user_charlie_principal_id, get_mock_user_dan_principal_id,
};

#[test]
fn test_comprehensive_follower_following_functionality() {
    let (pocket_ic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_service_canister = service_canisters.user_info_service_canister_id;

    let alice = get_mock_user_alice_principal_id();
    let bob = get_mock_user_bob_principal_id();
    let charlie = get_mock_user_charlie_principal_id();
    let dan = get_mock_user_dan_principal_id();

    // Setup: Register all users
    let users = vec![alice, bob, charlie, dan];
    for user in &users {
        let result = update::<_, Result<(), String>>(
            &pocket_ic,
            user_service_canister,
            *user,
            "register_new_user",
            (),
        )
        .expect("Failed to register user");
        assert!(
            result.is_ok(),
            "User registration failed for {:?}: {:?}",
            user,
            result
        );
    }

    // Test Follow Operations

    // Alice follows Bob
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (bob,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_ok(),
        "Alice should be able to follow Bob: {:?}",
        result
    );

    // Alice follows Charlie
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (charlie,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_ok(),
        "Alice should be able to follow Charlie: {:?}",
        result
    );

    // Bob follows Alice
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "follow_user",
        (alice,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_ok(),
        "Bob should be able to follow Alice: {:?}",
        result
    );

    // Charlie follows Alice
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "follow_user",
        (alice,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_ok(),
        "Charlie should be able to follow Alice: {:?}",
        result
    );

    // Charlie follows Dan
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "follow_user",
        (dan,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_ok(),
        "Charlie should be able to follow Dan: {:?}",
        result
    );

    // Test validation: Cannot follow yourself
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (alice,),
    )
    .expect("Failed to call follow_user");
    assert!(result.is_err(), "Should not be able to follow yourself");
    assert_eq!(result.unwrap_err(), "Cannot follow yourself");

    // Test validation: Cannot follow same user twice
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (bob,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_err(),
        "Should not be able to follow same user twice"
    );
    assert_eq!(result.unwrap_err(), "Already following this user");

    // Test validation: Cannot follow non-existent user
    let non_existent = Principal::from_text("2vxsx-fae").unwrap();
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (non_existent,),
    )
    .expect("Failed to call follow_user");
    assert!(
        result.is_err(),
        "Should not be able to follow non-existent user"
    );
    assert_eq!(result.unwrap_err(), "Target user not found");

    // Verify Followers

    // Alice should have 2 followers: Bob and Charlie
    let alice_followers = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Alice's followers");

    assert_eq!(
        alice_followers.total_count, 2,
        "Alice should have 2 followers"
    );
    assert!(
        alice_followers
            .followers
            .iter()
            .any(|f| f.principal_id == bob),
        "Bob should be in Alice's followers"
    );
    assert!(
        alice_followers
            .followers
            .iter()
            .any(|f| f.principal_id == charlie),
        "Charlie should be in Alice's followers"
    );

    // Bob should have 1 follower: Alice
    let bob_followers = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "get_followers",
        (bob, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Bob's followers");

    assert_eq!(bob_followers.total_count, 1, "Bob should have 1 follower");
    assert!(
        bob_followers
            .followers
            .iter()
            .any(|f| f.principal_id == alice),
        "Alice should be in Bob's followers"
    );

    // Charlie should have 1 follower: Alice
    let charlie_followers = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "get_followers",
        (charlie, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Charlie's followers");

    assert_eq!(
        charlie_followers.total_count, 1,
        "Charlie should have 1 follower"
    );
    assert!(
        charlie_followers
            .followers
            .iter()
            .any(|f| f.principal_id == alice),
        "Alice should be in Charlie's followers"
    );

    // Dan should have 1 follower: Charlie
    let dan_followers = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        dan,
        "get_followers",
        (dan, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Dan's followers");

    assert_eq!(dan_followers.total_count, 1, "Dan should have 1 follower");
    assert!(
        dan_followers
            .followers
            .iter()
            .any(|f| f.principal_id == charlie),
        "Charlie should be in Dan's followers"
    );

    // Verify Following

    // Alice should be following 2: Bob and Charlie
    let alice_following = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_following",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Alice's following");

    assert_eq!(
        alice_following.total_count, 2,
        "Alice should be following 2 users"
    );
    assert!(
        alice_following
            .following
            .iter()
            .any(|f| f.principal_id == bob),
        "Alice should be following Bob"
    );
    assert!(
        alice_following
            .following
            .iter()
            .any(|f| f.principal_id == charlie),
        "Alice should be following Charlie"
    );

    // Bob should be following 1: Alice
    let bob_following = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "get_following",
        (bob, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Bob's following");

    assert_eq!(
        bob_following.total_count, 1,
        "Bob should be following 1 user"
    );
    assert!(
        bob_following
            .following
            .iter()
            .any(|f| f.principal_id == alice),
        "Bob should be following Alice"
    );

    // Charlie should be following 2: Alice and Dan
    let charlie_following = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "get_following",
        (charlie, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Charlie's following");

    assert_eq!(
        charlie_following.total_count, 2,
        "Charlie should be following 2 users"
    );
    assert!(
        charlie_following
            .following
            .iter()
            .any(|f| f.principal_id == alice),
        "Charlie should be following Alice"
    );
    assert!(
        charlie_following
            .following
            .iter()
            .any(|f| f.principal_id == dan),
        "Charlie should be following Dan"
    );

    // Dan should be following 0
    let dan_following = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        dan,
        "get_following",
        (dan, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Dan's following");

    assert_eq!(
        dan_following.total_count, 0,
        "Dan should not be following anyone"
    );
    assert!(
        dan_following.following.is_empty(),
        "Dan's following list should be empty"
    );

    // Test Pagination

    // Test pagination with cursor - get first page with limit 1
    let alice_followers_page1 = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, None::<Principal>, 1u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Alice's followers page 1");

    assert_eq!(
        alice_followers_page1.followers.len(),
        1,
        "Should get 1 follower with limit 1"
    );
    assert_eq!(
        alice_followers_page1.total_count, 2,
        "Total count should be 2"
    );
    assert!(
        alice_followers_page1.next_cursor.is_some(),
        "Should have a next cursor for more pages"
    );

    // Test pagination with cursor - get second page using cursor
    println!(
        "First page followers: {:?}",
        alice_followers_page1.followers
    );
    println!(
        "Next cursor from first page: {:?}",
        alice_followers_page1.next_cursor
    );

    let alice_followers_page2 = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, alice_followers_page1.next_cursor, 1u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Alice's followers page 2");

    println!(
        "Second page followers: {:?}",
        alice_followers_page2.followers
    );
    println!(
        "Next cursor from second page: {:?}",
        alice_followers_page2.next_cursor
    );

    assert_eq!(
        alice_followers_page2.followers.len(),
        1,
        "Should get 1 follower on second page"
    );
    assert_eq!(
        alice_followers_page2.total_count, 2,
        "Total count should still be 2"
    );
    assert!(
        alice_followers_page2.next_cursor.is_none(),
        "Should not have a next cursor on last page"
    );

    // Test Unfollow Operations

    // Alice unfollows Bob
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "unfollow_user",
        (bob,),
    )
    .expect("Failed to call unfollow_user");
    assert!(
        result.is_ok(),
        "Alice should be able to unfollow Bob: {:?}",
        result
    );

    // Charlie unfollows Alice
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "unfollow_user",
        (alice,),
    )
    .expect("Failed to call unfollow_user");
    assert!(
        result.is_ok(),
        "Charlie should be able to unfollow Alice: {:?}",
        result
    );

    // Test validation: Cannot unfollow someone you're not following
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "unfollow_user",
        (dan,),
    )
    .expect("Failed to call unfollow_user");
    assert!(
        result.is_err(),
        "Should not be able to unfollow someone you're not following"
    );
    assert_eq!(result.unwrap_err(), "Not following this user");

    // Test validation: Cannot unfollow yourself
    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "unfollow_user",
        (alice,),
    )
    .expect("Failed to call unfollow_user");
    assert!(result.is_err(), "Should not be able to unfollow yourself");
    assert_eq!(result.unwrap_err(), "Cannot unfollow yourself");

    // Final Verification after unfollows

    // Alice should now have 1 follower: Bob (Charlie unfollowed)
    let alice_followers_final = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Alice's final followers");

    assert_eq!(
        alice_followers_final.total_count, 1,
        "Alice should have 1 follower after unfollows"
    );
    assert!(
        alice_followers_final
            .followers
            .iter()
            .any(|f| f.principal_id == bob),
        "Bob should still be in Alice's followers"
    );
    assert!(
        !alice_followers_final
            .followers
            .iter()
            .any(|f| f.principal_id == charlie),
        "Charlie should not be in Alice's followers"
    );

    // Alice should now be following 1: Charlie (unfollowed Bob)
    let alice_following_final = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_following",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Alice's final following");

    assert_eq!(
        alice_following_final.total_count, 1,
        "Alice should be following 1 user after unfollows"
    );
    assert!(
        !alice_following_final
            .following
            .iter()
            .any(|f| f.principal_id == bob),
        "Alice should not be following Bob"
    );
    assert!(
        alice_following_final
            .following
            .iter()
            .any(|f| f.principal_id == charlie),
        "Alice should still be following Charlie"
    );

    // Bob should have 0 followers (Alice unfollowed)
    let bob_followers_final = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "get_followers",
        (bob, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Bob's final followers");

    assert_eq!(
        bob_followers_final.total_count, 0,
        "Bob should have 0 followers after Alice unfollowed"
    );
    assert!(
        bob_followers_final.followers.is_empty(),
        "Bob's followers list should be empty"
    );

    // Charlie should be following 1: Dan (unfollowed Alice)
    let charlie_following_final = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "get_following",
        (charlie, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Charlie's final following");

    assert_eq!(
        charlie_following_final.total_count, 1,
        "Charlie should be following 1 user after unfollows"
    );
    assert!(
        !charlie_following_final
            .following
            .iter()
            .any(|f| f.principal_id == alice),
        "Charlie should not be following Alice"
    );
    assert!(
        charlie_following_final
            .following
            .iter()
            .any(|f| f.principal_id == dan),
        "Charlie should still be following Dan"
    );

    println!("✅ All follower/following functionality tests passed!");

    // Test profile details v4 includes correct follower/following counts
    println!("\n📊 Testing profile details v4 follower/following counts...");

    // Re-establish some follow relationships for testing profile counts
    // Current state: Alice follows Charlie, Bob follows Alice, Charlie follows Dan
    // We need: Alice to also follow Bob again
    update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "follow_user",
        (bob,),
    )
    .expect("Failed to follow Bob")
    .expect("Alice should follow Bob");

    // Alice is already following Charlie, Bob is already following Alice
    // No need to re-establish those relationships

    // Get Alice's profile details
    use shared_utils::canister_specific::individual_user_template::types::profile::UserProfileDetailsForFrontendV4;

    let alice_profile = query::<_, Result<UserProfileDetailsForFrontendV4, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_profile_details_v4",
        (alice,),
    )
    .expect("Failed to query profile details")
    .expect("Failed to get Alice's profile details");

    assert_eq!(
        alice_profile.following_count, 2,
        "Alice should be following 2 users (Bob and Charlie)"
    );
    assert_eq!(
        alice_profile.followers_count, 1,
        "Alice should have 1 follower (Bob)"
    );
    assert_eq!(
        alice_profile.caller_follows_user, None,
        "Alice querying own profile should have caller_follows_user as None"
    );

    // Get Bob's profile details
    let bob_profile = query::<_, Result<UserProfileDetailsForFrontendV4, String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "get_profile_details_v4",
        (bob,),
    )
    .expect("Failed to query profile details")
    .expect("Failed to get Bob's profile details");

    assert_eq!(
        bob_profile.following_count, 1,
        "Bob should be following 1 user (Alice)"
    );
    assert_eq!(
        bob_profile.followers_count, 1,
        "Bob should have 1 follower (Alice)"
    );
    assert_eq!(
        bob_profile.caller_follows_user, None,
        "Bob querying own profile should have caller_follows_user as None"
    );

    // Get Charlie's profile details (Charlie has Dan following from earlier test)
    let charlie_profile = query::<_, Result<UserProfileDetailsForFrontendV4, String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "get_profile_details_v4",
        (charlie,),
    )
    .expect("Failed to query profile details")
    .expect("Failed to get Charlie's profile details");

    assert_eq!(
        charlie_profile.following_count, 1,
        "Charlie should be following 1 user (Dan from earlier test)"
    );
    assert_eq!(
        charlie_profile.followers_count, 1,
        "Charlie should have 1 follower (Alice)"
    );
    assert_eq!(
        charlie_profile.caller_follows_user, None,
        "Charlie querying own profile should have caller_follows_user as None"
    );

    // Test caller_follows_user field when querying another user's profile
    println!("\n📊 Testing caller_follows_user field for cross-user queries...");

    // Alice queries Bob's profile (Alice is following Bob)
    let bob_profile_from_alice = query::<_, Result<UserProfileDetailsForFrontendV4, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_profile_details_v4",
        (bob,),
    )
    .expect("Failed to query Bob's profile from Alice")
    .expect("Failed to get Bob's profile from Alice");

    assert_eq!(
        bob_profile_from_alice.caller_follows_user,
        Some(true),
        "Alice is following Bob, so caller_follows_user should be Some(true)"
    );

    // Bob queries Charlie's profile (Bob is not following Charlie)
    let charlie_profile_from_bob = query::<_, Result<UserProfileDetailsForFrontendV4, String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "get_profile_details_v4",
        (charlie,),
    )
    .expect("Failed to query Charlie's profile from Bob")
    .expect("Failed to get Charlie's profile from Bob");

    assert_eq!(
        charlie_profile_from_bob.caller_follows_user,
        Some(false),
        "Bob is not following Charlie, so caller_follows_user should be Some(false)"
    );

    println!(
        "✅ Profile details v4 follower/following counts and caller_follows_user field verified!"
    );

    // Test profile pictures in follower/following lists
    println!("\n🖼️ Testing profile pictures in follower/following lists...");

    // Update Alice's profile picture
    let alice_profile_update = ProfileUpdateDetails {
        bio: Some("Alice's bio".to_string()),
        website_url: Some("https://alice.example.com".to_string()),
        profile_picture_url: Some("https://example.com/alice.jpg".to_string()),
    };

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "update_profile_details",
        (alice_profile_update,),
    )
    .expect("Failed to update Alice's profile");
    assert!(
        result.is_ok(),
        "Failed to update Alice's profile: {:?}",
        result
    );

    // Update Bob's profile picture
    let bob_profile_update = ProfileUpdateDetails {
        bio: None,
        website_url: None,
        profile_picture_url: Some("https://example.com/bob.jpg".to_string()),
    };

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        bob,
        "update_profile_details",
        (bob_profile_update,),
    )
    .expect("Failed to update Bob's profile");
    assert!(
        result.is_ok(),
        "Failed to update Bob's profile: {:?}",
        result
    );

    // Update Charlie's profile picture
    let charlie_profile_update = ProfileUpdateDetails {
        bio: Some("Charlie here!".to_string()),
        website_url: None,
        profile_picture_url: Some("https://example.com/charlie.jpg".to_string()),
    };

    let result = update::<_, Result<(), String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "update_profile_details",
        (charlie_profile_update,),
    )
    .expect("Failed to update Charlie's profile");
    assert!(
        result.is_ok(),
        "Failed to update Charlie's profile: {:?}",
        result
    );

    // Test get_followers WITHOUT profile pictures (default behavior)
    println!("\n📋 Testing get_followers without profile pictures...");
    let alice_followers_no_pics = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query followers")
    .expect("Failed to get Alice's followers without pics");

    // Verify Bob is a follower without profile picture
    let bob_follower = alice_followers_no_pics
        .followers
        .iter()
        .find(|f| f.principal_id == bob)
        .expect("Bob should be in Alice's followers");

    assert_eq!(
        bob_follower.profile_picture_url, None,
        "Profile picture should be None when not requested"
    );

    // Test get_followers WITH profile pictures
    println!("\n🖼️ Testing get_followers with profile pictures...");
    let alice_followers_with_pics = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_followers",
        (alice, None::<Principal>, 10u64, Some(true)),
    )
    .expect("Failed to query followers with pics")
    .expect("Failed to get Alice's followers with pics");

    // Verify Bob is a follower with profile picture
    let bob_follower_with_pic = alice_followers_with_pics
        .followers
        .iter()
        .find(|f| f.principal_id == bob)
        .expect("Bob should be in Alice's followers");

    assert_eq!(
        bob_follower_with_pic.profile_picture_url,
        Some("https://example.com/bob.jpg".to_string()),
        "Bob's profile picture should be included when requested"
    );

    // Test get_following WITHOUT profile pictures
    println!("\n📋 Testing get_following without profile pictures...");
    let alice_following_no_pics = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_following",
        (alice, None::<Principal>, 10u64, None::<bool>),
    )
    .expect("Failed to query following")
    .expect("Failed to get Alice's following without pics");

    // Verify Charlie is being followed without profile picture
    let charlie_following = alice_following_no_pics
        .following
        .iter()
        .find(|f| f.principal_id == charlie)
        .expect("Charlie should be in Alice's following");

    assert_eq!(
        charlie_following.profile_picture_url, None,
        "Profile picture should be None when not requested"
    );

    // Test get_following WITH profile pictures
    println!("\n🖼️ Testing get_following with profile pictures...");
    let alice_following_with_pics = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        alice,
        "get_following",
        (alice, None::<Principal>, 10u64, Some(true)),
    )
    .expect("Failed to query following with pics")
    .expect("Failed to get Alice's following with pics");

    // Verify both Bob and Charlie are being followed with profile pictures
    let bob_following = alice_following_with_pics
        .following
        .iter()
        .find(|f| f.principal_id == bob)
        .expect("Bob should be in Alice's following");

    assert_eq!(
        bob_following.profile_picture_url,
        Some("https://example.com/bob.jpg".to_string()),
        "Bob's profile picture should be included when requested"
    );

    let charlie_following_with_pic = alice_following_with_pics
        .following
        .iter()
        .find(|f| f.principal_id == charlie)
        .expect("Charlie should be in Alice's following");

    assert_eq!(
        charlie_following_with_pic.profile_picture_url,
        Some("https://example.com/charlie.jpg".to_string()),
        "Charlie's profile picture should be included when requested"
    );

    // Test with a user who has no profile picture set
    println!("\n📋 Testing user without profile picture...");
    let _dan_followers_with_pics = query::<_, Result<FollowersResponse, String>>(
        &pocket_ic,
        user_service_canister,
        dan,
        "get_followers",
        (dan, None::<Principal>, 10u64, Some(true)),
    )
    .expect("Failed to query Dan's followers with pics")
    .expect("Failed to get Dan's followers with pics");

    // Dan has no followers, but let's check Charlie's following list for Dan
    let charlie_following_with_dan = query::<_, Result<FollowingResponse, String>>(
        &pocket_ic,
        user_service_canister,
        charlie,
        "get_following",
        (charlie, None::<Principal>, 10u64, Some(true)),
    )
    .expect("Failed to query Charlie's following with pics")
    .expect("Failed to get Charlie's following with pics");

    let dan_in_following = charlie_following_with_dan
        .following
        .iter()
        .find(|f| f.principal_id == dan)
        .expect("Dan should be in Charlie's following");

    assert_eq!(
        dan_in_following.profile_picture_url, None,
        "Dan's profile picture should be None (never set)"
    );

    println!("✅ Profile pictures in follower/following lists tests passed!");
}
