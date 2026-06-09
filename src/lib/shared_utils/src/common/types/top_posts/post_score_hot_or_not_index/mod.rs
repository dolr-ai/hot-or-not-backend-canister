use candid::{CandidType, Deserialize};
use serde::Serialize;
use std::{
    collections::{btree_map::Iter, BTreeMap, HashMap},
    iter::{Chain, Rev},
    slice, vec,
};

use crate::common::utils::system_time::get_current_system_time;

use super::{
    post_score_index_item::PostScoreIndexItemV1, CreatedAt, GlobalPostId, Score,
    LATEST_POSTS_WINDOW,
};

#[derive(Default, Debug, Clone, CandidType, Deserialize, Serialize)]
pub struct PostScoreHotOrNotIndex {
    pub items_sorted_by_score: BTreeMap<Score, Vec<GlobalPostId>>,
    pub items_latest_sorted_by_score: BTreeMap<Score, Vec<GlobalPostId>>,
    pub item_presence_index: HashMap<GlobalPostId, PostScoreIndexItemV1>,
    // TODO: Add below indexes
    // pub item_nsfw_index: HashMap<IsNsfw, HashSet<GlobalPostId>>,
    // pub item_status_index: HashMap<PostStatus, HashSet<GlobalPostId>>,
    pub item_time_index: BTreeMap<CreatedAt, Vec<GlobalPostId>>,
}

impl PostScoreHotOrNotIndex {
    pub fn replace(&mut self, item: &PostScoreIndexItemV1) {
        // insert the item into the presence index accounting
        //  for already present items
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);
        let item_score = item.score;

        let _ = self.remove(item);
        self.item_presence_index
            .insert(item_presence_index_entry, item.clone());

        // insert the item into the sorted index, nsfw, time and sorted and latest sorted indexes
        let now = get_current_system_time();

        // if item created within last 48 hrs, insert into latest sorted index
        // else insert into sorted index
        if let Some(created_at) = item.created_at {
            if created_at > (now - LATEST_POSTS_WINDOW) {
                let latest_score_index_entry = self
                    .items_latest_sorted_by_score
                    .entry(item_score)
                    .or_insert_with(Vec::new);
                latest_score_index_entry.push(item_presence_index_entry);
            } else {
                let score_index_entry = self
                    .items_sorted_by_score
                    .entry(item_score)
                    .or_insert_with(Vec::new);
                score_index_entry.push(item_presence_index_entry);
            }
        } else {
            let score_index_entry = self
                .items_sorted_by_score
                .entry(item_score)
                .or_insert_with(Vec::new);
            score_index_entry.push(item_presence_index_entry);
        }

        if let Some(created_at) = item.created_at {
            let time_index_entry = self
                .item_time_index
                .entry(created_at)
                .or_insert_with(Vec::new);
            time_index_entry.push(item_presence_index_entry);
        }
    }

    pub fn remove(&mut self, item: &PostScoreIndexItemV1) -> Option<PostScoreIndexItemV1> {
        // remove the item from the presence index
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);

        let old_item = self.item_presence_index.remove(&item_presence_index_entry);

        // if the item was already present, remove it from the sorted index, latest sorted, nsfw, status, time
        if let Some(old_item) = old_item.clone() {
            let old_score = old_item.score;
            let old_created_at = old_item.created_at;

            if let Some(old_score_index_entry) = self.items_sorted_by_score.get_mut(&old_score) {
                old_score_index_entry.retain(|old_item| {
                    old_item.0 != item.publisher_canister_id || old_item.1 != item.post_id
                });
                if old_score_index_entry.is_empty() {
                    self.items_sorted_by_score.remove(&old_score);
                }
            }

            if let Some(old_latest_score_index_entry) =
                self.items_latest_sorted_by_score.get_mut(&old_score)
            {
                old_latest_score_index_entry.retain(|old_item| {
                    old_item.0 != item.publisher_canister_id || old_item.1 != item.post_id
                });
                if old_latest_score_index_entry.is_empty() {
                    self.items_latest_sorted_by_score.remove(&old_score);
                }
            }

            if let Some(old_created_at) = old_created_at {
                let old_time_index_entry = self.item_time_index.get_mut(&old_created_at).unwrap();
                old_time_index_entry.retain(|old_item| {
                    old_item.0 != item.publisher_canister_id || old_item.1 != item.post_id
                });
                if old_time_index_entry.is_empty() {
                    self.item_time_index.remove(&old_created_at);
                }
            }
        }

        old_item
    }

    pub fn iter(&'_ self) -> PostScoreHotOrNotIndexIterator<'_> {
        let latest_iter = self.items_latest_sorted_by_score.iter();
        let old_iter = self.items_sorted_by_score.iter();

        PostScoreHotOrNotIndexIterator {
            id_to_item: &self.item_presence_index,
            inner: latest_iter.rev().chain(old_iter.rev()),
            current_vec: None,
        }
    }
}

pub struct PostScoreHotOrNotIndexIterator<'a> {
    id_to_item: &'a HashMap<GlobalPostId, PostScoreIndexItemV1>,
    inner: Chain<Rev<Iter<'a, Score, Vec<GlobalPostId>>>, Rev<Iter<'a, Score, Vec<GlobalPostId>>>>,
    current_vec: Option<slice::Iter<'a, GlobalPostId>>,
}

impl<'a> Iterator for PostScoreHotOrNotIndexIterator<'a> {
    type Item = &'a PostScoreIndexItemV1;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_vec) = &mut self.current_vec {
            if let Some(item) = current_vec.next() {
                return Some(self.id_to_item.get(item).unwrap());
            }
        }

        if let Some((_, vec)) = self.inner.next() {
            self.current_vec = Some(vec.iter());
            return self.next();
        }

        None
    }
}

impl<'a> IntoIterator for &'a PostScoreHotOrNotIndex {
    type Item = &'a PostScoreIndexItemV1;
    type IntoIter = PostScoreHotOrNotIndexIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<PostScoreIndexItemV1> for PostScoreHotOrNotIndex {
    fn from_iter<T: IntoIterator<Item = PostScoreIndexItemV1>>(iter: T) -> Self {
        let mut post_score_index_items = PostScoreHotOrNotIndex::default();

        for item in iter {
            post_score_index_items.replace(&item);
        }

        post_score_index_items
    }
}

#[cfg(all(test, feature = "mockdata"))]
mod tests {
    use candid::Principal;

    use crate::common::types::top_posts::post_score_index_item::PostStatus;

    use super::*;

    #[test]
    fn test_post_score_index_v1_normal_functionality() {
        let mut post_score_index = PostScoreHotOrNotIndex::default();
        let created_at_now = SystemTime::now();
        let creted_at_earlier = created_at_now - (LATEST_POSTS_WINDOW + Duration::from_secs(1));

        let posts = vec![
            PostScoreIndexItemV1 {
                score: 1,
                post_id: 1,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            },
            PostScoreIndexItemV1 {
                score: 2,
                post_id: 2,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            },
            PostScoreIndexItemV1 {
                score: 3,
                post_id: 3,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            },
            PostScoreIndexItemV1 {
                score: 4,
                post_id: 4,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            },
            PostScoreIndexItemV1 {
                score: 5,
                post_id: 5,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            },
        ];

        for post in posts {
            post_score_index.replace(&post);
        }

        let mut post_score_index_iter = post_score_index.iter();
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 2,
                post_id: 2,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 1,
                post_id: 1,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 5,
                post_id: 5,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 4,
                post_id: 4,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 3,
                post_id: 3,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(creted_at_earlier),
            })
        );
        assert_eq!(post_score_index_iter.next(), None);
    }

    #[test]
    fn test_post_score_index_v1_replace() {
        let mut post_score_index = PostScoreHotOrNotIndex::default();
        let created_at_now = SystemTime::now();

        let posts = vec![
            PostScoreIndexItemV1 {
                score: 1,
                post_id: 1,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::Uploaded,
                created_at: Some(created_at_now),
            },
            PostScoreIndexItemV1 {
                score: 2,
                post_id: 2,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: true,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            },
            PostScoreIndexItemV1 {
                score: 3,
                post_id: 1,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: true,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            },
        ];

        for post in posts {
            post_score_index.replace(&post);
        }

        let mut post_score_index_iter = post_score_index.iter();
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 3,
                post_id: 1,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: true,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 2,
                post_id: 2,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: true,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_now),
            })
        );
        assert_eq!(post_score_index_iter.next(), None);
    }
}
