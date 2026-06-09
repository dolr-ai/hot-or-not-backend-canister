use candid::CandidType;
use ic_stable_structures::{storable::Bound, Storable};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;

use candid::Principal;
use std::{collections::HashSet, time::SystemTime};

use crate::{
    canister_specific::user_post_service::types::args::PostDetailsForFrontend,
    common::{
        types::top_posts::post_score_index_item::PostStatus,
        utils::system_time::get_current_system_time,
    },
};

//TODO: Create new struct for PostForFrontend

#[derive(CandidType, Clone, Deserialize, Debug, Serialize)]
pub struct Post {
    pub id: String,
    pub creator_principal: Principal,
    pub video_uid: String,
    pub description: String,
    pub hashtags: Vec<String>,
    pub status: PostStatus,
    pub created_at: SystemTime,
    pub likes: HashSet<Principal>,
    pub share_count: u64,
    pub view_stats: PostViewStatistics,
}

#[derive(Deserialize, CandidType)]
pub enum PostViewDetailsFromFrontend {
    WatchedPartially {
        percentage_watched: u8,
    },
    WatchedMultipleTimes {
        // * only send complete watches as part of this count
        watch_count: u8,
        percentage_watched: u8,
    },
}

#[derive(CandidType, Clone, Deserialize, Debug, Serialize, Default)]
pub struct PostViewStatistics {
    pub total_view_count: u64,
    pub threshold_view_count: u64,
    pub average_watch_percentage: u8,
}

impl Post {
    pub fn add_view_details(&mut self, details: &PostViewDetailsFromFrontend) {
        match details {
            PostViewDetailsFromFrontend::WatchedPartially { percentage_watched } => {
                assert!(*percentage_watched <= 100 && *percentage_watched > 0);
                self.view_stats.average_watch_percentage =
                    self.recalculate_average_watched(*percentage_watched, 0);
                self.view_stats.total_view_count += 1;
                if *percentage_watched > 20 {
                    self.view_stats.threshold_view_count += 1;
                }
            }
            PostViewDetailsFromFrontend::WatchedMultipleTimes {
                watch_count,
                percentage_watched,
            } => {
                assert!(*percentage_watched <= 100 && *percentage_watched > 0);
                self.view_stats.average_watch_percentage =
                    self.recalculate_average_watched(*percentage_watched, *watch_count);
                self.view_stats.total_view_count += (*watch_count + 1) as u64;
                self.view_stats.threshold_view_count += *watch_count as u64;
                if *percentage_watched > 20 {
                    self.view_stats.threshold_view_count += 1;
                }
            }
        }
    }

    pub fn get_post_details_for_frontend_for_user(
        &self,
        user: Principal,
    ) -> PostDetailsForFrontend {
        PostDetailsForFrontend {
            id: self.id.clone(),
            description: (self.description.clone()),
            hashtags: self.hashtags.clone(),
            video_uid: self.video_uid.clone(),
            creator_principal: self.creator_principal.clone(),
            created_at: self.created_at.clone(),
            total_view_count: self.view_stats.total_view_count,
            like_count: self.likes.len() as u64,
            created_by_user_principal_id: self.creator_principal,
            liked_by_me: self.likes.contains(&user),
        }
    }

    pub fn increment_share_count(&mut self) -> u64 {
        self.share_count += 1;
        self.share_count
    }

    fn recalculate_average_watched(&self, percentage_watched: u8, full_view_count: u8) -> u8 {
        let earlier_sum_component =
            self.view_stats.average_watch_percentage as u64 * self.view_stats.total_view_count;
        let current_full_view_component = 100 * full_view_count as u64;
        let current_total_dividend =
            earlier_sum_component + current_full_view_component + percentage_watched as u64;
        let current_total_divisor = self.view_stats.total_view_count + full_view_count as u64 + 1;

        (current_total_dividend / current_total_divisor) as u8
    }

    pub fn toggle_like_status(&mut self, user_principal_id: &Principal) -> bool {
        // if liked, return true & if unliked, return false
        if self.likes.contains(user_principal_id) {
            self.likes.remove(user_principal_id);
            false
        } else {
            self.likes.insert(*user_principal_id);
            true
        }
    }

    pub fn update_status(&mut self, status: PostStatus) {
        match (status, self.status) {
            (PostStatus::Uploaded, PostStatus::Draft) => {
                self.created_at = get_current_system_time();
            }
            _ => {}
        }
        self.status = status;
    }
}

impl Storable for Post {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = vec![];
        ciborium::ser::into_writer(self, &mut bytes).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, CandidType)]
pub struct PostIdList(pub Vec<u64>);

impl Storable for PostIdList {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = vec![];
        ciborium::ser::into_writer(self, &mut bytes).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).unwrap()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, CandidType)]
pub struct PostIdStringList(pub Vec<String>);

impl PostIdStringList {
    pub fn add_unique(&mut self, post_id: String) {
        if !self.0.contains(&post_id) {
            self.0.insert(0, post_id);
        }
    }

    pub fn remove(&mut self, post_id: &str) {
        self.0.retain(|id| id != post_id);
    }

    pub fn contains(&self, post_id: &str) -> bool {
        self.0.contains(&post_id.to_string())
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn push(&mut self, post_id: String) {
        if !self.0.contains(&post_id) {
            self.0.push(post_id);
        }
    }

    pub fn sort_by_creation_time<F>(&mut self, get_timestamp: F)
    where
        F: Fn(&str) -> Option<std::time::SystemTime>,
    {
        let mut posts_with_timestamps: Vec<(String, std::time::SystemTime)> = self
            .0
            .iter()
            .filter_map(|post_id| get_timestamp(post_id).map(|ts| (post_id.clone(), ts)))
            .collect();

        posts_with_timestamps.sort_by(|a, b| b.1.cmp(&a.1)); // Newest first
        self.0 = posts_with_timestamps
            .into_iter()
            .map(|(id, _)| id)
            .collect();
    }
}

impl Storable for PostIdStringList {
    const BOUND: Bound = Bound::Unbounded;

    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = vec![];
        ciborium::ser::into_writer(self, &mut bytes).unwrap();
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).unwrap()
    }
}
