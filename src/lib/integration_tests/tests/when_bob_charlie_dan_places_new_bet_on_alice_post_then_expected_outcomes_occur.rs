use std::time::{Duration, SystemTime};

use candid::Principal;
use ic_state_machine_tests::{CanisterId, PrincipalId, StateMachine, WasmResult};
use shared_utils::{
    canister_specific::individual_user_template::types::post::{
        PostDetailsForFrontend, PostDetailsFromFrontend,
    },
    common::types::{
        known_principal::KnownPrincipalType, top_posts::post_score_index_item::PostScoreIndexItem,
    },
    types::canister_specific::post_cache::error_types::TopPostsFetchError,
};
use test_utils::setup::{
    env::v0::{
        get_canister_id_of_specific_type_from_principal_id_map,
        get_initialized_env_with_provisioned_known_canisters,
    },
    test_constants::get_alice_principal_id,
};

#[test]
fn when_bob_charlie_dan_places_new_bet_on_alice_post_then_expected_outcomes_occur() {
    let state_machine = StateMachine::new();
    let known_principal_map = get_initialized_env_with_provisioned_known_canisters(&state_machine);
    let user_index_canister_id = get_canister_id_of_specific_type_from_principal_id_map(
        &known_principal_map,
        KnownPrincipalType::CanisterIdUserIndex,
    );
    let post_cache_canister_id = get_canister_id_of_specific_type_from_principal_id_map(
        &known_principal_map,
        KnownPrincipalType::CanisterIdPostCache,
    );
    let alice_principal_id = get_alice_principal_id();

    println!("🧪 user_index_canister_id: {:?}", user_index_canister_id);

    let alice_canister_id = state_machine.execute_ingress_as(
        alice_principal_id,
        user_index_canister_id,
        "get_requester_principals_canister_id_create_if_not_exists_and_optionally_allow_referrer",
        candid::encode_one(()).unwrap(),
    ).map(|reply_payload| {
        let (alice_canister_id,): (Principal,) = match reply_payload {
            WasmResult::Reply(payload) => candid::decode_args(&payload).unwrap(),
            _ => panic!("\n🛑 get_requester_principals_canister_id_create_if_not_exists_and_optionally_allow_referrer failed\n"),
        };
        alice_canister_id
    }).unwrap();

    println!("🧪 alice_canister_id: {:?}", alice_canister_id.to_text());

    let post_creation_time = SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_secs(1_678_438_993))
        .unwrap();
    state_machine.set_time(post_creation_time);

    let newly_created_post_id = state_machine
        .execute_ingress_as(
            alice_principal_id,
            CanisterId::new(PrincipalId(alice_canister_id)).unwrap(),
            "add_post_v2",
            candid::encode_args((PostDetailsFromFrontend {
                description: "This is a fun video to watch".to_string(),
                hashtags: vec!["fun".to_string(), "video".to_string()],
                video_uid: "abcd#1234".to_string(),
                creator_consent_for_inclusion_in_hot_or_not: true,
            },))
            .unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<u64, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 add_post_v2 failed\n"),
            };
            assert!(result.is_ok());
            result.unwrap()
        })
        .unwrap();

    println!("🧪 newly_created_post_id: {:?}", newly_created_post_id);

    let alice_first_post_detail = state_machine
        .query(
            CanisterId::new(PrincipalId(alice_canister_id)).unwrap(),
            "get_individual_post_details_by_id",
            candid::encode_args((newly_created_post_id,)).unwrap(),
        )
        .map(|reply_payload| {
            let post_details: PostDetailsForFrontend = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_individual_post_details_by_id failed\n"),
            };
            post_details
        })
        .unwrap();

    assert_eq!(alice_first_post_detail.created_at, post_creation_time);
    assert_eq!(alice_first_post_detail.home_feed_ranking_score, 3000);
    assert_eq!(
        alice_first_post_detail.hot_or_not_feed_ranking_score,
        Some(3000)
    );

    // Bob likes post
    // Bob shares post
    // Bob
    // Charlie likes post
    // Charlie shares post
    // Dan likes post
    // Dan shares post

    // let returned_posts: Vec<PostScoreIndexItem> = state_machine
    //     .query(
    //         post_cache_canister_id,
    //         "get_top_posts_aggregated_from_canisters_on_this_network_for_hot_or_not_feed",
    //         candid::encode_args((0 as u64,10 as u64)).unwrap(),
    //     )
    //     .map(|reply_payload| {
    //         let returned_posts: Result<Vec<PostScoreIndexItem>, TopPostsFetchError> = match reply_payload {
    //             WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
    //             _ => panic!("\n🛑 get_top_posts_aggregated_from_canisters_on_this_network_for_hot_or_not_feed failed\n"),
    //         };
    //         returned_posts.unwrap()
    //     })
    //     .unwrap();

    // assert_eq!(returned_posts.len(), 1);
}
