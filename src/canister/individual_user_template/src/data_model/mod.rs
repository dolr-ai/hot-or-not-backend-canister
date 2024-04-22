use std::{
    alloc::System,
    borrow::Borrow,
    collections::{BTreeMap, BTreeSet, HashSet},
    time::SystemTime,
};

use candid::{CandidType, Deserialize, Principal};
use ic_cdk::api::management_canister::provisional::CanisterId;
use serde::Serialize;
use serde_json_any_key::*;
use shared_utils::{
    canister_specific::individual_user_template::types::{
        configuration::IndividualUserConfiguration,
        follow::FollowData,
        hot_or_not::{
            AggregateStats, BetDetails, BetMaker, BetMakerPrincipal, GlobalBetId, GlobalRoomId,
            HotOrNotDetails, PlacedBetDetail, RoomDetailsV1, RoomId, SlotDetailsV1, SlotId,
            StablePrincipal,
        },
        post::{FeedScore, Post, PostViewStatistics},
        profile::UserProfile,
        session::SessionType,
        token::TokenBalance,
    },
    common::types::{
        app_primitive_type::PostId,
        known_principal::KnownPrincipalMap,
        top_posts::{post_score_index::PostScoreIndex, post_score_index_item::PostStatus},
        version_details::VersionDetails,
    },
};

use self::memory::{
    get_bet_details_memory, get_post_principal_memory, get_room_details_memory,
    get_slot_details_memory, Memory,
};

pub mod memory;

#[derive(Deserialize, Serialize)]
pub struct CanisterData {
    // Key is Post ID
    pub all_created_posts: BTreeMap<u64, Post>,
    #[serde(skip, default = "_default_room_details")]
    pub room_details_map:
        ic_stable_structures::btreemap::BTreeMap<GlobalRoomId, RoomDetailsV1, Memory>,
    #[serde(skip, default = "_default_bet_details")]
    pub bet_details_map: ic_stable_structures::btreemap::BTreeMap<GlobalBetId, BetDetails, Memory>,
    #[serde(skip, default = "_default_post_principal_map")]
    pub post_principal_map:
        ic_stable_structures::btreemap::BTreeMap<(PostId, StablePrincipal), (), Memory>,
    #[serde(skip, default = "_default_slot_details_map")]
    pub slot_details_map:
        ic_stable_structures::btreemap::BTreeMap<(PostId, SlotId), SlotDetailsV1, Memory>,
    pub all_hot_or_not_bets_placed: BTreeMap<(CanisterId, PostId), PlacedBetDetail>,
    pub configuration: IndividualUserConfiguration,
    pub follow_data: FollowData,
    pub known_principal_ids: KnownPrincipalMap,
    pub my_token_balance: TokenBalance,
    pub posts_index_sorted_by_home_feed_score: PostScoreIndex,
    pub posts_index_sorted_by_hot_or_not_feed_score: PostScoreIndex,
    pub principals_i_follow: BTreeSet<Principal>,
    pub principals_that_follow_me: BTreeSet<Principal>,
    pub profile: UserProfile,
    pub version_details: VersionDetails,
    #[serde(default)]
    pub session_type: Option<SessionType>,
    #[serde(default)]
    pub last_access_time: Option<SystemTime>,
    #[serde(default)]
    pub last_canister_functionality_access_time: Option<SystemTime>,
}

pub fn _default_room_details(
) -> ic_stable_structures::btreemap::BTreeMap<GlobalRoomId, RoomDetailsV1, Memory> {
    ic_stable_structures::btreemap::BTreeMap::init(get_room_details_memory())
}

pub fn _default_bet_details(
) -> ic_stable_structures::btreemap::BTreeMap<GlobalBetId, BetDetails, Memory> {
    ic_stable_structures::btreemap::BTreeMap::init(get_bet_details_memory())
}

pub fn _default_post_principal_map(
) -> ic_stable_structures::btreemap::BTreeMap<(PostId, StablePrincipal), (), Memory> {
    ic_stable_structures::btreemap::BTreeMap::init(get_post_principal_memory())
}

pub fn _default_slot_details_map(
) -> ic_stable_structures::btreemap::BTreeMap<(PostId, SlotId), SlotDetailsV1, Memory> {
    ic_stable_structures::btreemap::BTreeMap::init(get_slot_details_memory())
}

impl Default for CanisterData {
    fn default() -> Self {
        Self {
            all_created_posts: BTreeMap::new(),
            room_details_map: _default_room_details(),
            bet_details_map: _default_bet_details(),
            post_principal_map: _default_post_principal_map(),
            slot_details_map: _default_slot_details_map(),
            all_hot_or_not_bets_placed: BTreeMap::new(),
            configuration: IndividualUserConfiguration::default(),
            follow_data: FollowData::default(),
            known_principal_ids: KnownPrincipalMap::default(),
            my_token_balance: TokenBalance::default(),
            posts_index_sorted_by_home_feed_score: PostScoreIndex::default(),
            posts_index_sorted_by_hot_or_not_feed_score: PostScoreIndex::default(),
            principals_i_follow: BTreeSet::new(),
            principals_that_follow_me: BTreeSet::new(),
            profile: UserProfile::default(),
            version_details: VersionDetails::default(),
            session_type: None,
            last_access_time: None,
            last_canister_functionality_access_time: None,
        }
    }
}
