// sigil: REPAIR
use crate::{ApolloService, InterruptionAttachmentRequest};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::try_run_bounded_async;
use axum::extract::{Request, State};
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
struct SubmitRequest {
    task_id: String,
    agent_id: String,
    payload: Option<Value>,
    priority: Option<String>,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ExecuteRequest {
    task_id: String,
}

#[derive(Debug, Deserialize)]
struct InterruptRequest {
    task_id: String,
    source: Option<String>,
    sender: Option<String>,
    content: Option<String>,
    disposition: Option<String>,
    run_id: Option<String>,
    session_id: Option<String>,
}

pub async fn run_http_server(service: ApolloService, addr: &str) -> Result<()> {
    let app = build_router(service);
    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "apollo".to_string(),
                message: format!("failed to bind HTTP listener on {addr}: {e}"),
            })?;
    tracing::info!(addr = %addr, "APOLLO HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "apollo".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

pub fn build_router(service: ApolloService) -> Router {
    Router::new()
        .route("/health", get(status))
        .route("/status", get(status))
        .route("/submit", post(submit))
        .route("/execute", post(execute))
        .route("/interrupt", post(interrupt))
        .route("/paths", get(paths))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("apollo_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "APOLLO HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ANNUNIMAS_APOLLO_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn status(State(service): State<ApolloService>) -> impl IntoResponse {
    map_result_async(async move { service.status().await }).await
}

async fn submit(
    State(service): State<ApolloService>,
    Json(req): Json<SubmitRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        Ok(json!({
            "task_id": service.submit(crate::ExecutionRequest {
                task_id: req.task_id,
                agent_id: req.agent_id,
                payload: req.payload.unwrap_or_else(|| json!({})),
                priority: match req.priority.unwrap_or_else(|| "normal".to_string()).to_ascii_lowercase().as_str() {
                    "low" => crate::ExecutionPriority::Low,
                    "high" => crate::ExecutionPriority::High,
                    "critical" => crate::ExecutionPriority::Critical,
                    _ => crate::ExecutionPriority::Normal
                },
                timeout_secs: req.timeout_secs.unwrap_or(60),
            }).await?
        }))
    }).await
}

async fn execute(
    State(service): State<ApolloService>,
    Json(req): Json<ExecuteRequest>,
) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"result": service.execute(&req.task_id).await?})) })
        .await
}

async fn interrupt(
    State(service): State<ApolloService>,
    Json(req): Json<InterruptRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        Ok(json!({
            "interrupt": service.attach_interrupt(InterruptionAttachmentRequest {
                task_id: &req.task_id,
                source: req.source.as_deref().unwrap_or("http"),
                sender: req.sender.as_deref().unwrap_or("operator"),
                content: req.content.as_deref().unwrap_or("interrupt"),
                disposition: req.disposition.as_deref().unwrap_or("note"),
                run_id: req.run_id,
                session_id: req.session_id,
            }).await?
        }))
    })
    .await
}

async fn paths(State(service): State<ApolloService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.runtime_paths()}))
}

async fn events(
    State(service): State<ApolloService>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5)))
        .then(move |_| {
            let service = service.clone();
            async move {
                let payload = service
                    .status()
                    .await
                    .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
                let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string());
                Ok(Event::default().event("status").data(data))
            }
        });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn map_result_async<F>(future: F) -> Json<Value>
where
    F: std::future::Future<Output = anyhow::Result<Value>>,
{
    match future.await {
        Ok(v) => Json(v),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_router, execute, http_request_limit, interrupt, paths, status, submit,
        ExecuteRequest, InterruptRequest, SubmitRequest,
    };
    use crate::ApolloService;
    use axum::body::{to_bytes, Body};
    use axum::extract::State;
    use axum::http::{header, Request, StatusCode};
    use axum::response::IntoResponse;
    use axum::Json;
    use serde_json::{json, Value};
    use tokio::time::{timeout, Duration};
    use tokio_stream::StreamExt;
    use tower::ServiceExt;

    async fn response_json(response: axum::response::Response) -> Value {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        serde_json::from_slice(&body).expect("json body")
    }

    #[tokio::test]
    async fn http_request_limit_uses_env_when_valid_and_falls_back_otherwise() {
        // SAFETY: warden-owned by `annunimas-apollo` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ANNUNIMAS_APOLLO_HTTP_MAX_CONCURRENCY", "9");
        }
        assert_eq!(http_request_limit(), 9);

        // SAFETY: warden-owned by `annunimas-apollo` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ANNUNIMAS_APOLLO_HTTP_MAX_CONCURRENCY", "0");
        }
        assert_eq!(http_request_limit(), 16);

        // SAFETY: warden-owned by `annunimas-apollo` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ANNUNIMAS_APOLLO_HTTP_MAX_CONCURRENCY", "bad");
        }
        assert_eq!(http_request_limit(), 16);

        // SAFETY: warden-owned by `annunimas-apollo` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ANNUNIMAS_APOLLO_HTTP_MAX_CONCURRENCY");
        }
        assert_eq!(http_request_limit(), 16);
    }

    #[tokio::test]
    async fn submit_and_execute_handlers_round_trip_task() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");

        let submit_body = response_json(
            submit(
                State(service.clone()),
                Json(SubmitRequest {
                    task_id: "task_http".to_string(),
                    agent_id: "apollo".to_string(),
                    payload: Some(json!({"repository_path": "/tmp/repo"})),
                    priority: Some("high".to_string()),
                    timeout_secs: Some(45),
                }),
            )
            .await
            .into_response(),
        )
        .await;
        assert_eq!(submit_body["task_id"], "task_http");

        let execute_body = response_json(
            execute(
                State(service),
                Json(ExecuteRequest {
                    task_id: "task_http".to_string(),
                }),
            )
            .await
            .into_response(),
        )
        .await;

        assert_eq!(execute_body["result"]["task_id"], "task_http");
        assert_eq!(execute_body["result"]["status"], "Completed");
        assert_eq!(execute_body["result"]["agent_id"], "apollo");
        assert_eq!(
            execute_body["result"]["output"]["harness_policy"]["approval_required"],
            true
        );
    }

    #[tokio::test]
    async fn interrupt_handler_uses_default_metadata_and_reports_record() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        service
            .submit(crate::ExecutionRequest {
                task_id: "task_interrupt".to_string(),
                agent_id: "hermes".to_string(),
                payload: json!({"op":"message"}),
                priority: crate::ExecutionPriority::Normal,
                timeout_secs: 30,
            })
            .await
            .expect("submit");

        let body = response_json(
            interrupt(
                State(service),
                Json(InterruptRequest {
                    task_id: "task_interrupt".to_string(),
                    source: None,
                    sender: None,
                    content: None,
                    disposition: None,
                    run_id: None,
                    session_id: None,
                }),
            )
            .await
            .into_response(),
        )
        .await;

        assert_eq!(body["interrupt"]["task_id"], "task_interrupt");
        assert_eq!(body["interrupt"]["source"], "http");
        assert_eq!(body["interrupt"]["sender"], "operator");
        assert_eq!(body["interrupt"]["content"], "interrupt");
        assert_eq!(body["interrupt"]["disposition"], "note");
    }

    #[tokio::test]
    async fn status_and_paths_handlers_surface_runtime_metadata() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");

        let status_body = response_json(status(State(service.clone())).await.into_response()).await;
        assert_eq!(status_body["authority"], "apollo_service");
        assert_eq!(
            status_body["paths"]["home"],
            temp.path().to_string_lossy().to_string()
        );

        let paths_body = response_json(paths(State(service)).await.into_response()).await;
        assert_eq!(paths_body["ok"], true);
        assert_eq!(
            paths_body["paths"]["requests_path"],
            temp.path()
                .join("pending_requests.json")
                .to_string_lossy()
                .to_string()
        );
    }

    #[tokio::test]
    async fn health_route_aliases_status_for_supervision() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        let app = build_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .expect("health request"),
            )
            .await
            .expect("health response");

        assert_eq!(response.status(), StatusCode::OK);
        let body = response_json(response).await;
        assert_eq!(body["authority"], "apollo_service");
        assert_eq!(body["schema_version"], "annunimas.apollo.runtime.v1");
    }

    #[tokio::test]
    async fn events_route_streams_status_sse_frames() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        let app = build_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/events")
                    .body(Body::empty())
                    .expect("events request"),
            )
            .await
            .expect("events response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&header::HeaderValue::from_static("text/event-stream"))
        );

        let mut stream = response.into_body().into_data_stream();
        let first_chunk = timeout(Duration::from_secs(1), stream.next())
            .await
            .expect("first sse frame ready")
            .expect("stream item")
            .expect("frame bytes");
        let frame = String::from_utf8_lossy(&first_chunk);

        assert!(frame.contains("event: status"));
        assert!(frame.contains("\"authority\":\"apollo_service\""));
        assert!(frame.contains("\"schema_version\":\"annunimas.apollo.runtime.v1\""));
    }

    #[tokio::test]
    async fn submit_route_rejects_malformed_json_payload() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        let app = build_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/submit")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"task_id":"bad","agent_id":"apollo""#))
                    .expect("submit request"),
            )
            .await
            .expect("submit response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn submit_route_rejects_missing_required_fields() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = ApolloService::from_home(temp.path()).expect("service");
        let app = build_router(service);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/submit")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"agent_id":"apollo"}"#))
                    .expect("submit request"),
            )
            .await
            .expect("submit response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }
}
