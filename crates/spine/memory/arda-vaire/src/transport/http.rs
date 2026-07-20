// sigil: REPAIR
use crate::service::{InformantEvent, MnemosyneService};
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Deserialize)]
struct RecallRecentParams {
    hours: Option<i64>,
    crate_name: Option<String>,
    scope: Option<String>,
    query: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ConsolidateRequest {
    hours: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ObsidianSyncRequest {
    vault_path: Option<String>,
    max_files: Option<usize>,
}

fn build_router(service: MnemosyneService) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/stats", get(stats))
        .route("/paths", get(paths))
        .route("/identity_state", get(identity_state))
        .route("/encode", post(encode))
        .route("/recall_recent", get(recall_recent))
        .route("/consolidate", post(consolidate))
        .route("/obsidian_sync", post(obsidian_sync))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

pub async fn run_http_server(service: MnemosyneService, addr: &str) -> Result<()> {
    let app = build_router(service);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("failed to bind HTTP listener on {addr}: {e}"),
        })?;

    tracing::info!(addr = %addr, "MNEMOSYNE HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) = try_run_bounded_async(
        "mnemosyne_http_request",
        http_request_limit(),
        || async move { next.run(req).await },
    )
    .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "MNEMOSYNE HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_MNEMOSYNE_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn status(State(service): State<MnemosyneService>) -> impl IntoResponse {
    map_result(|| service.status())
}

async fn stats(State(service): State<MnemosyneService>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "stats": service.stats()?})))
}

async fn paths(State(service): State<MnemosyneService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.paths()}))
}

async fn identity_state(State(service): State<MnemosyneService>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "identity": service.identity_state()?})))
}

async fn encode(
    State(service): State<MnemosyneService>,
    Json(event): Json<InformantEvent>,
) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "encoded": service.encode(event)?})))
}

async fn recall_recent(
    State(service): State<MnemosyneService>,
    Query(params): Query<RecallRecentParams>,
) -> impl IntoResponse {
    map_result(|| {
        let hours = params.hours.unwrap_or(24);
        let crate_name = params.crate_name.as_deref();
        let scope = params.scope.as_deref();
        let memories = if let Some(query) = params.query.as_deref() {
            service.recall_relevant(query, hours, crate_name, scope, params.limit.unwrap_or(12))?
        } else {
            service.recall_recent_scoped(hours, crate_name, scope)?
        };
        Ok(json!({
            "ok": true,
            "memories": memories
        }))
    })
}

async fn consolidate(
    State(service): State<MnemosyneService>,
    Json(req): Json<ConsolidateRequest>,
) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "report": service.consolidate(req.hours.unwrap_or(24))?})))
}

async fn obsidian_sync(
    State(service): State<MnemosyneService>,
    Json(req): Json<ObsidianSyncRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "report": service.sync_obsidian(
                req.vault_path.as_deref().unwrap_or("human/.obsidian"),
                req.max_files.unwrap_or(200),
            )?
        }))
    })
}

async fn events(
    State(service): State<MnemosyneService>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5))).map(
        move |_| {
            let payload = build_event_payload(&service)
                .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
            Ok(Event::default().event("status").data(payload.to_string()))
        },
    );
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn map_result<F>(f: F) -> Json<Value>
where
    F: FnOnce() -> Result<Value>,
{
    match f() {
        Ok(v) => Json(v),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}

fn build_event_payload(service: &MnemosyneService) -> Result<Value> {
    let stats = service.stats()?;
    let identity = service.identity_state()?;
    let recent_memories = service.recall_recent(48, None)?;
    let noise = service.recent_noise_events(8);
    let obsidian = service.recent_obsidian_entries(8);
    let high_significance = recent_memories
        .iter()
        .filter(|entry| entry.significance >= 0.8)
        .count();
    let continuity_alert = stats.chain_integrity != "head_present";

    Ok(json!({
        "ok": true,
        "stream_version": "mnemosyne.events.v2",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "stats": stats,
        "identity": identity,
        "memory_flow": {
            "recent_memories": recent_memories,
            "noise_events": noise,
            "obsidian_bridge": obsidian,
            "counts": {
                "recent_memory_count": identity.recent_events.len(),
                "high_significance_memories": high_significance,
                "noise_events": noise.len(),
                "obsidian_entries": obsidian.len()
            }
        },
        "arda_hints": {
            "primary_panel": "memory_continuity",
            "boardroom_section": "identity_and_growth",
            "alert_on_chain_integrity": continuity_alert,
            "alert_on_memory_drought": identity.recent_events.is_empty()
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::{build_event_payload, build_router};
    use crate::{InformantEvent, MnemosyneService};
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tempfile::tempdir;
    use tower::ServiceExt;

    #[tokio::test]
    async fn http_contract_status_encode_recall_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let service = MnemosyneService::new(dir.path()).expect("service");
        let app = build_router(service);

        let status_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("status request"),
            )
            .await
            .expect("status");
        assert_eq!(status_response.status(), StatusCode::OK);

        let encode_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/encode")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"informant_id":"audit","crate_name":"prometheus","event_type":"decision_completed","ts_utc":"{}","content":"Boardroom routing continuity decision","confidence_hint":0.91,"tags":["boardroom","routing"]}}"#,
                        chrono::Utc::now().to_rfc3339()
                    )))
                    .expect("encode request"),
            )
            .await
            .expect("encode");
        assert_eq!(encode_response.status(), StatusCode::OK);

        let recall_response = app
            .oneshot(
                Request::builder()
                    .uri("/recall_recent?hours=24&crate_name=prometheus&query=routing&limit=5")
                    .body(Body::empty())
                    .expect("recall request"),
            )
            .await
            .expect("recall");
        assert_eq!(recall_response.status(), StatusCode::OK);

        let body = to_bytes(recall_response.into_body(), usize::MAX)
            .await
            .expect("recall body");
        let value: Value = serde_json::from_slice(&body).expect("recall json");
        let memories = value["memories"].as_array().expect("memories");
        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0]["source_crate"].as_str(), Some("prometheus"));
    }

    #[test]
    fn event_payload_includes_continuity_sections() {
        let dir = tempdir().expect("tempdir");
        let service = MnemosyneService::new(dir.path()).expect("service");
        service
            .encode(InformantEvent {
                informant_id: "audit".to_string(),
                crate_name: "prometheus".to_string(),
                event_type: "decision_completed".to_string(),
                ts_utc: chrono::Utc::now().to_rfc3339(),
                content: "Validated continuity payload".to_string(),
                confidence_hint: Some(0.91),
                tags: vec!["audit".to_string(), "continuity".to_string()],
            })
            .expect("encode");

        let payload = build_event_payload(&service).expect("payload");
        assert_eq!(payload["stream_version"], "mnemosyne.events.v2");
        assert!(payload["memory_flow"]["recent_memories"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        assert_eq!(payload["arda_hints"]["primary_panel"], "memory_continuity");
        assert_eq!(payload["stats"]["chain_integrity"], "head_present");
    }
}
