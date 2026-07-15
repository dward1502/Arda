// sigil: REPAIR
use crate::service::PrometheusService;
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
struct ThoughtParams {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct EscalationParams {
    limit: Option<usize>,
    include_resolved: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ResolveEscalationRequest {
    escalation_id: String,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CouncilFanoutRequest {
    topic: String,
    participants: Option<Vec<String>>,
    context: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ExecutionIntentParams {
    limit: Option<usize>,
    include_terminal: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TransitionIntentRequest {
    intent_id: String,
    status: String,
    note: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CompactExecutionIntentRequest {
    retention_days: Option<i64>,
    max_keep: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DriftDetectRequest {
    auto_open: Option<bool>,
}

pub async fn run_http_server(service: PrometheusService, addr: &str) -> Result<()> {
    let app = build_router(service);

    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "prometheus".to_string(),
                message: format!("failed to bind HTTP listener on {addr}: {e}"),
            })?;

    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "prometheus".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

fn build_router(service: PrometheusService) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/roster", get(roster))
        .route("/thoughts", get(thoughts))
        .route("/escalations", get(escalations))
        .route("/escalations/resolve", post(resolve_escalation))
        .route("/council/fanout", post(council_fanout))
        .route("/interrupt/reroute", post(interrupt_reroute))
        .route("/execution-intents", get(execution_intents))
        .route(
            "/execution-intents/recovery",
            get(execution_intents_recovery),
        )
        .route(
            "/execution-intents/transition",
            post(transition_execution_intent),
        )
        .route(
            "/execution-intents/compact",
            post(compact_execution_intents),
        )
        .route("/drift/detect", post(drift_detect_reconcile))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) = try_run_bounded_async(
        "prometheus_http_request",
        http_request_limit(),
        || async move { next.run(req).await },
    )
    .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "PROMETHEUS HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(24)
}

async fn status(State(service): State<PrometheusService>) -> impl IntoResponse {
    json_result(|| Ok(json!({"ok": true, "status": service.status()?})))
}

async fn roster(State(service): State<PrometheusService>) -> impl IntoResponse {
    json_result(|| Ok(json!({"ok": true, "roster": service.roster()})))
}

async fn thoughts(
    State(service): State<PrometheusService>,
    Query(params): Query<ThoughtParams>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({"ok": true, "thoughts": service.thoughts(params.limit.unwrap_or(20))?}))
    })
}

async fn escalations(
    State(service): State<PrometheusService>,
    Query(params): Query<EscalationParams>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "escalations": service.escalations(
                params.limit.unwrap_or(20),
                params.include_resolved.unwrap_or(false)
            )?
        }))
    })
}

async fn resolve_escalation(
    State(service): State<PrometheusService>,
    Json(req): Json<ResolveEscalationRequest>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "escalation": service.resolve_escalation(
                &req.escalation_id,
                req.note.as_deref().unwrap_or("resolved")
            )?
        }))
    })
}

async fn council_fanout(
    State(service): State<PrometheusService>,
    Json(req): Json<CouncilFanoutRequest>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "fanout": service.council_fanout(
                &req.topic,
                req.participants.unwrap_or_default(),
                req.context
            )?
        }))
    })
}

async fn interrupt_reroute(
    State(service): State<PrometheusService>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    json_result(|| Ok(json!({"ok": true, "result": service.interrupt_reroute(payload)?})))
}

async fn execution_intents(
    State(service): State<PrometheusService>,
    Query(params): Query<ExecutionIntentParams>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "intents": service.execution_intents(
                params.limit.unwrap_or(50),
                params.include_terminal.unwrap_or(false)
            )?
        }))
    })
}

async fn execution_intents_recovery(State(service): State<PrometheusService>) -> impl IntoResponse {
    json_result(|| Ok(json!({"ok": true, "recovery": service.execution_intents_recovery()?})))
}

async fn transition_execution_intent(
    State(service): State<PrometheusService>,
    Json(req): Json<TransitionIntentRequest>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "result": service.transition_execution_intent(
                &req.intent_id,
                &req.status,
                req.note.as_deref()
            )?
        }))
    })
}

async fn compact_execution_intents(
    State(service): State<PrometheusService>,
    Json(req): Json<CompactExecutionIntentRequest>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "result": service.compact_execution_intents(
                req.retention_days.unwrap_or(14),
                req.max_keep.unwrap_or(5000)
            )?
        }))
    })
}

async fn drift_detect_reconcile(
    State(service): State<PrometheusService>,
    Json(req): Json<DriftDetectRequest>,
) -> impl IntoResponse {
    json_result(|| {
        Ok(json!({
            "ok": true,
            "result": service.drift_detect_reconcile(req.auto_open.unwrap_or(false))?
        }))
    })
}

async fn events(
    State(service): State<PrometheusService>,
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

fn json_result<F>(f: F) -> Json<Value>
where
    F: FnOnce() -> Result<Value>,
{
    match f() {
        Ok(v) => Json(v),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}

fn build_event_payload(service: &PrometheusService) -> Result<Value> {
    let status = service.status()?;
    let roster = service.roster();
    let thoughts = service.thoughts(8)?;
    let escalations = service.escalations(8, false)?;
    let council = service.recent_council_events(8);
    let intents = service.execution_intents(12, false)?;
    let recovery = service.execution_intents_recovery()?;
    let drift = service.latest_drift_report().unwrap_or_else(|| json!({}));

    let pending_review_intents = intents
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("pending_review"))
        .count();
    let queued_intents = intents
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("queued"))
        .count();
    let unresolved_escalations = escalations.len();
    let council_events = council.len();

    Ok(json!({
        "ok": true,
        "stream_version": "prometheus.events.v2",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "status": status,
        "roster": roster,
        "executive": {
            "recent_thoughts": thoughts,
            "pending_escalations": escalations,
            "recent_council_events": council,
            "execution_intents": intents,
            "execution_intents_recovery": recovery,
            "drift_report": drift,
            "counts": {
                "queued_intents": queued_intents,
                "pending_review_intents": pending_review_intents,
                "unresolved_escalations": unresolved_escalations,
                "recent_council_events": council_events
            }
        },
        "arda_hints": {
            "primary_panel": "executive_command",
            "boardroom_section": "orders_and_escalations",
            "alert_on_pending_review": pending_review_intents > 0,
            "alert_on_drift": drift.get("drift_count").and_then(Value::as_u64).unwrap_or(0) > 0
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::{build_event_payload, build_router, http_request_limit, json_result};
    use crate::PrometheusService;
    use arda_core::error::ArdaError;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;
    use tower::ServiceExt;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn test_service() -> PrometheusService {
        let dir = tempdir().expect("tempdir");
        let root = dir.keep();
        let core_root = root.join("core");
        let prometheus_home = root.join("prometheus_home");
        let minds_home = root.join("minds");
        fs::create_dir_all(core_root.join("realm")).expect("realm mkdir");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(&prometheus_home).expect("prometheus mkdir");
        fs::create_dir_all(&minds_home).expect("minds mkdir");

        fs::write(
            core_root.join("realm/boot.toml"),
            "[ceo]\nagent_id = \"arandur\"\nheartbeat_ms = 500\ntriad_bypass = true\n",
        )
        .expect("boot write");
        fs::write(
            core_root.join("realm/arda.toml"),
            "[identity]\nname = \"Arda\"\nsigil = \"𓀀\"\n[[realms.definition]]\nid = \"command\"\ncolor = \"#fff\"\n",
        )
        .expect("identity write");
        fs::write(
            core_root.join("realm/agents.toml"),
            "[[agent]]\nid = \"arandur\"\nsigil = \"𓀀\"\nname = \"Arandur\"\nrealm = \"command\"\nclearance = \"sovereign\"\n",
        )
        .expect("agents write");
        fs::write(
            core_root.join("state/world.json"),
            "{\"system\":{\"status\":\"READY\"},\"metrics\":{\"system_resonance\":0.9},\"agents\":[{\"id\":\"arandur\",\"status\":\"ONLINE\",\"active_tasks\":1}]}\n",
        )
        .expect("world write");

        PrometheusService::from_core_for_test(&core_root, &prometheus_home, &minds_home)
            .expect("service")
    }

    #[test]
    fn event_payload_includes_executive_sections() {
        let service = test_service();
        let prometheus_home = service
            .core_root()
            .parent()
            .expect("root")
            .join("prometheus_home");
        fs::write(
            prometheus_home.join("council_fanout.jsonl"),
            "{\"event\":\"council_fanout\",\"payload\":{\"topic\":\"test\"}}\n",
        )
        .expect("council write");
        fs::write(
            prometheus_home.join("execution_intents.jsonl"),
            concat!(
                "{\"ts_utc\":\"2026-03-09T12:00:00Z\",\"intent_id\":\"i1\",\"status\":\"queued\"}\n",
                "{\"ts_utc\":\"2026-03-09T12:01:00Z\",\"intent_id\":\"i2\",\"status\":\"pending_review\"}\n"
            ),
        )
        .expect("intents write");
        fs::write(
            prometheus_home.join("execution_intents_recovery_last.json"),
            "{\"open_intents\":2}\n",
        )
        .expect("recovery write");
        fs::write(
            prometheus_home.join("drift_report_last.json"),
            "{\"drift_count\":1}\n",
        )
        .expect("drift write");

        let payload = build_event_payload(&service).expect("payload");
        assert_eq!(payload["stream_version"], "prometheus.events.v2");
        assert_eq!(payload["executive"]["counts"]["queued_intents"], 1);
        assert_eq!(payload["executive"]["counts"]["pending_review_intents"], 1);
        assert_eq!(payload["executive"]["counts"]["recent_council_events"], 1);
        assert_eq!(payload["arda_hints"]["primary_panel"], "executive_command");
    }

    #[test]
    fn http_request_limit_uses_env_when_valid_and_falls_back_otherwise() {
        let _guard = ENV_LOCK.lock().expect("env lock");

        std::env::remove_var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY", "48");
        assert_eq!(http_request_limit(), 48);

        std::env::set_var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY", "0");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY", "invalid");
        assert_eq!(http_request_limit(), 24);

        std::env::remove_var("ARDA_PROMETHEUS_HTTP_MAX_CONCURRENCY");
    }

    #[test]
    fn json_result_wraps_agent_errors_as_ok_false_payloads() {
        let payload = json_result(|| {
            Err(ArdaError::Agent {
                agent: "prometheus".to_string(),
                message: "synthetic failure".to_string(),
            })
        });

        assert_eq!(payload.0["ok"], false);
        assert!(payload.0["error"]
            .as_str()
            .expect("error string")
            .contains("synthetic failure"));
    }

    #[tokio::test]
    async fn interrupt_reroute_route_queues_intent_and_execution_intents_route_reads_it() {
        let service = test_service();
        let app = build_router(service);

        let reroute_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/interrupt/reroute")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"event_id":"int_http_1","source":"voice","sender":"operator","content":"reroute queue","triad_passed":true,"triad_score":0.81,"policy_safe":true,"requires_operator_review":false,"context":{"task_ids":["task_alpha"]}}"#,
                    ))
                    .expect("reroute request"),
            )
            .await
            .expect("reroute response");
        assert_eq!(reroute_response.status(), StatusCode::OK);
        let reroute_body = to_bytes(reroute_response.into_body(), usize::MAX)
            .await
            .expect("reroute body");
        let reroute_json: Value = serde_json::from_slice(&reroute_body).expect("reroute json");
        assert_eq!(reroute_json["ok"], true);
        assert_eq!(reroute_json["result"]["queued"], 1);
        assert_eq!(
            reroute_json["result"]["intents"][0]["target_task_id"],
            "task_alpha"
        );

        let intents_response = app
            .oneshot(
                Request::builder()
                    .uri("/execution-intents?limit=10")
                    .body(Body::empty())
                    .expect("intents request"),
            )
            .await
            .expect("intents response");
        assert_eq!(intents_response.status(), StatusCode::OK);
        let intents_body = to_bytes(intents_response.into_body(), usize::MAX)
            .await
            .expect("intents body");
        let intents_json: Value = serde_json::from_slice(&intents_body).expect("intents json");
        assert_eq!(intents_json["ok"], true);
        assert_eq!(intents_json["intents"][0]["target_task_id"], "task_alpha");
        assert_eq!(intents_json["intents"][0]["status"], "queued");
    }

    #[tokio::test]
    async fn status_route_returns_prometheus_runtime_contract() {
        let app = build_router(test_service());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("status request"),
            )
            .await
            .expect("status response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("status body");
        let payload: Value = serde_json::from_slice(&body).expect("status json");
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["status"]["heartbeat_mode"], "interval");
        assert_eq!(payload["status"]["agents_online"], 1);
    }

    #[tokio::test]
    async fn roster_route_returns_current_agent_projection() {
        let app = build_router(test_service());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/roster")
                    .body(Body::empty())
                    .expect("roster request"),
            )
            .await
            .expect("roster response");
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("roster body");
        let payload: Value = serde_json::from_slice(&body).expect("roster json");
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["roster"]["online_agents"], 1);
        assert_eq!(payload["roster"]["agents"][0]["id"], "arandur");
    }
}
