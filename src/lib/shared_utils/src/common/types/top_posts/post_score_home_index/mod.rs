use candid::{CandidType, Deserialize};
use serde::Serialize;
use std::{
    collections::{btree_map::Iter, BTreeMap, HashMap},
    iter::Rev,
    slice, vec,
};

use super::{post_score_index_item::PostScoreIndexItemV1, GlobalPostId, Score};

#[derive(Default, Debug, Clone, CandidType, Deserialize, Serialize)]
pub struct PostScoreHomeIndex {
    pub items_sorted_by_score: BTreeMap<Score, Vec<GlobalPostId>>,
    pub item_presence_index: HashMap<GlobalPostId, PostScoreIndexItemV1>,
    // TODO: Add below indexes
    // pub item_nsfw_index: HashMap<IsNsfw, HashSet<GlobalPostId>>,
    // pub item_status_index: HashMap<PostStatus, HashSet<GlobalPostId>>,
}

impl PostScoreHomeIndex {
    pub fn replace(&mut self, item: &PostScoreIndexItemV1) {
        // insert the item into the presence index accounting
        //  for already present items
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);
        let item_score = item.score;

        let _ = self.remove(item);
        self.item_presence_index
            .insert(item_presence_index_entry, item.clone());

        // insert the item into the sorted index, nsfw, time and sorted and latest sorted indexes

        let score_index_entry = self
            .items_sorted_by_score
            .entry(item_score)
            .or_insert_with(Vec::new);
        score_index_entry.push(item_presence_index_entry);
    }

    pub fn remove(&mut self, item: &PostScoreIndexItemV1) -> Option<PostScoreIndexItemV1> {
        // remove the item from the presence index
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);

        let old_item = self.item_presence_index.remove(&item_presence_index_entry);

        // if the item was already present, remove it from the sorted index, latest sorted, nsfw, status, time
        if let Some(old_item) = old_item.clone() {
            let old_score = old_item.score;

            if let Some(old_score_index_entry) = self.items_sorted_by_score.get_mut(&old_score) {
                old_score_index_entry.retain(|old_item| {
                    old_item.0 != item.publisher_canister_id || old_item.1 != item.post_id
                });
            }
        }

        old_item
    }

    pub fn iter(&'_ self) -> PostScoreHomeIndexIterator<'_> {
        PostScoreHomeIndexIterator {
            id_to_item: &self.item_presence_index,
            inner: self.items_sorted_by_score.iter().rev(),
            current_vec: None,
        }
    }
}

pub struct PostScoreHomeIndexIterator<'a> {
    id_to_item: &'a HashMap<GlobalPostId, PostScoreIndexItemV1>,
    inner: Rev<Iter<'a, Score, Vec<GlobalPostId>>>,
    current_vec: Option<slice::Iter<'a, GlobalPostId>>,
}

impl<'a> Iterator for PostScoreHomeIndexIterator<'a> {
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

impl<'a> IntoIterator for &'a PostScoreHomeIndex {
    type Item = &'a PostScoreIndexItemV1;
    type IntoIter = PostScoreHomeIndexIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<PostScoreIndexItemV1> for PostScoreHomeIndex {
    fn from_iter<T: IntoIterator<Item = PostScoreIndexItemV1>>(iter: T) -> Self {
        let mut post_score_index_items = PostScoreHomeIndex::default();

        for item in iter {
            post_score_index_items.replace(&item);
        }

        post_score_index_items
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use candid::Principal;

    use crate::common::types::top_posts::post_score_index_item::PostStatus;

    use super::*;

    #[test]
    fn test_post_score_index_v1_normal_functionality() {
        let mut post_score_index = PostScoreHomeIndex::default();
        let created_at_now = SystemTime::now();
        let created_at_earlier = created_at_now - Duration::from_secs(60 * 60 * 48 + 1);

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
                created_at: Some(created_at_earlier),
            },
            PostScoreIndexItemV1 {
                score: 4,
                post_id: 4,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_earlier),
            },
            PostScoreIndexItemV1 {
                score: 5,
                post_id: 5,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_earlier),
            },
        ];

        for post in posts {
            post_score_index.replace(&post);
        }

        let mut post_score_index_iter = post_score_index.iter();
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItemV1 {
                score: 5,
                post_id: 5,
                publisher_canister_id: Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap(),
                is_nsfw: false,
                status: PostStatus::ReadyToView,
                created_at: Some(created_at_earlier),
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
                created_at: Some(created_at_earlier),
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
                created_at: Some(created_at_earlier),
            })
        );
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
        assert_eq!(post_score_index_iter.next(), None);
    }

    #[test]
    fn test_post_score_index_v1_replace() {
        let mut post_score_index = PostScoreHomeIndex::default();
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
