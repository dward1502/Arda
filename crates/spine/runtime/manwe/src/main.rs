#![allow(path_statements)]
//! `manwe` — single local OpenAI-compatible inference gateway.
//!
//! Default: listens on `127.0.0.1:7171` and serves `/v1/chat/completions`
//! + `/v1/models` against a static provider catalog. When `MANWE_ROUTING_MODE=adaptive`
//! or `--adaptive` is set, requests may flow through the routing adapter.

mod config;
mod grpc;

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

use config::ManweConfig;
use manwe::routing_adapter::AdaptiveRoutingAdapter;

#[derive(Parser, Debug)]
#[command(
    name = "manwe",
    version,
    about = "Local OpenAI-compat inference gateway"
)]
struct Cli {
    #[arg(long)]
    port: Option<u16>,
    #[arg(long)]
    bind: Option<String>,
    #[arg(long, default_value = "manwe.toml")]
    config: PathBuf,
    #[arg(long)]
    adaptive: bool,
    #[arg(long)]
    grpc: bool,
}

#[derive(Clone)]
struct AppState {
    config: Arc<ManweConfig>,
    client: reqwest::Client,
    adapter: Arc<AdaptiveRoutingAdapter>,
    adaptive: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
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

    let adaptive = cli.adaptive
        || std::env::var("MANWE_ROUTING_MODE")
            .ok()
            .as_deref()
            .map(|v| v.eq_ignore_ascii_case("adaptive"))
            .unwrap_or(false);

    let state = AppState {
        config: Arc::new(cfg.clone()),
        client: reqwest::Client::new(),
        adapter: Arc::new(AdaptiveRoutingAdapter::new()),
        adaptive,
    };

    let bind = cfg.bind.clone();
    let port = cfg.port;

    let addr: SocketAddr = format!("{}:{}", bind, port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind/port: {e}"))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tracing::info!(
        "manwe: gateway listening on {} ({} providers, adaptive={})",
        bound,
        cfg.providers.len(),
        adaptive
    );

    if cli.grpc {
        let http_state = state.clone();
        let grpc_state = grpc::GrpcState {
            config: state.config.clone(),
            client: state.client.clone(),
        };
        let http_handle = tokio::spawn(async move {
            if let Err(e) = run_http(http_state).await {
                tracing::error!("manwe http exited: {e}");
            }
        });
        let grpc_handle = tokio::spawn(async move {
            if let Err(e) = grpc::serve_grpc(grpc_state).await {
                tracing::error!("manwe grpc exited: {e}");
            }
        });
        let _ = tokio::join!(http_handle, grpc_handle);
    } else {
        run_http(state).await?;
    }
    Ok(())
}

async fn run_http(state: AppState) -> anyhow::Result<()> {
    let adaptive = state.adaptive;
    let cfg = state.config.clone();
    let _app: Router = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/models", get(list_models))
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/capabilities", get(manifest_capabilities))
        .with_state(state);

    let addr: SocketAddr = format!("{}:{}", cfg.bind, cfg.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind/port: {e}"))?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    tracing::info!(
        "manwe http: listening on {} (providers={}, adaptive={})",
        bound,
        cfg.providers.len(),
        adaptive
    );
    axum::serve(listener, _app).await?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}

async fn manifest_capabilities(State(state): State<AppState>) -> Response {
    let mode = if state.adaptive { "adaptive" } else { "static" };
    Json(json!({
        "mode": mode,
        "adaptive_routing": state.adaptive,
        "governance": false,
        "quota_mesh": false,
        "configured_providers": state.config.providers.len(),
        "healthy_providers": if state.adaptive { 0 } else { state.config.providers.len() },
    }))
    .into_response()
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
    if state.adaptive {
        match state.adapter.route_chat_completions(req.clone()) {
            Ok(adapted) => return fallback_static(state.config, state.client, adapted).await,
            Err(_) => {}
        }
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
