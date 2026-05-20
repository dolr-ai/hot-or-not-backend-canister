use ic_cdk_macros::post_upgrade;

/// Deliberately empty post_upgrade to test whether pre_upgrade (on the v10 wasm)
/// is the source of all upgrade failures. If this wasm installs, pre_upgrade is fine
/// and post_upgrade was the culprit. If it still fails, pre_upgrade is panicking.
#[post_upgrade]
pub fn post_upgrade() {}
