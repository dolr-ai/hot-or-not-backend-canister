use std::time::{Duration, SystemTime};

use candid::{encode_one, Principal};
use pocket_ic::WasmResult;
use shared_utils::{
    canister_specific::platform_orchestrator::types::args::PlatformOrchestratorInitArgs,
    common::types::known_principal::KnownPrincipalType,
};
use test_utils::setup::{
    env::pocket_ic_env::get_new_pocket_ic_env,
    test_constants::{
        get_mock_user_alice_principal_id, get_mock_user_bob_principal_id,
        get_mock_user_charlie_principal_id, get_mock_user_dan_principal_id,
    },
};

const PF_ORCH_WASM_PATH: &str =
    "../../../target/wasm32-unknown-unknown/release/platform_orchestrator.wasm.gz";

fn pf_orch_canister_wasm() -> Vec<u8> {
    std::fs::read(PF_ORCH_WASM_PATH).unwrap()
}

// TODO: remove this when removing the update_last_access_time API from PF Orch

#[test]
#[ignore]
fn update_canisters_last_access_time_test() {
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
    let alice_principal_id = get_mock_user_alice_principal_id();
    let bob_principal_id = get_mock_user_bob_principal_id();
    let dan_principal_id = get_mock_user_dan_principal_id();

    pocket_ic
        .update_call(
            platform_canister_id,
            super_admin,
            "add_principal_as_global_admin",
            candid::encode_one(charlie_global_admin).unwrap(),
        )
        .unwrap();

    let user_index_canister_id: Principal = pocket_ic
        .update_call(
            platform_canister_id,
            super_admin,
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

    for _ in 0..50 {
        pocket_ic.tick();
    }

    // upgrade pf_orch

    let platform_orchestrator_init_args = PlatformOrchestratorInitArgs {
        version: "v1.0.0".into(),
    };
    pocket_ic
        .upgrade_canister(
            platform_canister_id,
            pf_orch_canister_wasm(),
            candid::encode_one(platform_orchestrator_init_args).unwrap(),
            Some(super_admin),
        )
        .unwrap();
    for _ in 0..20 {
        pocket_ic.tick();
    }

    // User Index available details - call get_subnet_available_capacity

    let res = pocket_ic
        .query_call(
            user_index_canister_id,
            super_admin,
            "get_subnet_available_capacity",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let subnet_capacity: u64 = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_subnet_available_capacity failed\n"),
            };
            subnet_capacity
        })
        .unwrap();
    println!("Avail capacity: {:?}", res);

    // call get_subnet_backup_capacity

    let res = pocket_ic
        .query_call(
            user_index_canister_id,
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
    println!("Backup capacity: {:?}", res);

    // create user canisters

    let alice_individual_template_canister_id = pocket_ic
        .update_call(
            user_index_canister_id,
            alice_principal_id,
            "get_requester_principals_canister_id_create_if_not_exists",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<Principal, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!(
                    "\n🛑 get_requester_principals_canister_id_create_if_not_exists failed\n"
                ),
            };
            result
        })
        .unwrap()
        .unwrap();
    println!("res1: {:?}", alice_individual_template_canister_id);

    let bob_individual_template_canister_id = pocket_ic
        .update_call(
            user_index_canister_id,
            bob_principal_id,
            "get_requester_principals_canister_id_create_if_not_exists",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<Principal, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!(
                    "\n🛑 get_requester_principals_canister_id_create_if_not_exists failed\n"
                ),
            };
            result
        })
        .unwrap()
        .unwrap();
    println!("res2: {:?}", bob_individual_template_canister_id);

    let dan_individual_template_canister_id = pocket_ic
        .update_call(
            user_index_canister_id,
            dan_principal_id,
            "get_requester_principals_canister_id_create_if_not_exists",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<Principal, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!(
                    "\n🛑 get_requester_principals_canister_id_create_if_not_exists failed\n"
                ),
            };
            result
        })
        .unwrap()
        .unwrap();
    println!("res3: {:?}", dan_individual_template_canister_id);

    pocket_ic.add_cycles(alice_individual_template_canister_id, 2_000_000_000_000);
    pocket_ic.add_cycles(bob_individual_template_canister_id, 2_000_000_000_000);
    pocket_ic.add_cycles(dan_individual_template_canister_id, 2_000_000_000_000);

    // Call get_last_canister_functionality_access_time for all users

    let res1 = pocket_ic
        .query_call(
            alice_individual_template_canister_id,
            alice_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res1: {:?}", res1);

    let res2 = pocket_ic
        .query_call(
            bob_individual_template_canister_id,
            bob_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res2: {:?}", res2);

    let res3 = pocket_ic
        .query_call(
            dan_individual_template_canister_id,
            dan_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res3: {:?}", res3);

    // Forward timer
    pocket_ic.advance_time(Duration::from_secs(60 * 60));
    for _ in 0..5 {
        pocket_ic.tick();
    }

    // Call update_canisters_last_functionality_access_time on pf orch

    let res = pocket_ic
        .update_call(
            platform_canister_id,
            super_admin,
            "update_canisters_last_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<String, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 update_canisters_last_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res: {:?}", res);

    for _ in 0..5 {
        pocket_ic.tick();
    }

    // Call get_last_canister_functionality_access_time for all users

    let res11 = pocket_ic
        .query_call(
            alice_individual_template_canister_id,
            alice_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res11: {:?}", res11);
    assert!(res11 > res1);

    let res22 = pocket_ic
        .query_call(
            bob_individual_template_canister_id,
            bob_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res22: {:?}", res22);
    assert!(res22 > res2);

    let res33 = pocket_ic
        .query_call(
            dan_individual_template_canister_id,
            dan_principal_id,
            "get_last_canister_functionality_access_time",
            encode_one(()).unwrap(),
        )
        .map(|reply_payload| {
            let result: Result<SystemTime, String> = match reply_payload {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                _ => panic!("\n🛑 get_last_canister_functionality_access_time failed\n"),
            };
            result
        })
        .unwrap();
    println!("res33: {:?}", res33);
    assert!(res33 > res3);
}
