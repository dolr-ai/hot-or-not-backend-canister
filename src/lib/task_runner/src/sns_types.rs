use candid::{CandidType, Deserialize, Principal};

// ── SNS governance manage_neuron types ────────────────────────────────────────

#[derive(CandidType, Deserialize)]
pub struct ManageNeuron {
    pub subaccount: Vec<u8>,
    pub command: Option<Command>,
}

#[derive(CandidType, Deserialize)]
pub enum Command {
    MakeProposal(Proposal),
}

#[derive(CandidType, Deserialize)]
pub struct Proposal {
    pub title: String,
    pub url: String,
    pub summary: String,
    pub action: Option<Action>,
}

#[derive(CandidType, Deserialize)]
pub enum Action {
    UpgradeSnsControlledCanister(UpgradeSnsControlledCanister),
}

#[derive(CandidType, Deserialize)]
pub struct UpgradeSnsControlledCanister {
    pub canister_id: Option<Principal>,
    pub new_canister_wasm: Vec<u8>,
    pub canister_upgrade_arg: Option<Vec<u8>>,
    /// Install mode: 1=Install, 2=Reinstall, 3=Upgrade (default)
    pub mode: Option<i32>,
    pub chunked_canister_wasm: Option<()>,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ManageNeuronResponse {
    pub command: Option<CommandResponse>,
}

#[derive(CandidType, Deserialize, Debug)]
pub enum CommandResponse {
    MakeProposal(MakeProposalResponse),
}

#[derive(CandidType, Deserialize, Debug)]
pub struct MakeProposalResponse {
    pub proposal_id: Option<ProposalId>,
}

#[derive(CandidType, Deserialize, Debug)]
pub struct ProposalId {
    pub id: u64,
}

// ── Constants ─────────────────────────────────────────────────────────────────

pub const SNS_GOVERNANCE_ID: &str = "6wcax-haaaa-aaaaq-aaava-cai";
pub const PLATFORM_ORCHESTRATOR_ID: &str = "74zq4-iqaaa-aaaam-ab53a-cai";

/// Neuron subaccount bytes for neuron 4de673e9...
pub const NEURON_SUBACCOUNT: [u8; 32] = [
    0x4d, 0xe6, 0x73, 0xe9, 0xcd, 0x7a, 0x13, 0x39,
    0xaf, 0xea, 0x65, 0x23, 0xa5, 0xf2, 0x27, 0xd2,
    0x5e, 0x9d, 0x73, 0x9f, 0xf5, 0x26, 0x35, 0xac,
    0x86, 0xdb, 0xdb, 0x04, 0x47, 0xae, 0x10, 0x6a,
];
