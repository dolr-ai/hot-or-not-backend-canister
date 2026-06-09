use crate::service::GetVersion;
use candid::{CandidType, Deserialize, Principal};
use ic_stable_structures::{storable::Bound, Storable};
use serde::Serialize;
use std::borrow::Cow;

#[derive(CandidType, Deserialize)]
pub struct RateLimitsInitArgs {
    pub version: String,
}

impl GetVersion for RateLimitsInitArgs {
    fn get_version(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.version)
    }
}

#[derive(CandidType, Deserialize, Clone, Debug, Serialize)]
pub struct GlobalRateLimitConfig {
    pub max_requests_per_window_registered: u64,
    pub max_requests_per_window_unregistered: u64,
    pub window_duration_seconds: u64,
}

#[derive(CandidType, Deserialize, Clone, Debug, Serialize)]
pub struct RateLimitConfig {
    pub max_requests_per_window: u64,
    pub window_duration_seconds: u64,
}

#[derive(Default, CandidType, Debug, Deserialize, Clone, Serialize, Copy, PartialEq)]
pub enum TokenType {
    Sats,
    Dolr,
    #[default]
    Free,
    YralProSubscription,
}

#[derive(CandidType, Deserialize, Clone, Debug, Serialize)]
pub struct PropertyRateLimitConfig {
    pub property: String,
    pub max_requests_per_window_registered: u64,
    pub max_requests_per_window_unregistered: u64,
    pub window_duration_seconds: u64,
    pub max_requests_per_property_all_users: Option<u64>,
    pub property_rate_limit_window_duration_seconds: Option<u64>,
}

impl Storable for PropertyRateLimitConfig {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes)
            .expect("Failed to serialize PropertyRateLimitConfig");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref())
            .expect("Failed to deserialize PropertyRateLimitConfig")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Clone)]
pub enum RateLimitResult {
    Ok(String),
    Err(String),
}

#[derive(CandidType, Deserialize, Clone)]
pub struct RateLimitStatus {
    pub principal: Principal,
    pub request_count: u64,
    pub window_start: u64,
    pub window_duration_seconds: u64,
    pub max_requests_per_window_per_user: u64,
    pub is_limited: bool,
}

impl Default for GlobalRateLimitConfig {
    fn default() -> Self {
        GlobalRateLimitConfig {
            max_requests_per_window_registered:
                super::consts::DEFAULT_MAX_REQUESTS_PER_WINDOW_REGISTERED,
            max_requests_per_window_unregistered:
                super::consts::DEFAULT_MAX_REQUESTS_PER_WINDOW_UNREGISTERED,
            window_duration_seconds: super::consts::DEFAULT_WINDOW_DURATION_SECONDS,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RateLimitKey {
    pub principal: Principal,
    pub property: String,
}

impl RateLimitKey {
    pub fn new(principal: Principal, property: String) -> Self {
        Self {
            principal,
            property,
        }
    }

    pub fn default_property(principal: Principal) -> Self {
        Self {
            principal,
            property: "default".to_string(),
        }
    }
}

impl Storable for RateLimitKey {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes).expect("Failed to serialize RateLimitKey");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to deserialize RateLimitKey")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RateLimitEntry {
    pub request_count: u64,
    pub window_start: u64,
    pub config: Option<RateLimitConfig>,
}

impl Storable for RateLimitEntry {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes).expect("Failed to serialize RateLimitEntry");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to deserialize RateLimitEntry")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct VideoGenRequestKey {
    pub principal: Principal,
    pub counter: u64,
}

impl VideoGenRequestKey {
    pub fn new(principal: Principal, counter: u64) -> Self {
        Self { principal, counter }
    }
}

impl Storable for VideoGenRequestKey {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes)
            .expect("Failed to serialize VideoGenRequestKey");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to deserialize VideoGenRequestKey")
    }

    const BOUND: Bound = Bound::Unbounded;
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum VideoGenRequestStatus {
    Pending,
    Processing,
    Complete(String), // Contains result URL
    Failed(String),   // Contains error message
}

#[derive(CandidType, Deserialize, Serialize, Clone, Debug)]
pub struct VideoGenRequest {
    pub model_name: String,
    pub prompt: String,
    pub status: VideoGenRequestStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub payment_amount: Option<String>,
    #[serde(default)]
    pub token_type: Option<TokenType>,
}

impl Storable for VideoGenRequest {
    fn to_bytes(&'_ self) -> Cow<'_, [u8]> {
        let mut bytes = Vec::new();
        ciborium::ser::into_writer(self, &mut bytes).expect("Failed to serialize VideoGenRequest");
        Cow::Owned(bytes)
    }

    fn from_bytes(bytes: Cow<[u8]>) -> Self {
        ciborium::de::from_reader(bytes.as_ref()).expect("Failed to deserialize VideoGenRequest")
    }

    const BOUND: Bound = Bound::Unbounded;
}
