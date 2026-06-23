use crate::common::types::known_principal::{KnownPrincipalMap, KnownPrincipalType};
use candid::Principal;
pub const EMPTY_CANISTER_RECHARGE_AMOUNT: u128 = 700_000_000_000; //0.7T Cycles

pub const SUBNET_ORCHESTRATOR_CANISTER_INITIAL_CYCLES: u128 = 2_500_000_000_000_000; //2.5kT Cycles
pub const SUBNET_ORCHESTRATOR_CANISTER_CYCLES_THRESHOLD: u128 = 1_000_000_000_000_000; //1kT Cycles
pub const USER_SNS_CANISTER_INITIAL_CYCLES: u128 = 500_000_000_000; //0.5T Cycles
pub const PAGE_SIZE_RECHARGE_DIVIDER: u128 = 500; // 500 pages (recharge by page_size/page_size_recharge_divider * recharge_amount)

pub const MAX_USERS_IN_FOLLOWER_FOLLOWING_LIST: u64 = 10000;
pub const MAX_POSTS_IN_ONE_REQUEST: u64 = 100;
pub const HOME_FEED_DIFFERENCE_TO_INITIATE_SYNCHRONISATION: u64 = 100;
pub const HOT_OR_NOT_FEED_DIFFERENCE_TO_INITIATE_SYNCHRONISATION: u64 = 100;

//  Cycles costs of IC
pub const DEFAULT_FREEZING_THRESHOLD: u128 = 30; // 30 days
pub const BASE_COST_FOR_INGRESS_MESSAGE: u128 = 1_200_000; //1.2M cycles
pub const BASE_COST_FOR_EXECUTION: u128 = 5_000_000; // 5M cycles
pub const COST_PER_BYTE_FOR_INGRESS_MESSAGE: u128 = 2_000; //2k cycles
pub const COST_PER_BILLION_INSTRUCTION_EXECUTED: u128 = 1_000_000_000; //1B cycles for per 1B instructions
pub const ASSUMED_NUMBER_OF_INGRESS_CALL_PER_DAY: u128 = 10;
pub const ASSUMED_BYTES_PER_INGRESS_CALL: u128 = 100; // 100 bytes
pub const ASSUMED_NUMBER_OF_INSTRUCTIONS_PER_INGRESS_CALL: u128 = 5_000_000_000; //5B instructions (no matter the number of instructions executed 5B is reserved)
pub const RESERVED_NUMBER_OF_INSTRUCTIONS_FOR_INSTALL_CODE: u128 = 300_000_000_000; //300B instructions
pub const THRESHOLD_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING: u128 = 1;
pub const MAX_NUMBER_OF_DAYS_TO_KEEP_CANISTER_RUNNING: u128 = 7;

pub const SNS_TOKEN_GOVERNANCE_MODULE_HASH: &'static str =
    "51fd3d1a529f3f7bad808b19074e761ce3538282ac8189bd7067b4156360c279";
pub const SNS_TOKEN_LEDGER_MODULE_HASH: &'static str =
    "3d808fa63a3d8ebd4510c0400aa078e99a31afaa0515f0b68778f929ce4b2a46";
pub const SNS_TOKEN_ROOT_MODULE_HASH: &'static str =
    "431cb333feb3f762f742b0dea58745633a2a2ca41075e9933183d850b4ddb259";
pub const SNS_TOKEN_SWAP_MODULE_HASH: &'static str =
    "8313ac22d2ef0a0c1290a85b47f235cfa24ca2c96d095b8dbed5502483b9cd18";
pub const SNS_TOKEN_INDEX_MODULE_HASH: &'static str =
    "67b5f0bf128e801adf4a959ea26c3c9ca0cd399940e169a26a2eb237899a94dd";
pub const SNS_TOKEN_ARCHIVE_MODULE_HASH: &'static str =
    "317771544f0e828a60ad6efc97694c425c169c4d75d911ba592546912dba3116";

// 1 DOLLR = 100 GDOLLR
pub const DOLLR_TO_GDOLLR: u64 = 100;
// 1 DOLLR = 1e8 "e8s"
// => 1 GDOLLR = 1e6 "e8s"
pub const GDOLLR_TO_E8S: u64 = 1e6 as u64;

// * Important Principal IDs
pub const GOVERNANCE_CANISTER_ID: &str = "6wcax-haaaa-aaaaq-aaava-cai";
pub const NNS_CYCLE_MINTING_CANISTER: &str = "rkp4c-7iaaa-aaaaa-aaaca-cai";
pub const NNS_LEDGER_CANISTER_ID: &str = "ryjl3-tyaaa-aaaaa-aaaba-cai";
pub const SNS_WASM_W_PRINCIPAL_ID: &'static str = "qaa6y-5yaaa-aaaaa-aaafa-cai";
pub const GLOBAL_SUPER_ADMIN_USER_ID: &str =
    "7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae";

pub const GLOBAL_SUPER_ADMIN_USER_ID_V1: &str =
    "zg7n3-345by-nqf6o-3moz4-iwxql-l6gko-jqdz2-56juu-ja332-unymr-fqe";
pub const RECLAIM_CANISTER_PRINCIPAL_ID: &str =
    "7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae";

pub fn get_global_super_admin_principal_id_v1(
    well_known_canisters: KnownPrincipalMap,
) -> Principal {
    match option_env!("DFX_NETWORK") {
        Some("ic") => {
            Principal::from_text("7gaq2-4kttl-vtbt4-oo47w-igteo-cpk2k-57h3p-yioqe-wkawi-wz45g-jae")
                .unwrap()
        }
        _ => *well_known_canisters
            .get(&KnownPrincipalType::UserIdGlobalSuperAdmin)
            .expect("USER ID for global super admin not found"),
    }
}
