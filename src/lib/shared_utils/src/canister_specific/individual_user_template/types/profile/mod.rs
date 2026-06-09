use candid::{CandidType, Deserialize, Principal};
use core::fmt;
use serde::{
    de::{self, MapAccess, Visitor},
    Serialize,
};
use std::marker::PhantomData;

use crate::canister_specific::user_info_service::types::{
    NSFWInfo, ProfilePictureData, SubscriptionPlan,
};

use super::migration::MigrationInfo;

#[derive(CandidType, Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum UserAccountType {
    MainAccount { bots: Vec<Principal> },
    BotAccount { owner: Principal },
}

impl Default for UserAccountType {
    fn default() -> Self {
        UserAccountType::MainAccount { bots: Vec::new() }
    }
}

#[derive(Default, Clone, CandidType, Debug, Serialize)]
pub struct UserProfile {
    pub principal_id: Option<Principal>,
    pub profile_stats: UserProfileGlobalStats,
    #[serde(default)]
    pub referrer_details: Option<UserCanisterDetails>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub website_url: Option<String>,
    #[serde(default)]
    pub subscription_plan: SubscriptionPlan,
    #[serde(default)]
    pub profile_picture: Option<ProfilePictureData>,
    #[serde(default)]
    pub is_ai_influencer: bool,
}

// Custom deserializer for UserProfile to handle backwards compatibility
// When deserializing old data that has profile_picture_url but no profile_picture field,
// we migrate by creating a ProfilePictureData from the profile_picture_url with is_nsfw defaulting to false
impl<'de> Deserialize<'de> for UserProfile {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        // Define the field names we expect
        const FIELDS: &[&str] = &[
            "principal_id",
            "profile_picture_url",
            "profile_stats",
            "referrer_details",
            "bio",
            "website_url",
            "subscription_plan",
            "profile_picture",
            "is_ai_influencer",
        ];

        // Field enum for identifying which field we're deserializing
        enum Field {
            PrincipalId,
            ProfilePictureUrl,
            ProfileStats,
            ReferrerDetails,
            Bio,
            WebsiteUrl,
            SubscriptionPlan,
            ProfilePicture,
            IsAiInfluencer,
            Unknown,
        }

        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = Field;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("field identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    "principal_id" => Ok(Field::PrincipalId),
                    "profile_picture_url" => Ok(Field::ProfilePictureUrl),
                    "profile_stats" => Ok(Field::ProfileStats),
                    "referrer_details" => Ok(Field::ReferrerDetails),
                    "bio" => Ok(Field::Bio),
                    "website_url" => Ok(Field::WebsiteUrl),
                    "subscription_plan" => Ok(Field::SubscriptionPlan),
                    "profile_picture" => Ok(Field::ProfilePicture),
                    "is_ai_influencer" => Ok(Field::IsAiInfluencer),
                    _ => Ok(Field::Unknown),
                }
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                match value {
                    b"principal_id" => Ok(Field::PrincipalId),
                    b"profile_picture_url" => Ok(Field::ProfilePictureUrl),
                    b"profile_stats" => Ok(Field::ProfileStats),
                    b"referrer_details" => Ok(Field::ReferrerDetails),
                    b"bio" => Ok(Field::Bio),
                    b"website_url" => Ok(Field::WebsiteUrl),
                    b"subscription_plan" => Ok(Field::SubscriptionPlan),
                    b"profile_picture" => Ok(Field::ProfilePicture),
                    b"is_ai_influencer" => Ok(Field::IsAiInfluencer),
                    _ => Ok(Field::Unknown),
                }
            }
        }

        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: de::Deserializer<'de>,
            {
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        struct UserProfileVisitor<'de> {
            marker: PhantomData<UserProfile>,
            lifetime: PhantomData<&'de ()>,
        }

        impl<'de> Visitor<'de> for UserProfileVisitor<'de> {
            type Value = UserProfile;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct UserProfile")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut principal_id: Option<Option<Principal>> = None;
                let mut profile_picture_url: Option<Option<String>> = None;
                let mut profile_stats: Option<UserProfileGlobalStats> = None;
                let mut referrer_details: Option<Option<UserCanisterDetails>> = None;
                let mut bio: Option<Option<String>> = None;
                let mut website_url: Option<Option<String>> = None;
                let mut subscription_plan: Option<SubscriptionPlan> = None;
                let mut profile_picture: Option<Option<ProfilePictureData>> = None;
                let mut is_ai_influencer: Option<bool> = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::PrincipalId => {
                            if principal_id.is_some() {
                                return Err(de::Error::duplicate_field("principal_id"));
                            }
                            principal_id = Some(map.next_value()?);
                        }
                        Field::ProfilePictureUrl => {
                            if profile_picture_url.is_some() {
                                return Err(de::Error::duplicate_field("profile_picture_url"));
                            }
                            profile_picture_url = Some(map.next_value()?);
                        }
                        Field::ProfileStats => {
                            if profile_stats.is_some() {
                                return Err(de::Error::duplicate_field("profile_stats"));
                            }
                            profile_stats = Some(map.next_value()?);
                        }
                        Field::ReferrerDetails => {
                            if referrer_details.is_some() {
                                return Err(de::Error::duplicate_field("referrer_details"));
                            }
                            referrer_details = Some(map.next_value()?);
                        }
                        Field::Bio => {
                            if bio.is_some() {
                                return Err(de::Error::duplicate_field("bio"));
                            }
                            bio = Some(map.next_value()?);
                        }
                        Field::WebsiteUrl => {
                            if website_url.is_some() {
                                return Err(de::Error::duplicate_field("website_url"));
                            }
                            website_url = Some(map.next_value()?);
                        }
                        Field::SubscriptionPlan => {
                            if subscription_plan.is_some() {
                                return Err(de::Error::duplicate_field("subscription_plan"));
                            }
                            subscription_plan = Some(map.next_value()?);
                        }
                        Field::ProfilePicture => {
                            if profile_picture.is_some() {
                                return Err(de::Error::duplicate_field("profile_picture"));
                            }
                            profile_picture = Some(map.next_value()?);
                        }
                        Field::IsAiInfluencer => {
                            if is_ai_influencer.is_some() {
                                return Err(de::Error::duplicate_field("is_ai_influencer"));
                            }
                            is_ai_influencer = Some(map.next_value()?);
                        }
                        Field::Unknown => {
                            // Skip unknown fields for forward compatibility
                            let _ = map.next_value::<de::IgnoredAny>()?;
                        }
                    }
                }

                let principal_id = principal_id.unwrap_or_default();
                let profile_picture_url = profile_picture_url.unwrap_or_default();
                let profile_stats = profile_stats.unwrap_or_default();
                let referrer_details = referrer_details.unwrap_or_default();
                let bio = bio.unwrap_or_default();
                let website_url = website_url.unwrap_or_default();
                let subscription_plan = subscription_plan.unwrap_or_default();
                let profile_picture_value = profile_picture.unwrap_or_default();
                let is_ai_influencer = is_ai_influencer.unwrap_or_default();

                // Migration logic: if profile_picture is None but profile_picture_url exists,
                // create a ProfilePictureData from the URL with nsfw_info defaulting to safe values
                let profile_picture = profile_picture_value.or_else(|| {
                    profile_picture_url.as_ref().map(|url| ProfilePictureData {
                        url: url.clone(),
                        nsfw_info: NSFWInfo::default(),
                    })
                });

                Ok(UserProfile {
                    principal_id,
                    profile_stats,
                    referrer_details,
                    bio,
                    website_url,
                    subscription_plan,
                    profile_picture,
                    is_ai_influencer,
                })
            }
        }

        deserializer.deserialize_struct(
            "UserProfile",
            FIELDS,
            UserProfileVisitor {
                marker: PhantomData::<UserProfile>,
                lifetime: PhantomData,
            },
        )
    }
}

#[derive(Clone, CandidType, Deserialize, Debug, Serialize, PartialEq, Eq)]
pub struct UserCanisterDetails {
    pub profile_owner: Principal,
    pub user_canister_id: Principal,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontend {
    pub display_name: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub principal_id: Principal,
    pub profile_picture_url: Option<String>,
    pub profile_stats: UserProfileGlobalStats,
    pub lifetime_earnings: u64, //Todo: This field should be removed
    pub unique_user_name: Option<String>,
    pub referrer_details: Option<UserCanisterDetails>,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV2 {
    pub display_name: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub principal_id: Principal,
    pub profile_picture_url: Option<String>,
    pub profile_stats: UserProfileGlobalStats,
    pub lifetime_earnings: u64,
    pub unique_user_name: Option<String>,
    pub referrer_details: Option<UserCanisterDetails>,
    pub migration_info: MigrationInfo,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV3 {
    pub principal_id: Principal,
    pub profile_stats: UserProfileGlobalStats,
    pub profile_picture_url: Option<String>,
}

#[derive(CandidType, Deserialize, Clone, Copy, Debug, Default, Serialize, PartialEq, Eq)]
pub struct UserProfileGlobalStats {
    pub hot_bets_received: u64,
    pub not_bets_received: u64,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV4 {
    pub principal_id: Principal,
    pub profile_stats: UserProfileGlobalStats,
    pub profile_picture_url: Option<String>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub caller_follows_user: Option<bool>,
    pub user_follows_caller: Option<bool>,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV5 {
    pub principal_id: Principal,
    pub profile_picture_url: Option<String>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub caller_follows_user: Option<bool>,
    pub user_follows_caller: Option<bool>,
    pub subscription_plan: SubscriptionPlan,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV6 {
    pub principal_id: Principal,
    pub profile_picture: Option<ProfilePictureData>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub caller_follows_user: Option<bool>,
    pub user_follows_caller: Option<bool>,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
}

#[derive(CandidType, Deserialize, Debug, PartialEq, Eq)]
pub struct UserProfileDetailsForFrontendV7 {
    pub principal_id: Principal,
    pub profile_picture: Option<ProfilePictureData>,
    pub bio: Option<String>,
    pub website_url: Option<String>,
    pub followers_count: u64,
    pub following_count: u64,
    pub caller_follows_user: Option<bool>,
    pub user_follows_caller: Option<bool>,
    pub subscription_plan: SubscriptionPlan,
    pub is_ai_influencer: bool,
    pub account_type: UserAccountType,
}

#[derive(Deserialize, CandidType)]
pub struct UserProfileUpdateDetailsFromFrontend {
    pub display_name: Option<String>,
    pub profile_picture_url: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ciborium::{de::from_reader, ser::into_writer};

    /// Old version of UserProfile without profile_picture field (for testing migration)
    #[derive(Serialize)]
    struct UserProfileV1 {
        pub principal_id: Option<Principal>,
        pub profile_picture_url: Option<String>,
        pub profile_stats: UserProfileGlobalStats,
        pub referrer_details: Option<UserCanisterDetails>,
        pub bio: Option<String>,
        pub website_url: Option<String>,
        pub subscription_plan: SubscriptionPlan,
    }

    #[test]
    fn test_userprofile_deserialize_migrates_old_format_without_profile_picture() {
        // Create old format UserProfile (without profile_picture field)
        let old_profile = UserProfileV1 {
            principal_id: Some(Principal::anonymous()),
            profile_picture_url: Some("https://example.com/pic.jpg".to_string()),
            profile_stats: UserProfileGlobalStats::default(),
            referrer_details: None,
            bio: Some("Test bio".to_string()),
            website_url: None,
            subscription_plan: SubscriptionPlan::Free,
        };

        // Serialize old format
        let mut bytes = Vec::new();
        into_writer(&old_profile, &mut bytes).expect("Failed to serialize old profile");

        // Deserialize as new format
        let new_profile: UserProfile =
            from_reader(bytes.as_slice()).expect("Failed to deserialize profile");

        // Verify migration: profile_picture should be created from profile_picture_url
        assert!(
            new_profile.profile_picture.is_some(),
            "profile_picture should be migrated from profile_picture_url"
        );
        let profile_picture = new_profile.profile_picture.unwrap();
        assert_eq!(profile_picture.url, "https://example.com/pic.jpg");
        assert!(
            !profile_picture.nsfw_info.is_nsfw,
            "is_nsfw should default to false"
        );
        assert_eq!(profile_picture.nsfw_info.nsfw_ec, "");
        assert_eq!(profile_picture.nsfw_info.nsfw_gore, "");
        assert!(
            !profile_picture.nsfw_info.csam_detected,
            "csam_detected should default to false"
        );

        // Verify other fields are preserved
        assert_eq!(new_profile.bio, Some("Test bio".to_string()));
    }

    #[test]
    fn test_userprofile_deserialize_preserves_new_format_with_profile_picture() {
        // Create new format UserProfile (with profile_picture field)
        let new_profile = UserProfile {
            principal_id: Some(Principal::anonymous()),
            profile_stats: UserProfileGlobalStats::default(),
            referrer_details: None,
            bio: Some("Test bio".to_string()),
            website_url: Some("https://example.com".to_string()),
            subscription_plan: SubscriptionPlan::Free,
            profile_picture: Some(ProfilePictureData {
                url: "https://example.com/pic.jpg".to_string(),
                nsfw_info: NSFWInfo {
                    is_nsfw: true,
                    nsfw_ec: "explicit".to_string(),
                    nsfw_gore: "none".to_string(),
                    csam_detected: false,
                },
            }),
            is_ai_influencer: false,
        };

        // Serialize new format
        let mut bytes = Vec::new();
        into_writer(&new_profile, &mut bytes).expect("Failed to serialize new profile");

        // Deserialize
        let deserialized: UserProfile =
            from_reader(bytes.as_slice()).expect("Failed to deserialize profile");

        // Verify profile_picture is preserved (not overwritten by migration logic)
        assert!(deserialized.profile_picture.is_some());
        let profile_picture = deserialized.profile_picture.unwrap();
        assert_eq!(profile_picture.url, "https://example.com/pic.jpg");
        assert!(
            profile_picture.nsfw_info.is_nsfw,
            "is_nsfw should be preserved as true"
        );
        assert_eq!(profile_picture.nsfw_info.nsfw_ec, "explicit");
        assert_eq!(profile_picture.nsfw_info.nsfw_gore, "none");
        assert!(!profile_picture.nsfw_info.csam_detected);
    }

    #[test]
    fn test_userprofile_deserialize_old_format_without_profile_picture() {
        // Create old format UserProfile without profile_picture_url
        let old_profile = UserProfileV1 {
            principal_id: Some(Principal::anonymous()),
            profile_picture_url: None,
            profile_stats: UserProfileGlobalStats::default(),
            referrer_details: None,
            bio: None,
            website_url: None,
            subscription_plan: SubscriptionPlan::Free,
        };

        // Serialize old format
        let mut bytes = Vec::new();
        into_writer(&old_profile, &mut bytes).expect("Failed to serialize old profile");

        // Deserialize as new format
        let new_profile: UserProfile =
            from_reader(bytes.as_slice()).expect("Failed to deserialize profile");

        // Verify: profile_picture should be None since profile_picture_url was None
        assert!(
            new_profile.profile_picture.is_none(),
            "profile_picture should be None when profile_picture_url is None"
        );
    }
}
