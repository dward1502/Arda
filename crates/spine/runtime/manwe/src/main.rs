#![allow(path_statements)]
//! `manwe` — single local OpenAI-compatible inference gateway.
//!
//! Default: listens on `127.0.0.1:7171` and serves `/v1/chat/completions`
//! + `/v1/models` against a static provider catalog. When `MANWE_ROUTING_MODE=adaptive`
//!   or `--adaptive` is set, requests may flow through the routing adapter.

mod config;
#[cfg(feature = "grpc")]
mod grpc;
mod provider;
mod receipts;
mod resource_limits;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
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
use std::time::Instant;

use config::{ManweConfig, StaticConfigSource};
use provider::{ProviderCatalog, ProviderDefinition};
use receipts::{receipt_from_response, QualityExpectation, ReceiptWriter};
use resource_limits::ResourceGroupLimiter;
use tokio::sync::RwLock;
#[cfg(feature = "telemetry")]
use tracing_subscriber::layer::SubscriberExt;
#[cfg(feature = "telemetry")]
use tracing_subscriber::util::SubscriberInitExt;
#[cfg(feature = "telemetry")]
use tracing_subscriber::Layer;

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

#[cfg(feature = "adaptive")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeMode {
    Static,
    FullGovernedAdaptive,
}

#[cfg(feature = "adaptive")]
fn runtime_mode(adaptive: bool) -> RuntimeMode {
    if adaptive {
        return RuntimeMode::FullGovernedAdaptive;
    }
    RuntimeMode::Static
}

#[derive(Clone)]
struct AppState {
    config: Arc<ManweConfig>,
    client: reqwest::Client,
    adaptive: bool,
    catalog: Arc<RwLock<ProviderCatalog>>,
    resource_limits: ResourceGroupLimiter,
    receipts: ReceiptWriter,
    config_path: Arc<PathBuf>,
    config_source: StaticConfigSource,
    fleet_config_path: Arc<PathBuf>,
    catalog_generation: Arc<AtomicU64>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter =
        tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());

    #[cfg(feature = "telemetry")]
    let _telemetry_shutdown = {
        tracing_subscriber::registry()
            .with(arda_aule::telemetry::tracing_layer())
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .init();
        arda_aule::telemetry::shutdown_guard()
    };

    #[cfg(not(feature = "telemetry"))]
    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    let cli = Cli::parse();
    let (mut cfg, config_source) = ManweConfig::load_with_source(&cli.config);
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

    if let Err(err) = cfg.validate() {
        anyhow::bail!("manwe config validation failed: {err}");
    }

    #[cfg(feature = "adaptive")]
    if runtime_mode(adaptive) == RuntimeMode::FullGovernedAdaptive {
        if cli.grpc {
            anyhow::bail!(
                "gRPC cannot currently be combined with the full governed adaptive runtime"
            );
        }
        let arda_home = std::env::var_os("ARDA_HOME").map(PathBuf::from);
        let root = config::adaptive_state_dir(arda_home.as_deref());
        let service = manwe::adaptive::service::ManweService::new(root)?;
        if let Err(err) = service.reload_provider_config().await {
            tracing::warn!(
                error = %err,
                "adaptive provider config reload failed; using governed bootstrap catalog"
            );
        }
        let addr = format!("{}:{}", cfg.bind, cfg.port);
        manwe::adaptive::transport::http::run_http_server(service, &addr)
            .await
            .map_err(|err| anyhow::anyhow!(err.to_string()))?;
        return Ok(());
    }

    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .build()?;
    let fleet_config_path = config::static_fleet_config_path();
    let mut catalog = ProviderCatalog::from_fleet_config(&fleet_config_path);
    catalog.probe_all(&client).await;
    let catalog = Arc::new(RwLock::new(catalog));

    let state = AppState {
        config: Arc::new(cfg.clone()),
        client: client.clone(),
        adaptive,
        catalog: catalog.clone(),
        resource_limits: ResourceGroupLimiter::default(),
        receipts: ReceiptWriter::new(config::arda_root().join("data/manwe/route_receipts.jsonl")),
        config_path: Arc::new(cli.config.clone()),
        config_source,
        fleet_config_path: Arc::new(fleet_config_path.clone()),
        catalog_generation: Arc::new(AtomicU64::new(1)),
    };

    tokio::spawn(refresh_fleet_catalog(
        catalog,
        client,
        fleet_config_path,
        state.catalog_generation.clone(),
    ));

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

async fn refresh_fleet_catalog(
    catalog: Arc<RwLock<ProviderCatalog>>,
    client: reqwest::Client,
    fleet_config_path: PathBuf,
    generation: Arc<AtomicU64>,
) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
    interval.tick().await;
    loop {
        interval.tick().await;
        let mut refreshed = ProviderCatalog::from_fleet_config(&fleet_config_path);
        refreshed.probe_all(&client).await;
        *catalog.write().await = refreshed;
        generation.fetch_add(1, Ordering::Relaxed);
    }
}

async fn run_http(state: AppState) -> anyhow::Result<()> {
    let adaptive = state.adaptive;
    let cfg = state.config.clone();
    let provider_count = state.catalog.read().await.len();
    let _app: Router = Router::new()
        .route("/healthz", get(healthz))
        .route("/health", get(healthz))
        .route("/status", get(healthz))
        .route("/providers", get(list_providers))
        .route("/state", get(list_providers))
        .route("/metrics", get(metrics))
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
    let (status, config_valid, config_error) = match state.config.validate() {
        Ok(_) => ("ok", true, None),
        Err(err) => ("degraded", false, Some(err.to_string())),
    };
    Json(json!({
        "status": status,
        "service": "manwe",
        "runtime": "arda-manwe",
        "bind": state.config.bind,
        "port": state.config.port,
        "mode": if state.adaptive { "adaptive" } else { "static" },
        "fleet_providers": fleet_providers,
        "healthy_providers": healthy_providers,
        "config_valid": config_valid,
        "config_error": config_error,
        "config_source": state.config_source,
        "config_path": state.config_path.display().to_string(),
        "fleet_config_path": state.fleet_config_path.display().to_string(),
        "catalog_generation": state.catalog_generation.load(Ordering::Relaxed),
    }))
    .into_response()
}

async fn manifest_capabilities(State(state): State<AppState>) -> Response {
    let mode = if state.adaptive { "adaptive" } else { "static" };
    let configured = state.config.providers.len();
    let catalog = state.catalog.read().await;
    let fleet_providers = catalog.len();
    let healthy = catalog.healthy_count();
    drop(catalog);
    let resource_groups = state.resource_limits.snapshots().await;
    Json(json!({
        "mode": mode,
        "adaptive_routing": state.adaptive,
        "routing_strategy": if state.adaptive { "deterministic_health_capability" } else { "explicit_model" },
        "governance": false,
        "quota_mesh": false,
        "configured_providers": configured,
        "healthy_providers": healthy,
        "fleet_providers": fleet_providers,
        "resource_groups": resource_groups,
        "route_receipts": state.receipts.path(),
        "config_source": state.config_source,
        "config_path": state.config_path.display().to_string(),
        "fleet_config_path": state.fleet_config_path.display().to_string(),
        "catalog_generation": state.catalog_generation.load(Ordering::Relaxed),
    }))
    .into_response()
}

async fn list_models(State(state): State<AppState>) -> Response {
    let created = chrono::Utc::now().timestamp();
    let catalog = state.catalog.read().await;
    Json(model_catalog(&catalog, &state.config, created)).into_response()
}

fn model_catalog(catalog: &ProviderCatalog, config: &ManweConfig, created: i64) -> Value {
    let mut data: Vec<Value> = vec![
        json!({
            "id": "auto",
            "object": "model",
            "created": created,
            "owned_by": "manwe",
            "route_policy": "adaptive_local_catalog"
        }),
        json!({
            "id": "local/auto",
            "object": "model",
            "created": created,
            "owned_by": "manwe",
            "route_policy": "local_only_fail_closed"
        }),
    ];
    if catalog.is_empty() {
        for provider in config.providers.values() {
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
    json!({ "object": "list", "data": data })
}

async fn list_providers(State(state): State<AppState>) -> Response {
    let catalog = state.catalog.read().await;
    let providers = catalog
        .iter()
        .map(|(_, provider)| {
            json!({
                "id": provider.id,
                "name": provider.name,
                "model_id": provider.model_id,
                "base_url": provider.base_url,
                "access_tier": provider.access_tier,
                "healthy": provider.healthy,
                "in_cooldown": provider.in_cooldown,
                "context_window": provider.context_window,
                "capabilities": provider.capabilities,
                "role": provider.role,
                "model_observed": provider.model_observed,
                "probe_latency_ms": provider.probe_latency_ms,
                "last_probe_utc": provider.last_probe_utc,
                "last_error": provider.last_error,
            })
        })
        .collect::<Vec<_>>();
    Json(json!({
        "service": "manwe",
        "runtime": "arda-manwe",
        "providers": providers,
    }))
    .into_response()
}

async fn metrics(State(state): State<AppState>) -> Response {
    let catalog = state.catalog.read().await;
    let body = render_metrics(&catalog);
    Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

fn render_metrics(catalog: &ProviderCatalog) -> String {
    let mut output = String::from(
        "# HELP manwe_runtime_info Arda Manwe runtime identity.\n\
# TYPE manwe_runtime_info gauge\n\
manwe_runtime_info{runtime=\"arda-manwe\"} 1\n\
# HELP manwe_providers_total Configured fleet provider count.\n\
# TYPE manwe_providers_total gauge\n",
    );
    output.push_str(&format!("manwe_providers_total {}\n", catalog.len()));
    output.push_str(
        "# HELP manwe_provider_healthy Whether a fleet provider passed its live probe.\n\
# TYPE manwe_provider_healthy gauge\n",
    );
    let mut providers = catalog
        .iter()
        .map(|(_, provider)| provider)
        .collect::<Vec<_>>();
    providers.sort_by(|left, right| left.id.cmp(&right.id));
    for provider in providers {
        output.push_str(&format!(
            "manwe_provider_healthy{{provider_id=\"{}\",model=\"{}\"}} {}\n",
            provider.id,
            provider.model_id,
            u8::from(provider.healthy),
        ));
    }
    output
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
    let local_only = requested_model == "local/auto"
        || routing_value(&req, "origin_preference") == Some("local")
        || routing_value(&req, "inference_origin") == Some("local");
    let mut provider = {
        state
            .catalog
            .read()
            .await
            .resolve_with_policy(
                &requested_model,
                state.adaptive,
                &task_type,
                required_context,
                local_only,
            )
            .cloned()
    };

    if state.adaptive {
        if let Some(selected) = provider.as_ref() {
            let resource_group = selected
                .resource_group
                .as_deref()
                .unwrap_or(selected.id.as_str());
            if state.resource_limits.is_saturated(resource_group).await {
                if let Some(alternate) = state
                    .catalog
                    .read()
                    .await
                    .resolve_alternate_resource_group(
                        selected,
                        &requested_model,
                        &task_type,
                        required_context,
                        local_only,
                    )
                    .cloned()
                {
                    provider = Some(alternate);
                }
            }
        }
    }

    if let Some(provider) = provider {
        return proxy_fleet_provider(
            state.client,
            provider,
            req,
            state.adaptive,
            task_type,
            state.resource_limits,
            state.receipts,
        )
        .await;
    }

    if state.adaptive {
        let catalog = state.catalog.read().await;
        if !catalog.is_empty() {
            let rejections =
                catalog.rejection_diagnostics(&task_type, required_context, local_only);
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": {
                        "message": format!("manwe: no compatible provider can satisfy model={requested_model} task={task_type}"),
                        "type": "manwe_routing_error",
                        "code": "no_compatible_model",
                        "runtime": "arda-manwe",
                        "requested_model": requested_model,
                        "task_type": task_type,
                        "required_context": required_context,
                        "local_only": local_only,
                        "rejected_providers": rejections,
                    }
                })),
            )
                .into_response();
        }
    }

    fallback_static(state.config, state.client, req).await
}

fn routing_value<'a>(req: &'a Value, key: &str) -> Option<&'a str> {
    req.get(key)
        .or_else(|| req.get("routing").and_then(|value| value.get(key)))
        .or_else(|| req.get("extra_body").and_then(|value| value.get(key)))
        .or_else(|| {
            req.get("extra_body")
                .and_then(|value| value.get("routing"))
                .and_then(|value| value.get(key))
        })
        .and_then(Value::as_str)
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
    task_type: String,
    resource_limits: ResourceGroupLimiter,
    receipts: ReceiptWriter,
) -> Response {
    let resource_group = provider
        .resource_group
        .clone()
        .unwrap_or_else(|| provider.id.clone());
    let started = Instant::now();
    let lease = match resource_limits
        .acquire(&resource_group, provider.resource_group_concurrency)
        .await
    {
        Ok(lease) => lease,
        Err(error) => {
            let receipt = receipt_from_response(
                provider.id.clone(),
                provider.model_id.clone(),
                resource_group,
                task_type,
                if adaptive { "adaptive" } else { "static" }.to_string(),
                false,
                StatusCode::SERVICE_UNAVAILABLE.as_u16(),
                started.elapsed().as_millis() as u64,
                None,
                None,
                Some(error.clone()),
            );
            if let Err(write_error) = receipts.append(&receipt).await {
                tracing::warn!("failed to append Manwe route receipt: {write_error}");
            }
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": { "message": error, "type": "manwe_resource_busy" } })),
            )
                .into_response();
        }
    };

    let expected_exact = req
        .get("manwe_quality_expectation")
        .and_then(|expectation| {
            Some(QualityExpectation {
                exact: expectation.get("exact")?.as_str()?.to_string(),
                benchmark_id: expectation
                    .get("benchmark_id")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            })
        });
    if let Some(object) = req.as_object_mut() {
        object.remove("manwe_quality_expectation");
    }
    let streaming = req.get("stream").and_then(Value::as_bool).unwrap_or(false);
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
                .header("x-manwe-provider", provider.id.clone())
                .header("x-manwe-model", provider.model_id.clone())
                .header("x-manwe-resource-group", resource_group.clone())
                .header(
                    "x-manwe-routing-mode",
                    if adaptive { "adaptive" } else { "static" },
                );
            if let Some(ref content_type) = content_type {
                builder = builder.header(header::CONTENT_TYPE, content_type);
            }

            if streaming {
                let started_local = started.elapsed().as_millis() as u64;
                let bytes = match response.bytes().await {
                    Ok(bytes) => bytes,
                    Err(error) => {
                        let message = format!("manwe: failed to read upstream stream: {error}");
                        let receipt = receipt_from_response(
                            provider.id,
                            provider.model_id,
                            resource_group,
                            task_type,
                            if adaptive { "adaptive" } else { "static" }.to_string(),
                            true,
                            StatusCode::BAD_GATEWAY.as_u16(),
                            started_local,
                            expected_exact,
                            None,
                            Some(message.clone()),
                        );
                        if let Err(write_error) = receipts.append(&receipt).await {
                            tracing::warn!("failed to append Manwe route receipt: {write_error}");
                        }
                        drop(lease);
                        return (
                            StatusCode::BAD_GATEWAY,
                            Json(json!({ "error": { "message": message, "type": "manwe_error" } })),
                        )
                            .into_response();
                    }
                };
                let body: Value = serde_json::from_slice(&bytes).unwrap_or_default();
                let receipt = receipt_from_response(
                    provider.id.clone(),
                    provider.model_id.clone(),
                    resource_group.clone(),
                    task_type.clone(),
                    if adaptive { "adaptive" } else { "static" }.to_string(),
                    true,
                    status.as_u16(),
                    started_local,
                    expected_exact.clone(),
                    Some(&body),
                    None,
                );
                if let Err(error) = receipts.append(&receipt).await {
                    tracing::warn!("failed to append Manwe route receipt: {error}");
                }
                drop(lease);
                let response_builder = builder.header("x-manwe-streaming-mode", "buffered");
                return response_builder
                    .body(Body::from(bytes))
                    .unwrap_or_else(|_| {
                        (
                            StatusCode::BAD_GATEWAY,
                            Json(json!({ "error": { "message": "manwe: failed to construct upstream response", "type": "manwe_error" } })),
                        )
                            .into_response()
                    });
            }

            match response.bytes().await {
                Ok(bytes) => {
                    let body = serde_json::from_slice::<Value>(&bytes).ok();
                    let receipt = receipt_from_response(
                        provider.id,
                        provider.model_id,
                        resource_group,
                        task_type,
                        if adaptive { "adaptive" } else { "static" }.to_string(),
                        false,
                        status.as_u16(),
                        started.elapsed().as_millis() as u64,
                        expected_exact,
                        body.as_ref(),
                        None,
                    );
                    if let Err(error) = receipts.append(&receipt).await {
                        tracing::warn!("failed to append Manwe route receipt: {error}");
                    }
                    drop(lease);
                    builder
                        .body(Body::from(bytes))
                        .unwrap_or_else(|_| StatusCode::BAD_GATEWAY.into_response())
                }
                Err(error) => {
                    let receipt = receipt_from_response(
                        provider.id,
                        provider.model_id,
                        resource_group,
                        task_type,
                        if adaptive { "adaptive" } else { "static" }.to_string(),
                        false,
                        StatusCode::BAD_GATEWAY.as_u16(),
                        started.elapsed().as_millis() as u64,
                        expected_exact,
                        None,
                        Some(error.to_string()),
                    );
                    if let Err(write_error) = receipts.append(&receipt).await {
                        tracing::warn!("failed to append Manwe route receipt: {write_error}");
                    }
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({ "error": { "message": "manwe: failed to read upstream response", "type": "manwe_error" } })),
                    )
                        .into_response()
                }
            }
        }
        Err(error) => {
            let message = format!("manwe: upstream {} unreachable: {error}", provider.id);
            let receipt = receipt_from_response(
                provider.id,
                provider.model_id,
                resource_group,
                task_type,
                if adaptive { "adaptive" } else { "static" }.to_string(),
                streaming,
                StatusCode::BAD_GATEWAY.as_u16(),
                started.elapsed().as_millis() as u64,
                expected_exact,
                None,
                Some(message.clone()),
            );
            if let Err(write_error) = receipts.append(&receipt).await {
                tracing::warn!("failed to append Manwe route receipt: {write_error}");
            }
            (
                StatusCode::BAD_GATEWAY,
                Json(json!({ "error": { "message": message, "type": "manwe_error" } })),
            )
                .into_response()
        }
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
    use config::ProviderConfig;
    use std::collections::HashMap;

    #[cfg(feature = "adaptive")]
    #[test]
    fn adaptive_flag_selects_full_governed_runtime() {
        assert_eq!(runtime_mode(true), RuntimeMode::FullGovernedAdaptive);
        assert_eq!(runtime_mode(false), RuntimeMode::Static);
    }

    async fn serve_test_upstream(status: StatusCode, body: &'static str) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test upstream");
        let addr = listener.local_addr().expect("test upstream address");
        let app = Router::new().route(
            "/chat/completions",
            post(move || async move { (status, body) }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        format!("http://{addr}")
    }

    fn static_config(base_url: String) -> ManweConfig {
        ManweConfig {
            bind: "127.0.0.1".to_string(),
            port: 7171,
            default_provider: Some("local".to_string()),
            providers: HashMap::from([(
                "local".to_string(),
                ProviderConfig {
                    base_url,
                    api_key: None,
                    models: vec!["test-model".to_string()],
                },
            )]),
        }
    }

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

    #[test]
    fn openai_catalog_identifies_manwe_and_local_only_route() {
        let catalog = ProviderCatalog::empty();
        let response = model_catalog(&catalog, &ManweConfig::embedded(), 0);
        let models = response["data"].as_array().expect("model array");
        assert!(models.iter().all(|model| model["owned_by"] == "manwe"));
        assert!(models.iter().any(|model| {
            model["id"] == "local/auto" && model["route_policy"] == "local_only_fail_closed"
        }));
    }

    #[test]
    fn nested_routing_metadata_preserves_local_only_intent() {
        let request = json!({
            "extra_body": {"routing": {"origin_preference": "local"}}
        });
        assert_eq!(routing_value(&request, "origin_preference"), Some("local"));
    }

    #[test]
    fn metrics_identify_arda_manwe_runtime() {
        let metrics = render_metrics(&ProviderCatalog::empty());
        assert!(metrics.contains("manwe_runtime_info{runtime=\"arda-manwe\"} 1"));
        assert!(metrics.contains("manwe_providers_total 0"));
    }

    #[tokio::test]
    async fn fallback_static_503s_when_no_provider_matches() {
        let config = ManweConfig::embedded();
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(1))
            .build()
            .expect("client");
        let req = json!({"model": "missing/model", "messages": [{"role":"user","content":"hi"}]});
        let response = fallback_static(Arc::new(config), client, req).await;
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn fallback_static_502s_on_upstream_non_json() {
        let base_url = serve_test_upstream(StatusCode::OK, "not-json").await;
        let response = fallback_static(
            Arc::new(static_config(base_url)),
            reqwest::Client::new(),
            json!({"model": "local/test-model", "messages": []}),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn fallback_static_502s_on_unreachable_upstream() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("reserve unused port");
        let addr = listener.local_addr().expect("unused address");
        drop(listener);
        let response = fallback_static(
            Arc::new(static_config(format!("http://{addr}"))),
            reqwest::Client::new(),
            json!({"model": "local/test-model", "messages": []}),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    }

    #[tokio::test]
    async fn fleet_stream_requests_return_buffered_sse_with_final_receipt() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind streaming upstream");
        let addr = listener.local_addr().expect("streaming upstream address");
        let sse = "data: {\"choices\":[{\"delta\":{\"content\":\"hello\"}}]}\n\ndata: [DONE]\n\n";
        let app = Router::new().route(
            "/chat/completions",
            post(move || async move {
                Response::builder()
                    .header(header::CONTENT_TYPE, "text/event-stream")
                    .body(Body::from(sse))
                    .expect("SSE response")
            }),
        );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let receipt_path = dir.path().join("route_receipts.jsonl");
        let response = proxy_fleet_provider(
            reqwest::Client::new(),
            ProviderDefinition::openai_compatible(
                "streaming",
                "Streaming Test",
                "test-model",
                format!("http://{addr}"),
            ),
            json!({"model": "test-model", "messages": [], "stream": true}),
            false,
            "chat".to_string(),
            ResourceGroupLimiter::default(),
            ReceiptWriter::new(&receipt_path),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "text/event-stream"
        );
        assert_eq!(
            response
                .headers()
                .get_all(header::CONTENT_TYPE)
                .iter()
                .count(),
            1
        );
        assert_eq!(response.headers()["x-manwe-streaming-mode"], "buffered");
        let receipt = tokio::fs::read_to_string(&receipt_path)
            .await
            .expect("final receipt exists before response body is consumed");
        let receipt: Value = serde_json::from_str(receipt.trim()).expect("receipt JSON");
        assert_eq!(receipt["streaming"], true);
        assert_eq!(receipt["status_code"], 200);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("buffered SSE body");
        assert_eq!(body.as_ref(), sse.as_bytes());
    }

    #[tokio::test]
    async fn fleet_stream_body_read_failure_returns_502_and_error_receipt() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind truncated streaming upstream");
        let addr = listener
            .local_addr()
            .expect("truncated streaming upstream address");
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept upstream request");
            let mut request = [0_u8; 4096];
            let _ = socket.read(&mut request).await;
            socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: 100\r\nconnection: close\r\n\r\ndata: partial\n\n",
                )
                .await
                .expect("write truncated upstream response");
        });
        let dir = tempfile::tempdir().expect("tempdir");
        let receipt_path = dir.path().join("route_receipts.jsonl");
        let response = proxy_fleet_provider(
            reqwest::Client::new(),
            ProviderDefinition::openai_compatible(
                "streaming",
                "Streaming Test",
                "test-model",
                format!("http://{addr}"),
            ),
            json!({"model": "test-model", "messages": [], "stream": true}),
            false,
            "chat".to_string(),
            ResourceGroupLimiter::default(),
            ReceiptWriter::new(&receipt_path),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let receipt = tokio::fs::read_to_string(&receipt_path)
            .await
            .expect("stream read error receipt");
        let receipt: Value = serde_json::from_str(receipt.trim()).expect("receipt JSON");
        assert_eq!(receipt["streaming"], true);
        assert_eq!(receipt["status_code"], 502);
        assert!(receipt["error"]
            .as_str()
            .is_some_and(|error| error.contains("failed to read upstream stream")));
    }
}
