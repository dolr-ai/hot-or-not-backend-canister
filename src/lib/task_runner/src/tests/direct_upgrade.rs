/// Direct canister upgrades via controller access (no SNS proposals).
///
/// Since actions_identity is a direct controller of platform_orchestrator,
/// we can call install_code(Upgrade) on the IC management canister — no
/// quill, no neuron voting, no waiting.
///
/// NOTE (post-PO-cleanup): The platform_orchestrator was intentionally reduced
/// to a minimal surface for cycle harvesting. The old PO methods `upload_wasms`,
/// `upgrade_canisters_in_network`, and the wasm storage/provisioning logic were
/// removed. Calls to `po.upload_wasms(...)` (as seen in the old `scripts/deploy-local.sh`
/// and in the raw `.update` sites below) now produce:
///   "Canister has no update method 'upload_wasms'".
///
/// Current local PO validation and setup is done via the cargo test
/// `setup_local_po_and_validate_harvest_methods` (see local_po_setup.rs).
/// For direct PO upgrades on a controller identity: use
///   `dfx canister install platform_orchestrator --mode=upgrade --network ic`
/// (the actions_identity.pem principal must be a controller of the PO).
///
/// The code below is retained for reference / future adaptation of fleet-wide
/// upgrade flows (user_index / individual templates are now driven from other
/// entry points after the PO simplification). It will fail against current PO
/// until the upload/upgrade paths are reimplemented outside PO or the tests
/// are updated to the post-cleanup architecture.
///
/// Run with:
///   cargo test -p task_runner -- --ignored upgrade_po_directly --nocapture
///   cargo test -p task_runner -- --ignored upgrade_po_and_ui_directly --nocapture
///   cargo test -p task_runner -- --ignored upgrade_all_directly --nocapture
///   cargo test -p task_runner -- --ignored deploy_user_info_service_directly --nocapture
use std::process::Command;

use anyhow::{Context, Result};
use candid::{Encode, Principal};
use sha2::{Digest, Sha256};

use crate::{
    agent::{agent_from_pem, workspace_root},
    sns_types::PLATFORM_ORCHESTRATOR_ID,
};

// ── Helper types for PO API calls ─────────────────────────────────────────────

#[derive(candid::CandidType, candid::Deserialize)]
pub enum WasmType {
    IndividualUserWasm,
    PostCacheWasm,
    SubnetOrchestratorWasm,
}

#[derive(candid::CandidType, candid::Deserialize)]
pub struct UpgradeCanisterArg {
    pub version: String,
    pub canister: WasmType,
    pub wasm_blob: Vec<u8>,
}

/// Which canisters to include in the upgrade.
#[derive(Clone, Copy)]
enum ReleaseScope {
    /// platform_orchestrator only
    PoOnly,
    /// platform_orchestrator + user_index fleet
    PoAndUi,
    /// platform_orchestrator + user_index + individual_user_template fleet
    All,
}

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

/// Get a canister's ID from dfx.
fn get_canister_id(name: &str) -> Result<Principal> {
    let root = workspace_root();
    let output = Command::new("dfx")
        .env("DFX_WARNING", "-mainnet_plaintext_identity")
        .args(["canister", "id", name, "--network=ic"])
        .current_dir(&root)
        .output()
        .context("dfx not found")?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Principal::from_text(&text).with_context(|| format!("invalid principal for {name}: {text}"))
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

/// Execute a direct fleet upgrade with the given scope.
async fn upgrade_fleet(scope: ReleaseScope) -> Result<()> {
    let root = workspace_root();
    let pem_path = root.join("actions_identity.pem");

    // ── Step 1: Build canisters ────────────────────────────────────────────────
    let po_wasm_path = build_canister("platform_orchestrator")?;

    let (ui_wasm_path, iu_wasm_path) = match scope {
        ReleaseScope::PoOnly => (None, None),
        ReleaseScope::PoAndUi => (Some(build_canister("user_index")?), None),
        ReleaseScope::All => (
            Some(build_canister("user_index")?),
            Some(build_canister("individual_user_template")?),
        ),
    };

    // Regenerate Candid interfaces.
    match scope {
        ReleaseScope::PoOnly => regenerate_candid(&["platform_orchestrator"])?,
        ReleaseScope::PoAndUi => regenerate_candid(&["platform_orchestrator", "user_index"])?,
        ReleaseScope::All => regenerate_candid(&[
            "platform_orchestrator",
            "user_index",
            "individual_user_template",
        ])?,
    }

    let version = timestamp_version();
    println!("\nVersion: {version}");

    // ── Step 2: Upgrade platform_orchestrator directly via dfx ─────────────────
    // ic-agent's HTTP transport to the management canister (aaaaa-aa) is unreliable.
    // dfx uses a different transport path that works consistently.
    println!("\n==> Upgrading platform_orchestrator...");
    upgrade_canister_via_dfx("platform_orchestrator", &po_wasm_path, &version)?;
    println!("✓ platform_orchestrator upgraded");

    // ── Step 3: Upgrade subnet canisters via PO API (ic-agent works fine for regular canisters) ──
    let agent = agent_from_pem(&pem_path).await?;
    let po = Principal::from_text(PLATFORM_ORCHESTRATOR_ID)?;

    // ── Step 3: Upgrade user_index fleet (if in scope) ─────────────────────────
    if let Some(ui_wasm_path) = ui_wasm_path {
        let ui_wasm = std::fs::read(&ui_wasm_path)?;

        println!("\n==> Uploading user_index wasm to platform_orchestrator...");
        let upload_arg = Encode!(&WasmType::SubnetOrchestratorWasm, &ui_wasm)?;

        agent
            .update(&po, "upload_wasms")
            .with_arg(upload_arg)
            .call_and_wait()
            .await?;

        println!("✓ user_index wasm uploaded");

        let ui_id = get_canister_id("user_index")?;
        println!("==> Triggering user_index fleet upgrade...");
        let trigger_arg = Encode!(&ui_id)?;

        agent
            .update(&po, "upgrade_subnet_orchestrator_canister_with_latest_wasm")
            .with_arg(trigger_arg)
            .call_and_wait()
            .await?;

        println!("✓ user_index fleet upgrade initiated");
    }

    // ── Step 4: Upgrade individual_user_template fleet (if in scope) ───────────
    if let Some(iu_wasm_path) = iu_wasm_path {
        let iu_wasm = std::fs::read(&iu_wasm_path)?;

        println!("\n==> Uploading individual_user_template wasm to platform_orchestrator...");
        let upload_arg = Encode!(&WasmType::IndividualUserWasm, &iu_wasm)?;

        agent
            .update(&po, "upload_wasms")
            .with_arg(upload_arg)
            .call_and_wait()
            .await?;

        println!("✓ individual_user_template wasm uploaded");

        // upgrade_canisters_in_network : (UpgradeCanisterArg) -> (Result_1)
        println!("==> Triggering individual_user_template fleet upgrade...");
        let fleet_arg = Encode!(&UpgradeCanisterArg {
            version: version.clone(),
            canister: WasmType::IndividualUserWasm,
            wasm_blob: iu_wasm,
        })?;

        agent
            .update(&po, "upgrade_canisters_in_network")
            .with_arg(fleet_arg)
            .call_and_wait()
            .await?;

        println!("✓ individual_user_template fleet upgrade initiated");
    }

    let scope_label = match scope {
        ReleaseScope::PoOnly => "platform_orchestrator",
        ReleaseScope::PoAndUi => "platform_orchestrator + user_index",
        ReleaseScope::All => "all canisters",
    };

    println!("\n✓ {scope_label} upgraded to version {version}");
    Ok(())
}

// ── Test entry points ─────────────────────────────────────────────────────────

/// Upgrade platform_orchestrator only.
#[tokio::test]
#[ignore = "upgrades platform_orchestrator on mainnet — run explicitly"]
async fn upgrade_po_directly() -> Result<()> {
    upgrade_fleet(ReleaseScope::PoOnly).await
}

/// Upgrade platform_orchestrator + user_index fleet.
#[tokio::test]
#[ignore = "upgrades platform_orchestrator and user_index on mainnet — run explicitly"]
async fn upgrade_po_and_ui_directly() -> Result<()> {
    upgrade_fleet(ReleaseScope::PoAndUi).await
}

/// Upgrade the full fleet: platform_orchestrator, user_index, individual_user_template.
#[tokio::test]
#[ignore = "upgrades all canisters on mainnet — run explicitly"]
async fn upgrade_all_directly() -> Result<()> {
    upgrade_fleet(ReleaseScope::All).await
}

// ── Deploy / upgrade user_info_service directly (controller identity) ─────

/// Build + deploy (or upgrade) `user_info_service` directly to mainnet using the
/// actions identity (must be a controller of the live canister).
///
/// This follows the same pattern as the fleet upgrades:
/// - regenerates candid (scripts/generate-candid.sh)
/// - builds via `dfx build --network=ic`
/// - installs via `dfx canister install --mode=upgrade --network=ic`
///   (passing the versioned init/upgrade arg that user_info_service expects)
///
/// Run with:
///   cargo test -p task_runner -- --ignored deploy_user_info_service_directly --nocapture
#[tokio::test]
#[ignore = "deploys user_info_service to mainnet — run explicitly"]
async fn deploy_user_info_service_directly() -> Result<()> {
    println!("==> Regenerating Candid for user_info_service ...");
    regenerate_candid(&["user_info_service"])?;

    println!("==> Building user_info_service for mainnet ...");
    let wasm_path = build_canister("user_info_service")?;

    let version = timestamp_version();
    println!("\nVersion: {version}");

    println!("\n==> Deploying user_info_service to mainnet ...");
    // For an existing canister this is an upgrade; for a brand-new id it would be install.
    // user_info_service accepts (record { version : text }) on both init and post_upgrade.
    upgrade_canister_via_dfx("user_info_service", &wasm_path, &version)?;

    println!("\n✓ user_info_service deployed (version {version})");
    Ok(())
}
