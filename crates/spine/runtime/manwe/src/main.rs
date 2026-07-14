//! `manwe` — single local OpenAI-compatible inference gateway.
//!
//! Frozen contract (REFACTOR_PLAN.md §0/§2): listens on `127.0.0.1:7171` and
//! serves `/v1/chat/completions` + `/v1/models`. A thin static provider catalog
//! (toml) is forwarded to — no runtime adaptive routing, no quota mesh. This is
//! the surface Hermes connects to for local inference; it is the clean local
//! root that later grows a `remote` adapter (NOT before).
//!
//! Models are referenced as `"provider/model"` (e.g. `"ollama/llama3"`); the
//! `provider/` prefix selects the upstream and is stripped before forwarding.

mod config;

use std::collections::HashMap;
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
use serde::Deserialize;
use serde_json::{json, Value};

use config::ManweConfig;

#[derive(Parser, Debug)]
#[command(name = "manwe", version, about = "Local OpenAI-compat inference gateway")]
struct Cli {
    /// HTTP port (overrides config / embedded default 7171).
    #[arg(long)]
    port: Option<u16>,
    /// Bind address (default 127.0.0.1).
    #[arg(long)]
    bind: Option<String>,
    /// Path to manwe.toml (default: ./manwe.toml).
    #[arg(long, default_value = "manwe.toml")]
    config: PathBuf,
}

#[derive(Clone)]
struct AppState {
    config: Arc<ManweConfig>,
    client: reqwest::Client,
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
    };

    let app = Router::new()
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
    let mut data = Vec::new();
    for (name, p) in &state.config.providers {
        let models = if p.models.is_empty() {
            vec![name.clone()]
        } else {
            p.models.clone()
        };
        for m in models {
            data.push(json!({
                "id": format!("{name}/{m}"),
                "object": "model",
                "created": created,
                "owned_by": name,
            }));
        }
    }
    Json(json!({ "object": "list", "data": data })).into_response()
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    model: String,
    #[serde(default)]
    messages: Value,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

async fn chat_completions(State(state): State<AppState>, Json(req): Json<ChatRequest>) -> Response {
    let Some((_prov_name, prov)) = state.config.resolve_provider(&req.model) else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": { "message": "manwe: no providers configured", "type": "manwe_error" } })),
        )
            .into_response();
    };

    let upstream = format!("{}/chat/completions", prov.base_url.trim_end_matches('/'));

    // Strip a "provider/" prefix before forwarding the model name upstream.
    let bare_model = match req.model.split_once('/') {
        Some((_, m)) => m.to_string(),
        None => req.model.clone(),
    };
    let mut body = req.extra;
    body.insert("model".to_string(), json!(bare_model));
    body.insert("messages".to_string(), req.messages.clone());

    let mut builder = state.client.post(&upstream).json(&body);
    if let Some(key) = &prov.api_key {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {key}"));
    }

    match builder.send().await {
        Ok(resp) => {
            let status = resp.status();
            match resp.json::<Value>().await {
                Ok(v) => (status, Json(v)).into_response(),
                Err(e) => (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({ "error": { "message": format!("manwe: upstream returned non-JSON: {e}"), "type": "manwe_error" } })),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": { "message": format!("manwe: upstream {upstream} unreachable: {e}"), "type": "manwe_error" } })),
        )
            .into_response(),
    }
}
