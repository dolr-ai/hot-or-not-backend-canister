/// Two-step SNS proposal flow to reinstall platform_orchestrator via the IC
/// management canister's install_code, bypassing UpgradeSnsControlledCanister.
///
/// STEP 1 — register_install_code_function:
///   Submits AddGenericNervousSystemFunction (function_id = 5001).
///   Vote this in, then run step 2.
///
/// STEP 2 — execute_po_reinstall_via_generic:
///   Submits ExecuteGenericNervousSystemFunction(5001) with the reinstall payload.
///
/// Run:
///   cargo test -p task_runner -- --ignored register_install_code_function --nocapture
///   cargo test -p task_runner -- --ignored execute_po_reinstall_via_generic --nocapture
use candid::{Encode, Principal};

use crate::{
    agent::{agent_from_pem, workspace_root},
    db::{next_release_version, open_pool},
    sns_types::{
        Action, CanisterInstallMode, ChangeCanisterRequest, Command,
        ExecuteGenericNervousSystemFunction, FunctionType, GenericNervousSystemFunction,
        ManageNeuron, NervousSystemFunction, Proposal, CHANGE_CANISTER_FUNCTION_ID,
        NEURON_SUBACCOUNT, PLATFORM_ORCHESTRATOR_ID, SNS_GOVERNANCE_ID, SNS_ROOT_ID,
        USER_INFO_SERVICE_ID,
    },
};

const WASM_PATH: &str =
    ".dfx/ic/canisters/platform_orchestrator/platform_orchestrator.wasm.gz";

async fn submit_proposal(action: Action) -> u64 {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");
    let db_path = root.join("src/lib/ic_canister_snapshot/ic_canisters.db");

    let pool = open_pool(db_path.to_str().unwrap()).await.unwrap();
    let version = next_release_version(&pool).await.unwrap();
    println!("releasing {version}");

    let agent = agent_from_pem(&pem_path).await.unwrap();
    let governance = Principal::from_text(SNS_GOVERNANCE_ID).unwrap();

    let arg = Encode!(&ManageNeuron {
        subaccount: NEURON_SUBACCOUNT.to_vec(),
        command: Some(Command::MakeProposal(Proposal {
            title: format!("Upgrade platform_orchestrator to {version}"),
            url: "https://yral.com".into(),
            summary: format!("# Upgrade platform_orchestrator to {version}"),
            action: Some(action),
        })),
    })
    .unwrap();

    let response = agent
        .update(&governance, "manage_neuron")
        .with_arg(arg)
        .call_and_wait()
        .await
        .expect("manage_neuron call failed");

    let response_text = candid::IDLArgs::from_bytes(&response)
        .map(|a| a.to_string())
        .unwrap_or_else(|e| format!("<decode failed: {e}>"));

    println!("Raw response: {response_text}");

    let proposal_id = response_text
        .split(": nat64")
        .next()
        .and_then(|s| s.rsplit_once('='))
        .and_then(|(_, num)| num.trim().replace('_', "").parse::<u64>().ok())
        .expect("could not extract proposal ID from response");

    println!("✓ Proposal #{proposal_id} ({version})");
    proposal_id
}

// ── Step 1 ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "submits a live SNS proposal — vote in before running step 2"]
async fn register_install_code_function() {
    let sns_root = Principal::from_text(SNS_ROOT_ID).unwrap();
    let validator = Principal::from_text(USER_INFO_SERVICE_ID).unwrap();

    let id = submit_proposal(Action::AddGenericNervousSystemFunction(
        NervousSystemFunction {
            id: CHANGE_CANISTER_FUNCTION_ID,
            name: format!("change_canister (id={CHANGE_CANISTER_FUNCTION_ID})"),
            description: Some(
                "Calls change_canister on the SNS root canister to reinstall platform_orchestrator"
                    .into(),
            ),
            function_type: Some(FunctionType::GenericNervousSystemFunction(
                GenericNervousSystemFunction {
                    target_canister_id: Some(sns_root),
                    target_method_name: Some("change_canister".into()),
                    validator_canister_id: Some(validator),
                    validator_method_name: Some("validate_install_code".into()),
                },
            )),
        },
    ))
    .await;

    println!("Step 1 done — proposal #{id} must be adopted before running step 2.");
}

// ── Step 2 ────────────────────────────────────────────────────────────────────

#[tokio::test]
#[ignore = "submits a live SNS proposal — only run after step 1 proposal is executed"]
async fn execute_po_reinstall_via_generic() {
    let root = workspace_root();
    let wasm_path = root.join(WASM_PATH);

    println!("Building platform_orchestrator for mainnet...");
    let status = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["build", "platform_orchestrator", "--network=ic"])
        .current_dir(&root)
        .status()
        .expect("dfx not found");
    assert!(status.success(), "dfx build failed");

    let wasm = std::fs::read(&wasm_path)
        .unwrap_or_else(|_| panic!("wasm not found at {}", wasm_path.display()));
    println!("wasm size: {} bytes", wasm.len());

    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();

    let payload = Encode!(&ChangeCanisterRequest {
        mode: CanisterInstallMode::Reinstall,
        canister_id: po,
        wasm_module: wasm,
        arg: vec![],
        stop_before_installing: true,
        chunked_canister_wasm: None,
    })
    .expect("failed to encode ChangeCanisterRequest");
    println!("payload size: {} bytes", payload.len());

    let id = submit_proposal(Action::ExecuteGenericNervousSystemFunction(
        ExecuteGenericNervousSystemFunction {
            function_id: CHANGE_CANISTER_FUNCTION_ID,
            payload,
        },
    ))
    .await;

    println!("Step 2 done — proposal #{id} must be adopted to complete the reinstall.");
}
