use anyhow::Result;
use ic_agent::{identity::Secp256k1Identity, Agent};
use std::path::Path;

pub const IC_URL: &str = "https://ic0.app";

/// Build an ic-agent authenticated with the secp256k1 key in the given PEM file.
pub async fn agent_from_pem(pem_path: impl AsRef<Path>) -> Result<Agent> {
    let identity = Secp256k1Identity::from_pem_file(pem_path)?;
    let agent = Agent::builder()
        .with_url(IC_URL)
        .with_identity(identity)
        .build()?;
    // Mainnet — no need to fetch root key
    Ok(agent)
}

/// Build an ic-agent authenticated with the secp256k1 key in the given PEM file,
/// configured against the *local* dfx replica (http://127.0.0.1:4943).
/// Automatically calls `fetch_root_key` (mandatory for local development replica).
pub async fn local_agent_from_pem(pem_path: impl AsRef<Path>) -> Result<Agent> {
    let identity = Secp256k1Identity::from_pem_file(pem_path)?;
    let agent = Agent::builder()
        .with_url("http://127.0.0.1:4943")
        .with_identity(identity)
        .build()?;
    agent.fetch_root_key().await?;
    Ok(agent)
}

/// Returns the workspace root (3 levels up from this crate's Cargo.toml).
pub fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent().unwrap() // src/lib
        .parent().unwrap() // src
        .parent().unwrap() // workspace root
        .to_path_buf()
}
