use candid::{encode_args, encode_one, Principal};
use pocket_ic::WasmResult;
use shared_utils::{
    common::types::known_principal::KnownPrincipalType,
    constant::TEST_BACKUP_INDIVIDUAL_USER_CANISTER_BATCH_SIZE,
};
use test_utils::setup::{
    env::pocket_ic_env::get_new_pocket_ic_env, test_constants::get_mock_user_charlie_principal_id,
};

#[test]
fn provision_empty_canisters_in_a_subnet_test() {
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
            candid::encode_one(application_subnets[0]).unwrap(),
        )
        .map(|res| {
            let canister_id_result: Result<Principal, String> = match res {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("Canister call failed"),
            };
            canister_id_result.unwrap()
        })
        .unwrap();

    for _i in 0..130 {
        pocket_ic.tick();
    }

    let empty_canisters_cnt = pocket_ic
        .query_call(
            subnet_orchestrator_canister_id,
            super_admin,
            "get_subnet_backup_capacity",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let subnet_capacity: u64 = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_subnet_backup_capacity failed\n"),
            };
            subnet_capacity
        })
        .unwrap();

    assert_eq!(
        empty_canisters_cnt,
        TEST_BACKUP_INDIVIDUAL_USER_CANISTER_BATCH_SIZE
    );

    pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "provision_empty_canisters_in_a_subnet",
            encode_args((subnet_orchestrator_canister_id, 100_u64)).unwrap(),
        )
        .map(|reply_payload| {
            let res: Result<(), String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 provision_empty_canisters_in_a_subnet failed\n"),
            };
            res
        })
        .unwrap()
        .unwrap();

    for _i in 0..110 {
        pocket_ic.tick();
    }

    let empty_canisters_cnt = pocket_ic
        .query_call(
            subnet_orchestrator_canister_id,
            super_admin,
            "get_subnet_backup_capacity",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let subnet_capacity: u64 = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_subnet_backup_capacity failed\n"),
            };
            subnet_capacity
        })
        .unwrap();

    assert_eq!(
        empty_canisters_cnt,
        TEST_BACKUP_INDIVIDUAL_USER_CANISTER_BATCH_SIZE + 100
    );
}
