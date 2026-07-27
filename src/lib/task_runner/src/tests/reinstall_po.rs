use candid::{Encode, Principal};

use crate::{
    agent::{agent_from_pem, workspace_root},
    sns_types::{
        Action, Command, ManageNeuron, Proposal, UpgradeSnsControlledCanister, NEURON_SUBACCOUNT,
        PLATFORM_ORCHESTRATOR_ID, SNS_GOVERNANCE_ID,
    },
};

/// Submit an SNS proposal to REINSTALL platform_orchestrator (mode=2).
/// Reinstall bypasses pre_upgrade, resetting heap state to default.
/// The new stateless wasm has an empty CanisterData and no serialisation overhead.
///
/// Run with: cargo test -p task_runner -- --ignored reinstall_po --nocapture
#[tokio::test]
#[ignore = "submits a live SNS proposal — run explicitly"]
async fn reinstall_po() {
    let root = workspace_root();
    let wasm_path = root.join(".dfx/ic/canisters/platform_orchestrator/platform_orchestrator.wasm.gz");
    let pem_path = root.join("actions_identity.pem");

    // Always build the latest wasm before submitting.
    println!("Building platform_orchestrator for mainnet...");
    let build_status = std::process::Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["build", "platform_orchestrator", "--network=ic"])
        .current_dir(&root)
        .status()
        .expect("dfx not found");
    assert!(build_status.success(), "dfx build platform_orchestrator failed");

    let wasm = std::fs::read(&wasm_path)
        .unwrap_or_else(|_| panic!("wasm not found at {}", wasm_path.display()));

    println!("wasm size: {} bytes", wasm.len());

    let version = format!("v{}", chrono::Utc::now().format("%Y%m%d%H%M%S"));
    println!("releasing {version}");

    let agent = agent_from_pem(&pem_path).await.expect("failed to create agent");
    let governance = Principal::from_text(SNS_GOVERNANCE_ID).unwrap();
    let po_canister = Principal::from_text(PLATFORM_ORCHESTRATOR_ID).unwrap();

    let arg = Encode!(&ManageNeuron {
        subaccount: NEURON_SUBACCOUNT.to_vec(),
        command: Some(Command::MakeProposal(Proposal {
            title: format!("Upgrade platform_orchestrator to {version}"),
            url: "https://yral.com".into(),
            summary: format!("# Upgrade platform_orchestrator to {version}"),
            action: Some(Action::UpgradeSnsControlledCanister(
                UpgradeSnsControlledCanister {
                    canister_id: Some(po_canister),
                    new_canister_wasm: wasm,
                    canister_upgrade_arg: None,
                    mode: Some(2), // 2 = Reinstall
                    chunked_canister_wasm: None,
                },
            )),
        })),
    })
    .unwrap();

    let response = agent
        .update(&governance, "manage_neuron")
        .with_arg(arg)
        .call_and_wait()
        .await
        .expect("manage_neuron call failed");

    // Decode the response as raw IDL text first to see the actual structure.
    let response_text = candid::IDLArgs::from_bytes(&response)
        .map(|a| a.to_string())
        .unwrap_or_else(|e| format!("<decode failed: {e}>"));
    println!("Raw response: {response_text}");

    // Extract proposal ID: the response contains `= NNN : nat64` for the proposal id field.
    let proposal_id = response_text
        .split(": nat64")
        .next()
        .and_then(|s| s.rsplit_once('='))
        .and_then(|(_, num)| num.trim().replace('_', "").parse::<u64>().ok());

    if let Some(id) = proposal_id {
        println!("✓ Proposal submitted: #{id}  ({version})");
    } else {
        println!("Response: {response_text}");
        panic!("could not extract proposal ID from response — check SNS dashboard for {version}");
    }
}
