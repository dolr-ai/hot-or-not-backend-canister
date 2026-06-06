use candid::Principal;
use pocket_ic::WasmResult;
use shared_utils::{
    canister_specific::user_index::types::args::UserIndexInitArgs,
    common::types::known_principal::KnownPrincipalType,
};
use test_utils::setup::{
    env::pocket_ic_env::get_new_pocket_ic_env, test_constants::get_mock_user_charlie_principal_id,
};

#[test]
fn register_subnet_orchestrator_with_platform_orchestrator_test() {
    let (pocket_ic, known_principal) = get_new_pocket_ic_env();

    let application_subnets = pocket_ic.topology().get_app_subnets();
    let platform_canister_id = known_principal
        .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
        .cloned()
        .unwrap();

    let super_admin = known_principal
        .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
        .cloned()
        .unwrap();

    let charlie_global_admin = get_mock_user_charlie_principal_id();

    pocket_ic
        .update_call(
            platform_canister_id,
            super_admin,
            "add_principal_as_global_admin",
            candid::encode_one(charlie_global_admin).unwrap(),
        )
        .unwrap();

    pocket_ic
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

    for _i in 0..110 {
        pocket_ic.tick();
    }

    let new_subnet_orchestrator_canister = pocket_ic.create_canister();
    pocket_ic.add_cycles(new_subnet_orchestrator_canister, 1_000_000_000_000_000);

    let register_new_subnet_orchestrator_res = pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "register_new_subnet_orchestrator",
            candid::encode_args((new_subnet_orchestrator_canister, true)).unwrap(),
        )
        .map(|wasm_result| {
            let res: Result<(), String> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(register_new_subnet_orchestrator_res.is_err());

    pocket_ic
        .set_controllers(
            new_subnet_orchestrator_canister,
            None,
            vec![platform_canister_id],
        )
        .unwrap();

    let register_new_subnet_orchestrator_res = pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "register_new_subnet_orchestrator",
            candid::encode_args((new_subnet_orchestrator_canister, true)).unwrap(),
        )
        .map(|wasm_result| {
            let res: Result<(), String> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(register_new_subnet_orchestrator_res.is_err());

    let subnet_orchestrator_wasm =
        include_bytes!("../../../../../target/wasm32-unknown-unknown/release/user_index.wasm.gz");
    pocket_ic.install_canister(
        new_subnet_orchestrator_canister,
        subnet_orchestrator_wasm.to_vec(),
        candid::encode_one(UserIndexInitArgs {
            ..Default::default()
        })
        .unwrap(),
        Some(platform_canister_id),
    );

    let register_new_subnet_orchestrator_res = pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "register_new_subnet_orchestrator",
            candid::encode_args((new_subnet_orchestrator_canister, false)).unwrap(),
        )
        .map(|wasm_result| {
            let res: Result<(), String> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(register_new_subnet_orchestrator_res.is_ok());

    let all_available_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_available_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(!all_available_subnet_orchestrator.contains(&new_subnet_orchestrator_canister));

    let all_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(all_subnet_orchestrator.contains(&new_subnet_orchestrator_canister));

    let register_new_subnet_orchestrator_res = pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "register_new_subnet_orchestrator",
            candid::encode_args((new_subnet_orchestrator_canister, true)).unwrap(),
        )
        .map(|wasm_result| {
            let res: Result<(), String> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(register_new_subnet_orchestrator_res.is_ok());

    let all_available_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_available_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(all_available_subnet_orchestrator.contains(&new_subnet_orchestrator_canister));
}

#[test]
fn deregister_subnet_orchestrator_from_platform_orchestrator() {
    let (pocket_ic, known_principal) = get_new_pocket_ic_env();

    let application_subnets = pocket_ic.topology().get_app_subnets();
    let platform_canister_id = known_principal
        .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
        .cloned()
        .unwrap();

    let super_admin = known_principal
        .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
        .cloned()
        .unwrap();

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

    pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "deregister_subnet_orchestrator",
            candid::encode_args((subnet_orchestrator_canister_id, false)).unwrap(),
        )
        .map(|wasm_result| {
            let res: () = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    let all_available_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_available_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(!all_available_subnet_orchestrator.contains(&subnet_orchestrator_canister_id));

    let all_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(all_subnet_orchestrator.contains(&subnet_orchestrator_canister_id));

    pocket_ic
        .update_call(
            platform_canister_id,
            charlie_global_admin,
            "deregister_subnet_orchestrator",
            candid::encode_args((subnet_orchestrator_canister_id, true)).unwrap(),
        )
        .map(|wasm_result| {
            let res: () = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    let all_subnet_orchestrator = pocket_ic
        .query_call(
            platform_canister_id,
            charlie_global_admin,
            "get_all_subnet_orchestrators",
            candid::encode_one(()).unwrap(),
        )
        .map(|wasm_result| {
            let res: Vec<Principal> = match wasm_result {
                WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
                WasmResult::Reject(e) => {
                    panic!("\n call to register_new_subnet_orchestrator failed {e}")
                }
            };
            res
        })
        .unwrap();

    assert!(!all_subnet_orchestrator.contains(&subnet_orchestrator_canister_id));
}
