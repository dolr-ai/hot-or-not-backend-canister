use candid::{CandidType, Deserialize};
use serde::Serialize;
use shared_utils::common::types::{
    known_principal::KnownPrincipalMap, top_posts::post_score_index::PostScoreIndex,
};

#[derive(Default, CandidType, Deserialize, Serialize)]
pub struct CanisterData {
    pub known_principal_ids: KnownPrincipalMap,
    pub posts_index_sorted_by_home_feed_score: PostScoreIndex,
    pub posts_index_sorted_by_hot_or_not_feed_score: PostScoreIndex,
}
