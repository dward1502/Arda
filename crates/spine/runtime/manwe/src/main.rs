#![allow(path_statements)]
//! `manwe` — single local OpenAI-compatible inference gateway.
//!
//! Default: listens on `127.0.0.1:7171` and serves `/v1/chat/completions`
//! + `/v1/models` against a static provider catalog. When `MANWE_ROUTING_MODE=adaptive`
//! or `--adaptive` is set, requests may flow through the routing adapter.

mod config;
#[cfg(feature = "grpc")]
mod grpc;
mod provider;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use serde_json::{json, Value};

use config::ManweConfig;
use provider::{ProviderCatalog, ProviderDefinition};
use tokio::sync::RwLock;

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
    adaptive: bool,
    catalog: Arc<RwLock<ProviderCatalog>>,
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

    if adaptive && !cfg!(feature = "adaptive") {
        anyhow::bail!(
            "adaptive routing requested, but this manwe binary was built without --features adaptive"
        );
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .build()?;
    let mut catalog = ProviderCatalog::default_bootstrap();
    catalog.probe_all(&client).await;
    let catalog = Arc::new(RwLock::new(catalog));

    let state = AppState {
        config: Arc::new(cfg.clone()),
        client: client.clone(),
        adaptive,
        catalog: catalog.clone(),
    };

    tokio::spawn(refresh_fleet_catalog(catalog, client));

    if cli.grpc && !cfg!(feature = "grpc") {
        anyhow::bail!("gRPC requested, but this manwe binary was built without --features grpc");
    }

    #[cfg(feature = "grpc")]
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
        return Ok(());
    }

    run_http(state).await?;
    Ok(())
}

async fn refresh_fleet_catalog(catalog: Arc<RwLock<ProviderCatalog>>, client: reqwest::Client) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.tick().await;
    loop {
        interval.tick().await;
        let mut refreshed = ProviderCatalog::from_fleet_config("config/fleet.toml");
        refreshed.probe_all(&client).await;
        *catalog.write().await = refreshed;
    }
}

async fn run_http(state: AppState) -> anyhow::Result<()> {
    let adaptive = state.adaptive;
    let cfg = state.config.clone();
    let provider_count = state.catalog.read().await.len();
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
        provider_count,
        adaptive
    );
    axum::serve(listener, _app).await?;
    Ok(())
}

async fn healthz(State(state): State<AppState>) -> Response {
    let catalog = state.catalog.read().await;
    let fleet_providers = catalog.len();
    let healthy_providers = catalog.healthy_count();
    let status = if fleet_providers == 0 || healthy_providers > 0 {
        "ok"
    } else {
        "degraded"
    };
    Json(json!({
        "status": status,
        "fleet_providers": fleet_providers,
        "healthy_providers": healthy_providers,
    }))
    .into_response()
}

async fn manifest_capabilities(State(state): State<AppState>) -> Response {
    let mode = if state.adaptive { "adaptive" } else { "static" };
    let configured = state.config.providers.len();
    let catalog = state.catalog.read().await;
    let fleet_providers = catalog.len();
    let healthy = catalog.healthy_count();
    Json(json!({
        "mode": mode,
        "adaptive_routing": state.adaptive,
        "routing_strategy": if state.adaptive { "deterministic_health_capability" } else { "explicit_model" },
        "governance": false,
        "quota_mesh": false,
        "configured_providers": configured,
        "healthy_providers": healthy,
        "fleet_providers": fleet_providers,
    }))
    .into_response()
}

async fn list_models(State(state): State<AppState>) -> Response {
    let created = chrono::Utc::now().timestamp();
    let mut data: Vec<Value> = Vec::new();
    let catalog = state.catalog.read().await;
    if catalog.is_empty() {
        for provider in state.config.providers.values() {
            let models = if provider.models.is_empty() {
                vec!["default".to_string()]
            } else {
                provider.models.clone()
            };
            for model in models {
                data.push(json!({
                    "id": model,
                    "object": "model",
                    "created": created,
                    "owned_by": "manwe",
                }));
            }
        }
    } else {
        for (_, provider) in catalog.iter().filter(|(_, provider)| provider.healthy) {
            data.push(json!({
                "id": provider.model_id,
                "object": "model",
                "created": created,
                "owned_by": "manwe",
                "provider": provider.id,
                "role": provider.role,
                "context_window": provider.context_window,
                "probe_latency_ms": provider.probe_latency_ms,
                "model_observed": provider.model_observed,
            }));
        }
    }
    Json(json!({ "object": "list", "data": data })).into_response()
}

async fn chat_completions(State(state): State<AppState>, Json(req): Json<Value>) -> Response {
    let Some(requested_model) = req.get("model").and_then(Value::as_str).map(str::to_string) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": { "message": "missing model", "type": "manwe_error" } })),
        )
            .into_response();
    };
    let task_type = infer_task_type(&req);
    let required_context = required_context_tokens(&req);
    let provider = {
        state
            .catalog
            .read()
            .await
            .resolve(
                &requested_model,
                state.adaptive,
                &task_type,
                required_context,
            )
            .cloned()
    };

    if let Some(provider) = provider {
        return proxy_fleet_provider(state.client, provider, req, state.adaptive).await;
    }

    if state.adaptive {
        let catalog = state.catalog.read().await;
        if !catalog.is_empty() {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": {
                        "message": format!("manwe: no healthy fleet provider can satisfy model={requested_model} task={task_type}"),
                        "type": "manwe_routing_error"
                    }
                })),
            )
                .into_response();
        }
    }

    fallback_static(state.config, state.client, req).await
}

fn infer_task_type(req: &Value) -> String {
    if let Some(task_type) = req.get("task_type").and_then(Value::as_str) {
        return task_type.to_string();
    }
    if req
        .get("tools")
        .and_then(Value::as_array)
        .is_some_and(|tools| !tools.is_empty())
    {
        return "code".to_string();
    }
    if req
        .get("messages")
        .is_some_and(|messages| messages.to_string().contains("image_url"))
    {
        return "vision".to_string();
    }
    "chat".to_string()
}

fn required_context_tokens(req: &Value) -> usize {
    let prompt_estimate = req
        .get("messages")
        .map(|messages| messages.to_string().len().div_ceil(4))
        .unwrap_or_default();
    let completion_budget = req
        .get("max_tokens")
        .or_else(|| req.get("max_completion_tokens"))
        .and_then(Value::as_u64)
        .unwrap_or(1024) as usize;
    prompt_estimate.saturating_add(completion_budget)
}

async fn proxy_fleet_provider(
    client: reqwest::Client,
    provider: ProviderDefinition,
    mut req: Value,
    adaptive: bool,
) -> Response {
    req["model"] = Value::String(provider.model_id.clone());
    let upstream = format!(
        "{}/chat/completions",
        provider.base_url.trim_end_matches('/')
    );
    let mut request = client.post(&upstream).json(&req);
    if let Some(api_key_env) = &provider.api_key_env {
        if let Ok(api_key) = std::env::var(api_key_env) {
            if !api_key.trim().is_empty() {
                request = request.header(header::AUTHORIZATION, format!("Bearer {api_key}"));
            }
        }
    }

    match request.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response.headers().get(header::CONTENT_TYPE).cloned();
            let mut builder = Response::builder()
                .status(status)
                .header("x-manwe-provider", provider.id)
                .header("x-manwe-model", provider.model_id)
                .header(
                    "x-manwe-routing-mode",
                    if adaptive { "adaptive" } else { "static" },
                );
            if let Some(content_type) = content_type {
                builder = builder.header(header::CONTENT_TYPE, content_type);
            }
            builder
                .body(Body::from_stream(response.bytes_stream()))
                .unwrap_or_else(|_| {
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({ "error": { "message": "manwe: failed to construct upstream response", "type": "manwe_error" } })),
                    )
                        .into_response()
                })
        }
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": {
                    "message": format!("manwe: upstream {} unreachable: {error}", provider.id),
                    "type": "manwe_error"
                }
            })),
        )
            .into_response(),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_shape_infers_tools_and_context_budget() {
        let request = json!({
            "model": "auto",
            "messages": [{"role": "user", "content": "write a function"}],
            "tools": [{"type": "function", "function": {"name": "edit"}}],
            "max_tokens": 4096
        });
        assert_eq!(infer_task_type(&request), "code");
        assert!(required_context_tokens(&request) > 4096);
    }

    #[test]
    fn explicit_task_type_wins_over_shape_inference() {
        let request = json!({
            "task_type": "research",
            "messages": [],
            "tools": [{"type": "function"}]
        });
        assert_eq!(infer_task_type(&request), "research");
    }
}
