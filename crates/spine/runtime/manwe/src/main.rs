#![allow(path_statements)]
//! `manwe` — single local OpenAI-compatible inference gateway.
//!
//! Default: listens on `127.0.0.1:7171` and serves `/v1/chat/completions`
//! + `/v1/models` against a static provider catalog. When `MANWE_ROUTING_MODE=adaptive`
//! is set, requests may flow through the routing adapter.

mod config;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde_json::{json, Value};

use manwe::routing_adapter::AdaptiveRoutingAdapter;
use config::ManweConfig;

#[derive(Parser, Debug)]
#[command(name = "manwe", version, about = "Local OpenAI-compat inference gateway")]
struct Cli {
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    bind: Option<String>,
    #[arg(long, default_value = "manwe.toml")]
    config: PathBuf,
}

#[derive(Clone)]
struct AppState {
    config: Arc<ManweConfig>,
    client: reqwest::Client,
    adapter: Arc<AdaptiveRoutingAdapter>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    let mut cfg = ManweConfig::load(&cli.config);
    if let Some(p) = cli.port {
        cfg.port = p;
    }
    if let Some(b) = cli.bind {
        cfg.bind = b;
    }

    let state = AppState {
        config: Arc::new(cfg.clone()),
        client: reqwest::Client::new(),
        adapter: Arc::new(AdaptiveRoutingAdapter::new()),
    };

    let app: Router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", cfg.bind, cfg.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind/port: {e}"))?;

    tracing::info!(
        "manwe: gateway listening on {addr} ({n} providers)",
        n = cfg.providers.len()
    );
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn list_models(State(state): State<AppState>) -> Response {
    let created = chrono::Utc::now().timestamp();
    let mut data: Vec<Value> = Vec::new();
    for (_name, p) in &state.config.providers {
        let models = if p.models.is_empty() {
            vec!["default".to_string()]
        } else {
            p.models.clone()
        };
        for m in models {
            data.push(json!({
                "id": m,
                "object": "model",
                "created": created,
                "owned_by": "manwe",
            }));
        }
    }
    Json(json!({ "object": "list", "data": data })).into_response()
}

async fn chat_completions(State(state): State<AppState>, Json(req): Json<Value>) -> Response {
    if state.adapter.route_chat_completions(req.clone()).is_ok() {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({
                "error": {
                    "message": "adaptive routing not wired yet",
                    "type": "manwe_error"
                }
            })),
        )
            .into_response();
    }

    fallback_static(state.config, state.client, req).await
}

async fn fallback_static(
    config: Arc<ManweConfig>,
    client: reqwest::Client,
    req: Value,
) -> Response {
    let Some(model) = req.get("model").and_then(|v| v.as_str()) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": { "message": "missing model", "type": "manwe_error" } })),
        )
            .into_response();
    };

    let Some((_, prov)) = config.resolve_provider(model) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": { "message": "manwe: no providers configured", "type": "manwe_error" } })),
        )
            .into_response();
    };

    let upstream = format!("{}/chat/completions", prov.base_url.trim_end_matches('/'));
    let mut request = client.post(&upstream).json(&req);
    if let Some(key) = &prov.api_key {
        request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }

    match request.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(v) => (status, Json(v)).into_response(),
                Err(_) => (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": { "message": "manwe: upstream returned non-JSON", "type": "manwe_error" } })),
                )
                    .into_response(),
            }
        }
        Err(_) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": { "message": "manwe: upstream unreachable", "type": "manwe_error" } })),
        )
            .into_response(),
    }
}
