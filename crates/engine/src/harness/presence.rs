//! Presence projection publish surface for arda-engine.
//!
//! Exposes `/v1/presence/snapshot` (JSON) and `/v1/presence/events` (SSE).
//!
//! Loopback callers receive the sanitized projection immediately.
//! Remote CITADEL access requires an enrolled outpost identity and the
//! `presence.read` capability.

use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use arda_aule::presence_projection::{build_presence_projection, ProjectionInputs};
use arda_outpost_protocol::presence::RuntimePresenceProjection;
use axum::http::{HeaderMap, StatusCode};
use axum::response::Sse;
use axum::{
    extract::{ConnectInfo, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use reqwest::header::AUTHORIZATION;
use serde::Serialize;
use tokio::{sync::RwLock, time::interval};
use tracing::warn;

use crate::harness::HarnessState;

#[derive(Debug, Clone)]
pub struct HarnessPresenceState {
    sequence: Arc<AtomicUsize>,
    inputs: Arc<RwLock<ProjectionInputs>>,
}

impl HarnessPresenceState {
    pub async fn update_inputs(&self, inputs: ProjectionInputs) {
        *self.inputs.write().await = inputs;
    }

    pub async fn read_inputs(&self) -> ProjectionInputs {
        self.inputs.read().await.clone()
    }
}

impl Default for HarnessPresenceState {
    fn default() -> Self {
        Self {
            sequence: Arc::new(AtomicUsize::new(0)),
            inputs: Arc::new(RwLock::new(ProjectionInputs::empty())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PresenceSnapshotResponse {
    pub schema_version: &'static str,
    pub snapshot_sequence: usize,
    pub generated_at: String,
    pub source_receipt_refs: Vec<String>,
    pub snapshot: RuntimePresenceProjection,
}

pub struct PresenceRouter;

impl PresenceRouter {
    pub fn routes() -> Vec<&'static str> {
        vec!["/v1/presence/snapshot", "/v1/presence/events"]
    }

    pub fn router(_state: HarnessPresenceState) -> Router<HarnessState> {
        Router::new()
            .route("/v1/presence/snapshot", get(presence_snapshot))
            .route("/v1/presence/events", get(presence_events))
    }
}

pub async fn presence_snapshot(
    State(harness): State<HarnessState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_authorized(&addr, &headers, "presence.read") {
        return not_authorized().into_response();
    }

    let projector = build_presence_projection(harness.presence_inputs.read_inputs().await);
    let snapshot_sequence = next_sequence(&harness.presence_inputs).await;

    let response = PresenceSnapshotResponse {
        schema_version: "arda.harness.presence.v1",
        snapshot_sequence,
        generated_at: projector.generated_at.to_rfc3339(),
        source_receipt_refs: projector.source_receipt_refs.clone(),
        snapshot: projector,
    };
    (StatusCode::OK, axum::Json(response)).into_response()
}

pub async fn presence_events(
    State(harness): State<HarnessState>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !is_authorized(&addr, &headers, "presence.read") {
        return not_authorized().into_response();
    }

    let mut ticker = interval(STREAM_TICK);
    let mut sequence = next_sequence(&harness.presence_inputs).await;

    let stream = async_stream::stream! {
        loop {
            let _ = ticker.tick().await;
            let projector = build_presence_projection(harness.presence_inputs.read_inputs().await);
            sequence += 1;

            let event = match axum::response::sse::Event::default()
                .event("presence")
                .json_data(PresenceEventEnvelope {
                    schema_version: "arda.harness.presence.v1",
                    snapshot_sequence: sequence,
                    generated_at: projector.generated_at.to_rfc3339(),
                    source_receipt_refs: projector.source_receipt_refs.clone(),
                    snapshot: projector,
                }) {
                Ok(ev) => ev,
                Err(error) => {
                    warn!("harness: presence event serialization failed: {error}");
                    continue;
                }
            };
            yield Ok::<axum::response::sse::Event, axum::Error>(event);
        }
    };

    Sse::new(stream).into_response()
}

#[derive(Debug, Clone, Serialize)]
struct PresenceEventEnvelope {
    schema_version: &'static str,
    snapshot_sequence: usize,
    generated_at: String,
    source_receipt_refs: Vec<String>,
    snapshot: RuntimePresenceProjection,
}

#[derive(Debug, Clone)]
struct OutpostEnrollment {
    outpost_id: &'static str,
    granted_capabilities: &'static [(&'static str, &'static str)],
    allowed_ips: &'static [&'static str],
}

static ENROLLED_CITADEL: OutpostEnrollment = OutpostEnrollment {
    outpost_id: "citadel-outpost-1",
    granted_capabilities: &[("presence", "read")],
    allowed_ips: &["127.0.0.1"],
};

fn is_authorized(addr: &std::net::SocketAddr, headers: &HeaderMap, required: &str) -> bool {
    // A loopback reverse proxy can carry a remote caller. Once it supplies
    // forwarding metadata, require the enrolled outpost capability rather
    // than treating the proxy socket itself as local authority.
    if addr.ip().is_loopback() && !headers.contains_key("x-forwarded-for") {
        return true;
    }

    let capability = match parse_bearer_capability(headers) {
        Some(token) => token,
        None => return false,
    };

    let parsed = parse_capability(&capability);
    verify_enrolled_outpost(addr, parsed, required)
}

fn parse_bearer_capability(headers: &HeaderMap) -> Option<String> {
    let authorization = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = authorization.split_once(' ')?;

    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }

    Some(token.to_string())
}

fn parse_capability(value: &str) -> Option<(&str, &str)> {
    let (namespace, action) = value.split_once(':')?;

    if namespace.is_empty() || action.is_empty() {
        return None;
    }

    Some((namespace, action))
}

fn verify_enrolled_outpost(
    addr: &std::net::SocketAddr,
    capability: Option<(&str, &str)>,
    required: &str,
) -> bool {
    let enrollment = &ENROLLED_CITADEL;

    if !enrollment
        .allowed_ips
        .iter()
        .any(|allowed| addr.ip().to_string() == *allowed)
    {
        return false;
    }

    let required_parts = match required.split_once('.') {
        Some(parts) => parts,
        None => return false,
    };

    if !enrollment
        .granted_capabilities
        .iter()
        .any(|(ns, action)| *ns == required_parts.0 && *action == required_parts.1)
    {
        return false;
    }

    let Some((outpost_id, granted_capability)) = capability else {
        return false;
    };

    outpost_id == enrollment.outpost_id && granted_capability == required
}

fn not_authorized() -> (StatusCode, axum::Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(
            serde_json::json!({"error": "enrolled outpost identity and presence.read capability required"}),
        ),
    )
}

async fn next_sequence(state: &HarnessPresenceState) -> usize {
    state
        .sequence
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        + 1
}

const STREAM_TICK: Duration = Duration::from_millis(250);
