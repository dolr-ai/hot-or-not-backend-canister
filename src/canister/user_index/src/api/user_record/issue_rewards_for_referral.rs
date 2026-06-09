use candid::Principal;
use ic_cdk::{call, notify};
use ic_cdk_macros::update;
use shared_utils::{
    canister_specific::individual_user_template::types::session::SessionType,
    common::utils::permissions::is_caller_global_admin,
};

#[update(guard = "is_caller_global_admin")]
pub async fn issue_rewards_for_referral(
    user_canister_id: Principal,
    referrer_principal: Principal,
    referee_principal: Principal,
) -> Result<String, String> {
    let (canister_session_type_result,): (Result<SessionType, String>,) =
        call(user_canister_id, "get_session_type", ())
            .await
            .map_err(|e| e.1)?;
    let canister_session_type = canister_session_type_result?;

    if canister_session_type == SessionType::AnonymousSession {
        return Err("user not signed up".into());
    }

    notify(
        user_canister_id,
        "get_rewarded_for_referral",
        (referrer_principal, referee_principal),
    )
    .map_err(|_| String::from("failed to reward the canister"))?;

    Ok("Success".into())
}
