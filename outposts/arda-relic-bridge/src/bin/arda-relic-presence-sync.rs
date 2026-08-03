use std::{env, fs, path::PathBuf};

use arda_outpost_protocol::presence::{DegradedReason, SceneState};
use arda_relic_bridge::{
    scene_adapter::{AccessibilityProfile, RelicSceneAdapterState},
    PresenceBridge, ReceivedPresence,
};
use chrono::Utc;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct PresenceEnvelope {
    snapshot_sequence: u64,
    snapshot: arda_outpost_protocol::presence::RuntimePresenceProjection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (endpoint, output) = arguments()?;
    let now = Utc::now();
    let state = match fetch_state(&endpoint, now).await {
        Ok(state) => state,
        Err(error) => {
            eprintln!("relic presence sync degraded: {error}");
            idle_degraded_state()
        }
    };
    write_state(&output, &state)
}

async fn fetch_state(
    endpoint: &str,
    now: chrono::DateTime<Utc>,
) -> Result<RelicSceneAdapterState, Box<dyn std::error::Error>> {
    let response = reqwest::get(endpoint).await?.error_for_status()?;
    let envelope: PresenceEnvelope = response.json().await?;
    let bridge = PresenceBridge::new();
    let cached = bridge.accept(
        ReceivedPresence {
            snapshot_sequence: envelope.snapshot_sequence,
            snapshot: envelope.snapshot,
        },
        now,
    )?;
    Ok(cached.to_relic_scene_state(AccessibilityProfile::default()))
}

fn arguments() -> Result<(String, PathBuf), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let endpoint = args
        .next()
        .unwrap_or_else(|| "http://127.0.0.1:7878/v1/presence/snapshot".into());
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("usage: arda-relic-presence-sync <snapshot-url> <output-path>")?;
    Ok((endpoint, output))
}

fn write_state(
    output: &PathBuf,
    state: &RelicSceneAdapterState,
) -> Result<(), Box<dyn std::error::Error>> {
    let bytes = serde_json::to_vec_pretty(state)?;
    let temporary = output.with_extension("json.new");
    fs::write(&temporary, bytes)?;
    fs::rename(temporary, output)?;
    Ok(())
}

fn idle_degraded_state() -> RelicSceneAdapterState {
    let now = Utc::now();
    RelicSceneAdapterState {
        schema_version: arda_relic_bridge::scene_adapter::RELIC_SCENE_ADAPTER_SCHEMA_VERSION.into(),
        projection_id: "none".into(),
        generated_at: now.to_rfc3339(),
        valid_until: now.to_rfc3339(),
        scene_state: SceneState::IdleDegraded,
        degraded_reason: Some(DegradedReason::Unverifiable),
        status_text: "idle degraded · no valid presence snapshot".into(),
        forms: Vec::new(),
        legend: Vec::new(),
        brightness: 0.55,
        accessibility: AccessibilityProfile::default(),
    }
}
