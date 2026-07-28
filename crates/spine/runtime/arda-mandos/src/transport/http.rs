// sigil: REPAIR
use super::dispatch::{
    dispatch, DispatchError, DispatchRequest, EvaluateRequest, ExportLedgerRequest,
};
use crate::OracleService;
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
use axum::extract::rejection::JsonRejection;
use axum::extract::{DefaultBodyLimit, Query, Request, State};
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
struct VerdictsParams {
    limit: Option<usize>,
}

pub async fn run_http_server(service: OracleService, addr: &str) -> Result<()> {
    let app = build_router(service);
    let listener = tokio::net::TcpListener::bind(addr)
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
        .route("/ledger/verify", get(verify_ledger))
        .route("/ledger/export", post(export_ledger))
        .route("/events", get(events))
        .layer(DefaultBodyLimit::max(1024 * 1024))
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
        return http_saturation_response();
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
    map_dispatch(dispatch(&service, DispatchRequest::Status).await)
}
async fn evaluate(
    State(service): State<OracleService>,
    request: std::result::Result<Json<EvaluateRequest>, JsonRejection>,
) -> impl IntoResponse {
    let req = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            let error = if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                DispatchError::payload_too_large(1024 * 1024)
            } else {
                DispatchError::invalid_request(rejection.body_text())
            };
            return map_dispatch(Err(error));
        }
    };
    map_dispatch(
        dispatch(
            &service,
            DispatchRequest::Evaluate {
                request: req,
                id_prefix: "oracle_http",
            },
        )
        .await,
    )
}
async fn verdicts(
    State(service): State<OracleService>,
    Query(params): Query<VerdictsParams>,
) -> impl IntoResponse {
    match dispatch(
        &service,
        DispatchRequest::Verdicts {
            limit: params.limit.unwrap_or(10),
        },
    )
    .await
    {
        Ok(verdicts) => (
            StatusCode::OK,
            Json(json!({"ok": true, "verdicts": verdicts})),
        )
            .into_response(),
        Err(error) => map_dispatch(Err(error)),
    }
}
async fn paths(State(service): State<OracleService>) -> impl IntoResponse {
    match dispatch(&service, DispatchRequest::Paths).await {
        Ok(paths) => (StatusCode::OK, Json(json!({"ok": true, "paths": paths}))).into_response(),
        Err(error) => map_dispatch(Err(error)),
    }
}
async fn verify_ledger(State(service): State<OracleService>) -> impl IntoResponse {
    map_dispatch(dispatch(&service, DispatchRequest::VerifyLedger).await)
}
async fn export_ledger(
    State(service): State<OracleService>,
    request: std::result::Result<Json<ExportLedgerRequest>, JsonRejection>,
) -> impl IntoResponse {
    let request = match request {
        Ok(Json(request)) => request,
        Err(rejection) => {
            let error = if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
                DispatchError::payload_too_large(1024 * 1024)
            } else {
                DispatchError::invalid_request(rejection.body_text())
            };
            return map_dispatch(Err(error));
        }
    };
    map_dispatch(
        dispatch(
            &service,
            DispatchRequest::ExportLedger {
                destination: request.destination,
            },
        )
        .await,
    )
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
fn map_dispatch(result: std::result::Result<Value, DispatchError>) -> Response {
    match result {
        Ok(v) => (StatusCode::OK, Json(v)).into_response(),
        Err(error) => (
            StatusCode::from_u16(error.http_status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            Json(error.body()),
        )
            .into_response(),
    }
}

fn http_saturation_response() -> Response {
    map_dispatch(Err(DispatchError {
        code: "SERVICE_UNAVAILABLE",
        message: "ORACLE HTTP concurrency gate saturated".to_string(),
        http_status: 503,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EvidenceRef;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use chrono::Utc;
    use tower::ServiceExt;

    #[tokio::test]
    async fn http_saturation_uses_the_shared_structured_error_envelope() {
        let response = http_saturation_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let error: Value = serde_json::from_slice(&body).expect("error JSON");
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "SERVICE_UNAVAILABLE");
        assert_eq!(
            error["error"]["message"],
            "ORACLE HTTP concurrency gate saturated"
        );
    }

    #[tokio::test]
    async fn http_evaluate_accepts_typed_evidence_and_redacts_sensitive_excerpt() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let evidence = EvidenceRef::supplied(
            "http-report",
            "http-fixture://report",
            Utc::now(),
            "http-sensitive excerpt",
        )
        .with_sensitive_excerpt(false);
        let request = Request::builder()
            .method("POST")
            .uri("/evaluate")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_vec(&json!({
                    "id": "http-evidence-query",
                    "task": "review deployment evidence",
                    "requester": "operator",
                    "evidence": [evidence]
                }))
                .expect("request body"),
            ))
            .expect("request");

        let response = build_router(service)
            .oneshot(request)
            .await
            .expect("HTTP response");
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let verdict: Value = serde_json::from_slice(&body).expect("verdict JSON");

        assert_eq!(
            verdict["gates"]["bacon"]["evidence"][0]["evidence"]["source_id"],
            "http-report"
        );
        assert_eq!(
            verdict["gates"]["bacon"]["evidence"][0]["evidence"]["excerpt"],
            "[REDACTED]"
        );
        assert!(!String::from_utf8_lossy(&body).contains("http-sensitive excerpt"));
    }

    #[tokio::test]
    async fn http_invalid_query_returns_actionable_client_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let request = Request::builder()
            .method("POST")
            .uri("/evaluate")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"id":"bad-query","task":"   "}"#))
            .expect("request");

        let response = build_router(service)
            .oneshot(request)
            .await
            .expect("HTTP response");
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let error: Value = serde_json::from_slice(&body).expect("error JSON");
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "INVALID_QUERY");
        assert!(error["error"]["message"]
            .as_str()
            .unwrap_or_default()
            .contains("task"));
    }

    #[tokio::test]
    async fn http_rejects_oversized_payload_with_structured_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let request = Request::builder()
            .method("POST")
            .uri("/evaluate")
            .header("content-type", "application/json")
            .body(Body::from(vec![b'x'; 1024 * 1024 + 1]))
            .expect("request");

        let response = build_router(service)
            .oneshot(request)
            .await
            .expect("HTTP response");
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let error: Value = serde_json::from_slice(&body).expect("error JSON");
        assert_eq!(error["error"]["code"], "PAYLOAD_TOO_LARGE");
    }

    #[tokio::test]
    async fn http_rejects_malformed_json_with_structured_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let request = Request::builder()
            .method("POST")
            .uri("/evaluate")
            .header("content-type", "application/json")
            .body(Body::from("{"))
            .expect("request");

        let response = build_router(service)
            .oneshot(request)
            .await
            .expect("HTTP response");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body");
        let error: Value = serde_json::from_slice(&body).expect("error JSON");
        assert_eq!(error["error"]["code"], "INVALID_REQUEST");
    }

    #[tokio::test]
    async fn http_verifies_and_exports_the_authoritative_ledger() {
        let temp = tempfile::tempdir().expect("tempdir");
        let service = OracleService::from_home(temp.path())
            .await
            .expect("service");
        let evaluate = Request::builder()
            .method("POST")
            .uri("/evaluate")
            .header("content-type", "application/json")
            .body(Body::from(
                r#"{"id":"http-export","task":"review export evidence"}"#,
            ))
            .expect("request");
        build_router(service.clone())
            .oneshot(evaluate)
            .await
            .expect("evaluate response");

        let verify = Request::builder()
            .uri("/ledger/verify")
            .body(Body::empty())
            .expect("verify request");
        let verify_response = build_router(service.clone())
            .oneshot(verify)
            .await
            .expect("verify response");
        assert_eq!(verify_response.status(), StatusCode::OK);
        let verify_body = to_bytes(verify_response.into_body(), usize::MAX)
            .await
            .expect("verify body");
        let report: Value = serde_json::from_slice(&verify_body).expect("verification report");
        assert_eq!(report["valid"], true);
        assert_eq!(report["valid_records"], 1);

        let destination = temp.path().join("exports/http-export.jsonl");
        let export = Request::builder()
            .method("POST")
            .uri("/ledger/export")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({"destination": "http-export.jsonl"}).to_string(),
            ))
            .expect("export request");
        let export_response = build_router(service)
            .oneshot(export)
            .await
            .expect("export response");
        assert_eq!(export_response.status(), StatusCode::OK);
        assert!(destination.exists());
    }
}
