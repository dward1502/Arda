// sigil: REPAIR
use crate::mnemosyne_integration::get_cache_stats;
use crate::service::HermesService;
use crate::types::{BoardroomPost, InboundMessage, InterruptionMessage, OutboundMessage};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::try_run_bounded_async;
use axum::extract::{Path, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::fs;
use std::path::Path as FsPath;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Deserialize)]
struct BoardroomParams {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct WebhookPayload {
    sender: Option<String>,
    content: String,
    channel: Option<String>,
    is_illuvatar: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct IlluvatarFanoutRequest {
    source_provider: Option<String>,
    sender: Option<String>,
    content: String,
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CouncilOpenRequest {
    topic: String,
    participants: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct CouncilReportRequest {
    session_id: String,
    from_agent: String,
    body: String,
}

#[derive(Debug, Deserialize)]
struct CouncilCloseRequest {
    session_id: String,
    outcome: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RetryDlqRequest {
    limit: Option<usize>,
}

pub async fn run_http_server(service: HermesService, addr: &str) -> Result<()> {
    let app = Router::new()
        .route("/status", get(status))
        .route("/providers", get(providers))
        .route("/subcomponents", get(subcomponents))
        .route("/paths", get(paths))
        .route("/l3-readiness", get(l3_readiness))
        .route("/classify", post(classify))
        .route("/send", post(send))
        .route("/interrupt", post(interrupt))
        .route("/reroute/retry", post(retry_reroute_dlq))
        .route("/boardroom/post", post(boardroom_post))
        .route("/boardroom/recent", get(boardroom_recent))
        .route("/council/open", post(council_open))
        .route("/council/report", post(council_report))
        .route("/council/close", post(council_close))
        .route("/webhook/:provider", post(webhook_ingest))
        .route("/illuvatar/fanout", post(illuvatar_fanout))
        .route("/calendar/sync", post(calendar_sync))
        .route("/events", get(events))
        .route("/cache_metrics", get(cache_metrics))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service);

    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "hermes".to_string(),
                message: format!("failed to bind HTTP listener on {addr}: {e}"),
            })?;
    axum::serve(listener, app)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "hermes".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("hermes_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "HERMES HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(24)
}

async fn status(State(service): State<HermesService>) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"ok": true, "status": service.status().await?})) })
        .await
}

async fn providers(State(service): State<HermesService>) -> impl IntoResponse {
    Json(json!({"ok": true, "providers": service.providers_status().await}))
}

async fn subcomponents(State(service): State<HermesService>) -> impl IntoResponse {
    Json(json!({"ok": true, "subcomponents": service.subcomponents()}))
}

async fn paths(State(service): State<HermesService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.paths()}))
}

async fn l3_readiness(State(service): State<HermesService>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "projection": service.l3_readiness_projection()?})))
}

async fn classify(
    State(service): State<HermesService>,
    Json(msg): Json<InboundMessage>,
) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "classification": service.classify(msg)?})))
}

async fn send(
    State(service): State<HermesService>,
    Json(msg): Json<OutboundMessage>,
) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"ok": true, "result": service.send(msg).await?})) })
        .await
}

async fn interrupt(
    State(service): State<HermesService>,
    Json(msg): Json<InterruptionMessage>,
) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "result": service.interrupt(msg)?})))
}

async fn retry_reroute_dlq(
    State(service): State<HermesService>,
    Json(req): Json<RetryDlqRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({"ok": true, "result": service.retry_reroute_dlq(req.limit.unwrap_or(100))?}))
    })
}

async fn boardroom_post(
    State(service): State<HermesService>,
    Json(post): Json<BoardroomPost>,
) -> impl IntoResponse {
    map_result(|| {
        service.boardroom_post(post)?;
        Ok(json!({"ok": true, "posted": true}))
    })
}

async fn boardroom_recent(
    State(service): State<HermesService>,
    Query(params): Query<BoardroomParams>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "boardroom": service.boardroom_recent(params.limit.unwrap_or(20))?
        }))
    })
}

async fn calendar_sync(State(service): State<HermesService>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "calendar": service.calendar_sync()?})))
}

async fn webhook_ingest(
    State(service): State<HermesService>,
    Path(provider): Path<String>,
    Json(payload): Json<WebhookPayload>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "classification": service.ingest_external(
                &provider,
                payload.sender.as_deref().unwrap_or("webhook"),
                &payload.content,
                payload.channel,
                payload.is_illuvatar.unwrap_or(false)
            )?
        }))
    })
}

async fn illuvatar_fanout(
    State(service): State<HermesService>,
    Json(req): Json<IlluvatarFanoutRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        let source_provider = req.source_provider.unwrap_or_else(|| "discord".to_string());
        let sender = req.sender.unwrap_or_else(|| "illuvatar".to_string());
        let mut msg = InboundMessage::new(source_provider.clone(), sender, req.content);
        msg.channel = req.channel;
        msg.is_illuvatar = true;
        Ok(json!({
            "ok": true,
            "fanout": service.fanout_illuvatar_directive(&source_provider, &msg).await?
        }))
    })
    .await
}

async fn council_open(
    State(service): State<HermesService>,
    Json(req): Json<CouncilOpenRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "session": service.council_open(&req.topic, req.participants.unwrap_or_default())?
        }))
    })
}

async fn council_report(
    State(service): State<HermesService>,
    Json(req): Json<CouncilReportRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "report": service.council_report(&req.session_id, &req.from_agent, &req.body)?
        }))
    })
}

async fn council_close(
    State(service): State<HermesService>,
    Json(req): Json<CouncilCloseRequest>,
) -> impl IntoResponse {
    map_result(|| {
        Ok(json!({
            "ok": true,
            "close": service.council_close(&req.session_id, req.outcome.as_deref().unwrap_or("closed"))?
        }))
    })
}

async fn events(
    State(service): State<HermesService>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5))).map(
        move |_| {
            let service = service.clone();
            async move {
                let payload = build_event_payload(&service)
                    .await
                    .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
                Ok(Event::default().event("status").data(payload.to_string()))
            }
        },
    );
    Sse::new(stream.then(|fut| fut)).keep_alive(KeepAlive::default())
}

/// HTTP endpoint to retrieve cache metrics for monitoring
async fn cache_metrics(State(_service): State<HermesService>) -> impl IntoResponse {
    let metrics = get_cache_stats();
    Json(json!({
        "ok": true,
        "cache": {
            "hits": metrics.hits,
            "misses": metrics.misses,
            "size": metrics.size,
            "evictions": metrics.evictions,
        }
    }))
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

async fn map_result_async<Fut>(fut: Fut) -> Json<Value>
where
    Fut: std::future::Future<Output = Result<Value>>,
{
    match fut.await {
        Ok(v) => Json(v),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}

async fn build_event_payload(service: &HermesService) -> Result<Value> {
    let status = service.status().await?;
    let providers = service.providers_status().await;
    let subcomponents = service.subcomponents();
    let boardroom = service.boardroom_recent(8)?;
    let interruptions = service.recent_interruptions(8);
    let reroute_metrics = service.recent_reroute_metrics(8);
    let reroute_acks = service.recent_reroute_acks(8);
    let decision_metrics = service.recent_decision_metrics(8);
    let council_sessions = service.recent_council_sessions(8);

    let deferred_reroutes = reroute_metrics
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("deferred"))
        .count();
    let denied_interrupts = interruptions
        .iter()
        .filter(|entry| entry.get("policy_authorized").and_then(Value::as_bool) == Some(false))
        .count();
    let open_councils = council_sessions
        .iter()
        .filter(|entry| entry.get("status").and_then(Value::as_str) == Some("open"))
        .count();
    let matrix_contract = load_matrix_boardroom_contract();
    let rooms = matrix_contract
        .get("rooms")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let activation = matrix_contract
        .get("activation_requirements")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let routing_contract = matrix_contract
        .get("routing_contract")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let bridge_contracts = matrix_contract
        .get("bridge_contracts")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let matrix_enabled = matrix_contract
        .get("defaults")
        .and_then(|value| value.get("provider"))
        .and_then(Value::as_str)
        == Some("matrix");
    let matrix_ready = activation
        .get("federated_rooms_ready")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(json!({
        "ok": true,
        "stream_version": "hermes.events.v2",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "status": status,
        "providers": providers,
        "subcomponents": subcomponents,
        "communications": {
            "boardroom": boardroom,
            "boardroom_contract": {
                "source": "core/state/matrix_boardrooms.json",
                "provider": "matrix",
                "client_surface": matrix_contract
                    .get("defaults")
                    .and_then(|value| value.get("client_surface"))
                    .cloned()
                    .unwrap_or_else(|| json!("element")),
                "rooms": rooms,
                "routing_contract": routing_contract,
                "bridge_contracts": bridge_contracts,
                "activation_requirements": activation,
                "room_count": rooms.len(),
                "matrix_ready": matrix_ready,
            },
            "interruptions": interruptions,
            "reroute_metrics": reroute_metrics,
            "reroute_acks": reroute_acks,
            "decision_metrics": decision_metrics,
            "council_sessions": council_sessions,
            "counts": {
                "boardroom_posts": status.messages_today.outbound,
                "boardroom_contract_rooms": rooms.len(),
                "deferred_reroutes": deferred_reroutes,
                "denied_interrupts": denied_interrupts,
                "open_councils": open_councils
            }
        },
        "arda_hints": {
            "primary_panel": "boardroom_and_comms",
            "boardroom_section": if matrix_enabled { "matrix_boardrooms" } else { "council_and_interrupts" },
            "alert_on_matrix_activation_gap": matrix_enabled && !matrix_ready,
            "alert_on_interrupt_denials": denied_interrupts > 0,
            "alert_on_reroute_backpressure": deferred_reroutes > 0
        }
    }))
}

fn load_matrix_boardroom_contract() -> Value {
    let path = FsPath::new("core/state/matrix_boardrooms.json");
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str::<Value>(&content).ok())
        .unwrap_or_else(|| {
            json!({
                "defaults": {
                    "provider": "matrix",
                    "client_surface": "element"
                },
                "rooms": [],
                "routing_contract": {},
                "bridge_contracts": {},
                "activation_requirements": {
                    "federated_rooms_ready": false
                }
            })
        })
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{build_event_payload, http_request_limit};
    use crate::{BoardroomPost, HermesService, InterruptionMessage, OutboundMessage};
    use axum::body::to_bytes;
    use axum::extract::State;
    use axum::response::IntoResponse;
    use axum::Json;
    use serde_json::Value;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    async fn json_body(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        serde_json::from_slice(&body).expect("json body")
    }

    #[test]
    fn event_payload_includes_comms_sections() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");
        service
            .boardroom_post(BoardroomPost::new(
                "athena",
                "report",
                "Corpus update",
                "Knowledge depth increased",
            ))
            .expect("boardroom");
        service
            .interrupt(InterruptionMessage::new(
                "voice",
                "operator",
                "note that boardroom review stays active",
            ))
            .expect("interrupt");
        fs::write(
            dir.path().join("reroute_metrics.jsonl"),
            "{\"event\":\"deferred\",\"reason\":\"reroute_rate_limited\"}\n",
        )
        .expect("reroute metrics");
        fs::write(
            dir.path().join("decision_metrics.jsonl"),
            "{\"event\":\"decision_prompt_created\",\"prompt_id\":\"p1\"}\n",
        )
        .expect("decision metrics");
        fs::write(
            dir.path().join("council_sessions.jsonl"),
            "{\"session_id\":\"c1\",\"status\":\"open\",\"topic\":\"test\"}\n",
        )
        .expect("council sessions");

        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let payload = rt.block_on(build_event_payload(&service)).expect("payload");
        assert_eq!(payload["stream_version"], "hermes.events.v2");
        assert!(payload["communications"]["boardroom"]
            .as_array()
            .is_some_and(|items| !items.is_empty()));
        assert_eq!(payload["communications"]["counts"]["deferred_reroutes"], 1);
        assert_eq!(payload["communications"]["counts"]["open_councils"], 1);
        assert_eq!(
            payload["communications"]["boardroom_contract"]["source"],
            "core/state/matrix_boardrooms.json"
        );
        assert_eq!(
            payload["arda_hints"]["primary_panel"],
            "boardroom_and_comms"
        );
    }

    #[test]
    fn http_request_limit_uses_env_when_valid_and_falls_back_otherwise() {
        let _guard = ENV_LOCK.lock().expect("env lock");

        std::env::remove_var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY", "32");
        assert_eq!(http_request_limit(), 32);

        std::env::set_var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY", "0");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY", "invalid");
        assert_eq!(http_request_limit(), 24);

        std::env::remove_var("ANNUNIMAS_HERMES_HTTP_MAX_CONCURRENCY");
    }

    #[tokio::test]
    async fn send_handler_returns_queued_payload_when_concurrency_gate_is_saturated() {
        let _send_gate_guard = crate::HERMES_PROVIDER_SEND_TEST_LOCK
            .lock()
            .expect("send gate test lock");
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        std::env::set_var("ANNUNIMAS_HERMES_SEND_MAX_CONCURRENCY", "1");
        let service = HermesService::new(dir.path()).expect("service");
        let acquired = std::sync::Arc::new(tokio::sync::Notify::new());
        let release = std::sync::Arc::new(tokio::sync::Notify::new());

        let holder_acquired = std::sync::Arc::clone(&acquired);
        let holder_release = std::sync::Arc::clone(&release);
        let holder = tokio::spawn(async move {
            loop {
                let acquired = std::sync::Arc::clone(&holder_acquired);
                let release = std::sync::Arc::clone(&holder_release);
                let result = annunimas_core::try_run_bounded_async(
                    "hermes_provider_send",
                    1,
                    || async move {
                        acquired.notify_waiters();
                        release.notified().await;
                    },
                )
                .await;
                if result.is_some() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        });
        acquired.notified().await;

        let response = super::send(
            State(service),
            Json(OutboundMessage::new(
                "discord",
                "boardroom",
                "Burst",
                "should shed",
            )),
        )
        .await
        .into_response();
        let payload = json_body(response).await;

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["result"]["queued"], true);
        assert_eq!(payload["result"]["dispatched"], false);
        assert_eq!(
            payload["result"]["error"].as_str(),
            Some("provider send concurrency gate saturated")
        );

        release.notify_waiters();
        holder.await.expect("holder");
        std::env::remove_var("ANNUNIMAS_HERMES_SEND_MAX_CONCURRENCY");
    }

    #[tokio::test]
    async fn interrupt_handler_reports_policy_blocked_override() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let response = super::interrupt(
            State(service),
            Json(InterruptionMessage::new(
                "voice",
                "guest",
                "override the active workflow immediately",
            )),
        )
        .await
        .into_response();
        let payload = json_body(response).await;

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["result"]["captured"], true);
        assert_eq!(payload["result"]["disposition"], "override");
        assert_eq!(payload["result"]["policy_safe"], false);
        assert_eq!(payload["result"]["policy_authorized"], false);
        assert_eq!(payload["result"]["requires_operator_review"], true);
        assert_eq!(
            payload["result"]["reroute_result"]["blocked"].as_bool(),
            Some(true)
        );
    }

    #[tokio::test]
    async fn webhook_handler_uses_default_sender_and_preserves_channel() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let response = super::webhook_ingest(
            State(service),
            axum::extract::Path("discord".to_string()),
            Json(super::WebhookPayload {
                sender: None,
                content: "status".to_string(),
                channel: Some("ops".to_string()),
                is_illuvatar: Some(false),
            }),
        )
        .await
        .into_response();
        let payload = json_body(response).await;
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["classification"]["route_to"], "prometheus");
        assert_eq!(payload["classification"]["priority"], "normal");
    }

    #[tokio::test]
    async fn boardroom_post_and_recent_handlers_round_trip_entries() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let post_response = super::boardroom_post(
            State(service.clone()),
            Json(BoardroomPost::new(
                "athena",
                "report",
                "Corpus update",
                "Knowledge depth increased",
            )),
        )
        .await
        .into_response();
        let post_payload = json_body(post_response).await;
        assert_eq!(post_payload["ok"], true);
        assert_eq!(post_payload["posted"], true);

        let recent_response = super::boardroom_recent(
            State(service),
            axum::extract::Query(super::BoardroomParams { limit: Some(5) }),
        )
        .await
        .into_response();
        let recent_payload = json_body(recent_response).await;
        assert_eq!(recent_payload["ok"], true);
        assert_eq!(
            recent_payload["boardroom"][0]["subject"].as_str(),
            Some("Corpus update")
        );
    }

    #[tokio::test]
    async fn council_handlers_return_session_report_and_close_payloads() {
        let dir = tempdir().expect("tempdir");
        let service = HermesService::new(dir.path()).expect("service");

        let open_response = super::council_open(
            State(service.clone()),
            Json(super::CouncilOpenRequest {
                topic: "WayVR integration".to_string(),
                participants: Some(vec!["athena".to_string(), "hades".to_string()]),
            }),
        )
        .await
        .into_response();
        let open_payload = json_body(open_response).await;
        let session_id = open_payload["session"]["session_id"]
            .as_str()
            .expect("session id")
            .to_string();
        assert_eq!(open_payload["ok"], true);
        assert_eq!(open_payload["session"]["topic"], "WayVR integration");

        let report_response = super::council_report(
            State(service.clone()),
            Json(super::CouncilReportRequest {
                session_id: session_id.clone(),
                from_agent: "athena".to_string(),
                body: "corpus depth is high".to_string(),
            }),
        )
        .await
        .into_response();
        let report_payload = json_body(report_response).await;
        assert_eq!(report_payload["ok"], true);
        assert_eq!(report_payload["report"]["reported_by"], "athena");

        let close_response = super::council_close(
            State(service),
            Json(super::CouncilCloseRequest {
                session_id,
                outcome: Some("proceed with delegation".to_string()),
            }),
        )
        .await
        .into_response();
        let close_payload = json_body(close_response).await;
        assert_eq!(close_payload["ok"], true);
        assert_eq!(close_payload["close"]["closed"], true);
        assert_eq!(close_payload["close"]["outcome"], "proceed with delegation");
    }
}
