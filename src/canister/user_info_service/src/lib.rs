use candid::Principal;
use std::cell::RefCell;

use ic_cdk::export_candid;
use shared_utils::canister_specific::individual_user_template::types::profile::{
    UserProfileDetailsForFrontendV3, UserProfileDetailsForFrontendV4,
    UserProfileDetailsForFrontendV5, UserProfileDetailsForFrontendV6,
    UserProfileDetailsForFrontendV7,
};
use shared_utils::canister_specific::individual_user_template::types::session::SessionType;
use shared_utils::canister_specific::user_info_service::args::UserInfoServiceInitArgs;
use shared_utils::canister_specific::user_info_service::types::SubscriptionPlan;

use crate::data_model::CanisterData;
use shared_utils::canister_specific::user_info_service::types::{
    FollowersResponse, FollowingResponse, NSFWInfo, ProfileUpdateDetails, ProfileUpdateDetailsV2,
};

mod api;
pub mod data_model;

thread_local! {
    static CANISTER_DATA: RefCell<CanisterData> = RefCell::default();
}

export_candid!();
