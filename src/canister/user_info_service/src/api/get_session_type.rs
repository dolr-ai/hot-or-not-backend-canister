use candid::Principal;
use ic_cdk_macros::query;
use shared_utils::canister_specific::individual_user_template::types::session::SessionType;

use crate::CANISTER_DATA;

#[query]
pub fn get_user_session_type(user: Principal) -> Result<SessionType, String> {
    CANISTER_DATA.with_borrow_mut(|canister_data| canister_data.get_session_type_for_user(user))
}
