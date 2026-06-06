use shared_utils::canister_specific::individual_user_template::types::error::GetPostsOfUserProfileError;
use shared_utils::canister_specific::user_post_service::types::{
    args::{
        PostDetailsForFrontend, PostDetailsFromFrontend, PostDetailsFromFrontendV1,
        PostStatusFromFrontend,
    },
    error::UserPostServiceError,
    storage::{Post, PostViewDetailsFromFrontend, PostViewStatistics},
};
use shared_utils::common::types::top_posts::post_score_index_item::PostStatus;
use std::{
    collections::HashSet,
    time::{Duration, SystemTime},
};
use test_utils::{
    canister_calls::{query, update},
    setup::{
        env::pocket_ic_env::get_new_pocket_ic_env_with_service_canisters_provisioned,
        test_constants::{
            get_global_super_admin_principal_id, get_mock_user_alice_principal_id,
            get_mock_user_bob_principal_id, get_mock_user_charlie_principal_id,
        },
    },
};

// Helper function to create a test post
fn create_test_post_details(
    id: &str,
    creator_principal: candid::Principal,
    video_uid: &str,
    description: &str,
) -> PostDetailsFromFrontend {
    PostDetailsFromFrontend {
        id: id.to_string(),
        creator_principal,
        video_uid: video_uid.to_string(),
        description: description.to_string(),
        hashtags: vec!["test".to_string(), "integration".to_string()],
    }
}

// Helper function to add a post as admin
fn add_post_as_admin(
    pic: &pocket_ic::PocketIc,
    user_post_service_canister_id: candid::Principal,
    post_details: PostDetailsFromFrontend,
) -> Result<(), UserPostServiceError> {
    let admin_principal = get_global_super_admin_principal_id();
    let result: Result<Result<(), UserPostServiceError>, Box<dyn std::error::Error>> = update(
        pic,
        user_post_service_canister_id,
        admin_principal,
        "add_post",
        (post_details,),
    );

    match result {
        Ok(inner_result) => inner_result,
        Err(e) => {
            eprintln!("Error calling add_post: {}", e);
            Err(UserPostServiceError::Unauthorized)
        }
    }
}

// Helper function to create a complete Post struct with controlled creation time
fn create_test_post_with_timestamp(
    id: &str,
    creator_principal: candid::Principal,
    video_uid: &str,
    description: &str,
    created_at: SystemTime,
) -> Post {
    Post {
        id: id.to_string(),
        creator_principal,
        video_uid: video_uid.to_string(),
        description: description.to_string(),
        hashtags: vec!["test".to_string(), "sorting".to_string()],
        status: PostStatus::Uploaded,
        created_at,
        likes: HashSet::new(),
        share_count: 0,
        view_stats: PostViewStatistics {
            total_view_count: 0,
            threshold_view_count: 0,
            average_watch_percentage: 0,
        },
    }
}

// Helper function to sync a post with controlled creation time
fn sync_post_with_timestamp(
    pic: &pocket_ic::PocketIc,
    user_post_service_canister_id: candid::Principal,
    post: Post,
) -> Result<Option<Post>, Box<dyn std::error::Error>> {
    let admin_principal = get_global_super_admin_principal_id();
    update(
        pic,
        user_post_service_canister_id,
        admin_principal,
        "sync_post_from_individual_canister",
        (post,),
    )
}

#[test]
fn test_get_posts_of_this_user_profile_with_pagination() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();

    // Add multiple posts for Alice
    for i in 0..5 {
        let post_details = create_test_post_details(
            &format!("alice_pagination_post_{}", i),
            alice_principal,
            &format!("video_alice_pagination_{}", i),
            &format!("Alice's pagination post #{}", i),
        );
        let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    }

    // Add multiple posts for Bob
    for i in 0..3 {
        let post_details = create_test_post_details(
            &format!("bob_pagination_post_{}", i),
            bob_principal,
            &format!("video_bob_pagination_{}", i),
            &format!("Bob's pagination post #{}", i),
        );
        let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    }

    // Get Alice's posts with pagination (limit: 3, offset: 0)
    let posts = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 0usize, 3usize),
    );
    let posts = posts.unwrap().unwrap();
    assert_eq!(posts.len(), 3);
    for post in &posts {
        assert_eq!(post.creator_principal, alice_principal);
        assert!(post.id.starts_with("alice_pagination_post_"));
    }

    let posts_res = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 6usize, 3usize),
    )
    .unwrap();

    assert!(posts_res.is_err());

    assert!(matches!(
        posts_res,
        Err(GetPostsOfUserProfileError::ReachedEndOfItemsList)
    ));
}

#[test]
fn test_add_post_success() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let post_details = create_test_post_details(
        "test_post_1",
        alice_principal,
        "video_123",
        "Test post description",
    );

    let result = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    assert!(result.is_ok());
}

#[test]
fn test_add_post_unauthorized() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let post_details = create_test_post_details(
        "test_post_1",
        alice_principal,
        "video_123",
        "Test post description",
    );

    // Try to add post as regular user (should fail)
    let result: Result<Result<(), UserPostServiceError>, Box<dyn std::error::Error>> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "add_post",
        (post_details,),
    );

    // Should fail at the transport level due to unauthorized access
    assert!(result.is_err());
}

#[test]
fn test_add_duplicate_post() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let post_details = create_test_post_details(
        "duplicate_post",
        alice_principal,
        "video_123",
        "Test post description",
    );

    // Add post first time - should succeed
    let result = add_post_as_admin(&pic, user_post_service_canister_id, post_details.clone());
    assert!(result.is_ok());

    // Try to add same post again - should fail
    let result = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    assert!(result.is_err());
}

#[test]
fn test_get_individual_post_details_by_id_success() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "get_test_post",
        alice_principal,
        "video_456",
        "Post to retrieve",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Get the post
    let retrieved_post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("get_test_post",),
    )
    .unwrap();

    let post = retrieved_post.unwrap();
    assert_eq!(post.id, "get_test_post");
    assert_eq!(post.creator_principal, alice_principal);
    assert_eq!(post.video_uid, "video_456");
    assert_eq!(post.description, "Post to retrieve");
    assert_eq!(
        post.hashtags,
        vec!["test".to_string(), "integration".to_string()]
    );
    assert_eq!(post.share_count, 0);
    assert!(post.likes.is_empty());
}

#[test]
fn test_get_individual_post_details_by_id_not_found() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Try to get a non-existent post
    let result: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("non_existent_post",),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_get_posts_of_this_user_profile_with_pagination_cursor() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();

    // Add multiple posts for Alice
    for i in 0..5 {
        let post_details = create_test_post_details(
            &format!("alice_post_{}", i),
            alice_principal,
            &format!("video_alice_{}", i),
            &format!("Alice's post #{}", i),
        );
        let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    }

    // Add multiple posts for Bob
    for i in 0..3 {
        let post_details = create_test_post_details(
            &format!("bob_post_{}", i),
            bob_principal,
            &format!("video_bob_{}", i),
            &format!("Bob's post #{}", i),
        );
        let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);
    }

    // Get Alice's posts with pagination
    let posts: Vec<Post> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination_cursor",
        (alice_principal, 3usize, 0usize), // limit: 3, offset: 0
    )
    .unwrap();

    assert_eq!(posts.len(), 3);
    // Verify they are Alice's posts
    for post in &posts {
        assert_eq!(post.creator_principal, alice_principal);
        assert!(post.id.starts_with("alice_post_"));
    }

    // Test offset
    let posts: Vec<Post> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination_cursor",
        (alice_principal, 3usize, 2usize), // limit: 3, offset: 2
    )
    .unwrap();

    assert_eq!(posts.len(), 3);
    for post in &posts {
        assert_eq!(post.creator_principal, alice_principal);
    }

    // Get Bob's posts
    let posts: Vec<Post> = query(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "get_posts_of_this_user_profile_with_pagination_cursor",
        (bob_principal, 10usize, 0usize),
    )
    .unwrap();

    assert_eq!(posts.len(), 3);
    for post in &posts {
        assert_eq!(post.creator_principal, bob_principal);
        assert!(post.id.starts_with("bob_post_"));
    }
}

#[test]
fn test_update_post_add_view_details_watched_partially() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "view_test_post",
        alice_principal,
        "video_view",
        "Post for view testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Add view details
    let view_details = PostViewDetailsFromFrontend::WatchedPartially {
        percentage_watched: 50,
    };

    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_add_view_details",
        ("view_test_post", view_details),
    )
    .unwrap();

    assert!(result.is_ok());

    // Verify the view stats were updated
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("view_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.view_stats.total_view_count, 1);
    assert_eq!(post.view_stats.threshold_view_count, 1); // 50% > 20%
    assert_eq!(post.view_stats.average_watch_percentage, 50);
}

#[test]
fn test_update_post_add_view_details_watched_multiple_times() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "multi_view_test_post",
        alice_principal,
        "video_multi_view",
        "Post for multiple view testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Add view details for multiple watches
    let view_details = PostViewDetailsFromFrontend::WatchedMultipleTimes {
        watch_count: 3,
        percentage_watched: 80,
    };

    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_add_view_details",
        ("multi_view_test_post", view_details),
    )
    .unwrap();

    assert!(result.is_ok());

    // Verify the view stats were updated
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("multi_view_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.view_stats.total_view_count, 4); // 3 + 1
    assert_eq!(post.view_stats.threshold_view_count, 4); // All views > 20%
                                                         // Average should consider 3 full views (100%) + 1 partial view (80%)
    assert_eq!(post.view_stats.average_watch_percentage, 95); // (100*3 + 80)/4 = 95
}

#[test]
fn test_update_post_add_view_details_nonexistent_post() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let view_details = PostViewDetailsFromFrontend::WatchedPartially {
        percentage_watched: 50,
    };

    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_add_view_details",
        ("nonexistent_post", view_details),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_update_post_increment_share_count() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "share_test_post",
        alice_principal,
        "video_share",
        "Post for share testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Increment share count
    let share_count: Result<u64, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_increment_share_count",
        ("share_test_post",),
    )
    .unwrap();

    assert_eq!(share_count.unwrap(), 1);

    // Increment again
    let share_count: Result<u64, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_increment_share_count",
        ("share_test_post",),
    )
    .unwrap();

    assert_eq!(share_count.unwrap(), 2);

    // Verify the post has the updated share count
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("share_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.share_count, 2);
}

#[test]
fn test_update_post_increment_share_count_nonexistent_post() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let result: Result<u64, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_increment_share_count",
        ("nonexistent_post",),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_update_post_toggle_like_status_by_caller() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "like_test_post",
        alice_principal,
        "video_like",
        "Post for like testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Bob likes the post
    let liked: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "update_post_toggle_like_status_by_caller",
        ("like_test_post",),
    )
    .unwrap();

    assert!(liked.unwrap());

    // Verify the post has the like
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("like_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.likes.len(), 1);
    assert!(post.likes.contains(&bob_principal));

    // Bob unlikes the post
    let liked: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "update_post_toggle_like_status_by_caller",
        ("like_test_post",),
    )
    .unwrap();

    assert!(!liked.unwrap());

    // Verify the like is removed
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("like_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.likes.len(), 0);
}

#[test]
fn test_update_post_toggle_like_status_multiple_users() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();
    let charlie_principal = get_mock_user_charlie_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "multi_like_test_post",
        alice_principal,
        "video_multi_like",
        "Post for multiple like testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Bob likes the post
    let liked: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "update_post_toggle_like_status_by_caller",
        ("multi_like_test_post",),
    )
    .unwrap();
    assert!(liked.unwrap());

    // Charlie likes the post
    let liked: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        charlie_principal,
        "update_post_toggle_like_status_by_caller",
        ("multi_like_test_post",),
    )
    .unwrap();
    assert!(liked.unwrap());

    // Alice likes her own post
    let liked: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_toggle_like_status_by_caller",
        ("multi_like_test_post",),
    )
    .unwrap();
    assert!(liked.unwrap());

    // Verify all likes are present
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("multi_like_test_post",),
    )
    .unwrap();

    let post = post.unwrap();
    assert_eq!(post.likes.len(), 3);
    assert!(post.likes.contains(&alice_principal));
    assert!(post.likes.contains(&bob_principal));
    assert!(post.likes.contains(&charlie_principal));
}

#[test]
fn test_update_post_toggle_like_status_nonexistent_post() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let result: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_toggle_like_status_by_caller",
        ("nonexistent_post",),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_delete_post_success() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "delete_test_post",
        alice_principal,
        "video_delete",
        "Post to delete",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Verify the post exists
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("delete_test_post",),
    )
    .unwrap();
    assert!(post.is_ok());

    // Delete the post as the creator (Alice)
    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "delete_post",
        ("delete_test_post",),
    )
    .unwrap();

    assert!(result.is_ok());

    // Verify the post no longer exists
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("delete_test_post",),
    )
    .unwrap();
    assert!(matches!(post, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_delete_post_unauthorized() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "delete_unauthorized_test_post",
        alice_principal,
        "video_delete_unauthorized",
        "Post to delete unauthorized",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Try to delete the post as a different user (Bob - should fail)
    let bob_principal = get_mock_user_bob_principal_id();
    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "delete_post",
        ("delete_unauthorized_test_post",),
    )
    .unwrap();

    assert!(result.is_err());

    // Verify the post still exists
    let post: Result<Post, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("delete_unauthorized_test_post",),
    )
    .unwrap();
    assert!(post.is_ok());
}

#[test]
fn test_delete_post_nonexistent() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();

    let result: Result<(), UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        admin_principal,
        "delete_post",
        ("nonexistent_post",),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_get_individual_post_details_by_id_for_user_success() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "test_post_for_user",
        alice_principal,
        "video_789",
        "Test post for user details",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Get post details as Alice (creator)
    let result: Result<PostDetailsForFrontend, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id_for_user",
        ("test_post_for_user".to_string(), alice_principal),
    )
    .unwrap();

    let post_details = result.unwrap();
    assert_eq!(post_details.id, "test_post_for_user");
    assert_eq!(post_details.creator_principal, alice_principal);
    assert_eq!(post_details.video_uid, "video_789");
    assert_eq!(post_details.description, "Test post for user details");
    assert_eq!(
        post_details.hashtags,
        vec!["test".to_string(), "integration".to_string()]
    );
    assert_eq!(post_details.like_count, 0);
    assert!(!post_details.liked_by_me);

    // Get post details as Bob (different user)
    let result: Result<PostDetailsForFrontend, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "get_individual_post_details_by_id_for_user",
        ("test_post_for_user".to_string(), bob_principal),
    )
    .unwrap();

    let post_details = result.unwrap();
    assert_eq!(post_details.id, "test_post_for_user");
    assert_eq!(post_details.creator_principal, alice_principal);
    assert_eq!(post_details.video_uid, "video_789");
    assert_eq!(post_details.description, "Test post for user details");
    assert_eq!(
        post_details.hashtags,
        vec!["test".to_string(), "integration".to_string()]
    );
    assert_eq!(post_details.like_count, 0);
    assert!(!post_details.liked_by_me);
}

#[test]
fn test_get_individual_post_details_by_id_for_user_not_found() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Try to get a non-existent post
    let result: Result<PostDetailsForFrontend, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id_for_user",
        ("non_existent_post".to_string(), alice_principal),
    )
    .unwrap();

    assert!(matches!(result, Err(UserPostServiceError::PostNotFound)));
}

#[test]
fn test_get_individual_post_details_by_id_for_user_with_likes() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();
    let bob_principal = get_mock_user_bob_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "liked_post",
        alice_principal,
        "video_likes",
        "Test post with likes",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Bob likes the post
    let _: Result<bool, UserPostServiceError> = update(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "update_post_toggle_like_status_by_caller",
        ("liked_post",),
    )
    .unwrap();

    // Get post details as Bob
    let result: Result<PostDetailsForFrontend, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        bob_principal,
        "get_individual_post_details_by_id_for_user",
        ("liked_post".to_string(), bob_principal),
    )
    .unwrap();

    let post_details = result.unwrap();
    assert_eq!(post_details.like_count, 1);
    assert!(post_details.liked_by_me);

    // Get post details as Alice
    let result: Result<PostDetailsForFrontend, UserPostServiceError> = query(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id_for_user",
        ("liked_post".to_string(), alice_principal),
    )
    .unwrap();

    let post_details = result.unwrap();
    assert_eq!(post_details.like_count, 1);
    assert!(!post_details.liked_by_me);
}

#[test]
fn test_update_post_status_success() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "status_test_post",
        alice_principal,
        "video_status",
        "Post for status testing",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Update post status as admin
    let result = update::<_, ()>(
        &pic,
        user_post_service_canister_id,
        admin_principal,
        "update_post_status",
        (
            "status_test_post".to_string(),
            PostStatus::BannedDueToUserReporting,
        ),
    );
    assert!(result.is_ok());

    // Verify status was updated
    let post = query::<_, Result<Post, UserPostServiceError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_individual_post_details_by_id",
        ("status_test_post",),
    )
    .unwrap();
    let post = post.unwrap();
    assert_eq!(post.status, PostStatus::BannedDueToUserReporting);
}

#[test]
fn test_update_post_status_unauthorized() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    // Add a post first
    let post_details = create_test_post_details(
        "unauth_status_post",
        alice_principal,
        "video_unauth_status",
        "Post for unauthorized status update",
    );
    let _ = add_post_as_admin(&pic, user_post_service_canister_id, post_details);

    // Try to update post status as non-admin (should fail)
    let result: Result<Result<(), UserPostServiceError>, Box<dyn std::error::Error>> = update(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "update_post_status",
        (
            "unauth_status_post".to_string(),
            PostStatus::BannedDueToUserReporting,
        ),
    );
    assert!(result.is_err());
}

#[test]
fn test_update_post_status_nonexistent_post() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();

    // Try to update status of a non-existent post
    let result = update::<_, ()>(
        &pic,
        user_post_service_canister_id,
        admin_principal,
        "update_post_status",
        (
            "nonexistent_status_post".to_string(),
            PostStatus::BannedDueToUserReporting,
        ),
    );
    // Should error (likely panic or unwrap error in canister)
    assert!(result.is_err());
}

#[test]
fn test_posts_are_sorted_by_creation_time_desc() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let now = SystemTime::now();
    let one_hour_ago = now - std::time::Duration::from_secs(3600);
    let two_hours_ago = now - std::time::Duration::from_secs(7200);

    let post2 = create_test_post_with_timestamp(
        "post_second_created",
        alice_principal,
        "video_2",
        "Second created post (middle)",
        one_hour_ago,
    );
    let _ = sync_post_with_timestamp(&pic, user_post_service_canister_id, post2);

    let post3 = create_test_post_with_timestamp(
        "post_third_created",
        alice_principal,
        "video_3",
        "Third created post (newest)",
        now,
    );
    let _ = sync_post_with_timestamp(&pic, user_post_service_canister_id, post3);

    // Create posts with controlled timestamps
    let post1 = create_test_post_with_timestamp(
        "post_first_created",
        alice_principal,
        "video_1",
        "First created post (oldest)",
        two_hours_ago,
    );
    let _ = sync_post_with_timestamp(&pic, user_post_service_canister_id, post1);

    // Get posts using pagination method
    let posts_result = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 0usize, 10usize),
    );

    let posts = posts_result.unwrap().unwrap();
    assert_eq!(posts.len(), 3);

    // Posts should be in reverse chronological order (most recent first)
    assert_eq!(posts[0].id, "post_third_created"); // Most recent
    assert_eq!(posts[1].id, "post_second_created"); // Middle
    assert_eq!(posts[2].id, "post_first_created"); // Oldest

    // Verify that each post has a later or equal creation time than the next
    for i in 0..posts.len() - 1 {
        assert!(
            posts[i].created_at >= posts[i + 1].created_at,
            "Posts not properly sorted by creation time. Post {} created at {:?}, Post {} created at {:?}",
            i, posts[i].created_at, i + 1, posts[i + 1].created_at
        );
    }

    // Verify the exact timestamps match what we set
    assert_eq!(posts[0].created_at, now);
    assert_eq!(posts[1].created_at, one_hour_ago);
    assert_eq!(posts[2].created_at, two_hours_ago);
}

#[test]
fn test_posts_pagination_preserves_sorting() {
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let alice_principal = get_mock_user_alice_principal_id();

    let now = SystemTime::now();

    // Create 6 posts with controlled timestamps (each 1 hour apart)
    for i in 0..6 {
        let created_at = now - std::time::Duration::from_secs((5 - i) * 3600); // i=0 -> 5 hours ago, i=5 -> now
        let post = create_test_post_with_timestamp(
            &format!("sorted_post_{}", i),
            alice_principal,
            &format!("video_{}", i),
            &format!("Post number {}", i),
            created_at,
        );
        let _ = sync_post_with_timestamp(&pic, user_post_service_canister_id, post);
    }

    // Get first page (3 posts)
    let page1_result = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 0usize, 3usize),
    );

    // Get second page (3 posts)
    let page2_result = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 3usize, 3usize),
    );

    let page1_posts = page1_result.unwrap().unwrap();
    let page2_posts = page2_result.unwrap().unwrap();

    assert_eq!(page1_posts.len(), 3);
    assert_eq!(page2_posts.len(), 3);

    // First page should have the most recent posts (sorted_post_5, sorted_post_4, sorted_post_3)
    assert_eq!(page1_posts[0].id, "sorted_post_5"); // Most recent
    assert_eq!(page1_posts[1].id, "sorted_post_4");
    assert_eq!(page1_posts[2].id, "sorted_post_3");

    // Second page should have the older posts (sorted_post_2, sorted_post_1, sorted_post_0)
    assert_eq!(page2_posts[0].id, "sorted_post_2");
    assert_eq!(page2_posts[1].id, "sorted_post_1");
    assert_eq!(page2_posts[2].id, "sorted_post_0"); // Oldest
}

#[test]
fn test_published_posts_ordered_by_publish_time_not_creation_time() {
    // Verifies that when draft posts are published in a different order than they were
    // created, the feed reflects the publish order (newest publish first).
    let (pic, service_canisters) = get_new_pocket_ic_env_with_service_canisters_provisioned();
    let user_post_service_canister_id = service_canisters.user_post_service_canister_id;
    let admin_principal = get_global_super_admin_principal_id();
    let alice_principal = get_mock_user_alice_principal_id();

    // Add three posts as drafts (creation order: A, B, C)
    for id in ["draft_post_a", "draft_post_b", "draft_post_c"] {
        let result: Result<Result<(), UserPostServiceError>, Box<dyn std::error::Error>> = update(
            &pic,
            user_post_service_canister_id,
            admin_principal,
            "add_post_v1",
            (PostDetailsFromFrontendV1 {
                id: id.to_string(),
                description: format!("Draft post {}", id),
                hashtags: vec![],
                video_uid: format!("video_{}", id),
                creator_principal: alice_principal,
                status: PostStatusFromFrontend::Draft,
            },),
        );
        assert!(result.is_ok(), "Failed to add draft post {}", id);
    }

    // Confirm none appear in the published feed yet
    let feed_before: Vec<Post> = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 0usize, 10usize),
    )
    .unwrap()
    .unwrap_or_default();
    assert_eq!(feed_before.len(), 0, "Draft posts must not appear in feed");

    // Publish in order: C → A → B, each 1 hour apart
    // Expected feed order (newest publish first): B, A, C
    for id in ["draft_post_c", "draft_post_a", "draft_post_b"] {
        pic.advance_time(Duration::from_secs(3600));
        pic.tick();
        let result = update::<_, ()>(
            &pic,
            user_post_service_canister_id,
            admin_principal,
            "update_post_status",
            (id.to_string(), PostStatus::Uploaded),
        );
        assert!(result.is_ok(), "Failed to publish {}", id);
    }

    // Fetch the published feed
    let feed: Vec<Post> = query::<_, Result<Vec<Post>, GetPostsOfUserProfileError>>(
        &pic,
        user_post_service_canister_id,
        alice_principal,
        "get_posts_of_this_user_profile_with_pagination",
        (alice_principal, 0usize, 10usize),
    )
    .unwrap()
    .unwrap();

    assert_eq!(feed.len(), 3);

    // Most recently published should come first
    assert_eq!(feed[0].id, "draft_post_b"); // published last
    assert_eq!(feed[1].id, "draft_post_a"); // published second
    assert_eq!(feed[2].id, "draft_post_c"); // published first

    // created_at timestamps must be strictly decreasing (newest first)
    assert!(
        feed[0].created_at > feed[1].created_at,
        "draft_post_b should have a later created_at than draft_post_a"
    );
    assert!(
        feed[1].created_at > feed[2].created_at,
        "draft_post_a should have a later created_at than draft_post_c"
    );
}
