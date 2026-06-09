use std::{borrow::Cow, collections::BTreeMap, time::SystemTime};

use candid::{CandidType, Decode, Deserialize, Encode, Principal};
use ic_cdk::api::{call::CallResult, management_canister::provisional::CanisterId};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;

use crate::common::types::app_primitive_type::PostId;

use super::{arg::PlaceBetArg, error::BetOnCurrentlyViewingPostError, post::FeedScore};

pub trait HotOrNotGame {
    fn validate_incoming_bet(
        &self,
        bet_maker_principal: Principal,
        place_bet_arg: &PlaceBetArg,
    ) -> Result<(), BetOnCurrentlyViewingPostError>;
    fn prepare_for_bet(
        &mut self,
        bet_maker_principal: Principal,
        place_bet_arg: &PlaceBetArg,
        current_timestamp: SystemTime,
    ) -> Result<(), BetOnCurrentlyViewingPostError>;

    fn process_place_bet_status(
        &mut self,
        bet_response: CallResult<(Result<BettingStatus, BetOnCurrentlyViewingPostError>,)>,
        place_bet_arg: &PlaceBetArg,
        current_timestamp: SystemTime,
    ) -> Result<BettingStatus, BetOnCurrentlyViewingPostError>;

    fn receive_bet_from_bet_maker_canister(
        &mut self,
        bet_maker_principal_id: Principal,
        bet_maker_canister_id: Principal,
        place_bet_arg: &PlaceBetArg,
        current_timestamp: SystemTime,
    ) -> Result<BettingStatus, BetOnCurrentlyViewingPostError>;

    fn tabulate_hot_or_not_outcome_for_post_slot(
        &mut self,
        post_id: u64,
        slot_id: u8,
        current_timestamp: SystemTime,
    );

    fn receive_earnings_for_the_bet(
        &mut self,
        post_id: u64,
        post_creator_canister_id: Principal,
        outcome: BetOutcomeForBetMaker,
        current_timestamp: SystemTime,
    );
}

#[derive(CandidType, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum BettingStatus {
    BettingOpen {
        started_at: SystemTime,
        number_of_participants: u8,
        ongoing_slot: u8,
        ongoing_room: u64,
        has_this_user_participated_in_this_post: Option<bool>,
    },
    BettingClosed,
}

pub const MAXIMUM_NUMBER_OF_SLOTS: u8 = 48;
pub const DURATION_OF_EACH_SLOT_IN_SECONDS: u64 = 60 * 60;
pub const TOTAL_DURATION_OF_ALL_SLOTS_IN_SECONDS: u64 =
    MAXIMUM_NUMBER_OF_SLOTS as u64 * DURATION_OF_EACH_SLOT_IN_SECONDS;

#[derive(CandidType)]
pub enum UserStatusForSpecificHotOrNotPost {
    NotParticipatedYet,
    AwaitingResult(BetDetail),
    ResultAnnounced(BetResult),
}

#[derive(CandidType)]
pub enum BetResult {
    Won(u64),
    Lost,
    Draw,
}

#[derive(CandidType)]
pub struct BetDetail {
    amount: u64,
    bet_direction: BetDirection,
    bet_made_at: SystemTime,
}

#[derive(CandidType, Clone, Deserialize, Serialize, Debug, PartialEq, Eq, Copy)]
pub enum BetDirection {
    Hot,
    Not,
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Hash)]
pub struct HotOrNotBetId {
    pub canister_id: Principal,
    pub post_id: u64,
}

#[derive(CandidType, Clone, Deserialize, Debug, Serialize, Default)]
pub struct HotOrNotDetails {
    pub hot_or_not_feed_score: FeedScore,
    pub aggregate_stats: AggregateStats,
    pub slot_history: BTreeMap<SlotId, SlotDetails>,
}

#[derive(CandidType, Clone, Deserialize, Debug, Serialize, Default)]
pub struct AggregateStats {
    pub total_number_of_hot_bets: u64,
    pub total_number_of_not_bets: u64,
    pub total_amount_bet: u64,
}

pub type SlotId = u8;

#[derive(CandidType, Clone, Deserialize, Default, Debug, Serialize)]
pub struct SlotDetails {
    pub room_details: BTreeMap<RoomId, RoomDetails>,
}

pub type RoomId = u64;

#[derive(CandidType, Clone, Deserialize, Default, Debug, Serialize)]
pub struct RoomDetails {
    pub bets_made: BTreeMap<BetMaker, BetDetails>,
    pub bet_outcome: RoomBetPossibleOutcomes,
    pub room_bets_total_pot: u64,
    pub total_hot_bets: u64,
    pub total_not_bets: u64,
}

#[derive(CandidType, Clone, Deserialize, Debug, Serialize)]
pub struct SlotDetailsV1 {
    pub active_room_id: RoomId,
}

impl Default for SlotDetailsV1 {
    fn default() -> Self {
        SlotDetailsV1 { active_room_id: 1 }
    }
}

const MAX_SLOT_DETAILS_VALUE_SIZE: u32 = 100 as u32;

impl Storable for SlotDetailsV1 {
    fn to_bytes(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_SLOT_DETAILS_VALUE_SIZE,
        is_fixed_size: false,
    };
}

#[derive(CandidType, Clone, Deserialize, Default, Debug, Serialize)]
pub struct RoomDetailsV1 {
    pub bet_outcome: RoomBetPossibleOutcomes,
    pub room_bets_total_pot: u64,
    pub total_hot_bets: u64,
    pub total_not_bets: u64,
}
const MAX_ROOM_DETAILS_VALUE_SIZE: u32 = 100 as u32;

impl Storable for RoomDetailsV1 {
    fn to_bytes(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: MAX_ROOM_DETAILS_VALUE_SIZE,
        is_fixed_size: false,
    };
}

pub type BetMaker = Principal;

#[derive(CandidType, Clone, Deserialize, Debug, Serialize, PartialEq, Eq)]
pub enum BetMakerInformedStatus {
    InformedSuccessfully,
    Failed(String),
}

#[derive(CandidType, Clone, Deserialize, Debug, Serialize)]
pub struct BetDetails {
    pub amount: u64,
    pub bet_direction: BetDirection,
    pub payout: BetPayout,
    pub bet_maker_canister_id: CanisterId,
    #[serde(default)]
    pub bet_maker_informed_status: Option<BetMakerInformedStatus>,
}
const MAX_BET_DETAILS_VALUE_SIZE: u32 = 200 as u32;

impl Storable for BetDetails {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(
    CandidType, Clone, Deserialize, Debug, Serialize, Ord, PartialOrd, Eq, PartialEq, Hash,
)]
pub struct StablePrincipal(pub Principal);

impl Default for StablePrincipal {
    fn default() -> Self {
        Self(Principal::anonymous())
    }
}

impl Storable for StablePrincipal {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 60,
        is_fixed_size: false,
    };
}

pub type BetMakerPrincipal = StablePrincipal;

#[derive(
    CandidType,
    Clone,
    Deserialize,
    Debug,
    Serialize,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Default,
    Copy,
    Hash,
)]
pub struct GlobalRoomId(pub PostId, pub SlotId, pub RoomId);

impl Storable for GlobalRoomId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 50,
        is_fixed_size: false,
    };
}

#[derive(
    CandidType, Clone, Deserialize, Debug, Serialize, Ord, PartialOrd, Eq, PartialEq, Default,
)]
pub struct GlobalBetId(pub GlobalRoomId, pub BetMakerPrincipal);

impl Storable for GlobalBetId {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        Cow::Owned(Encode!(self).unwrap())
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        Decode!(bytes.as_ref(), Self).unwrap()
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: 100,
        is_fixed_size: false,
    };
}

#[derive(Clone, Deserialize, Debug, CandidType, Serialize, Default, PartialEq)]
pub enum BetPayout {
    #[default]
    NotCalculatedYet,
    Calculated(u64),
}

#[derive(CandidType, Clone, Default, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum RoomBetPossibleOutcomes {
    #[default]
    BetOngoing,
    HotWon,
    NotWon,
    Draw,
}

#[derive(Deserialize, Serialize, Clone, CandidType, Debug, PartialEq, Eq)]
pub struct PlacedBetDetail {
    pub canister_id: CanisterId,
    pub post_id: PostId,
    pub slot_id: SlotId,
    pub room_id: RoomId,
    pub amount_bet: u64,
    pub bet_direction: BetDirection,
    pub bet_placed_at: SystemTime,
    pub outcome_received: BetOutcomeForBetMaker,
}

#[derive(Deserialize, Serialize, Default, CandidType, PartialEq, Eq, Clone, Debug)]
pub enum BetOutcomeForBetMaker {
    #[default]
    AwaitingResult,
    Won(u64),
    Lost,
    Draw(u64),
}
