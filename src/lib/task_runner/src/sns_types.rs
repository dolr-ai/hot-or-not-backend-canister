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
    AddGenericNervousSystemFunction(NervousSystemFunction),
    ExecuteGenericNervousSystemFunction(ExecuteGenericNervousSystemFunction),
    DeregisterDappCanisters(DeregisterDappCanisters),
    RegisterDappCanisters(RegisterDappCanisters),
}

// ── DeregisterDappCanisters / RegisterDappCanisters ───────────────────────────

#[derive(CandidType, Deserialize)]
pub struct DeregisterDappCanisters {
    pub canister_ids: Vec<Principal>,
    pub new_controllers: Vec<Principal>,
}

#[derive(CandidType, Deserialize)]
pub struct RegisterDappCanisters {
    pub canister_ids: Vec<Principal>,
}

// ── AddGenericNervousSystemFunction ───────────────────────────────────────────

#[derive(CandidType, Deserialize)]
pub struct NervousSystemFunction {
    pub id: u64,
    pub name: String,
    pub description: Option<String>,
    pub function_type: Option<FunctionType>,
}

#[derive(CandidType, Deserialize)]
pub enum FunctionType {
    NativeNervousSystemFunction(()),
    GenericNervousSystemFunction(GenericNervousSystemFunction),
}

#[derive(CandidType, Deserialize)]
pub struct GenericNervousSystemFunction {
    pub target_canister_id: Option<Principal>,
    pub target_method_name: Option<String>,
    pub validator_canister_id: Option<Principal>,
    pub validator_method_name: Option<String>,
}

// ── ExecuteGenericNervousSystemFunction ───────────────────────────────────────

#[derive(CandidType, Deserialize)]
pub struct ExecuteGenericNervousSystemFunction {
    pub function_id: u64,
    pub payload: Vec<u8>,
}

// ── SNS root change_canister types ────────────────────────────────────────────

#[derive(CandidType, Deserialize)]
pub enum CanisterInstallMode {
    #[serde(rename = "install")]
    Install,
    #[serde(rename = "reinstall")]
    Reinstall,
    #[serde(rename = "upgrade")]
    Upgrade,
}

#[derive(CandidType, Deserialize)]
pub struct ChangeCanisterRequest {
    pub mode: CanisterInstallMode,
    pub canister_id: Principal,
    pub wasm_module: Vec<u8>,
    pub arg: Vec<u8>,
    pub stop_before_installing: bool,
    pub chunked_canister_wasm: Option<()>,
}

/// Function ID for the change_canister generic function we register with the SNS.
pub const CHANGE_CANISTER_FUNCTION_ID: u64 = 5001;
/// SNS root canister — controls platform_orchestrator, has change_canister().
pub const SNS_ROOT_ID: &str = "67bll-riaaa-aaaaq-aaauq-cai";
/// user_info_service — controlled by actions identity, hosts validate_install_code.
pub const USER_INFO_SERVICE_ID: &str = "ivkka-7qaaa-aaaas-qbg3q-cai";
/// The actions identity principal (neuron controller + co-controller after deregister).
pub const ACTIONS_PRINCIPAL: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";

// IC management canister install_code mode values (raw i32 for direct calls).
/// 1 = Install (fresh installation)
pub const INSTALL_MODE_INSTALL: i32 = 1;
/// 2 = Reinstall (wipes state, reinstalls)
pub const INSTALL_MODE_REINSTALL: i32 = 2;
/// 3 = Upgrade (preserves state, upgrades wasm)
pub const INSTALL_MODE_UPGRADE: i32 = 3;

// ── IC management canister install_code types ─────────────────────────────────

#[derive(CandidType, Deserialize)]
pub struct InstallCodeArgument {
    pub mode: CanisterInstallMode,
    pub canister_id: Principal,
    pub wasm_module: Vec<u8>,
    pub arg: Vec<u8>,
    pub sender_canister_version: Option<u64>,
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
