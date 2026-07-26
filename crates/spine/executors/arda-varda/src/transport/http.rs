// sigil: REPAIR
use crate::ingest::AthenaStore;
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
use axum::extract::{DefaultBodyLimit, Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::Stream;
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize)]
struct DigestParams {
    source_id: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PolicyReadinessParams {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct IngestRequest {
    input: String,
    submitted_by: Option<String>,
    task_context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IngestBatchRequest {
    inputs: Vec<String>,
    submitted_by: Option<String>,
    task_context: Option<String>,
}

#[derive(Debug, Deserialize)]
struct QueryRequest {
    query: String,
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct DeepRequest {
    source_id: String,
    reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeepProcessRequest {
    limit: Option<usize>,
    retry_failed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ScholarlyReenrichRequest {
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct PolicyPromoteRequest {
    limit: Option<usize>,
    reevaluate: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct HarvestOppositionRequest {
    source_id: String,
    topic: Option<String>,
    submitted_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratePlanningTasksRequest {
    source_id: String,
    limit: Option<usize>,
}

pub async fn run_http_server(store: AthenaStore, addr: &str) -> Result<()> {
    let app = build_router(store);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("failed to bind HTTP listener on {addr}: {e}"),
        })?;

    tracing::info!(addr = %addr, "ATHENA HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

pub fn build_router(store: AthenaStore) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/metrics", get(metrics))
        .route("/ingest", post(ingest))
        .route("/ingest_batch", post(ingest_batch))
        .route("/query", post(query))
        .route("/query/stream", post(query_stream))
        .route("/deep_analyze", post(deep_analyze))
        .route("/deep_process", post(deep_process))
        .route("/scholarly_reenrich", post(scholarly_reenrich))
        .route("/digest", get(digest))
        .route("/policy_readiness", get(policy_readiness))
        .route("/policy_promote", post(policy_promote))
        .route("/harvest_opposition", post(harvest_opposition))
        .route("/generate_planning_tasks", post(generate_planning_tasks))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_timeout_gate))
        .layer(middleware::from_fn(http_admission_gate))
        .layer(DefaultBodyLimit::max(http_request_body_limit()))
        .with_state(store)
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("athena_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "ATHENA HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

async fn http_timeout_gate(req: Request, next: Next) -> Response {
    // SSE /events stream is a long-lived response; skip the per-request timeout
    // for it so clients can hold the connection open.
    if matches!(req.uri().path(), "/events" | "/query/stream") {
        return next.run(req).await;
    }
    let deadline = Duration::from_secs(http_request_timeout_seconds());
    match timeout(deadline, next.run(req)).await {
        Ok(response) => response,
        Err(_) => (
            StatusCode::REQUEST_TIMEOUT,
            Json(json!({
                "ok": false,
                "error": "ATHENA HTTP request timeout",
                "timeout_seconds": deadline.as_secs(),
            })),
        )
            .into_response(),
    }
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_ATHENA_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(24)
}

fn http_request_body_limit() -> usize {
    std::env::var("ARDA_ATHENA_HTTP_BODY_LIMIT_BYTES")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2 * 1024 * 1024)
}

fn http_request_timeout_seconds() -> u64 {
    std::env::var("ARDA_ATHENA_HTTP_REQUEST_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30)
}

async fn status(State(store): State<AthenaStore>) -> impl IntoResponse {
    map_result(|| Ok(json!({"ok": true, "status": store.status()?})))
}

async fn metrics(State(store): State<AthenaStore>) -> impl IntoResponse {
    let _ = store.status();
    (
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        store.metrics().render_prometheus(),
    )
}

async fn ingest(
    State(store): State<AthenaStore>,
    Json(req): Json<IngestRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let result = store.ingest(
            &req.input,
            req.submitted_by.as_deref().unwrap_or("http"),
            req.task_context.as_deref().unwrap_or("http ingest"),
        )?;
        Ok(json!({"ok": true, "record": result}))
    })
}

async fn ingest_batch(
    State(store): State<AthenaStore>,
    Json(req): Json<IngestBatchRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let result = store.ingest_batch(
            &req.inputs,
            req.submitted_by.as_deref().unwrap_or("http"),
            req.task_context.as_deref().unwrap_or("http batch ingest"),
        )?;
        Ok(json!({"ok": true, "report": result}))
    })
}

async fn query(
    State(store): State<AthenaStore>,
    Json(req): Json<QueryRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let response = store.query(&req.query, req.limit.unwrap_or(8))?;
        Ok(json!({"ok": true, "query": response}))
    })
}

async fn query_stream(
    State(store): State<AthenaStore>,
    Json(req): Json<QueryRequest>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let query = req.query;
    let limit = req.limit.unwrap_or(8);
    let (tx, rx) = tokio::sync::mpsc::channel::<std::result::Result<Event, Infallible>>(8);

    tokio::task::spawn_blocking(move || {
        let mut rank = 0usize;
        let result = store.stream_query_matches(&query, limit, |matched| {
            rank += 1;
            let payload = json!({
                "stream_version": "athena.query.v1",
                "query": query,
                "rank": rank,
                "match": matched,
            });
            tx.blocking_send(Ok(Event::default()
                .event("match")
                .data(payload.to_string())))
                .is_ok()
        });

        let event = match result {
            Ok(total_matches) => Event::default().event("complete").data(
                json!({
                    "stream_version": "athena.query.v1",
                    "query": query,
                    "total_matches": total_matches,
                })
                .to_string(),
            ),
            Err(err) => Event::default().event("error").data(
                json!({
                    "stream_version": "athena.query.v1",
                    "query": query,
                    "error": err.to_string(),
                })
                .to_string(),
            ),
        };
        let _ = tx.blocking_send(Ok(event));
    });

    let stream = futures::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|event| (event, rx))
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn deep_analyze(
    State(store): State<AthenaStore>,
    Json(req): Json<DeepRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let queued = store.queue_deep_analysis(
            &req.source_id,
            "http",
            req.reason.as_deref().unwrap_or("http deep request"),
        )?;
        let deep = store.deep_analyze(&req.source_id)?;
        Ok(json!({"ok": true, "queued": queued, "deep": deep}))
    })
}

async fn deep_process(
    State(store): State<AthenaStore>,
    Json(req): Json<DeepProcessRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let out =
            store.process_deep_queue(req.limit.unwrap_or(25), req.retry_failed.unwrap_or(false))?;
        Ok(json!({"ok": true, "result": out}))
    })
}

async fn scholarly_reenrich(
    State(store): State<AthenaStore>,
    Json(req): Json<ScholarlyReenrichRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let out = store.process_scholarly_reenrichment_queue(req.limit.unwrap_or(25))?;
        Ok(json!({"ok": true, "result": out}))
    })
}

async fn digest(
    State(store): State<AthenaStore>,
    Query(params): Query<DigestParams>,
) -> impl IntoResponse {
    map_result(|| {
        let items = store.read_digest(params.source_id.as_deref(), params.limit.unwrap_or(25))?;
        Ok(json!({"ok": true, "digest": items}))
    })
}

async fn policy_readiness(
    State(store): State<AthenaStore>,
    Query(params): Query<PolicyReadinessParams>,
) -> impl IntoResponse {
    map_result(|| {
        let items = store.policy_readiness(params.limit.unwrap_or(25))?;
        Ok(json!({"ok": true, "policy_readiness": items}))
    })
}

async fn policy_promote(
    State(store): State<AthenaStore>,
    Json(req): Json<PolicyPromoteRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let out = store
            .promote_policy_readiness(req.limit.unwrap_or(25), req.reevaluate.unwrap_or(false))?;
        Ok(json!({"ok": true, "result": out}))
    })
}

async fn harvest_opposition(
    State(store): State<AthenaStore>,
    Json(req): Json<HarvestOppositionRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let out = store.harvest_opposition_evidence(
            &req.source_id,
            req.topic.as_deref(),
            req.submitted_by.as_deref().unwrap_or("http"),
        )?;
        Ok(json!({"ok": true, "result": out}))
    })
}

async fn generate_planning_tasks(
    State(store): State<AthenaStore>,
    Json(req): Json<GeneratePlanningTasksRequest>,
) -> impl IntoResponse {
    map_result(|| {
        let out = store.generate_planning_tasks(&req.source_id, req.limit.unwrap_or(5))?;
        Ok(json!({"ok": true, "result": out}))
    })
}

async fn events(
    State(store): State<AthenaStore>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5))).map(
        move |_| {
            let payload = build_event_payload(&store)
                .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
            let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
            Ok(Event::default().event("status").data(data))
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

fn build_event_payload(store: &AthenaStore) -> Result<Value> {
    let status = store.status()?;
    let recent_digest = store.read_digest(None, 8)?;
    let recent_policy = store.policy_readiness(8)?;
    let recent_deep_queue = store.recent_deep_queue_events(8)?;
    let recent_deep_graph = store.recent_deep_graph_events(8)?;
    let mut warnings = Vec::new();
    if status.deep_queue_depth > 100 {
        warnings.push(json!({
            "kind": "deep_queue_backlog",
            "severity": "warning",
            "message": format!("ATHENA deep queue backlog above threshold: {}", status.deep_queue_depth),
            "threshold": 100
        }));
    }
    if status.deep_queue_failed > 0 {
        warnings.push(json!({
            "kind": "deep_queue_failed",
            "severity": "warning",
            "message": format!("ATHENA deep queue has {} failed items", status.deep_queue_failed)
        }));
    }

    let latest_digest_at = recent_digest.iter().rev().find_map(event_ts_utc);
    let latest_policy_at = recent_policy.iter().rev().find_map(event_ts_utc);
    let latest_deep_queue_at = recent_deep_queue.iter().rev().find_map(event_ts_utc);
    let latest_deep_graph_at = recent_deep_graph.iter().rev().find_map(event_ts_utc);

    Ok(json!({
        "ok": true,
        "stream_version": "athena.events.v2",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "status": status,
        "knowledge": {
            "corpus": {
                "books_count": status.books_count,
                "digest_events": status.digest_events,
                "deep_graph_events": status.deep_graph_events,
            },
            "pipeline": {
                "active_crawls_total": status.active_crawls_total,
                "active_crawls": status.active_crawls,
                "deep_queue_depth": status.deep_queue_depth,
                "deep_queue_failed": status.deep_queue_failed,
                "policy_ready_count": status.policy_ready_count,
                "reference_only_count": status.reference_only_count,
                "recent_completed_pipelines": status.recent_completed_pipelines,
                "last_error": status.last_activity_error,
            },
            "latest_activity": {
                "digest_at_utc": latest_digest_at,
                "policy_at_utc": latest_policy_at,
                "deep_queue_at_utc": latest_deep_queue_at,
                "deep_graph_at_utc": latest_deep_graph_at,
            },
            "recent_digest": recent_digest,
            "recent_policy_readiness": recent_policy,
            "recent_deep_queue": recent_deep_queue,
            "recent_deep_graph": recent_deep_graph,
            "warnings": warnings,
        },
        "arda_hints": {
            "primary_panel": "knowledge",
            "suggested_views": [
                "digest_activity",
                "policy_readiness",
                "deep_pipeline",
                "knowledge_graph"
            ]
        }
    }))
}

fn event_ts_utc(value: &Value) -> Option<String> {
    value
        .get("ts_utc")
        .or_else(|| value.get("processed_at_utc"))
        .or_else(|| value.get("received_at_utc"))
        .or_else(|| value.get("ts"))
        .and_then(|v| v.as_str())
        .and_then(normalize_ts_utc)
}

fn normalize_ts_utc(raw: &str) -> Option<String> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|ts| ts.with_timezone(&Utc).to_rfc3339())
}

#[cfg(test)]
// Tests that mutate process environment variables must serialize across await
// points so no other test observes a partially configured runtime. This is
// test-scaffolding only; production code must not hold std mutex guards across
// async boundaries.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{build_event_payload, build_router};
    use crate::ingest::AthenaStore;
    use arda_core::try_run_bounded_async;
    use axum::body::to_bytes;
    use axum::body::Body;
    use axum::http::Request;
    use axum::http::StatusCode;
    use serde_json::Value;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use tempfile::tempdir;
    use tokio::sync::oneshot;
    use tower::ServiceExt;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_support::env_guard()
    }

    #[tokio::test]
    async fn http_contract_status_ingest_query() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let app = build_router(store);

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

        let ingest_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"input":"https://github.com/example/rust-api","submitted_by":"http-test","task_context":"contract"}"#,
                    ))
                    .expect("ingest request"),
            )
            .await
            .expect("ingest");
        assert_eq!(ingest_response.status(), StatusCode::OK);

        let refreshed_status = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("refreshed status request"),
            )
            .await
            .expect("refreshed status");
        let body = to_bytes(refreshed_status.into_body(), usize::MAX)
            .await
            .expect("refreshed status body");
        let value: Value = serde_json::from_slice(&body).expect("refreshed status json");
        assert_eq!(value["status"]["source_freshness_total"], 1);
        assert!(value["status"]["oldest_source_age_seconds"].is_number());
        assert_eq!(value["status"]["active_crawls_total"], 0);
        assert_eq!(
            value["status"]["recent_completed_pipelines"]
                .as_array()
                .map(Vec::len),
            Some(1)
        );
        assert_eq!(
            value["status"]["source_freshness"].as_array().map(Vec::len),
            Some(1)
        );

        let query_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/query")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"rust","limit":5}"#))
                    .expect("query request"),
            )
            .await
            .expect("query");
        assert_eq!(query_response.status(), StatusCode::OK);

        let body = to_bytes(query_response.into_body(), usize::MAX)
            .await
            .expect("query body");
        let value: Value = serde_json::from_slice(&body).expect("query json");
        let total_matches = value
            .get("query")
            .and_then(|q| q.get("total_matches"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        assert!(total_matches >= 1);
        assert_eq!(value["query"]["matches"][0]["shallow_only"], true);
    }

    #[tokio::test]
    async fn http_query_stream_emits_scored_matches_then_complete() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        store
            .ingest(
                "https://github.com/example/rust-streaming-api",
                "http-test",
                "streaming contract",
            )
            .expect("ingest");
        let app = build_router(store);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/query/stream")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"query":"rust","limit":5}"#))
                    .expect("streaming query request"),
            )
            .await
            .expect("streaming query response");

        assert_eq!(response.status(), StatusCode::OK);
        assert!(response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with("text/event-stream")));
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("streaming query body");
        let text = String::from_utf8(body.to_vec()).expect("SSE utf8");

        assert!(text.contains("event: match\n"), "SSE body: {text}");
        assert!(text.contains("\"score\":"), "SSE body: {text}");
        assert!(text.contains("\"citations\":"), "SSE body: {text}");
        assert!(text.contains("\"shallow_only\":true"), "SSE body: {text}");
        assert!(text.contains("event: complete\n"), "SSE body: {text}");
        assert!(text.contains("\"stream_version\":\"athena.query.v1\""));
        assert!(
            text.find("event: match\n").expect("match event")
                < text.find("event: complete\n").expect("complete event")
        );
    }

    #[tokio::test]
    async fn http_status_reports_a_live_production_crawl() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let app = build_router(store.clone());
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(err) => panic!("bind: {err}"),
        };
        let addr = listener.local_addr().expect("addr");
        let (accepted_tx, accepted_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).expect("read request");
            let _ = accepted_tx.send(());
            release_rx.recv().expect("release response");
            let body =
                r##"{"url":"https://example.com/live","markdown":"# Live\n","success":true}"##;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        let crawling_store = store.clone();
        let crawl = tokio::spawn(async move {
            crawling_store
                .crawl4ai_fetch_markdown(
                    &format!("http://{}", addr),
                    "https://example.com/live?token=secret",
                    "fit",
                    None,
                )
                .await
        });
        accepted_rx.await.expect("crawl accepted");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("status request"),
            )
            .await
            .expect("status");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("status body");
        let status: Value = serde_json::from_slice(&body).expect("status json");
        assert_eq!(status["status"]["active_crawls_total"], 1);
        assert_eq!(
            status["status"]["active_crawls"][0]["url"],
            "https://example.com/live"
        );

        release_tx.send(()).expect("release server");
        crawl.await.expect("crawl task").expect("crawl result");
        server.join().expect("join server");
    }

    #[tokio::test]
    async fn http_contract_metrics_endpoint_exports_prometheus_text() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        store
            .ingest(
                "https://github.com/example/rust-api",
                "http-test",
                "metrics contract",
            )
            .expect("ingest");
        store.metrics().observe_query(0.010);
        let app = build_router(store);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .expect("metrics request"),
            )
            .await
            .expect("metrics");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("metrics body");
        let text = String::from_utf8(body.to_vec()).expect("metrics utf8");
        assert!(text.contains("# TYPE athena_ingest_documents_total counter"));
        assert!(text.contains("athena_query_total 1"));
        assert!(text.contains("# TYPE athena_source_age_seconds gauge"));
        assert!(text.contains("athena_source_age_seconds{source_id=\"src_"));
    }

    #[tokio::test]
    async fn http_admission_gate_sheds_excess_burst_requests() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        std::env::set_var("ARDA_ATHENA_HTTP_MAX_CONCURRENCY", "1");
        let store = AthenaStore::new(dir.path()).expect("store");
        let app = build_router(store);
        let (tx, rx) = oneshot::channel::<()>();

        let holder = tokio::spawn(async move {
            let _ = try_run_bounded_async("athena_http_request", 1, || async move {
                let _ = rx.await;
            })
            .await;
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/status")
                    .body(Body::empty())
                    .expect("status request"),
            )
            .await
            .expect("status");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            value.get("error").and_then(Value::as_str),
            Some("ATHENA HTTP concurrency gate saturated")
        );

        let _ = tx.send(());
        holder.await.expect("holder");
        std::env::remove_var("ARDA_ATHENA_HTTP_MAX_CONCURRENCY");
    }

    #[tokio::test]
    async fn http_rejects_oversized_request_body() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        std::env::set_var("ARDA_ATHENA_HTTP_BODY_LIMIT_BYTES", "256");
        let store = AthenaStore::new(dir.path()).expect("store");
        let app = build_router(store);

        let mut oversized = String::from(r#"{"input":""#);
        oversized.push_str(&"x".repeat(2048));
        oversized.push_str(r#"","submitted_by":"overflow","task_context":"test"}"#);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/ingest")
                    .header("content-type", "application/json")
                    .body(Body::from(oversized))
                    .expect("oversized request"),
            )
            .await
            .expect("oversized dispatch");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        std::env::remove_var("ARDA_ATHENA_HTTP_BODY_LIMIT_BYTES");
    }

    #[tokio::test]
    async fn http_enforces_request_timeout() {
        use axum::middleware;
        use axum::routing::get;
        use axum::Router;

        let _guard = env_guard();
        std::env::set_var("ARDA_ATHENA_HTTP_REQUEST_TIMEOUT_SECS", "1");

        // Test the timeout middleware in isolation around a handler that
        // intentionally outlives the configured timeout.
        let app: Router = Router::new()
            .route(
                "/slow",
                get(|| async {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    "ok"
                }),
            )
            .layer(middleware::from_fn(super::http_timeout_gate));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/slow")
                    .body(Body::empty())
                    .expect("slow request"),
            )
            .await
            .expect("slow");
        assert_eq!(response.status(), StatusCode::REQUEST_TIMEOUT);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body");
        let value: Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(
            value.get("error").and_then(Value::as_str),
            Some("ATHENA HTTP request timeout")
        );
        assert_eq!(
            value.get("timeout_seconds").and_then(Value::as_u64),
            Some(1)
        );

        std::env::remove_var("ARDA_ATHENA_HTTP_REQUEST_TIMEOUT_SECS");
    }

    #[tokio::test]
    async fn http_timeout_gate_passes_through_events_path() {
        use axum::middleware;
        use axum::routing::get;
        use axum::Router;

        let _guard = env_guard();
        // Timeout would fire before the "slow" handler returns, but the
        // middleware must short-circuit the /events path.
        std::env::set_var("ARDA_ATHENA_HTTP_REQUEST_TIMEOUT_SECS", "1");
        let app: Router = Router::new()
            .route(
                "/events",
                get(|| async {
                    tokio::time::sleep(std::time::Duration::from_millis(1_500)).await;
                    "keep-alive"
                }),
            )
            .layer(middleware::from_fn(super::http_timeout_gate));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .expect("events request"),
            )
            .await
            .expect("events");
        assert_eq!(response.status(), StatusCode::OK);
        std::env::remove_var("ARDA_ATHENA_HTTP_REQUEST_TIMEOUT_SECS");
    }

    #[test]
    fn event_payload_includes_knowledge_snapshot_sections() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://example.com/governance-rust",
                "test",
                "event snapshot",
            )
            .expect("ingest");
        let _ = store
            .queue_deep_analysis(&record.id, "test", "queued for event payload")
            .expect("queue");
        let _ = store.deep_analyze(&record.id).expect("deep");

        let payload = build_event_payload(&store).expect("event payload");
        assert_eq!(
            payload.get("stream_version").and_then(|v| v.as_str()),
            Some("athena.events.v2")
        );
        assert_eq!(
            payload
                .get("knowledge")
                .and_then(|v| v.get("corpus"))
                .and_then(|v| v.get("books_count"))
                .and_then(|v| v.as_u64()),
            Some(1)
        );
        assert!(payload
            .get("knowledge")
            .and_then(|v| v.get("recent_digest"))
            .and_then(|v| v.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert!(payload
            .get("knowledge")
            .and_then(|v| v.get("recent_policy_readiness"))
            .and_then(|v| v.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert!(payload
            .get("knowledge")
            .and_then(|v| v.get("recent_deep_graph"))
            .and_then(|v| v.as_array())
            .map(|items| !items.is_empty())
            .unwrap_or(false));
        assert_eq!(payload["knowledge"]["pipeline"]["active_crawls_total"], 0);
        assert!(
            payload["knowledge"]["pipeline"]["recent_completed_pipelines"]
                .as_array()
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        );
    }
}
