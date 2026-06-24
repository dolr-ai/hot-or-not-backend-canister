pub mod memory;

use candid::Principal;
use ic_stable_structures::{memory_manager::VirtualMemory, DefaultMemoryImpl, StableBTreeMap};
use serde::{Deserialize, Serialize};
use shared_utils::{
    canister_specific::{
        individual_user_template::types::error::GetPostsOfUserProfileError,
        user_post_service::types::{
            args::{FetchPostsResult, PostDetailsForFrontend},
            error::UserPostServiceError,
            storage::{Post, PostIdStringList},
        },
    },
    common::types::top_posts::post_score_index_item::PostStatus,
    pagination::{self, PaginationError},
};

type Memory = VirtualMemory<DefaultMemoryImpl>;
type PostId = String;

#[derive(Serialize, Deserialize)]
pub struct CanisterData {
    /// Map of post_id -> Post details
    #[serde(skip, default = "_init_posts")]
    posts: StableBTreeMap<PostId, Post, Memory>,

    /// Index for fast lookup of posts by creator - Map of creator_principal -> Vec<post_id>
    #[serde(skip, default = "_init_posts_by_creator")]
    posts_by_creator: StableBTreeMap<Principal, PostIdStringList, Memory>,

    /// Semantic version of this canister's code/data schema
    pub version: String,
}

impl CanisterData {
    pub fn fetch_posts(
        &self,
        last_uuid_processed: Option<String>,
        limit: usize,
    ) -> FetchPostsResult {
        let mut posts = vec![];

        let mut last_processed_uuid = last_uuid_processed.clone();

        let range = match last_uuid_processed {
            Some(u) => self
                .posts
                .range((std::ops::Bound::Excluded(u), std::ops::Bound::Unbounded)),
            None => self.posts.range(..),
        };

        for (processed, (post_id, post)) in range.into_iter().enumerate() {
            if processed >= limit {
                break;
            }

            {
                posts.push(post);
            }

            last_processed_uuid = Some(post_id.clone());
        }

        FetchPostsResult {
            posts,
            last_post_id_fetched: last_processed_uuid,
        }
    }

    pub fn initialize_posts_by_creator_index(
        &mut self,
        last_uuid_processed: Option<String>,
        limit: usize,
    ) -> Option<String> {
        let mut processed = 0;

        let mut last_processed_uuid = last_uuid_processed.clone();

        let range = match last_uuid_processed {
            Some(u) => self
                .posts
                .range((std::ops::Bound::Excluded(u), std::ops::Bound::Unbounded)),
            None => self.posts.range(..),
        };

        for (post_id, post) in range {
            if processed >= limit {
                break;
            }

            if post.status != PostStatus::Deleted
                && post.status != PostStatus::BannedDueToUserReporting
            {
                let creator = post.creator_principal;
                let mut post_ids = self.posts_by_creator.get(&creator).unwrap_or_default();

                post_ids.push(post_id.clone());
                self.posts_by_creator.insert(creator, post_ids);
            }

            last_processed_uuid = Some(post_id.clone());
            processed += 1;
        }

        last_processed_uuid
    }

    pub fn add_post_to_memory(
        &mut self,
        post_from_frontend: impl Into<Post>,
    ) -> Result<(), UserPostServiceError> {
        let post: Post = post_from_frontend.into();

        if self.posts.contains_key(&post.id) {
            return Err(UserPostServiceError::DuplicatePostId);
        }

        self.posts.insert(post.id.clone(), post.clone());

        self.add_post_to_creator_index(&post);

        Ok(())
    }

    fn add_post_to_creator_index(&mut self, post: &Post) {
        if post.status == PostStatus::Deleted || post.status == PostStatus::BannedDueToUserReporting
        {
            return;
        }

        let creator = post.creator_principal;
        let mut post_ids = self.posts_by_creator.get(&creator).unwrap_or_default();

        post_ids.add_unique(post.id.clone()); // Uses helper method that prevents duplicates and maintains order
        self.posts_by_creator.insert(creator, post_ids);
    }

    fn remove_post_from_creator_index(&mut self, post_id: &str, creator: Principal) {
        if let Some(mut post_ids) = self.posts_by_creator.get(&creator) {
            post_ids.remove(post_id); // Uses helper method
            if post_ids.is_empty() {
                self.posts_by_creator.remove(&creator);
            } else {
                self.posts_by_creator.insert(creator, post_ids);
            }
        }
    }

    pub fn add_post(&mut self, post: Post) -> Option<Post> {
        let post_id = post.id.clone();
        let result = self.posts.insert(post_id.clone(), post.clone());

        self.add_post_to_creator_index(&post);

        result
    }

    pub fn get_post_details_for_user(
        &self,
        post_id: &PostId,
        user: Principal,
    ) -> Result<PostDetailsForFrontend, UserPostServiceError> {
        let post = self.get_post(post_id)?;
        Ok(post.get_post_details_for_frontend_for_user(user))
    }

    pub fn get_posts_of_this_user_profile_with_pagination_cursor(
        &self,
        creator: Principal,
        mut limit: usize,
        offset: usize,
    ) -> Vec<Post> {
        limit = limit.min(100);

        let post_ids = match self.posts_by_creator.get(&creator) {
            Some(mut post_ids) => {
                post_ids.sort_by_creation_time(|post| {
                    self.posts.get(&post.to_string()).map(|p| p.created_at)
                });

                post_ids.0.clone()
            }
            None => return Vec::new(),
        };

        // Get posts from the index (already sorted by creation time)
        let posts: Vec<Post> = post_ids
            .iter()
            .skip(offset)
            .take(limit)
            .filter_map(|post_id| self.posts.get(post_id))
            .filter(|post| {
                post.status != PostStatus::Deleted
                    && post.status != PostStatus::BannedDueToUserReporting
                    && post.status != PostStatus::Draft
            })
            .collect();

        posts
    }

    /// OPTIMIZED: Gets posts for a user profile using the posts_by_creator index
    /// This function now uses O(1) lookup instead of O(n) iteration over all posts
    pub fn get_posts_of_this_user_profile_with_pagination(
        &self,
        creator: Principal,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Post>, GetPostsOfUserProfileError> {
        // Use the posts_by_creator index for fast lookup - O(1) instead of O(n)
        let post_ids = match self.posts_by_creator.get(&creator) {
            Some(mut post_ids) => {
                post_ids.sort_by_creation_time(|post| {
                    self.posts.get(&post.to_string()).map(|p| p.created_at)
                });

                post_ids.0.clone()
            }
            None => return Ok(Vec::new()), // No posts for this creator
        };

        // Get total count of valid posts for pagination calculation
        let valid_post_count = post_ids.len() as u64;

        let (from_inclusive_index, limit) =
            pagination::get_pagination_bounds_cursor(offset as u64, limit as u64, valid_post_count)
                .map_err(|e| match e {
                    PaginationError::InvalidBoundsPassed => {
                        GetPostsOfUserProfileError::InvalidBoundsPassed
                    }
                    PaginationError::ReachedEndOfItemsList => {
                        GetPostsOfUserProfileError::ReachedEndOfItemsList
                    }
                    PaginationError::ExceededMaxNumberOfItemsAllowedInOneRequest => {
                        GetPostsOfUserProfileError::ExceededMaxNumberOfItemsAllowedInOneRequest
                    }
                })?;

        let posts: Vec<Post> = post_ids
            .iter()
            .skip(from_inclusive_index as usize)
            .take(limit as usize)
            .filter_map(|post_id| self.posts.get(post_id))
            .filter(|post| {
                post.status != PostStatus::Deleted
                    && post.status != PostStatus::BannedDueToUserReporting
                    && post.status != PostStatus::Draft
            })
            .collect();

        Ok(posts)
    }

    pub fn get_draft_posts_of_this_user_profile_with_pagination(
        &self,
        creator: Principal,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<Post>, GetPostsOfUserProfileError> {
        // Use the posts_by_creator index for fast lookup - O(1) instead of O(n)
        let post_ids = match self.posts_by_creator.get(&creator) {
            Some(mut post_ids) => {
                post_ids.sort_by_creation_time(|post| {
                    self.posts.get(&post.to_string()).map(|p| p.created_at)
                });

                post_ids.0.clone()
            }
            None => return Ok(Vec::new()), // No posts for this creator
        };

        // Get total count of valid posts for pagination calculation
        let valid_post_count = post_ids.len() as u64;

        let (from_inclusive_index, limit) =
            pagination::get_pagination_bounds_cursor(offset as u64, limit as u64, valid_post_count)
                .map_err(|e| match e {
                    PaginationError::InvalidBoundsPassed => {
                        GetPostsOfUserProfileError::InvalidBoundsPassed
                    }
                    PaginationError::ReachedEndOfItemsList => {
                        GetPostsOfUserProfileError::ReachedEndOfItemsList
                    }
                    PaginationError::ExceededMaxNumberOfItemsAllowedInOneRequest => {
                        GetPostsOfUserProfileError::ExceededMaxNumberOfItemsAllowedInOneRequest
                    }
                })?;

        let posts: Vec<Post> = post_ids
            .iter()
            .skip(from_inclusive_index as usize)
            .take(limit as usize)
            .filter_map(|post_id| self.posts.get(post_id))
            .filter(|post| post.status == PostStatus::Draft)
            .collect();

        Ok(posts)
    }

    pub fn get_post(&self, post_id: &PostId) -> Result<Post, UserPostServiceError> {
        match self.posts.get(post_id) {
            Some(post) => match post.status {
                PostStatus::Deleted => Err(UserPostServiceError::PostNotFound),
                _ => Ok(post),
            },
            None => Err(UserPostServiceError::PostNotFound),
        }
    }

    pub fn delete_post(
        &mut self,
        post_id: &PostId,
        caller_principal: Principal,
    ) -> Result<(), UserPostServiceError> {
        let mut post = self
            .posts
            .get(&post_id)
            .ok_or(UserPostServiceError::PostNotFound)?;

        if post.creator_principal != caller_principal {
            return Err(UserPostServiceError::Unauthorized);
        }

        match post.status {
            PostStatus::Deleted => Err(UserPostServiceError::PostNotFound),
            _ => {
                let creator = post.creator_principal;
                post.status = PostStatus::Deleted;

                // Update the main posts map
                self.posts.insert(post_id.clone(), post);

                // Remove from posts_by_creator index
                self.remove_post_from_creator_index(post_id, creator);

                Ok(())
            }
        }
    }
}

impl Default for CanisterData {
    fn default() -> Self {
        Self {
            posts: _init_posts(),
            posts_by_creator: _init_posts_by_creator(),
            version: String::from("v1.0.0"),
        }
    }
}

fn _init_posts() -> StableBTreeMap<PostId, Post, Memory> {
    StableBTreeMap::init(memory::get_posts_memory())
}

fn _init_posts_by_creator() -> StableBTreeMap<Principal, PostIdStringList, Memory> {
    StableBTreeMap::init(memory::get_posts_by_creator_memory())
}
