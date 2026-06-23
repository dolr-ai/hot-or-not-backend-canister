use candid::{CandidType, Deserialize};
use ic_cdk::api::call::RejectionCode;
use icrc_ledger_types::icrc1::transfer::TransferError;

#[derive(CandidType, Deserialize, PartialEq, Eq, Debug)]
pub enum GetPostsOfUserProfileError {
    InvalidBoundsPassed,
    ReachedEndOfItemsList,
    ExceededMaxNumberOfItemsAllowedInOneRequest,
}

#[derive(CandidType, Deserialize, PartialEq, Eq, Debug)]
pub enum GetFollowerOrFollowingError {
    InvalidBoundsPassed,
    ReachedEndOfItemsList,
    ExceededMaxNumberOfItemsAllowedInOneRequest,
}

#[derive(CandidType, Deserialize, PartialEq, Eq, Debug)]
pub enum GetFollowerOrFollowingPageError {
    Unauthenticated,
    Unauthorized,
}

#[derive(CandidType, PartialEq, Eq, Debug, Deserialize)]
pub enum BetOnCurrentlyViewingPostError {
    BettingClosed,
    InsufficientBalance,
    Unauthorized,
    UserAlreadyParticipatedInThisPost,
    UserNotLoggedIn,
    UserPrincipalNotSet,
    PostCreatorCanisterCallFailed,
}

#[derive(CandidType, Deserialize, PartialEq, Eq, Debug)]
pub enum FollowAnotherUserProfileError {
    Unauthenticated,
    Unauthorized,
    UsersICanFollowListIsFull,
    UserITriedToFollowCrossCanisterCallFailed,
    UserITriedToFollowHasTheirFollowersListFull,
}

#[derive(CandidType, Deserialize, PartialEq, Eq, Debug)]
pub enum AirdropError {
    CanisterPrincipalDoNotMatch,
    AlreadyClaimedAirdrop,
    RequestedAmountTooLow,
    NoBalance,
    InvalidRoot,
    CallError(RejectionCode, String),
    Transfer(TransferError),
}

impl From<(RejectionCode, String)> for AirdropError {
    fn from(value: (RejectionCode, String)) -> Self {
        AirdropError::CallError(value.0, value.1)
    }
}