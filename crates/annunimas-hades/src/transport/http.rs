// sigil: REPAIR
use crate::service::HadesService;
use crate::types::{QuorumProof, SigilVacuumRule};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::try_run_bounded_async;
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
struct QueueParams {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct LogParams {
    limit: Option<usize>,
    event_filter: Option<String>,
    sigil_code_regex: Option<String>,
    sigil_retention: Option<String>,
    sigil_tag: Option<String>,
    sigil_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SigilMatchParams {
    path: String,
    limit: Option<usize>,
    sigil_code_regex: Option<String>,
    sigil_retention: Option<String>,
    sigil_tag: Option<String>,
    sigil_source: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SweepRequest {
    sweep_type: Option<String>,
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RemoveRequest {
    file: String,
    authorized_by: Option<String>,
    quorum_proof: Option<QuorumProof>,
}

fn build_router(service: HadesService) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/queue", get(queue))
        .route("/log", get(log))
        .route("/sigil_match", get(sigil_match))
        .route("/sweep", post(sweep))
        .route("/remove", post(remove))
        .route("/paths", get(paths))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

pub async fn run_http_server(service: HadesService, addr: &str) -> Result<()> {
    let app = build_router(service);

    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "hades".to_owned(),
                message: format!("failed to bind HTTP listener on {addr}: {e}"),
            })?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "hades".to_owned(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("hades_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "HADES HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ANNUNIMAS_HADES_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn status(State(service): State<HadesService>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "status": service.status()?})))
}

async fn queue(
    State(service): State<HadesService>,
    Query(params): Query<QueueParams>,
) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "queue": service.queue(params.limit.unwrap_or(100))?})))
}

async fn log(
    State(service): State<HadesService>,
    Query(params): Query<LogParams>,
) -> impl IntoResponse {
    let rule = sigil_rule_from_parts(
        params.sigil_code_regex,
        params.sigil_retention,
        params.sigil_tag,
        params.sigil_source,
    );
    map_result(|| {
        Ok(json!({
            "ok": true,
            "log": service.log(params.limit.unwrap_or(100), params.event_filter.as_deref(), rule.as_ref())?
        }))
    })
}

async fn sigil_match(
    State(service): State<HadesService>,
    Query(params): Query<SigilMatchParams>,
) -> impl IntoResponse {
    let rule = sigil_rule_from_parts(
        params.sigil_code_regex,
        params.sigil_retention,
        params.sigil_tag,
        params.sigil_source,
    );
    map_result(|| {
        Ok(json!({
            "ok": true,
            "matches": service.sigil_match(&params.path, &rule.unwrap_or_default(), params.limit.unwrap_or(100))?
        }))
    })
}

async fn sweep(
    State(service): State<HadesService>,
    Json(req): Json<SweepRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "result": service.sweep(req.sweep_type.as_deref().unwrap_or("manual"), req.path.as_deref())?
        }))
    })
}

async fn remove(
    State(service): State<HadesService>,
    Json(req): Json<RemoveRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "queued": service.queue_remove_with_proof(
                &req.file,
                req.authorized_by.as_deref().unwrap_or("orchestrator"),
                req.quorum_proof,
            )?
        }))
    })
}

async fn paths(State(service): State<HadesService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.paths()}))
}

async fn events(
    State(service): State<HadesService>,
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

fn build_event_payload(service: &HadesService) -> Result<Value> {
    let status = service.status()?;
    let queue = service.queue(12)?;
    let log = service.log(16, None, None)?;
    let joulework = service.recent_joulework(8)?;
    let warden_queue = service.recent_warden_queue(8)?;
    let athena_handoffs = service.recent_athena_handoffs(8)?;
    let repair_events = log
        .iter()
        .filter(|entry| entry.event == "repair_detected")
        .count();
    let orphan_events = log
        .iter()
        .filter(|entry| entry.event == "orphan_found")
        .count();
    let removal_events = log
        .iter()
        .filter(|entry| {
            matches!(
                entry.event.as_str(),
                "coin_detected" | "removed" | "archived" | "destructive_quorum_denied"
            )
        })
        .count();

    Ok(json!({
        "ok": true,
        "stream_version": "hades.events.v2",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "status": status,
        "lifecycle": {
            "queue": queue,
            "recent_log": log,
            "recent_joulework": joulework,
            "recent_warden_handoffs": warden_queue,
            "recent_athena_handoffs": athena_handoffs,
            "counts": {
                "repair_events": repair_events,
                "orphan_events": orphan_events,
                "removal_events": removal_events
            }
        },
        "arda_hints": {
            "primary_panel": "lifecycle_maintenance",
            "boardroom_section": "cleanup_and_repair",
            "alert_on_pending_actions": status.pending_actions > 0,
            "alert_on_quarantine": status.quarantined > 0
        }
    }))
}

fn sigil_rule_from_parts(
    code_regex: Option<String>,
    retention: Option<String>,
    tag: Option<String>,
    source: Option<String>,
) -> Option<SigilVacuumRule> {
    if code_regex.is_none() && retention.is_none() && tag.is_none() && source.is_none() {
        return None;
    }
    Some(SigilVacuumRule {
        code_regex,
        retention,
        tag,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::{build_event_payload, build_router};
    use crate::HadesService;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use std::fs;
    use tempfile::tempdir;
    use tower::ServiceExt;

    #[tokio::test]
    async fn http_contract_status_remove_queue_roundtrip() {
        let dir = tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        let app = build_router(service);
        let target = dir.path().join("artifact.jsonl");
        fs::write(&target, "{\"ok\":true}\n").expect("write target");

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

        let remove_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/remove")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"file":"{}","authorized_by":"orchestrator","quorum_proof":{{"approvers":["aurelius","bacon"],"evidence":["ticket:http-1"],"asserted_at_utc":"2026-04-21T00:00:00Z"}}}}"#,
                        target.to_string_lossy()
                    )))
                    .expect("remove request"),
            )
            .await
            .expect("remove");
        assert_eq!(remove_response.status(), StatusCode::OK);

        let queue_response = app
            .oneshot(
                Request::builder()
                    .uri("/queue?limit=5")
                    .body(Body::empty())
                    .expect("queue request"),
            )
            .await
            .expect("queue");
        assert_eq!(queue_response.status(), StatusCode::OK);

        let body = to_bytes(queue_response.into_body(), usize::MAX)
            .await
            .expect("queue body");
        let value: Value = serde_json::from_slice(&body).expect("queue json");
        let queue = value["queue"].as_array().expect("queue array");
        assert_eq!(queue.len(), 1);
        assert_eq!(
            queue[0]["file"].as_str(),
            Some(target.to_string_lossy().as_ref())
        );
    }

    #[test]
    fn event_payload_includes_lifecycle_sections() {
        let dir = tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        fs::write(
            dir.path().join("hades_log.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-09T11:00:00Z\",\"event\":\"repair_detected\",\"file\":\"alpha\",\"details\":{\"athena_task_queued\":true}}\n",
                "{\"ts\":\"2026-03-09T11:01:00Z\",\"event\":\"warden_handoff\",\"file\":\"alpha\",\"details\":{\"local_queue_written\":true}}\n"
            ),
        )
        .expect("log write");
        fs::write(
            dir.path().join("action_queue.jsonl"),
            "{\"task_id\":\"h1\",\"queued_at_utc\":\"2026-03-09T11:00:00Z\",\"action\":\"remove\",\"file\":\"alpha\",\"authorized_by\":\"orchestrator\",\"reason\":\"test\",\"execute_after_utc\":null,\"quorum_proof\":null}\n",
        )
        .expect("queue write");
        fs::write(
            dir.path().join("joulework.jsonl"),
            "{\"ts_utc\":\"2026-03-09T11:02:00Z\",\"component\":\"hades\",\"operation\":\"sweep\"}\n",
        )
        .expect("joule write");
        fs::write(
            dir.path().join("warden_queue.jsonl"),
            "{\"ts\":\"2026-03-09T11:03:00Z\",\"event\":\"repair_detected\",\"file\":\"alpha\",\"synced\":false}\n",
        )
        .expect("warden write");
        fs::write(
            dir.path().join("athena_handoff_queue.jsonl"),
            "{\"ts_utc\":\"2026-03-09T11:04:00Z\",\"event\":\"repair_detected\",\"status\":\"queued_fallback\"}\n",
        )
        .expect("athena write");

        let payload = build_event_payload(&service).expect("payload");
        assert_eq!(payload["stream_version"], "hades.events.v2");
        assert_eq!(payload["lifecycle"]["counts"]["repair_events"], 1);
        assert!(payload["lifecycle"]["queue"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        assert_eq!(
            payload["arda_hints"]["primary_panel"],
            "lifecycle_maintenance"
        );
    }
}
