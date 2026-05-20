use candid::{Decode, Encode, Principal};

use crate::{
    agent::agent_from_pem,
    db::{next_release_version, open_pool, DB_PATH},
    sns_types::{
        Action, Command, ManageNeuron, ManageNeuronResponse, Proposal,
        UpgradeSnsControlledCanister, NEURON_SUBACCOUNT, PLATFORM_ORCHESTRATOR_ID,
        SNS_GOVERNANCE_ID,
    },
};

const PEM_PATH: &str = "actions_identity.pem";
const WASM_PATH: &str =
    ".dfx/ic/canisters/platform_orchestrator/platform_orchestrator.wasm.gz";

/// Submit an SNS proposal to REINSTALL platform_orchestrator (mode=2).
/// Reinstall bypasses pre_upgrade, resetting heap state to default.
/// The new stateless wasm has an empty CanisterData and no serialisation overhead.
///
/// Run with: cargo test -p task_runner -- --ignored reinstall_po --nocapture
#[tokio::test]
#[ignore = "submits a live SNS proposal — run explicitly"]
async fn reinstall_po() {
    let wasm = std::fs::read(WASM_PATH)
        .unwrap_or_else(|_| panic!("wasm not found at {WASM_PATH} — run `dfx build platform_orchestrator --network=ic` first"));

    println!("wasm size: {} bytes", wasm.len());

    let pool = open_pool(DB_PATH).await.expect("failed to open db");
    let version = next_release_version(&pool).await.expect("failed to get version");
    println!("releasing {version}");

    let agent = agent_from_pem(PEM_PATH).await.expect("failed to create agent");
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

    let decoded = Decode!(&response, ManageNeuronResponse).expect("failed to decode response");
    println!("Response: {decoded:?}");

    match decoded.command {
        Some(crate::sns_types::CommandResponse::MakeProposal(p)) => {
            let id = p.proposal_id.expect("no proposal_id in response").id;
            println!("✓ Proposal submitted: #{id}  ({version})");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}
