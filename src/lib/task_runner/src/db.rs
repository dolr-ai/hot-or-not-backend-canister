use anyhow::Result;
use candid::Principal;
use sqlx::SqlitePool;

pub const DB_PATH: &str = "src/lib/ic_canister_snapshot/ic_canisters.db";

pub async fn open_pool(db_path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(&format!("sqlite:{db_path}?mode=rwc")).await?;

    // ── PRAGMA connection tuning (the single documented exception to the
    // checked-macro rule) ──────────────────────────────────────────────────────
    //
    // SQLite PRAGMA *assignment* statements and even direct "PRAGMA xxx" getters
    // do not reliably expose column type metadata to sqlx's compile-time probe
    // (reports NULL / () or Option<T>). Table-valued forms (pragma_xxx) also
    // sometimes present the column under a name the probe doesn't see on first
    // pass, or as nullable.
    //
    // Therefore:
    // - The two *setters* ("= WAL", "= NORMAL") are the sole use of the raw
    //   runtime `sqlx::query("...").execute(...)` form in the entire shared DB
    //   surface (documented carve-out in AGENTS.md).
    // - For verification *reads* we use a scalar subquery over the table-valued
    //   pragma function, wrapped in an outer SELECT CAST. This gives sqlx a
    //   concrete typed column expression it can prepare against.
    //
    // All schema (CREATE), DML (po_*, cycle_*, decommission_*, progress,
    // snapshot population, checkpoints, harvest queries, etc.) remain 100%
    // checked query! / query_scalar! / query_as!. See AGENTS.md.
    sqlx::query("PRAGMA journal_mode = WAL")
        .execute(&pool)
        .await?;
    sqlx::query("PRAGMA synchronous = NORMAL")
        .execute(&pool)
        .await?;

    // Checked verification reads (recorded by sqlx prepare).
    // We accept Option<T> because the probe may still mark the scalar subquery
    // result as nullable; we expect() immediately since these are
    // post-setter confirmations and must succeed.
    let _journal: Option<String> = sqlx::query_scalar!(
        r#"SELECT CAST((SELECT journal_mode FROM pragma_journal_mode) AS TEXT)"#
    )
    .fetch_one(&pool)
    .await?;
    let _journal = _journal.expect("journal_mode must be set after PRAGMA");

    let _sync: Option<i64> = sqlx::query_scalar!(
        r#"SELECT CAST((SELECT synchronous FROM pragma_synchronous) AS INTEGER)"#
    )
    .fetch_one(&pool)
    .await?;
    let _sync = _sync.expect("synchronous must be set after PRAGMA");

    ensure_schema(&pool).await?;
    Ok(pool)
}

async fn ensure_schema(pool: &SqlitePool) -> Result<()> {
    // All schema statements use the compile-time checked query! macro.
    // See AGENTS.md (SQL / Database Access section).
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS decommissioned (
            principal         TEXT PRIMARY KEY,
            decommissioned_at TEXT NOT NULL
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS decommission_failures (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            principal TEXT NOT NULL,
            reason    TEXT NOT NULL,
            failed_at TEXT NOT NULL
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS cycle_harvested (
            principal          TEXT PRIMARY KEY,
            pre_balance        INTEGER NOT NULL,
            pre_reserved       INTEGER NOT NULL,
            post_uninstall     INTEGER NOT NULL,
            cycles_transferred INTEGER NOT NULL,
            topped_up          INTEGER NOT NULL DEFAULT 0,
            harvested_at       TEXT NOT NULL
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS cycle_harvest_failures (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            principal TEXT NOT NULL,
            reason    TEXT NOT NULL,
            failed_at TEXT NOT NULL
         )"
    )
    .execute(pool)
    .await?;

    // The po_* tables are populated by the external operator script
    // `scripts/snapshot-po-state.sh` (or manually). We ensure their *structure*
    // here (additively) so that the checked harvest queries (pending_harvests,
    // pending_decommissions, subnet_orchestrators) can be introspected by
    // sqlx during `cargo sqlx prepare`. This is the canonical way to "add the
    // new tables" for the cycle reclaiming workflow.
    // See AGENTS.md (SQL section) — only additive CREATE TABLE IF NOT EXISTS.
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS po_metadata (
            key   TEXT PRIMARY KEY,
            value TEXT
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS po_subnet_orchestrators (
            principal TEXT PRIMARY KEY
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS po_controlled_canisters (
            principal TEXT PRIMARY KEY
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS po_decommission_failed (
            principal TEXT,
            reason    TEXT
         )"
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Return up to `limit` principals from po_controlled_canisters not yet decommissioned.
pub async fn pending_decommissions(pool: &SqlitePool, limit: i64) -> Result<Vec<Principal>> {
    let rows = sqlx::query!(
        "SELECT p.principal
         FROM po_controlled_canisters p
         WHERE NOT EXISTS (
             SELECT 1 FROM decommissioned d WHERE d.principal = p.principal
         )
         LIMIT ?",
        limit
    )
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|r| {
            let s = r.principal.as_deref().unwrap_or_default();
            Principal::from_text(s).map_err(anyhow::Error::from)
        })
        .collect()
}

pub async fn mark_decommissioned(pool: &SqlitePool, principal: &Principal) -> Result<()> {
    let text = principal.to_text();
    sqlx::query!(
        "INSERT OR IGNORE INTO decommissioned (principal) VALUES (?)",
        text
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_failed(pool: &SqlitePool, principal: &Principal, reason: &str) -> Result<()> {
    let text = principal.to_text();
    sqlx::query!(
        "INSERT INTO decommission_failures (principal, reason) VALUES (?, ?)",
        text,
        reason
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn decommission_counts(pool: &SqlitePool) -> Result<(i64, i64)> {
    let done = sqlx::query_scalar!("SELECT COUNT(*) FROM decommissioned")
        .fetch_one(pool)
        .await?;
    let failed = sqlx::query_scalar!("SELECT COUNT(*) FROM decommission_failures")
        .fetch_one(pool)
        .await?;
    Ok((done, failed))
}

/// Return the next release version string ("v1", "v2", ...) and increment the counter.
/// The counter is stored in the `release_counter` table and survives across runs.
pub async fn next_release_version(pool: &SqlitePool) -> Result<String> {
    // DDL and idempotent seed use checked query! (see AGENTS.md).
    sqlx::query!(
        "CREATE TABLE IF NOT EXISTS release_counter (
            id      INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL DEFAULT 0
         )"
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO release_counter (id, version) VALUES (1, 0)
         ON CONFLICT(id) DO NOTHING"
    )
    .execute(pool)
    .await?;

    let version: i64 = sqlx::query_scalar!("SELECT version FROM release_counter WHERE id = 1")
        .fetch_one(pool)
        .await?;

    let next = version + 1;
    sqlx::query!("UPDATE release_counter SET version = ? WHERE id = 1", next)
        .execute(pool)
        .await?;

    Ok(format!("v{next}"))
}

pub async fn subnet_orchestrators(pool: &SqlitePool) -> Result<Vec<Principal>> {
    let rows = sqlx::query!("SELECT principal FROM po_subnet_orchestrators ORDER BY principal")
        .fetch_all(pool)
        .await?;
    rows.iter()
        .map(|r| {
            let s = r.principal.as_deref().unwrap_or_default();
            Principal::from_text(s).map_err(anyhow::Error::from)
        })
        .collect()
}

/// Return up to `limit` principals from po_controlled_canisters not yet cycle-harvested.
pub async fn pending_harvests(pool: &SqlitePool, limit: i64) -> Result<Vec<Principal>> {
    let rows = sqlx::query!(
        "SELECT p.principal
         FROM po_controlled_canisters p
         WHERE NOT EXISTS (
             SELECT 1 FROM cycle_harvested d WHERE d.principal = p.principal
         )
         LIMIT ?",
        limit
    )
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|r| {
            let s = r.principal.as_deref().unwrap_or_default();
            Principal::from_text(s).map_err(anyhow::Error::from)
        })
        .collect()
}

/// Mark a canister as cycle-harvested. Called only after all steps succeed and validations pass.
pub async fn mark_harvested(
    pool: &SqlitePool,
    principal: &Principal,
    pre_balance: u128,
    pre_reserved: u128,
    post_uninstall: u128,
    cycles_transferred: u128,
    topped_up: u128,
) -> Result<()> {
    let text = principal.to_text();
    sqlx::query!(
        "INSERT OR IGNORE INTO cycle_harvested (principal, pre_balance, pre_reserved, post_uninstall, cycles_transferred, topped_up, harvested_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
        text,
        pre_balance as i64,
        pre_reserved as i64,
        post_uninstall as i64,
        cycles_transferred as i64,
        topped_up as i64
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_harvest_failed(
    pool: &SqlitePool,
    principal: &Principal,
    reason: &str,
) -> Result<()> {
    let text = principal.to_text();
    sqlx::query!(
        "INSERT INTO cycle_harvest_failures (principal, reason, failed_at) VALUES (?, ?, datetime('now'))",
        text,
        reason
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn harvest_counts(pool: &SqlitePool) -> Result<(i64, i64)> {
    let done = sqlx::query_scalar!("SELECT COUNT(*) FROM cycle_harvested")
        .fetch_one(pool)
        .await?;
    let failed = sqlx::query_scalar!("SELECT COUNT(*) FROM cycle_harvest_failures")
        .fetch_one(pool)
        .await?;
    Ok((done, failed))
}

/// Total number of principals recorded in po_controlled_canisters (the source list
/// from the last snapshot-po-state.sh run or equivalent).
pub async fn total_controlled_count(pool: &SqlitePool) -> Result<i64> {
    let n = sqlx::query_scalar!("SELECT COUNT(*) FROM po_controlled_canisters")
        .fetch_one(pool)
        .await?;
    Ok(n)
}

/// Number of principals in po_controlled_canisters that have not yet been
/// recorded in cycle_harvested (i.e. still need harvesting).
pub async fn pending_harvest_count(pool: &SqlitePool) -> Result<i64> {
    let n = sqlx::query_scalar!(
        "SELECT COUNT(*)
         FROM po_controlled_canisters p
         WHERE NOT EXISTS (
             SELECT 1 FROM cycle_harvested d WHERE d.principal = p.principal
         )"
    )
    .fetch_one(pool)
    .await?;
    Ok(n)
}

/// Bootstrap test: creates the harvest tables (cycle_harvested, cycle_harvest_failures,
/// decommissioned, etc.) **and** the po_* tables (po_controlled_canisters,
/// po_subnet_orchestrators, po_metadata, po_decommission_failed) via additive
/// `CREATE TABLE IF NOT EXISTS` (using checked query! macros).
///
/// This is the supported way to stand up a complete local DB structure for the
/// cycle reclaiming / decommission logic so that `cargo sqlx prepare` can succeed
/// for all query! sites (including the ones that join against po_* tables).
///
///   cargo test -p task_runner -- --ignored bootstrap_schema --nocapture
///
/// Then (use the exact path printed above):
///   DATABASE_URL=sqlite:<absolute-path-to-ic_canisters.db> cargo sqlx prepare --workspace
///
/// Re-run prepare whenever you add/change a query! macro that touches new columns/tables.
/// All table creation is strictly additive (no DROP). See AGENTS.md.
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "creates tables in the local DB — run once before sqlx prepare"]
    async fn bootstrap_schema() -> Result<()> {
        let root = crate::agent::workspace_root();
        let db_path = root.join(DB_PATH);
        let _pool = open_pool(db_path.to_str().unwrap()).await?;
        println!("✓ Schema ensured at {}", db_path.display());
        println!(
            "  Now run: DATABASE_URL=sqlite:{} cargo sqlx prepare",
            db_path.display()
        );
        Ok(())
    }
}
