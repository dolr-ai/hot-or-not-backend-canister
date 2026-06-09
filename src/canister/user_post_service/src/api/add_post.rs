use crate::CANISTER_DATA;
use ic_cdk_macros::update;
use shared_utils::canister_specific::user_post_service::types::args::{
    PostDetailsFromFrontend, PostDetailsFromFrontendV1,
};
use shared_utils::canister_specific::user_post_service::types::error::UserPostServiceError;
use shared_utils::common::utils::permissions::is_caller_global_admin;

#[update(guard = "is_caller_global_admin")]
fn add_post(post: PostDetailsFromFrontend) -> Result<(), UserPostServiceError> {
    CANISTER_DATA.with_borrow_mut(|canister_data| canister_data.add_post_to_memory(post))
}

#[update(guard = "is_caller_global_admin")]
fn add_post_v1(post: PostDetailsFromFrontendV1) -> Result<(), UserPostServiceError> {
    CANISTER_DATA.with_borrow_mut(|canister_data| canister_data.add_post_to_memory(post))
}
