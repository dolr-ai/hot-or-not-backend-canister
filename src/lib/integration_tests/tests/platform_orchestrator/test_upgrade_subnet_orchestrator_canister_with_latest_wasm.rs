use candid::Principal;
use pocket_ic::WasmResult;
use shared_utils::common::types::known_principal::KnownPrincipalType;
use test_utils::setup::{
    env::pocket_ic_env::get_new_pocket_ic_env, test_constants::get_mock_user_charlie_principal_id,
};

#[test]
fn test_upgrade_subnet_orchestrator_canister_with_latest_wasm() {
    let (pocket_ic, known_principal) = get_new_pocket_ic_env();

    let platform_canister_id = known_principal
        .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
        .cloned()
        .unwrap();

    let super_admin = known_principal
        .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
        .cloned()
        .unwrap();

    let application_subnets = pocket_ic.topology().get_app_subnets();

    let charlie_global_admin = get_mock_user_charlie_principal_id();

    pocket_ic
        .update_call(
            platform_canister_id,
            super_admin,
            "add_principal_as_global_admin",
            candid::encode_one(charlie_global_admin).unwrap(),
        )
        .unwrap();

    let subnet_orchestrator_canister_id: Principal = pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "provision_subnet_orchestrator_canister",
            candid::encode_one(application_subnets[1]).unwrap(),
        )
        .map(|res| {
            let canister_id_result: Result<Principal, String> = match res {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("Canister call failed"),
            };
            canister_id_result.unwrap()
        })
        .unwrap();

    for _ in 0..100 {
        pocket_ic.tick();
    }

    let initial_version_number = pocket_ic
        .query_call(
            subnet_orchestrator_canister_id,
            Principal::anonymous(),
            "get_version_number",
            candid::encode_one(()).unwrap(),
        )
        .map(|res| {
            let version_number: u64 = match res {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("uprade subnet orchestrator with latest wasm call failed"),
            };
            version_number
        })
        .unwrap();

    pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "upgrade_subnet_orchestrator_canister_with_latest_wasm",
            candid::encode_one(subnet_orchestrator_canister_id).unwrap(),
        )
        .map(|res| {
            let upgrade_result: Result<(), String> = match res {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("uprade subnet orchestrator with latest wasm call failed"),
            };
            upgrade_result.unwrap()
        })
        .unwrap();

    let final_version_number = pocket_ic
        .query_call(
            subnet_orchestrator_canister_id,
            Principal::anonymous(),
            "get_version_number",
            candid::encode_one(()).unwrap(),
        )
        .map(|res| {
            let version_number: u64 = match res {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("uprade subnet orchestrator with latest wasm call failed"),
            };
            version_number
        })
        .unwrap();

    assert_eq!(final_version_number, initial_version_number + 1);
}
