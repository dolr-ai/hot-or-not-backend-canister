use pocket_ic::WasmResult;
use shared_utils::{
    common::types::known_principal::KnownPrincipalType,
    constant::{
        SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD, SUBNET_ORCHESTRATOR_CANISTER_INITIAL_CYCLES,
    },
};
use test_utils::setup::env::pocket_ic_env::get_new_pocket_ic_env;

#[test]
pub fn recharge_subnet_orchestrator_test() {
    let (pocket_ic, known_principal_map) = get_new_pocket_ic_env();

    let platform_orchestrator_canister_id = known_principal_map
        .get(&KnownPrincipalType::CanisterIdPlatformOrchestrator)
        .copied()
        .unwrap();

    let global_admin_principal = known_principal_map
        .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
        .copied()
        .unwrap();

    let subnet_orchestrator_canister_id = pocket_ic.create_canister();

    pocket_ic
        .set_controllers(
            subnet_orchestrator_canister_id,
            None,
            vec![platform_orchestrator_canister_id],
        )
        .unwrap();

    pocket_ic
        .update_call(
            platform_orchestrator_canister_id,
            global_admin_principal,
            "register_new_subnet_orchestrator",
            candid::encode_args((subnet_orchestrator_canister_id, true)).unwrap(),
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
        .unwrap()
        .unwrap();

    let mut message_ids = vec![];
    for _ in 0..10 {
        let message_id = pocket_ic
            .submit_call(
                platform_orchestrator_canister_id,
                subnet_orchestrator_canister_id,
                "recharge_subnet_orchestrator",
                candid::encode_one(()).unwrap(),
            )
            .unwrap();

        message_ids.push(message_id);
    }

    for message in message_ids {
        pocket_ic.await_call(message).unwrap();
    }

    pocket_ic.tick();

    let subnet_orchestrator_cycle_balance =
        pocket_ic.cycle_balance(subnet_orchestrator_canister_id);

    assert!(subnet_orchestrator_cycle_balance >= SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD);
    assert!(
        subnet_orchestrator_cycle_balance
            < SUBNET_ORCHESTRATOR_CANISTER_INITIAL_CYCLES
                + SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD
    );
}
