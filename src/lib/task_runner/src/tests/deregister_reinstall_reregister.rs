/// Three-step flow to reinstall platform_orchestrator by temporarily acquiring
/// direct controller access.
///
/// STEP 1 — deregister_po (SNS proposal, needs vote):
///   DeregisterDappCanisters for PO, setting new_controllers to
///   [actions_identity, sns_root].  After adoption, actions_identity is a
///   direct controller of PO.
///
/// STEP 2 — reinstall_po_directly (no vote, run after step 1 executes):
///   Calls install_code(Reinstall) on the IC management canister from
///   actions_identity — bypasses the broken pre_upgrade entirely.
///
/// STEP 3 — reregister_po (SNS proposal, needs vote):
///   RegisterDappCanisters to restore PO as an SNS-governed dapp so future
///   UpgradeSnsControlledCanister proposals work again.
///
/// Run order:
///   cargo test -p task_runner -- --ignored deregister_po --nocapture
///   (wait for vote + execution)
///   cargo test -p task_runner -- --ignored reinstall_po_directly --nocapture
///   cargo test -p task_runner -- --ignored reregister_po --nocapture
///   (wait for vote + execution)
use candid::{Encode, Principal};

use crate::{
    agent::{agent_from_pem, workspace_root},
    sns_types::{
        Action, CanisterInstallMode, Command, DeregisterDappCanisters, InstallCodeArgument,
        ManageNeuron, Proposal, RegisterDappCanisters, ACTIONS_PRINCIPAL, NEURON_SUBACCOUNT,
        PLATFORM_ORCHESTRATOR_ID, SNS_GOVERNANCE_ID, SNS_ROOT_ID,
    },
};

const WASM_PATH: &str =
    ".dfx/ic/canisters/platform_orchestrator/platform_orchestrator.wasm.gz";

async fn submit_proposal(action: Action) -> u64 {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");

    let version = format!("v{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
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
    println!("Response: {response_text}");

    let proposal_id = response_text
        .split(": nat64")
        .next()
        .and_then(|s| s.rsplit_once('='))
        .and_then(|(_, num)| num.trim().replace('_', "").parse::<u64>().ok())
        .expect("could not extract proposal ID");

    println!("✓ Proposal #{proposal_id}");
    proposal_id
}

// ── Step 1: deregister PO, add actions_identity as controller ─────────────────

#[tokio::test]
#[ignore = "submits a live SNS proposal — vote in before running step 2"]
async fn deregister_po() {
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();
    let actions = Principal::from_text(ACTIONS_PRINCIPAL).unwrap();
    let sns_root = Principal::from_text(SNS_ROOT_ID).unwrap();

    let id = submit_proposal(Action::DeregisterDappCanisters(DeregisterDappCanisters {
        canister_ids: vec![po],
        new_controllers: vec![actions, sns_root],
    }))
    .await;

    println!(
        "Step 1 done — proposal #{id} must be adopted before running step 2.\n\
         After execution, actions_identity ({ACTIONS_PRINCIPAL}) will be a \
         direct controller of {PLATFORM_ORCHESTRATOR_ID}."
    );
}

// ── Step 2: reinstall PO directly (no vote needed) ───────────────────────────

#[tokio::test]
#[ignore = "calls install_code(Reinstall) directly — only run after step 1 executes"]
async fn reinstall_po_directly() {
    let root = workspace_root();
    let wasm_path = root.join(WASM_PATH);
    let pem_path = root.join("actions_identity.pem");

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

    let agent = agent_from_pem(&pem_path).await.expect("failed to create agent");
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();
    let management = Principal::from_text("aaaaa-aa").unwrap();

    let arg = Encode!(&InstallCodeArgument {
        mode: CanisterInstallMode::Reinstall,
        canister_id: po,
        wasm_module: wasm,
        arg: vec![],
        sender_canister_version: None,
    })
    .expect("failed to encode InstallCodeArgument");

    println!("Calling install_code(Reinstall) on management canister...");
    let response = agent
        .update(&management, "install_code")
        .with_arg(arg)
        .call_and_wait()
        .await
        .expect("install_code call failed");

    let response_text = candid::IDLArgs::from_bytes(&response)
        .map(|a| a.to_string())
        .unwrap_or_else(|_| "(empty — expected for () return type)".to_string());
    println!("Response: {response_text}");

    println!("✓ install_code(Reinstall) completed for {PLATFORM_ORCHESTRATOR_ID}");
    println!("  Run `dfx canister info {PLATFORM_ORCHESTRATOR_ID} --network=ic` to verify the module hash changed.");
    println!("  Then run step 3 (reregister_po) to restore SNS governance.");
}

// ── Step 3: re-register PO with SNS governance ────────────────────────────────

#[tokio::test]
#[ignore = "submits a live SNS proposal — run after step 2 confirms new module hash"]
async fn reregister_po() {
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();

    let id = submit_proposal(Action::RegisterDappCanisters(RegisterDappCanisters {
        canister_ids: vec![po],
    }))
    .await;

    println!(
        "Step 3 done — proposal #{id} must be adopted to restore SNS governance of {PLATFORM_ORCHESTRATOR_ID}."
    );
}
