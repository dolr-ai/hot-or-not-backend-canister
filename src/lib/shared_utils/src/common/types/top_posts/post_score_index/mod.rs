use candid::{CandidType, Deserialize};
use serde::Serialize;
use std::{
    collections::{
        btree_map::{self},
        BTreeMap, HashMap,
    },
    iter::Rev,
    slice, vec,
};

use crate::common::types::top_posts::post_score_index_item::PostScoreIndexItem;

use super::{PostId, PublisherCanisterId, Score};

#[derive(Default, Debug, Clone, CandidType, Deserialize, Serialize)]
pub struct PostScoreIndex {
    pub items_sorted_by_score: BTreeMap<Score, Vec<PostScoreIndexItem>>,
    pub item_presence_index: HashMap<(PublisherCanisterId, PostId), Score>,
}

impl PostScoreIndex {
    pub fn replace(&mut self, item: &PostScoreIndexItem) {
        // insert the item into the presence index accounting
        //  for already present items
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);
        let item_score = item.score;

        let old_score = self
            .item_presence_index
            .insert(item_presence_index_entry, item_score);

        // if the item was already present, remove it from the sorted index
        if let Some(old_score) = old_score {
            let old_score_index_entry = self.items_sorted_by_score.get_mut(&old_score).unwrap();
            old_score_index_entry.retain(|old_item| {
                old_item.publisher_canister_id != item.publisher_canister_id
                    || old_item.post_id != item.post_id
            });
        }

        // insert the item into the sorted index
        self.items_sorted_by_score
            .entry(item_score)
            .or_insert(vec![])
            .push(item.clone());
    }

    pub fn remove(&mut self, item: &PostScoreIndexItem) -> Option<PostScoreIndexItem> {
        // remove the item from the presence index
        let item_presence_index_entry = (item.publisher_canister_id, item.post_id);
        let item_score = self.item_presence_index.remove(&item_presence_index_entry);

        // remove the item from the sorted index
        if let Some(item_score) = item_score {
            let old_score_index_entry = self.items_sorted_by_score.get_mut(&item_score);
            if let Some(old_score_index_entry) = old_score_index_entry {
                old_score_index_entry.retain(|old_item| {
                    old_item.publisher_canister_id != item.publisher_canister_id
                        || old_item.post_id != item.post_id
                });
                Some(PostScoreIndexItem {
                    score: item_score,
                    post_id: item.post_id,
                    publisher_canister_id: item.publisher_canister_id,
                })
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn iter(&'_ self) -> PostScoreIndexIterator<'_> {
        PostScoreIndexIterator {
            inner: self.items_sorted_by_score.iter().rev(),
            current_vec: None,
        }
    }
}

pub struct PostScoreIndexIterator<'a> {
    inner: Rev<btree_map::Iter<'a, Score, Vec<PostScoreIndexItem>>>,
    current_vec: Option<slice::Iter<'a, PostScoreIndexItem>>,
}

impl<'a> Iterator for PostScoreIndexIterator<'a> {
    type Item = &'a PostScoreIndexItem;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(current_vec) = &mut self.current_vec {
            if let Some(item) = current_vec.next() {
                return Some(item);
            }
        }

        if let Some((_, vec)) = self.inner.next() {
            self.current_vec = Some(vec.iter());
            return self.next();
        }

        None
    }
}

impl FromIterator<PostScoreIndexItem> for PostScoreIndex {
    fn from_iter<T: IntoIterator<Item = PostScoreIndexItem>>(iter: T) -> Self {
        let mut items_sorted_by_score = BTreeMap::new();
        let mut item_presence_index = HashMap::new();

        for item in iter {
            let score = item.score;
            let publisher_canister_id = item.publisher_canister_id;
            let post_id = item.post_id;

            items_sorted_by_score
                .entry(score)
                .or_insert_with(Vec::new)
                .push(item.clone());

            item_presence_index.insert((publisher_canister_id, post_id), score);
        }

        Self {
            items_sorted_by_score,
            item_presence_index,
        }
    }
}

impl<'a> IntoIterator for &'a PostScoreIndex {
    type Item = &'a PostScoreIndexItem;
    type IntoIter = PostScoreIndexIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod test {
    use candid::Principal;

    use super::*;

    #[test]
    fn if_iterating_over_post_score_index_then_iterates_over_all_post_score_index_items_without_consuming_them(
    ) {
        let mut post_score_index = PostScoreIndex {
            items_sorted_by_score: BTreeMap::new(),
            item_presence_index: HashMap::new(),
        };

        let publisher_canister_id_1 = Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap();

        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 1,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 2,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 3,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 4,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 5,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 6,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 7,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 8,
            publisher_canister_id: publisher_canister_id_1,
        });

        let mut post_score_index_iter = post_score_index.iter();

        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 7,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 8,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 5,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 6,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 2,
                post_id: 3,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 2,
                post_id: 4,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 1,
                post_id: 1,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 1,
                post_id: 2,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(post_score_index_iter.next(), None);

        assert_eq!(post_score_index.iter().count(), 8);
        assert_eq!(post_score_index.items_sorted_by_score.len(), 4);
    }

    #[test]
    fn if_taking_top_items_then_high_score_items_taken() {
        let mut post_score_index = PostScoreIndex {
            items_sorted_by_score: BTreeMap::new(),
            item_presence_index: HashMap::new(),
        };

        let publisher_canister_id_1 = Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap();

        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 1,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 2,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 3,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 4,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 5,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 6,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 7,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 8,
            publisher_canister_id: publisher_canister_id_1,
        });

        let mut top_items = post_score_index.iter().take(4).cloned();

        assert_eq!(
            top_items.next(),
            Some(PostScoreIndexItem {
                score: 4,
                post_id: 7,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items.next(),
            Some(PostScoreIndexItem {
                score: 4,
                post_id: 8,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items.next(),
            Some(PostScoreIndexItem {
                score: 3,
                post_id: 5,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items.next(),
            Some(PostScoreIndexItem {
                score: 3,
                post_id: 6,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(top_items.next(), None);
        assert_eq!(post_score_index.iter().count(), 8);
        assert_eq!(post_score_index.items_sorted_by_score.len(), 4);
    }

    #[test]
    fn if_taking_from_collection_via_into_iter_cloned_then_original_collection_unchanged() {
        let mut post_score_index = PostScoreIndex {
            items_sorted_by_score: BTreeMap::new(),
            item_presence_index: HashMap::new(),
        };

        let publisher_canister_id_1 = Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap();

        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 1,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 2,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 3,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 4,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 5,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 6,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 7,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 8,
            publisher_canister_id: publisher_canister_id_1,
        });

        let top_items: PostScoreIndex = post_score_index.into_iter().take(4).cloned().collect();
        let mut top_items_iter = top_items.iter();

        assert_eq!(
            top_items_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 7,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 8,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 5,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            top_items_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 6,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(top_items_iter.next(), None);

        let mut post_score_index_iter = post_score_index.iter();
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 7,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 8,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 5,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 3,
                post_id: 6,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 2,
                post_id: 3,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 2,
                post_id: 4,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 1,
                post_id: 1,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 1,
                post_id: 2,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(post_score_index_iter.next(), None);

        assert_eq!(post_score_index.iter().count(), 8);
        assert_eq!(post_score_index.items_sorted_by_score.len(), 4);
    }

    #[test]
    fn if_inserting_duplicate_items_with_different_scores_then_only_unique_items_with_last_entered_score_saved(
    ) {
        let mut post_score_index = PostScoreIndex {
            items_sorted_by_score: BTreeMap::new(),
            item_presence_index: HashMap::new(),
        };

        let publisher_canister_id_1 = Principal::from_text("w4nuc-waaaa-aaaao-aal2a-cai").unwrap();

        post_score_index.replace(&PostScoreIndexItem {
            score: 1,
            post_id: 1,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 2,
            post_id: 1,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 3,
            post_id: 2,
            publisher_canister_id: publisher_canister_id_1,
        });
        post_score_index.replace(&PostScoreIndexItem {
            score: 4,
            post_id: 2,
            publisher_canister_id: publisher_canister_id_1,
        });

        let mut post_score_index_iter = post_score_index.iter();
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 4,
                post_id: 2,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(
            post_score_index_iter.next(),
            Some(&PostScoreIndexItem {
                score: 2,
                post_id: 1,
                publisher_canister_id: publisher_canister_id_1,
            })
        );
        assert_eq!(post_score_index_iter.next(), None);
    }
}
