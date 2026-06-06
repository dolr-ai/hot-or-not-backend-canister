use candid::{Encode, Principal};
use pocket_ic::WasmResult;
use shared_utils::{
    canister_specific::platform_orchestrator::types::args::PlatformOrchestratorInitArgs,
    common::types::known_principal::KnownPrincipalType, constant::RECLAIM_CANISTER_PRINCIPAL_ID,
};
use test_utils::setup::{
    env::pocket_ic_env::get_new_pocket_ic_env, test_constants::get_mock_user_charlie_principal_id,
};

#[test]
fn all_canister_snapshot_tests() {
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

    for _i in 0..50 {
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
    for _i in 0..20 {
        pocket_ic.tick();
    }

    let reclaim_principal_id = Principal::from_text(RECLAIM_CANISTER_PRINCIPAL_ID).unwrap();

    let response = pocket_ic
        .update_call(
            platform_canister_id,
            reclaim_principal_id,
            "save_snapshot_json",
            Encode!().unwrap(),
        )
        .unwrap();
    let snapshot_len: u32 = match response {
        WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
        _ => panic!("\n🛑 save_snapshot_json failed for platform orchestrator\n"),
    };

    let mut data: Vec<u8> = Vec::new();
    let mut offset: u64 = 0;
    let chunk_size = 100_000;

    while offset < snapshot_len as u64 {
        let length = std::cmp::min(chunk_size, snapshot_len as u64 - offset);

        let response = pocket_ic
            .query_call(
                platform_canister_id,
                reclaim_principal_id,
                "download_snapshot",
                Encode!(&offset, &length).unwrap(),
            )
            .unwrap();
        let chunk: Vec<u8> = match response {
            WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
            _ => panic!("\n🛑 download_snapshot platform orchestrator failed for {offset}\n"),
        };

        data.extend_from_slice(&chunk);
        offset += length;
    }

    println!("data: {}", std::str::from_utf8(&data).unwrap());

    let snapshot_len = data.len() as u64;
    let mut offset: u64 = 0;
    let chunk_size: u64 = 100_000;

    while offset < snapshot_len {
        let length = std::cmp::min(chunk_size, snapshot_len - offset);
        let chunk = &data[(offset as usize)..((offset + length) as usize)];

        if pocket_ic
            .update_call(
                platform_canister_id,
                reclaim_principal_id,
                "receive_and_save_snaphot",
                Encode!(&offset, &chunk).unwrap(),
            )
            .is_err()
        {
            panic!("\n🛑 receive_and_save_snaphot failed for platform orchestrator\n")
        };
        offset += length;
    }

    pocket_ic
        .update_call(
            platform_canister_id,
            reclaim_principal_id,
            "load_snapshot",
            Encode!(&()).unwrap(),
        )
        .unwrap();

    let response = pocket_ic
        .update_call(
            user_index_canister_id,
            reclaim_principal_id,
            "save_snapshot_json",
            Encode!().unwrap(),
        )
        .unwrap();
    let snapshot_len: u32 = match response {
        WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
        _ => panic!("\n🛑 save_snapshot failed for user index\n"),
    };

    let mut data: Vec<u8> = Vec::new();
    let mut offset: u64 = 0;
    let chunk_size = 100_000;

    while offset < snapshot_len as u64 {
        let length = std::cmp::min(chunk_size, snapshot_len as u64 - offset);

        let response = pocket_ic
            .query_call(
                user_index_canister_id,
                reclaim_principal_id,
                "download_snapshot",
                Encode!(&offset, &length).unwrap(),
            )
            .unwrap();
        let chunk: Vec<u8> = match response {
            WasmResult::Reply(payload) => candid::decode_one(&payload).unwrap(),
            _ => panic!("\n🛑 download_snapshot failed for user index\n"),
        };

        data.extend_from_slice(&chunk);
        offset += length;
    }

    println!("data: {}", std::str::from_utf8(&data).unwrap());

    let snapshot_len = data.len() as u64;
    let mut offset: u64 = 0;
    let chunk_size: u64 = 100_000;

    while offset < snapshot_len {
        let length = std::cmp::min(chunk_size, snapshot_len - offset);
        let chunk = &data[(offset as usize)..((offset + length) as usize)];

        if pocket_ic
            .update_call(
                user_index_canister_id,
                reclaim_principal_id,
                "receive_and_save_snaphot",
                Encode!(&offset, &chunk).unwrap(),
            )
            .is_err()
        {
            panic!("\n🛑receive_and_save_snaphot failed for user index\n")
        };
        offset += length;
    }

    if pocket_ic
        .update_call(
            user_index_canister_id,
            reclaim_principal_id,
            "load_snapshot",
            Encode!(&()).unwrap(),
        )
        .is_err()
    {
        panic!("\n🛑Load snapshot failed for user index\n")
    };
}

const PF_ORCH_WASM_PATH: &str =
    "../../../target/wasm32-unknown-unknown/release/platform_orchestrator.wasm.gz";

fn pf_orch_canister_wasm() -> Vec<u8> {
    std::fs::read(PF_ORCH_WASM_PATH).unwrap()
}
