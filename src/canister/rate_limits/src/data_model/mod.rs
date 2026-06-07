pub mod memory;
use candid::Principal;
use ic_stable_structures::StableBTreeMap;
use memory::{
    Memory, get_property_configs_memory, get_property_rate_limits_memory, get_rate_limits_memory,
    get_user_request_counters_memory, get_video_gen_requests_memory,
};
use serde::{Deserialize, Serialize};
use shared_utils::canister_specific::rate_limits::types::{
    RateLimitEntry, RateLimitKey, TokenType,
};
use shared_utils::canister_specific::rate_limits::{
    GlobalRateLimitConfig, PropertyRateLimitConfig, RateLimitConfig, RateLimitStatus,
    VideoGenRequest, VideoGenRequestKey, VideoGenRequestStatus,
};
use shared_utils::service::SetVersion;
use std::collections::HashSet;

#[derive(Serialize, Deserialize)]
pub struct CanisterData {
    #[serde(skip, default = "init_rate_limits")]
    pub rate_limits: StableBTreeMap<RateLimitKey, RateLimitEntry, Memory>,
    #[serde(skip, default = "init_property_configs")]
    pub property_configs: StableBTreeMap<String, PropertyRateLimitConfig, Memory>,
    #[serde(skip, default = "init_property_rate_limits")]
    pub property_rate_limits: StableBTreeMap<String, RateLimitEntry, Memory>,
    #[serde(skip, default = "init_video_gen_requests")]
    pub video_gen_requests: StableBTreeMap<VideoGenRequestKey, VideoGenRequest, Memory>,
    #[serde(skip, default = "init_user_request_counters")]
    pub user_request_counters: StableBTreeMap<Principal, u64, Memory>,
    pub version: String,
    pub default_config: GlobalRateLimitConfig,
    pub blacklist: HashSet<String>,
}

impl SetVersion for CanisterData {
    fn set_version(&mut self, version: &str) {
        self.version = version.into();
    }
}

fn init_rate_limits() -> StableBTreeMap<RateLimitKey, RateLimitEntry, Memory> {
    StableBTreeMap::init(get_rate_limits_memory())
}

fn init_property_configs() -> StableBTreeMap<String, PropertyRateLimitConfig, Memory> {
    StableBTreeMap::init(get_property_configs_memory())
}

fn init_property_rate_limits() -> StableBTreeMap<String, RateLimitEntry, Memory> {
    StableBTreeMap::init(get_property_rate_limits_memory())
}

fn init_video_gen_requests() -> StableBTreeMap<VideoGenRequestKey, VideoGenRequest, Memory> {
    StableBTreeMap::init(get_video_gen_requests_memory())
}

fn init_user_request_counters() -> StableBTreeMap<Principal, u64, Memory> {
    StableBTreeMap::init(get_user_request_counters_memory())
}

impl Default for CanisterData {
    fn default() -> Self {
        Self {
            rate_limits: init_rate_limits(),
            property_configs: init_property_configs(),
            property_rate_limits: init_property_rate_limits(),
            video_gen_requests: init_video_gen_requests(),
            user_request_counters: init_user_request_counters(),
            version: "v1.0.0".into(),
            default_config: GlobalRateLimitConfig::default(),
            blacklist: HashSet::new(),
        }
    }
}

impl CanisterData {
    pub fn is_rate_limited_with_property(
        &self,
        principal: &Principal,
        property: &str,
        is_registered: bool,
    ) -> bool {
        // Check blacklist first - if property or "all" is blacklisted, always return true (rate limited)
        if self.blacklist.contains(property) || self.blacklist.contains("all") {
            return true;
        }

        // Check property-wide daily rate limit first
        if self.is_property_daily_rate_limited(property).is_limited {
            return true;
        }

        if let Some(status) =
            self.get_rate_limit_status_with_property(principal, property, is_registered)
        {
            status.is_limited
        } else {
            false
        }
    }

    pub fn increment_request_with_property(&mut self, principal: &Principal, property: &str) {
        let current_time = ic_cdk::api::time() / 1_000_000_000; // Convert nanoseconds to seconds
        let key = RateLimitKey::new(*principal, property.to_string());

        // First, increment the per-principal counter
        let entry = if let Some(mut existing) = self.rate_limits.get(&key) {
            // Determine window duration from config, property config, or default
            let window_duration = if let Some(config) = &existing.config {
                config.window_duration_seconds
            } else if let Some(prop_config) = self.property_configs.get(&property.to_string()) {
                prop_config.window_duration_seconds
            } else {
                self.default_config.window_duration_seconds
            };

            // Check if we need to reset the window
            if current_time >= existing.window_start + window_duration {
                RateLimitEntry {
                    request_count: 1,
                    window_start: current_time,
                    config: existing.config.clone(),
                }
            } else {
                existing.request_count += 1;
                existing
            }
        } else {
            RateLimitEntry {
                request_count: 1,
                window_start: current_time,
                config: None, // Use default config for new entries
            }
        };

        self.rate_limits.insert(key, entry);

        // Then, increment the property-wide counter if configured
        if let Some(prop_config) = self.property_configs.get(&property.to_string())
            && prop_config.max_requests_per_property_all_users.is_some()
        {
            let window_duration = prop_config
                .property_rate_limit_window_duration_seconds
                .unwrap_or(86400); // Default to 24 hours

            let property_entry =
                if let Some(mut existing) = self.property_rate_limits.get(&property.to_string()) {
                    // Check if we need to reset the window
                    if current_time >= existing.window_start + window_duration {
                        RateLimitEntry {
                            request_count: 1,
                            window_start: current_time,
                            config: None,
                        }
                    } else {
                        existing.request_count += 1;
                        existing
                    }
                } else {
                    RateLimitEntry {
                        request_count: 1,
                        window_start: current_time,
                        config: None,
                    }
                };

            self.property_rate_limits
                .insert(property.to_string(), property_entry);
        }
    }

    pub fn decrement_request_with_property(&mut self, principal: &Principal, property: &str) {
        let current_time = ic_cdk::api::time() / 1_000_000_000; // Convert nanoseconds to seconds
        let key = RateLimitKey::new(*principal, property.to_string());

        // First, decrement the per-principal counter
        if let Some(mut entry) = self.rate_limits.get(&key) {
            // Determine window duration from config, property config, or default
            let window_duration = if let Some(config) = &entry.config {
                config.window_duration_seconds
            } else if let Some(prop_config) = self.property_configs.get(&property.to_string()) {
                prop_config.window_duration_seconds
            } else {
                self.default_config.window_duration_seconds
            };

            // Only decrement if we're still within the same window and count is greater than 0
            if current_time < entry.window_start + window_duration && entry.request_count > 0 {
                entry.request_count -= 1;
                self.rate_limits.insert(key, entry);
            }
        }

        // Then, decrement the property-wide counter if configured
        if let Some(prop_config) = self.property_configs.get(&property.to_string())
            && prop_config.max_requests_per_property_all_users.is_some()
        {
            let window_duration = prop_config
                .property_rate_limit_window_duration_seconds
                .unwrap_or(86400); // Default to 24 hours

            if let Some(mut entry) = self.property_rate_limits.get(&property.to_string()) {
                // Only decrement if we're still within the same window and count is greater than 0
                if current_time < entry.window_start + window_duration && entry.request_count > 0 {
                    entry.request_count -= 1;
                    self.property_rate_limits
                        .insert(property.to_string(), entry);
                }
            }
        }
    }

    pub fn get_rate_limit_status_with_property(
        &self,
        principal: &Principal,
        property: &str,
        is_registered: bool,
    ) -> Option<RateLimitStatus> {
        let key = RateLimitKey::new(*principal, property.to_string());
        let current_time = ic_cdk::api::time() / 1_000_000_000;

        // Check if blacklisted first - if so, always return a status showing rate limit exceeded
        if self.blacklist.contains(property) || self.blacklist.contains("all") {
            return Some(RateLimitStatus {
                principal: *principal,
                request_count: 1, // Any value >= 0 will exceed max_requests of 0
                window_start: current_time,
                is_limited: true, // Always limited when blacklisted
                window_duration_seconds: 0,
                max_requests_per_window_per_user: 0,
            });
        }

        // Check property-wide daily rate limit

        let property_wide_limit_status = self.is_property_daily_rate_limited(property);

        let _user_rate_limit_entry = self.rate_limits.get(&key).unwrap_or(RateLimitEntry {
            request_count: 0,
            window_start: current_time,
            config: None,
        });

        if property_wide_limit_status.is_limited {
            // Get the principal's current count for the status
            let entry = self.rate_limits.get(&key).unwrap_or(RateLimitEntry {
                request_count: 0,
                window_start: current_time,
                config: None,
            });

            return Some(RateLimitStatus {
                principal: *principal,
                request_count: entry.request_count,
                window_start: entry.window_start,
                window_duration_seconds: property_wide_limit_status.window_duration_seconds,
                max_requests_per_window_per_user: property_wide_limit_status
                    .max_requests_per_window_per_user,
                is_limited: true, // Limited due to property-wide limit
            });
        }

        // Create dummy entry for when no entry exists
        let dummy_entry = RateLimitEntry {
            request_count: 0,
            window_start: current_time,
            config: None,
        };

        // Get entry or use dummy
        let entry = self.rate_limits.get(&key).unwrap_or(dummy_entry);

        let (max_requests, window_duration) = if let Some(config) = &entry.config {
            (
                config.max_requests_per_window,
                config.window_duration_seconds,
            )
        } else if let Some(prop_config) = self.property_configs.get(&property.to_string()) {
            // Use property-specific config based on registration status
            let max_req = if is_registered {
                prop_config.max_requests_per_window_registered
            } else {
                prop_config.max_requests_per_window_unregistered
            };
            (max_req, prop_config.window_duration_seconds)
        } else {
            // Use default config based on registration status
            let max_req = if is_registered {
                self.default_config.max_requests_per_window_registered
            } else {
                self.default_config.max_requests_per_window_unregistered
            };
            (max_req, self.default_config.window_duration_seconds)
        };

        // Check if we're still within the window
        let within_window = current_time < entry.window_start + window_duration;

        // Only limited if within window AND exceeded max requests
        let is_limited =
            within_window && (max_requests == 0 || entry.request_count >= max_requests);

        Some(RateLimitStatus {
            principal: *principal,
            request_count: entry.request_count,
            window_start: entry.window_start,
            is_limited,
            window_duration_seconds: window_duration,
            max_requests_per_window_per_user: max_requests,
        })
    }

    pub fn reset_rate_limit_with_property(&mut self, principal: &Principal, property: &str) {
        let key = RateLimitKey::new(*principal, property.to_string());
        self.rate_limits.remove(&key);
    }

    pub fn reset_all_principal_rate_limits(&mut self, principal: &Principal) {
        // Remove all entries for a principal across all properties
        let keys_to_remove: Vec<RateLimitKey> = self
            .rate_limits
            .iter()
            .filter(|(k, _)| k.principal == *principal)
            .map(|(k, _)| k)
            .collect();

        for key in keys_to_remove {
            self.rate_limits.remove(&key);
        }
    }

    pub fn clear_all_rate_limits(&mut self) {
        // Clear all entries from the StableBTreeMap
        let keys: Vec<RateLimitKey> = self.rate_limits.iter().map(|(k, _)| k).collect();
        for key in keys {
            self.rate_limits.remove(&key);
        }
    }

    pub fn set_principal_property_rate_limit(
        &mut self,
        principal: &Principal,
        property: &str,
        config: RateLimitConfig,
    ) {
        let key = RateLimitKey::new(*principal, property.to_string());
        if let Some(mut entry) = self.rate_limits.get(&key) {
            entry.config = Some(config);
            self.rate_limits.insert(key, entry);
        } else {
            let current_time = ic_cdk::api::time() / 1_000_000_000;
            let entry = RateLimitEntry {
                request_count: 0,
                window_start: current_time,
                config: Some(config),
            };
            self.rate_limits.insert(key, entry);
        }
    }

    pub fn get_principal_property_config(
        &self,
        principal: &Principal,
        property: &str,
    ) -> Option<RateLimitConfig> {
        let key = RateLimitKey::new(*principal, property.to_string());
        self.rate_limits
            .get(&key)
            .and_then(|entry| entry.config.clone())
    }

    pub fn set_property_config(&mut self, config: PropertyRateLimitConfig) {
        self.property_configs
            .insert(config.property.clone(), config);
    }

    pub fn get_property_config(&self, property: &str) -> Option<PropertyRateLimitConfig> {
        self.property_configs.get(&property.to_string())
    }

    pub fn remove_property_config(&mut self, property: &str) {
        self.property_configs.remove(&property.to_string());
    }

    pub fn get_all_property_configs(&self) -> Vec<PropertyRateLimitConfig> {
        self.property_configs
            .iter()
            .map(|(_, v)| v.clone())
            .collect()
    }

    pub fn add_to_blacklist(&mut self, property: String) {
        self.blacklist.insert(property);
    }

    pub fn remove_from_blacklist(&mut self, property: &str) -> bool {
        self.blacklist.remove(property)
    }

    pub fn get_blacklist(&self) -> Vec<String> {
        self.blacklist.iter().cloned().collect()
    }

    pub fn clear_blacklist(&mut self) {
        self.blacklist.clear();
    }

    pub fn is_blacklisted(&self, property: &str) -> bool {
        self.blacklist.contains(property) || self.blacklist.contains("all")
    }

    fn is_property_daily_rate_limited(&self, property: &str) -> RateLimitStatus {
        // First check if property has a property-wide limit configured
        if let Some(prop_config) = self.property_configs.get(&property.to_string())
            && let Some(property_limit) = prop_config.max_requests_per_property_all_users
        {
            // Get the window duration (default to 24 hours if not specified)
            let window_duration = prop_config
                .property_rate_limit_window_duration_seconds
                .unwrap_or(86400); // Default to 24 hours

            let current_time = ic_cdk::api::time() / 1_000_000_000;

            if let Some(entry) = self.property_rate_limits.get(&property.to_string()) {
                // Check if we're still within the same window
                if current_time < entry.window_start + window_duration {
                    // Within same window, check if limit exceeded
                    return RateLimitStatus {
                        principal: Principal::anonymous(),
                        request_count: entry.request_count,
                        window_start: entry.window_start,
                        window_duration_seconds: window_duration,
                        max_requests_per_window_per_user: 0,
                        is_limited: entry.request_count >= property_limit,
                    };
                }
            }
        }
        RateLimitStatus {
            principal: Principal::anonymous(),
            request_count: 0,
            window_start: 0,
            window_duration_seconds: 0,
            max_requests_per_window_per_user: 0,
            is_limited: false,
        }
    }

    pub fn get_property_daily_usage(&self, property: &str) -> u64 {
        let current_time = ic_cdk::api::time() / 1_000_000_000;

        if let Some(entry) = self.property_rate_limits.get(&property.to_string()) {
            // Get the window duration from config
            let window_duration =
                if let Some(prop_config) = self.property_configs.get(&property.to_string()) {
                    prop_config
                        .property_rate_limit_window_duration_seconds
                        .unwrap_or(86400)
                } else {
                    86400 // Default to 24 hours
                };

            // Check if we're still within the same window
            if current_time < entry.window_start + window_duration {
                return entry.request_count;
            }
        }
        0
    }

    pub fn reset_property_daily_limit(&mut self, property: &str) {
        self.property_rate_limits.remove(&property.to_string());
    }

    // Check only property-wide limit for paid requests
    pub fn is_property_daily_rate_limited_for_paid(&self, property: &str) -> bool {
        self.is_property_daily_rate_limited(property).is_limited
    }

    // Increment only property counter for paid requests
    pub fn increment_paid_request_property_only(&mut self, property: &str) {
        let current_time = ic_cdk::api::time() / 1_000_000_000;

        // Only increment the property-wide counter if configured
        if let Some(prop_config) = self.property_configs.get(&property.to_string())
            && prop_config.max_requests_per_property_all_users.is_some()
        {
            let window_duration = prop_config
                .property_rate_limit_window_duration_seconds
                .unwrap_or(86400); // Default to 24 hours

            let property_entry =
                if let Some(mut existing) = self.property_rate_limits.get(&property.to_string()) {
                    // Check if we need to reset the window
                    if current_time >= existing.window_start + window_duration {
                        RateLimitEntry {
                            request_count: 1,
                            window_start: current_time,
                            config: None,
                        }
                    } else {
                        existing.request_count += 1;
                        existing
                    }
                } else {
                    RateLimitEntry {
                        request_count: 1,
                        window_start: current_time,
                        config: None,
                    }
                };

            self.property_rate_limits
                .insert(property.to_string(), property_entry);
        }
    }

    // Decrement only property counter (for paid requests)
    pub fn decrement_property_counter_only(&mut self, property: &str) {
        let current_time = ic_cdk::api::time() / 1_000_000_000;

        // Only decrement the property-wide counter if configured
        if let Some(prop_config) = self.property_configs.get(&property.to_string())
            && prop_config.max_requests_per_property_all_users.is_some()
        {
            let window_duration = prop_config
                .property_rate_limit_window_duration_seconds
                .unwrap_or(86400); // Default to 24 hours

            if let Some(mut entry) = self.property_rate_limits.get(&property.to_string()) {
                // Only decrement if we're still within the same window and count is greater than 0
                if current_time < entry.window_start + window_duration && entry.request_count > 0 {
                    entry.request_count -= 1;
                    self.property_rate_limits
                        .insert(property.to_string(), entry);
                }
            }
        }
    }

    // Video generation request methods
    pub fn create_video_gen_request(
        &mut self,
        principal: Principal,
        model_name: String,
        prompt: String,
        token_type: Option<TokenType>,
        payment_amount: Option<String>,
    ) -> VideoGenRequestKey {
        let current_time = ic_cdk::api::time() / 1_000_000_000; // Convert nanoseconds to seconds

        // Get current counter for user or initialize to 0
        let counter = self.user_request_counters.get(&principal).unwrap_or(0);
        let new_counter = counter + 1;
        self.user_request_counters.insert(principal, new_counter);

        let key = VideoGenRequestKey::new(principal, new_counter);
        let request = VideoGenRequest {
            model_name,
            prompt,
            status: VideoGenRequestStatus::Pending,
            token_type,
            created_at: current_time,
            updated_at: current_time,
            payment_amount,
        };

        self.video_gen_requests.insert(key.clone(), request);
        key
    }

    pub fn update_video_gen_request_status(
        &mut self,
        key: &VideoGenRequestKey,
        status: VideoGenRequestStatus,
    ) -> Result<(), String> {
        let current_time = ic_cdk::api::time() / 1_000_000_000;

        if let Some(mut request) = self.video_gen_requests.get(key) {
            request.status = status;
            request.updated_at = current_time;
            self.video_gen_requests.insert(key.clone(), request);
            Ok(())
        } else {
            Err("Video generation request not found".to_string())
        }
    }

    pub fn get_video_gen_request(&self, key: &VideoGenRequestKey) -> Option<VideoGenRequest> {
        self.video_gen_requests.get(key)
    }

    pub fn get_user_video_gen_requests(
        &self,
        principal: Principal,
        start: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<(VideoGenRequestKey, VideoGenRequest)> {
        let max_counter = self.user_request_counters.get(&principal).unwrap_or(0);
        if max_counter == 0 {
            return Vec::new();
        }

        let limit = limit.unwrap_or(10).min(100); // Default 10, max 100

        // If start is not provided, start from the most recent (max_counter)
        // Otherwise, start from the provided counter going backwards
        let start_counter = start.unwrap_or(max_counter);

        // Ensure start_counter doesn't exceed max_counter
        let start_counter = start_counter.min(max_counter);

        // Calculate the end counter (going backwards)
        let end_counter = if start_counter > limit {
            start_counter - limit + 1
        } else {
            1
        };

        let mut results = Vec::new();

        // Iterate backwards from start_counter to end_counter
        for counter in (end_counter..=start_counter).rev() {
            let key = VideoGenRequestKey::new(principal, counter);
            if let Some(request) = self.video_gen_requests.get(&key) {
                results.push((key, request));
            }
        }

        results
    }
}
