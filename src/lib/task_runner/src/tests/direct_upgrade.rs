use crate::agent::workspace_root;
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
/// Direct canister upgrades via controller access.
///
/// Since actions_identity is a direct controller of platform_orchestrator,
/// we can call install_code(Upgrade) on the IC management canister — no
/// quill, no neuron voting, no waiting.
///
/// NOTE (post-PO-cleanup): The platform_orchestrator was intentionally reduced
/// to a minimal surface for cycle harvesting. The old PO methods `upload_wasms`,
/// `upgrade_canisters_in_network`, and the wasm storage/provisioning logic were
/// removed. Calls to `po.upload_wasms(...)` now produce:
///   "Canister has no update method 'upload_wasms'".
///
/// Current local PO validation and setup is done via the cargo test
/// `setup_local_po_and_validate_harvest_methods` (see local_setup.rs).
/// For direct PO upgrades on a controller identity: use
///   `dfx canister install platform_orchestrator --mode=upgrade --network ic`
/// (the actions_identity.pem principal must be a controller of the PO).
///
/// Run with:
///   cargo test -p task_runner -- --ignored upgrade_po_directly --nocapture
use std::process::Command;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a canister via dfx and return the path to the gzipped wasm.
fn build_canister(name: &str) -> Result<std::path::PathBuf> {
    let root = workspace_root();
    println!("Building {name} for mainnet...");
    let status = Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["build", name, "--network=ic"])
        .current_dir(&root)
        .status()
        .context("dfx not found")?;
    anyhow::ensure!(status.success(), "dfx build {name} failed");

    let wasm_path = root.join(format!(".dfx/ic/canisters/{name}/{name}.wasm.gz"));
    let size = std::fs::metadata(&wasm_path).map(|m| m.len()).unwrap_or(0);
    println!("  wasm: {} ({} bytes)", wasm_path.display(), size);

    // Print SHA-256 for audit trail.
    let wasm_bytes = std::fs::read(&wasm_path)
        .with_context(|| format!("wasm not found at {}", wasm_path.display()))?;
    let hash = Sha256::digest(&wasm_bytes);
    println!("  sha256: {}", hex::encode(hash));

    Ok(wasm_path)
}

fn timestamp_version() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let days_since_epoch = secs / 86400;
    let year = 1970 + (days_since_epoch / 365);
    format!("{year:04}{:06}", secs % 86400)
}

/// Regenerate Candid interfaces for the given canisters.
fn regenerate_candid(canisters: &[&str]) -> Result<()> {
    let root = workspace_root();
    println!("Regenerating Candid interfaces...");
    let mut args = vec!["scripts/generate-candid.sh"];
    args.extend(canisters.iter().map(|s| *s));
    let status = Command::new("bash")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(&args)
        .current_dir(&root)
        .status()
        .context("generate-candid.sh not found")?;
    anyhow::ensure!(status.success(), "Candid regeneration failed");
    Ok(())
}

/// Upgrade a canister via dfx CLI (reliable for management canister calls).
fn upgrade_canister_via_dfx(
    canister_name: &str,
    _wasm_path: &std::path::Path,
    version: &str,
) -> Result<()> {
    let root = workspace_root();
    println!("  Running dfx canister install --mode=upgrade {canister_name}...");

    let output = Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args([
            "canister",
            "install",
            "--mode=upgrade",
            canister_name,
            "--network=ic",
            "--yes",
            "--argument",
            &format!("(record {{version=\"{version}\"}})"),
        ])
        .current_dir(&root)
        .output()
        .context("dfx not found")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        anyhow::bail!("dfx canister install failed:\nstdout: {stdout}\nstderr: {stderr}");
    }

    println!("  stdout: {}", stdout.trim());
    Ok(())
}

// ── Core upgrade logic ────────────────────────────────────────────────────────

/// Execute a direct platform_orchestrator upgrade.
async fn upgrade_po() -> Result<()> {
    let root = workspace_root();
    let _pem_path = root.join("actions_identity.pem");

    // ── Step 1: Build canister ─────────────────────────────────────────────────
    let po_wasm_path = build_canister("platform_orchestrator")?;

    // Regenerate Candid interface.
    regenerate_candid(&["platform_orchestrator"])?;

    let version = timestamp_version();
    println!("\nVersion: {version}");

    // ── Step 2: Upgrade platform_orchestrator directly via dfx ─────────────────
    // ic-agent's HTTP transport to the management canister (aaaaa-aa) is unreliable.
    // dfx uses a different transport path that works consistently.
    println!("\n==> Upgrading platform_orchestrator...");
    upgrade_canister_via_dfx("platform_orchestrator", &po_wasm_path, &version)?;
    println!("✓ platform_orchestrator upgraded");

    println!("\n✓ platform_orchestrator upgraded to version {version}");
    Ok(())
}

// ── Test entry points ─────────────────────────────────────────────────────────

/// Upgrade platform_orchestrator only.
#[tokio::test]
#[ignore = "upgrades platform_orchestrator on mainnet — run explicitly"]
async fn upgrade_po_directly() -> Result<()> {
    upgrade_po().await
}
