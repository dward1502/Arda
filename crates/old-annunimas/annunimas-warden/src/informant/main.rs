// sigil: REPAIR
//! Informant - Sidecar monitoring agent
//!
//! Simplified placeholder.

use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    println!("Informant v0.1.0");
    println!("Status: Ready (placeholder)");

    Ok(())
}
