// sigil: REPAIR
use crate::{OracleQuery, OracleService};
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
use axum::extract::{Query, Request, State};
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use std::convert::Infallible;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};

#[derive(Debug, Deserialize)]
struct EvaluateRequest {
    id: Option<String>,
    task: String,
    requester: Option<String>,
    context: Option<Vec<String>>,
}
#[derive(Debug, Deserialize)]
struct VerdictsParams {
    limit: Option<usize>,
}

pub async fn run_http_server(service: OracleService, addr: &str) -> Result<()> {
    let app = build_router(service);
    let listener =
        tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "oracle".to_string(),
                message: format!("failed to bind HTTP listener on {addr}: {e}"),
            })?;
    tracing::info!(addr = %addr, "ORACLE HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

pub fn build_router(service: OracleService) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/evaluate", post(evaluate))
        .route("/verdicts", get(verdicts))
        .route("/paths", get(paths))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("oracle_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "ORACLE HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_MANDOS_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn status(State(service): State<OracleService>) -> impl IntoResponse {
    map_result_async(async move { service.status().await }).await
}
async fn evaluate(
    State(service): State<OracleService>,
    Json(req): Json<EvaluateRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        Ok(serde_json::to_value(
            service
                .evaluate(OracleQuery {
                    id: req.id.unwrap_or_else(|| "oracle_http".to_string()),
                    task: req.task,
                    context: req.context.unwrap_or_default(),
                    requester: req.requester.unwrap_or_else(|| "operator".to_string()),
                    timestamp: Utc::now(),
                })
                .await?,
        )?)
    })
    .await
}
async fn verdicts(
    State(service): State<OracleService>,
    Query(params): Query<VerdictsParams>,
) -> impl IntoResponse {
    match service.recent_verdicts(params.limit.unwrap_or(10)) {
        Ok(v) => Json(json!({"ok": true, "verdicts": v})),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}
async fn paths(State(service): State<OracleService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.runtime_paths()}))
}
async fn events(
    State(service): State<OracleService>,
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
