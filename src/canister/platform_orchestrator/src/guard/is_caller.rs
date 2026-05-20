use ic_cdk::api::is_controller;
use ic_cdk::caller;

pub(crate) fn is_caller_platform_global_admin_or_controller() -> Result<(), String> {
    match is_controller(&caller()) {
        true => Ok(()),
        false => Err("Unauthorized".into()),
    }
}
