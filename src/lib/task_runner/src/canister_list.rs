use anyhow::{Context, Result};
use candid::Principal;
use std::path::Path;

/// Path to the CSV file listing all canisters controlled by platform_orchestrator.
/// One principal per line, with a `principal` header row.
pub const PRINCIPAL_CSV: &str = "src/canister/platform_orchestrator/principal.csv";

/// Read all PO-controlled canister principals from `principal.csv`.
/// Skips blank lines and the `principal` header row. Lines that fail to parse
/// as a Principal are skipped with a warning.
pub fn read_po_controlled_canisters(csv_path: &Path) -> Result<Vec<Principal>> {
    let content = std::fs::read_to_string(csv_path)
        .with_context(|| format!("failed to read {}", csv_path.display()))?;
    let mut principals = Vec::new();
    for (lineno, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "principal" {
            continue;
        }
        match Principal::from_text(trimmed) {
            Ok(p) => principals.push(p),
            Err(e) => {
                eprintln!(
                    "  ⚠ [principal.csv line {}] skipping invalid principal '{}': {}",
                    lineno + 1,
                    trimmed,
                    e
                );
            }
        }
    }
    Ok(principals)
}

/// Returns the first N canisters from the principal.csv list.
/// Used by harvest_single_canister when HARVEST_CANISTER_ID is not set.
pub fn read_canisters(csv_path: &Path, limit: usize) -> Result<Vec<Principal>> {
    let all = read_po_controlled_canisters(csv_path)?;
    Ok(all.into_iter().take(limit).collect())
}