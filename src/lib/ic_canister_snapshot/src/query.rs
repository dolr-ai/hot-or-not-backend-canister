/// Query helpers for the IC canister snapshot database.
///
/// Call these from any test after `populate_db` has been run at least once.
///
/// All functions are async and take `&sqlx::SqlitePool`.
use sqlx::SqlitePool;

use crate::fetch::{db_path, open_and_init_db};

// ─── open helper ──────────────────────────────────────────────────────────────

/// Open the snapshot database.  Pass `Some(path)` to override the default path
/// or the `IC_CANISTER_DB_PATH` env var.
///
/// Async: returns a SqlitePool.
pub async fn open_db(path: Option<&str>) -> SqlitePool {
    let resolved = path.map(|s| s.to_string()).unwrap_or_else(db_path);
    open_and_init_db(&resolved).await
}

// ─── query functions ──────────────────────────────────────────────────────────

/// Return the canister IDs of every canister whose controller list contains
/// `principal_id` (text form, e.g. `"rrkah-fqaaa-aaaaa-aaaaq-cai"`).
pub async fn find_canisters_by_controller(pool: &SqlitePool, principal_id: &str) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT canister_id FROM controllers
         WHERE controller = ?
         ORDER BY canister_id",
        principal_id
    )
    .fetch_all(pool)
    .await
    .expect("find_canisters_by_controller query failed")
}

/// Return all controllers for the given canister ID.
pub async fn get_controllers_for_canister(pool: &SqlitePool, canister_id: &str) -> Vec<String> {
    sqlx::query_scalar!(
        "SELECT controller FROM controllers
         WHERE canister_id = ?
         ORDER BY controller",
        canister_id
    )
    .fetch_all(pool)
    .await
    .expect("get_controllers_for_canister query failed")
}

// ─── metadata struct ──────────────────────────────────────────────────────────

/// Metadata for a single canister row.
#[derive(Debug, sqlx::FromRow)]
pub struct CanisterInfo {
    pub canister_id: String,
    pub api_id: Option<i64>,
    pub subnet_id: Option<String>,
    pub module_hash: Option<String>,
    pub language: Option<String>,
    pub updated_at: Option<String>,
}

/// Return metadata for a canister, or `None` if it is not in the database.
pub async fn get_canister_info(pool: &SqlitePool, canister_id: &str) -> Option<CanisterInfo> {
    // The "canister_id!" override tells sqlx's checked macro that this column is
    // NOT NULL (PRIMARY KEY in the schema). This is required because the
    // compile-time probe (especially when .sqlx/ metadata is being refreshed or
    // when DATABASE_URL is not set for a later cargo check) can report TEXT PK
    // columns as nullable. The ! suffix forces the correct non-Option type.
    sqlx::query_as!(
        CanisterInfo,
        r#"SELECT canister_id as "canister_id!",
                  api_id,
                  subnet_id,
                  module_hash,
                  language,
                  updated_at
           FROM canisters WHERE canister_id = ?"#,
        canister_id
    )
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
}

// ─── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Demonstrate finding canisters by controller.
    ///
    /// Uses the NNS governance canister as a well-known example controller.
    ///
    /// Run with:
    ///   cargo test -p ic_canister_snapshot test_find_canisters_by_controller -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_find_canisters_by_controller() {
        let pool = open_db(None).await;

        // NNS governance canister controls many NNS-related canisters.
        let principal = "rrkah-fqaaa-aaaaa-aaaaq-cai";
        let canisters = find_canisters_by_controller(&pool, principal).await;

        println!(
            "[test] {} controls {} canister(s)",
            principal,
            canisters.len()
        );
        for id in &canisters {
            let controllers = get_controllers_for_canister(&pool, id).await;
            println!("  {} => controllers: {:?}", id, controllers);
        }

        assert!(
            !canisters.is_empty(),
            "expected at least one canister controlled by {}",
            principal
        );
    }

    /// Show all controllers of a specific canister.
    ///
    /// Uses the NNS ICP ledger canister as an example.
    ///
    /// Run with:
    ///   cargo test -p ic_canister_snapshot test_get_controllers_for_canister -- --ignored
    #[tokio::test]
    #[ignore]
    async fn test_get_controllers_for_canister() {
        let pool = open_db(None).await;

        let canister_id = "ryjl3-tyaaa-aaaaa-aaaba-cai"; // NNS ICP ledger
        let controllers = get_controllers_for_canister(&pool, canister_id).await;
        let info = get_canister_info(&pool, canister_id).await;

        println!("[test] Controllers of {}: {:?}", canister_id, controllers);
        println!("[test] Canister info: {:#?}", info);

        assert!(
            !controllers.is_empty(),
            "expected controllers for {}",
            canister_id
        );
    }
}
