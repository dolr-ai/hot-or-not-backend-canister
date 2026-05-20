use anyhow::Result;
use ic_agent::{
    agent::http_transport::reqwest_transport::ReqwestTransport,
    identity::Secp256k1Identity,
    Agent,
};
use std::path::Path;

pub const IC_URL: &str = "https://ic0.app";

/// Build an ic-agent authenticated with the secp256k1 key in the given PEM file.
pub async fn agent_from_pem(pem_path: impl AsRef<Path>) -> Result<Agent> {
    let identity = Secp256k1Identity::from_pem_file(pem_path)?;
    let transport = ReqwestTransport::create(IC_URL)?;
    let agent = Agent::builder()
        .with_transport(transport)
        .with_identity(identity)
        .build()?;
    // Mainnet — no need to fetch root key
    Ok(agent)
}
