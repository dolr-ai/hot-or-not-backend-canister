use anyhow::Result;
use candid::Principal;
use sqlx::SqlitePool;

pub const DB_PATH: &str = "src/lib/ic_canister_snapshot/ic_canisters.db";

pub async fn open_pool(db_path: &str) -> Result<SqlitePool> {
    let pool = SqlitePool::connect(&format!("sqlite:{db_path}?mode=rwc")).await?;
    sqlx::query("PRAGMA journal_mode=WAL")
        .execute(&pool)
        .await?;
    ensure_schema(&pool).await?;
    Ok(pool)
}

async fn ensure_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS decommissioned (
            principal         TEXT PRIMARY KEY,
            decommissioned_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS decommission_failures (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            principal TEXT NOT NULL,
            reason    TEXT NOT NULL,
            failed_at TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS cycle_harvested (
            principal          TEXT PRIMARY KEY,
            pre_balance        INTEGER NOT NULL,
            pre_reserved       INTEGER NOT NULL,
            post_uninstall     INTEGER NOT NULL,
            cycles_transferred INTEGER NOT NULL,
            topped_up          INTEGER NOT NULL DEFAULT 0,
            harvested_at       TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS cycle_harvest_failures (
            id        INTEGER PRIMARY KEY AUTOINCREMENT,
            principal TEXT NOT NULL,
            reason    TEXT NOT NULL,
            failed_at TEXT NOT NULL
         );",
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
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS release_counter (
            id      INTEGER PRIMARY KEY CHECK (id = 1),
            version INTEGER NOT NULL DEFAULT 0
         )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO release_counter (id, version) VALUES (1, 0)
         ON CONFLICT(id) DO NOTHING",
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
    sqlx::query(
        "INSERT OR IGNORE INTO cycle_harvested (principal, pre_balance, pre_reserved, post_uninstall, cycles_transferred, topped_up, harvested_at) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))",
    )
    .bind(&text)
    .bind(pre_balance as i64)
    .bind(pre_reserved as i64)
    .bind(post_uninstall as i64)
    .bind(cycles_transferred as i64)
    .bind(topped_up as i64)
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
        "INSERT INTO cycle_harvest_failures (principal, reason) VALUES (?, ?)",
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
