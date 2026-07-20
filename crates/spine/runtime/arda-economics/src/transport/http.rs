// sigil: REPAIR
use crate::{CostModelConfig, JouleWorkUnit, PlutusService};
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
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
struct RegisterModelRequest {
    provider: String,
    input_rate: f64,
    output_rate: f64,
    batch_size: Option<usize>,
}
#[derive(Debug, Deserialize)]
struct RecordSpendRequest {
    provider: String,
    input_tokens: usize,
    output_tokens: usize,
}
#[derive(Debug, Deserialize)]
struct TrackWorkRequest {
    agent_id: String,
    amount: f64,
    unit: Option<String>,
    task_id: Option<String>,
}
#[derive(Debug, Deserialize)]
struct CreditRequest {
    account: String,
    amount: f64,
}
#[derive(Debug, Deserialize)]
struct RelationshipRequest {
    from: String,
    to: String,
    trust: f64,
    attention: f64,
    reciprocity: f64,
}

pub async fn run_http_server(service: PlutusService, addr: &str) -> Result<()> {
    let app = build_router(service);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "plutus".to_owned(),
            message: format!("failed to bind HTTP listener on {addr}: {e}"),
        })?;
    tracing::info!(addr = %addr, "PLUTUS HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "plutus".to_owned(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

pub fn build_router(service: PlutusService) -> Router {
    Router::new()
        .route("/status", get(status))
        .route("/register_model", post(register_model))
        .route("/record_spend", post(record_spend))
        .route("/track_work", post(track_work))
        .route("/credit", post(credit))
        .route("/relationship", post(relationship))
        .route("/paths", get(paths))
        .route("/events", get(events))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service)
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("plutus_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "PLUTUS HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_PLUTUS_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn status(State(service): State<PlutusService>) -> impl IntoResponse {
    map_result_async(async move { service.status().await }).await
}
async fn register_model(
    State(service): State<PlutusService>,
    Json(req): Json<RegisterModelRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        service
            .register_model(CostModelConfig {
                provider: req.provider,
                input_rate: req.input_rate,
                output_rate: req.output_rate,
                batch_size: req.batch_size.unwrap_or(1000),
            })
            .await?;
        Ok(json!({"registered": true}))
    })
    .await
}
async fn record_spend(
    State(service): State<PlutusService>,
    Json(req): Json<RecordSpendRequest>,
) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"cost": service.record_spend(&req.provider, req.input_tokens, req.output_tokens).await?})) }).await
}
async fn track_work(
    State(service): State<PlutusService>,
    Json(req): Json<TrackWorkRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        let unit = match req
            .unit
            .unwrap_or_else(|| "reasoning".to_owned())
            .to_ascii_lowercase()
            .as_str()
        {
            "compute" => JouleWorkUnit::Compute,
            "network" => JouleWorkUnit::Network,
            "storage" => JouleWorkUnit::Storage,
            "attention" => JouleWorkUnit::Attention,
            _ => JouleWorkUnit::Reasoning,
        };
        service
            .track_work(&req.agent_id, req.amount, unit, req.task_id)
            .await?;
        Ok(json!({"tracked": true}))
    })
    .await
}
async fn credit(
    State(service): State<PlutusService>,
    Json(req): Json<CreditRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        service.credit(&req.account, req.amount).await?;
        Ok(json!({"credited": true}))
    })
    .await
}
async fn relationship(
    State(service): State<PlutusService>,
    Json(req): Json<RelationshipRequest>,
) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"score": service.record_relationship(&req.from, &req.to, req.trust, req.attention, req.reciprocity).await?})) }).await
}
async fn paths(State(service): State<PlutusService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.runtime_paths()}))
}
async fn events(
    State(service): State<PlutusService>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5)))
        .then(move |_| {
            let service = service.clone();
            async move {
                let payload = service
                    .status()
                    .await
                    .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
                let data = serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_owned());
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
    use super::build_router;
    use crate::PlutusService;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn http_contract_status_track_work_and_paths_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = PlutusService::from_home(dir.path()).expect("service");
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

        let track_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/track_work")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"agent_id":"athena","amount":2.0,"unit":"reasoning","task_id":"task_http_1"}"#,
                    ))
                    .expect("track request"),
            )
            .await
            .expect("track");
        assert_eq!(track_response.status(), StatusCode::OK);

        let paths_response = app
            .oneshot(
                Request::builder()
                    .uri("/paths")
                    .body(Body::empty())
                    .expect("paths request"),
            )
            .await
            .expect("paths");
        assert_eq!(paths_response.status(), StatusCode::OK);

        let body = to_bytes(paths_response.into_body(), usize::MAX)
            .await
            .expect("paths body");
        let value: Value = serde_json::from_slice(&body).expect("paths json");
        assert_eq!(value["ok"], true);
        assert!(value["paths"]["status_path"].as_str().is_some());
    }
}
