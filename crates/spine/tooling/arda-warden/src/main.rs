// sigil: REPAIR
//! Arda Warden - Runtime Monitoring
//!
//! Simplified placeholder.

use arda_warden::alerts::post_heartbeat;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .init();

    println!("Arda Warden v0.1.0");
    println!("Status: Ready");

    if let Ok(webhook_url) = std::env::var("WARDEN_WEBHOOK_URL") {
        let node = std::env::var("WARDEN_NODE_NAME").unwrap_or_else(|_| "edge-warden".to_string());
        let role = std::env::var("WARDEN_NODE_ROLE").unwrap_or_else(|_| "warden".to_string());
        let client = reqwest::Client::new();

        if let Err(err) = post_heartbeat(&client, &webhook_url, &node, &role).await {
            tracing::warn!("Failed to post Warden heartbeat: {}", err);
        } else {
            tracing::info!("Warden heartbeat posted to Discord");
        }
    } else {
        tracing::info!("WARDEN_WEBHOOK_URL not set; skipping Discord heartbeat post");
    }

    Ok(())
}
