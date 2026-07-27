use std::collections::{HashMap, HashSet};

use candid::{utils::ArgumentEncoder, CandidType, Deserialize, Principal};
use ic_cdk::api::management_canister::main::CanisterId;
use ic_ledger_types::{AccountIdentifier, BlockIndex, Tokens, DEFAULT_SUBACCOUNT};
use pocket_ic::{
    management_canister::CanisterSettings, PocketIc, PocketIcBuilder, UserError, WasmResult,
};
use shared_utils::{
    canister_specific::{
        platform_orchestrator::types::args::PlatformOrchestratorInitArgs,
        user_info_service::args::UserInfoServiceInitArgs,
        user_post_service::types::args::UserPostServiceInitArgs,
    },
    common::types::{
        known_principal::{KnownPrincipalMap, KnownPrincipalType},
    },
    constant::{GLOBAL_SUPER_ADMIN_USER_ID_V1, NNS_CYCLE_MINTING_CANISTER, NNS_LEDGER_CANISTER_ID},
};

use crate::setup::test_constants::{
    get_global_super_admin_principal_id, v1::CANISTER_INITIAL_CYCLES_FOR_SPAWNING_CANISTERS,
};

#[derive(CandidType)]
struct NnsLedgerCanisterInitPayload {
    minting_account: String,
    initial_values: HashMap<String, Tokens>,
    send_whitelist: HashSet<CanisterId>,
    transfer_fee: Option<Tokens>,
}

#[derive(CandidType)]
struct CyclesMintingCanisterInitPayload {
    ledger_canister_id: CanisterId,
    governance_canister_id: CanisterId,
    minting_account_id: Option<String>,
    last_purged_notification: Option<BlockIndex>,
}

#[derive(CandidType)]
struct AuthorizedSubnetWorks {
    who: Option<Principal>,
    subnets: Vec<Principal>,
}

#[derive(Clone, Copy, Debug)]
pub struct ServiceCanisters {
    pub user_info_service_canister_id: Principal,
    pub user_post_service_canister_id: Principal,
}

pub fn get_new_pocket_ic_env_with_service_canisters_provisioned() -> (PocketIc, ServiceCanisters) {
    let pocket_ic = PocketIcBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .with_system_subnet()
        .build();

    let super_admin = get_global_super_admin_principal_id();

    let user_service_canister = pocket_ic.create_canister_with_settings(
        Some(super_admin),
        Some(CanisterSettings {
            controllers: Some(vec![super_admin]),
            ..Default::default()
        }),
    );

    let user_post_service_canister = pocket_ic.create_canister_with_settings(
        Some(super_admin),
        Some(CanisterSettings {
            controllers: Some(vec![super_admin]),
            ..Default::default()
        }),
    );

    pocket_ic.add_cycles(user_service_canister, 10_000_000_000_000_000);
    pocket_ic.add_cycles(user_post_service_canister, 10_000_000_000_000_000);

    let user_info_service_canister_wasm = include_bytes!(
        "../../../../../../target/wasm32-unknown-unknown/release/user_info_service.wasm.gz"
    );

    let user_post_service_canister_wasm = include_bytes!(
        "../../../../../../target/wasm32-unknown-unknown/release/user_post_service.wasm.gz"
    );

    let user_info_service_canister_init_args = UserInfoServiceInitArgs {
        version: "v1.0.0".into(),
    };

    let user_post_service_canister_init_args = UserPostServiceInitArgs {
        version: "v1.0.0".into(),
    };

    pocket_ic.install_canister(
        user_service_canister,
        user_info_service_canister_wasm.to_vec(),
        candid::encode_one(user_info_service_canister_init_args).unwrap(),
        Some(super_admin),
    );

    pocket_ic.install_canister(
        user_post_service_canister,
        user_post_service_canister_wasm.to_vec(),
        candid::encode_one(user_post_service_canister_init_args).unwrap(),
        Some(super_admin),
    );

    let service_canisters = ServiceCanisters {
        user_info_service_canister_id: user_service_canister,
        user_post_service_canister_id: user_post_service_canister,
    };

    (pocket_ic, service_canisters)
}

pub fn get_new_pocket_ic_env() -> (PocketIc, KnownPrincipalMap) {
    let pocket_ic = PocketIcBuilder::new()
        .with_nns_subnet()
        .with_application_subnet()
        .with_application_subnet()
        .with_system_subnet()
        .build();

    let mut known_principal = KnownPrincipalMap::new();

    let super_admin = Principal::from_text(GLOBAL_SUPER_ADMIN_USER_ID_V1).unwrap();
    known_principal.insert(KnownPrincipalType::UserIdGlobalSuperAdmin, super_admin);

    let application_subnets = pocket_ic.topology().get_app_subnets();

    let platform_canister_id = pocket_ic.create_canister_with_settings(
        Some(super_admin),
        Some(CanisterSettings {
            controllers: Some(vec![super_admin]),
            ..Default::default()
        }),
    );

    known_principal.insert(
        KnownPrincipalType::CanisterIdPlatformOrchestrator,
        platform_canister_id,
    );

    pocket_ic.add_cycles(
        platform_canister_id,
        CANISTER_INITIAL_CYCLES_FOR_SPAWNING_CANISTERS,
    );
    let platform_orchestrator_wasm = include_bytes!(
        "../../../../../../target/wasm32-unknown-unknown/release/platform_orchestrator.wasm.gz"
    );
    let platform_orchestrator_init_args = PlatformOrchestratorInitArgs {
        version: "v1.0.0".into(),
    };
    pocket_ic.install_canister(
        platform_canister_id,
        platform_orchestrator_wasm.into(),
        candid::encode_one(platform_orchestrator_init_args).unwrap(),
        Some(super_admin),
    );
    for _ in 0..30 {
        pocket_ic.tick()
    }
    pocket_ic.add_cycles(platform_canister_id, 10_000_000_000_000_000);

    //Ledger Canister
    let minting_account = AccountIdentifier::new(&super_admin, &DEFAULT_SUBACCOUNT);
    let ledger_canister_wasm = include_bytes!("../../../ledger-canister.wasm");
    let ledger_canister_id = pocket_ic
        .create_canister_with_id(
            Some(super_admin),
            None,
            Principal::from_text(NNS_LEDGER_CANISTER_ID).unwrap(),
        )
        .unwrap();
    let mut initial_balances = HashMap::new();
    initial_balances.insert(
        minting_account.to_string(),
        Tokens::from_e8s(1_000_000_000_000_000),
    );
    let icp_ledger_init_args = NnsLedgerCanisterInitPayload {
        minting_account: minting_account.to_string(),
        initial_values: initial_balances,
        send_whitelist: HashSet::new(),
        transfer_fee: Some(Tokens::from_e8s(10_000)),
    };
    pocket_ic.install_canister(
        ledger_canister_id,
        ledger_canister_wasm.into(),
        candid::encode_one(icp_ledger_init_args).unwrap(),
        Some(super_admin),
    );

    //Cycle Minting Canister
    let cycle_minting_canister_wasm = include_bytes!("../../../cycles-minting-canister.wasm");
    let cycle_minting_canister_id = pocket_ic
        .create_canister_with_id(
            Some(super_admin),
            None,
            Principal::from_text(NNS_CYCLE_MINTING_CANISTER).unwrap(),
        )
        .unwrap();
    pocket_ic.add_cycles(
        cycle_minting_canister_id,
        CANISTER_INITIAL_CYCLES_FOR_SPAWNING_CANISTERS,
    );
    let cycles_minting_canister_init_args = CyclesMintingCanisterInitPayload {
        ledger_canister_id,
        governance_canister_id: CanisterId::anonymous(),
        minting_account_id: Some(minting_account.to_string()),
        last_purged_notification: Some(0),
    };

    pocket_ic.install_canister(
        cycle_minting_canister_id,
        cycle_minting_canister_wasm.into(),
        candid::encode_one(cycles_minting_canister_init_args).unwrap(),
        Some(super_admin),
    );

    let authorized_subnetwork_list_args = AuthorizedSubnetWorks {
        who: Some(platform_canister_id),
        subnets: application_subnets.clone(),
    };
    pocket_ic
        .update_call(
            cycle_minting_canister_id,
            CanisterId::anonymous(),
            "set_authorized_subnetwork_list",
            candid::encode_one(authorized_subnetwork_list_args).unwrap(),
        )
        .unwrap();

    for _ in 0..50 {
        pocket_ic.tick();
    }

    (pocket_ic, known_principal)
}

pub fn execute_query<P: CandidType, R: CandidType + for<'x> Deserialize<'x>>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method_name: &str,
    payload: &P,
) -> R {
    unwrap_res(pic.query_call(
        canister_id,
        sender,
        method_name,
        candid::encode_one(payload).unwrap(),
    ))
}

pub fn execute_query_multi<P: ArgumentEncoder, R: CandidType + for<'x> Deserialize<'x>>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method_name: &str,
    payload: P,
) -> R {
    unwrap_res(pic.query_call(
        canister_id,
        sender,
        method_name,
        candid::encode_args(payload).unwrap(),
    ))
}

pub fn execute_update<P: CandidType, R: CandidType + for<'x> Deserialize<'x>>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method_name: &str,
    payload: &P,
) -> R {
    unwrap_res(pic.update_call(
        canister_id,
        sender,
        method_name,
        candid::encode_one(payload).unwrap(),
    ))
}

pub fn execute_update_multi<P: ArgumentEncoder, R: CandidType + for<'x> Deserialize<'x>>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method: &str,
    payload: P,
) -> R {
    unwrap_res(pic.update_call(
        canister_id,
        sender,
        method,
        candid::encode_args(payload).unwrap(),
    ))
}

pub fn execute_update_no_res<P: CandidType>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method: &str,
    payload: &P,
) {
    let res = pic.update_call(
        canister_id,
        sender,
        method,
        candid::encode_one(payload).unwrap(),
    );
    if let WasmResult::Reject(error) = res.unwrap() {
        panic!("{error}");
    }
}

pub fn execute_update_no_res_multi<P: ArgumentEncoder>(
    pic: &PocketIc,
    sender: Principal,
    canister_id: CanisterId,
    method: &str,
    payload: P,
) {
    let res = pic.update_call(
        canister_id,
        sender,
        method,
        candid::encode_args(payload).unwrap(),
    );
    if let WasmResult::Reject(error) = res.unwrap() {
        panic!("{error}");
    }
}

fn unwrap_res<R: CandidType + for<'x> Deserialize<'x>>(
    response: Result<WasmResult, UserError>,
) -> R {
    match response.unwrap() {
        WasmResult::Reply(bytes) => candid::decode_one(&bytes).unwrap(),
        WasmResult::Reject(error) => panic!("{error}"),
    }
}
