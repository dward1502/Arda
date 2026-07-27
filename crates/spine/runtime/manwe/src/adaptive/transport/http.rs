use crate::adaptive::service::ManweService;
use crate::adaptive::service::{
    classify_provider_operational_state, hermes_cli_readiness_summary, hermes_proxy_base_url,
};
use crate::types::ManweRequestEnvelope;
use crate::types::{ModelState, ProviderState};
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded_async;
use axum::body::{Body, Bytes};
use axum::extract::{Query, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use std::convert::Infallible;
use std::sync::OnceLock;
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::{Stream, StreamExt};
use tracing::warn;

// Security note: MANWE binds to localhost and IPC is a Unix-domain socket.
// These transport boundaries are the primary trust domain; auth here is
// defense-in-depth for mutation-exposed handlers only.

pub async fn run_http_server(service: ManweService, addr: &str) -> Result<()> {
    tracing::info!(addr = %addr, "starting MANWE HTTP server");
    // D1: spawn in-process active health probe loop. Pre-warms the
    // connection pool and emits liveness metrics every 60s so cold
    // providers don't surface their breakage on a real user request.
    service.spawn_health_probe();
    service.spawn_catalog_reconciliation();
    let app = Router::new()
        .route("/v1/models", get(openai_models))
        .route("/v1/chat/completions", post(openai_chat_completions))
        .route("/v1/capabilities", get(capabilities))
        .route("/healthz", get(health))
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/state", get(state))
        .route("/providers", get(providers))
        .route("/providers/capabilities", get(provider_capabilities))
        .route("/provider_candidates", get(provider_candidates))
        .route("/probe", post(probe))
        .route("/reconcile_catalogs", post(reconcile_catalogs))
        .route("/observability", get(observability))
        .route("/route_history", get(route_history))
        .route("/route", post(route))
        .route("/proxy", post(proxy))
        .route("/provider_result", post(provider_result))
        .route(
            "/model_streaming_validation",
            post(model_streaming_validation),
        )
        .route("/reload_config", post(reload_config))
        .route("/paths", get(paths))
        .route("/events", get(events))
        .route("/metrics", get(metrics))
        .layer(middleware::from_fn(http_admission_gate))
        .with_state(service);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "manwe".to_string(),
            message: format!("failed to bind HTTP listener on {addr}: {e}"),
        })?;
    tracing::info!(addr = %addr, "MANWE HTTP server listening");
    axum::serve(listener, app)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "manwe".to_string(),
            message: format!("HTTP server failed: {e}"),
        })?;
    Ok(())
}

async fn http_admission_gate(req: Request, next: Next) -> Response {
    let Some(response) =
        try_run_bounded_async("manwe_http_request", http_request_limit(), || async move {
            next.run(req).await
        })
        .await
    else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"ok": false, "error": "MANWE HTTP concurrency gate saturated"})),
        )
            .into_response();
    };

    response
}

fn http_request_limit() -> usize {
    std::env::var("ARDA_MANWE_HTTP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(24)
}

async fn status(State(service): State<ManweService>) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"ok": true, "status": service.status().await?})) })
        .await
}

async fn health(State(service): State<ManweService>) -> impl IntoResponse {
    match service.status().await {
        Ok(status) => {
            let blocked = status.providers_total.saturating_sub(
                status
                    .provider_state_counts
                    .get("ready")
                    .copied()
                    .unwrap_or(0)
                    + status
                        .provider_state_counts
                        .get("degraded")
                        .copied()
                        .unwrap_or(0),
            );
            let http_status = if status.providers_enabled > 0 && status.providers_healthy > 0 {
                StatusCode::OK
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            };
            (
                http_status,
                Json(json!({
                    "ok": http_status == StatusCode::OK,
                    "service": "manwe",
                    "providers_total": status.providers_total,
                    "providers_enabled": status.providers_enabled,
                    "providers_healthy": status.providers_healthy,
                    "providers_ready": status.providers_ready,
                    "providers_blocked": blocked,
                    "provider_state_counts": status.provider_state_counts,
                    "recent_route_failures": status.recent_route_failures,
                    "recent_route_successes": status.recent_route_successes,
                    "capability_summary": status.capability_summary,
                    "budget_pressure": status.budget_pressure,
                    "route_guardrails": status.route_guardrails,
                    "alerts": status.alerts,
                    "config_source": status.config_source,
                    "config_path": status.config_path,
                    "bootstrap_state_path": status.bootstrap_state_path,
                    "catalog_generation": status.catalog_generation,
                })),
            )
                .into_response()
        }
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "ok": false,
                "service": "manwe",
                "error": err.to_string(),
            })),
        )
            .into_response(),
    }
}

async fn capabilities(State(service): State<ManweService>) -> impl IntoResponse {
    match service.status().await {
        Ok(status) => {
            let paths = service.paths();
            Json(json!({
                "mode": "adaptive",
                "runtime": "full_governed",
                "adaptive_routing": true,
                "governance": true,
                "quota_mesh": true,
                "policy_authority": "manwe_service",
                "providers_total": status.providers_total,
                "providers_enabled": status.providers_enabled,
                "providers_healthy": status.providers_healthy,
                "route_guardrails": status.route_guardrails,
                "route_receipts": paths["state_path"],
                "governance_receipts": paths["governance_events_path"],
                "config_source": status.config_source,
                "config_path": status.config_path,
                "bootstrap_state_path": status.bootstrap_state_path,
                "catalog_generation": status.catalog_generation,
            }))
            .into_response()
        }
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": err.to_string()})),
        )
            .into_response(),
    }
}

async fn provider_capabilities(State(service): State<ManweService>) -> impl IntoResponse {
    map_result_async(async move {
        Ok(json!({
            "ok": true,
            "capabilities": service.provider_capability_view().await?
        }))
    })
    .await
}

async fn provider_candidates(State(service): State<ManweService>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "promotion_guard": service.provider_promotion_guard_view()
    }))
}

// Prometheus text-format metrics derived from the same status snapshot the
// `/status` route exposes. Hand-rendered to avoid pulling in the `prometheus`
// crate just for a liveness/coarse-state probe — the heavy metrics live on
// Arda-orchestrator :9101.
async fn metrics(State(service): State<ManweService>) -> impl IntoResponse {
    let body = match service.status().await {
        Ok(s) => {
            let mut buf = String::with_capacity(1024);
            // ---- providers ----
            buf.push_str("# HELP manwe_providers_total Configured provider count\n");
            buf.push_str("# TYPE manwe_providers_total gauge\n");
            buf.push_str(&format!("manwe_providers_total {}\n", s.providers_total));
            buf.push_str("# HELP manwe_providers_enabled Provider count currently enabled\n");
            buf.push_str("# TYPE manwe_providers_enabled gauge\n");
            buf.push_str(&format!(
                "manwe_providers_enabled {}\n",
                s.providers_enabled
            ));
            buf.push_str("# HELP manwe_providers_healthy Provider count reporting healthy\n");
            buf.push_str("# TYPE manwe_providers_healthy gauge\n");
            buf.push_str(&format!(
                "manwe_providers_healthy {}\n",
                s.providers_healthy
            ));
            buf.push_str("# HELP manwe_providers_degraded Provider count reporting degraded\n");
            buf.push_str("# TYPE manwe_providers_degraded gauge\n");
            buf.push_str(&format!(
                "manwe_providers_degraded {}\n",
                s.providers_degraded
            ));
            buf.push_str("# HELP manwe_providers_exhausted Provider count exhausted by budget\n");
            buf.push_str("# TYPE manwe_providers_exhausted gauge\n");
            buf.push_str(&format!(
                "manwe_providers_exhausted {}\n",
                s.providers_exhausted
            ));
            buf.push_str("# HELP manwe_providers_in_cooldown Provider count in cooldown\n");
            buf.push_str("# TYPE manwe_providers_in_cooldown gauge\n");
            buf.push_str(&format!(
                "manwe_providers_in_cooldown {}\n",
                s.providers_in_cooldown
            ));
            buf.push_str("# HELP manwe_provider_state_count Provider count by mutually-exclusive operational state\n");
            buf.push_str("# TYPE manwe_provider_state_count gauge\n");
            for (state, count) in &s.provider_state_counts {
                buf.push_str(&format!(
                    "manwe_provider_state_count{{state=\"{}\"}} {}\n",
                    prometheus_escape_label_value(state),
                    count
                ));
            }
            // ---- routing ----
            buf.push_str("# HELP manwe_recent_route_successes Recent successful routes\n");
            buf.push_str("# TYPE manwe_recent_route_successes gauge\n");
            buf.push_str(&format!(
                "manwe_recent_route_successes {}\n",
                s.recent_route_successes
            ));
            buf.push_str("# HELP manwe_recent_route_failures Recent failed routes\n");
            buf.push_str("# TYPE manwe_recent_route_failures gauge\n");
            buf.push_str(&format!(
                "manwe_recent_route_failures {}\n",
                s.recent_route_failures
            ));
            buf.push_str(
                "# HELP manwe_recent_local_fallback_routes Recent local-fallback routes\n",
            );
            buf.push_str("# TYPE manwe_recent_local_fallback_routes gauge\n");
            buf.push_str(&format!(
                "manwe_recent_local_fallback_routes {}\n",
                s.recent_local_fallback_routes
            ));
            // ---- state hygiene ----
            buf.push_str(
                "# HELP manwe_malformed_state_events Malformed state-log entries observed\n",
            );
            buf.push_str("# TYPE manwe_malformed_state_events gauge\n");
            buf.push_str(&format!(
                "manwe_malformed_state_events {}\n",
                s.malformed_state_events
            ));
            buf.push_str("# HELP manwe_malformed_governance_events Malformed governance-log entries observed\n");
            buf.push_str("# TYPE manwe_malformed_governance_events gauge\n");
            buf.push_str(&format!(
                "manwe_malformed_governance_events {}\n",
                s.malformed_governance_events
            ));
            // ---- budget pressure rollup ----
            buf.push_str("# HELP manwe_budget_pressure_warning Provider count with warning-level budget pressure\n");
            buf.push_str("# TYPE manwe_budget_pressure_warning gauge\n");
            buf.push_str(&format!(
                "manwe_budget_pressure_warning {}\n",
                s.budget_pressure.warning_total
            ));
            buf.push_str("# HELP manwe_budget_pressure_critical Provider count with critical-level budget pressure\n");
            buf.push_str("# TYPE manwe_budget_pressure_critical gauge\n");
            buf.push_str(&format!(
                "manwe_budget_pressure_critical {}\n",
                s.budget_pressure.critical_total
            ));
            buf.push_str("# HELP manwe_budget_pressure_cooldown Provider count in cooldown\n");
            buf.push_str("# TYPE manwe_budget_pressure_cooldown gauge\n");
            buf.push_str(&format!(
                "manwe_budget_pressure_cooldown {}\n",
                s.budget_pressure.cooldown_total
            ));
            buf.push_str(
                "# HELP manwe_budget_pressure_exhausted Provider count exhausted in this window\n",
            );
            buf.push_str("# TYPE manwe_budget_pressure_exhausted gauge\n");
            buf.push_str(&format!(
                "manwe_budget_pressure_exhausted {}\n",
                s.budget_pressure.exhausted_total
            ));
            // ---- runtime build cache ----
            buf.push_str("# HELP manwe_runtime_build_cache_observed_bytes Bytes observed in runtime build cache\n");
            buf.push_str("# TYPE manwe_runtime_build_cache_observed_bytes gauge\n");
            buf.push_str(&format!(
                "manwe_runtime_build_cache_observed_bytes {}\n",
                s.runtime_build_cache_observed_bytes
            ));
            buf.push_str("# HELP manwe_runtime_build_cache_removed_bytes Bytes removed by runtime build cache compactor\n");
            buf.push_str("# TYPE manwe_runtime_build_cache_removed_bytes counter\n");
            buf.push_str(&format!(
                "manwe_runtime_build_cache_removed_bytes {}\n",
                s.runtime_build_cache_removed_bytes
            ));
            // ---- liveness ----
            buf.push_str("# HELP manwe_up 1 if Manwe /status responded successfully\n");
            buf.push_str("# TYPE manwe_up gauge\n");
            buf.push_str("manwe_up 1\n");
            buf.push_str("# HELP manwe_alerts_total Active alert count from /status\n");
            buf.push_str("# TYPE manwe_alerts_total gauge\n");
            buf.push_str(&format!("manwe_alerts_total {}\n", s.alerts.len()));
            // ---- in-process counters/histograms (route picks, failures,
            //      streaming chunk errors, proxy latency) ----
            buf.push_str(&service.metrics().render_prometheus());
            buf
        }
        Err(_) => "manwe_up 0\n".to_string(),
    };
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        body,
    )
}

async fn state(State(service): State<ManweService>) -> impl IntoResponse {
    map_result_async(async move { Ok(json!({"ok": true, "state": service.state().await?})) }).await
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct ProvidersQuery {
    #[serde(default)]
    ids: Option<String>,
    #[serde(default)]
    provider_ids: Option<String>,
    #[serde(default)]
    compact: Option<bool>,
    #[serde(default)]
    include_models: Option<bool>,
}

async fn providers(
    State(service): State<ManweService>,
    Query(query): Query<ProvidersQuery>,
) -> impl IntoResponse {
    map_result_async(async move {
        let mut rows = Vec::new();
        let recent_events = service.recent_state_events(500);
        let requested_ids = providers_query_ids(&query);
        let compact = query.compact.unwrap_or(false);
        let include_models = query.include_models.unwrap_or(!compact);
        for provider in service
            .providers()
            .await
            .into_iter()
            .filter(|provider| {
                requested_ids.is_empty()
                    || requested_ids
                        .iter()
                        .any(|requested| requested == &provider.id)
            })
            .map(filter_provider_catalog_models)
        {
            let probe = service.capacity_probe_record(&provider.id).await;
            let mut row = provider_row(provider, probe, &recent_events, include_models);
            if compact {
                row = compact_provider_row(row);
            }
            rows.push(row);
        }
        Ok(json!({
            "ok": true,
            "providers": rows,
            "filters": {
                "ids": requested_ids,
                "compact": compact,
                "include_models": include_models,
            }
        }))
    })
    .await
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProbeRequest {
    #[serde(default)]
    lane: Option<String>,
    #[serde(default)]
    provider_id: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    marker: Option<String>,
    #[serde(default)]
    max_tokens: Option<u64>,
    #[serde(default)]
    ignore_throttle: Option<bool>,
}

async fn probe(
    State(service): State<ManweService>,
    Json(req): Json<ProbeRequest>,
) -> impl IntoResponse {
    map_result_async(async move {
        let marker = req
            .marker
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("ANKH");
        let lane = req
            .lane
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("interactive");
        let provider_id = req
            .provider_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut model = req
            .model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("auto")
            .to_string();
        let providers_snapshot = service.providers().await;
        let recent_events = service.recent_state_events(probe_recent_event_limit());
        let ignore_throttle = req.ignore_throttle.unwrap_or(false);
        if let Some(provider_id) = provider_id.as_deref() {
            let provider = providers_snapshot
                .iter()
                .find(|provider| provider.id == provider_id)
                .ok_or_else(|| ArdaError::Agent {
                    agent: "manwe".to_string(),
                    message: format!("unknown probe provider `{provider_id}`"),
                })?;
            if let Some(throttle) =
                probe_throttle_decision(provider, model.as_str(), &recent_events, ignore_throttle)
            {
                let attempts = vec![json!({
                    "ok": false,
                    "status": 429,
                    "marker_found": false,
                    "latency_ms": 0,
                    "route": probe_route_stub(provider_id, model.as_str()),
                    "error": throttle.message,
                    "outcome_class": "probe_throttled",
                    "throttle": throttle.to_json(),
                })];
                service.record_probe_result(probe_result_payload(
                    false,
                    "probe_throttled",
                    marker,
                    lane,
                    attempts.last().cloned().unwrap_or_else(|| json!({})),
                    &attempts,
                ))?;
                return Ok(json!({
                    "ok": false,
                    "status": 429,
                    "marker": marker,
                    "marker_found": false,
                    "content": "",
                    "latency_ms": 0,
                    "route": probe_route_stub(provider_id, model.as_str()),
                    "error": throttle.message,
                    "outcome_class": "probe_throttled",
                    "throttle": throttle.to_json(),
                    "attempts": attempts,
                }));
            }
            if model == "auto" {
                model = default_probe_model_for_provider(&providers_snapshot, provider_id)
                    .ok_or_else(|| ArdaError::Agent {
                    agent: "manwe".to_string(),
                    message: format!(
                        "no healthy chat model available for probe provider `{provider_id}`"
                    ),
                })?;
            }
        }
        let max_tokens = req.max_tokens.unwrap_or(64).clamp(1, 512);
        let forced_probe = provider_id.is_some();
        let mut body = json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You are answering a Manwe health probe. Keep the response terse."
                },
                {
                    "role": "user",
                    "content": format!("Health probe. Return exactly {marker} on a single line.")
                }
            ],
            "max_tokens": max_tokens,
            "stream": false,
            "agent_id": "manwe_probe",
            "routing": {
                "agent_id": "manwe_probe"
            },
            "prefer_probe_model": true,
            "execution_lane": lane,
            "workload_role": lane,
            "context_window_target": 1024,
            "cost_policy": "low",
            "quality_priority": "low"
        });
        if let Some(provider_id) = provider_id.as_deref() {
            body["force_provider_id"] = Value::String(provider_id.to_string());
            if model != "auto" {
                body["force_model_id"] = Value::String(model.clone());
            }
        }

        let provider_count = providers_snapshot.len();
        let max_attempts = if forced_probe {
            1
        } else {
            provider_count.clamp(1, 8)
        };
        let mut excluded_provider_ids = initial_probe_exclusions(
            &providers_snapshot,
            model.as_str(),
            &recent_events,
            ignore_throttle,
        );
        let mut attempts = Vec::<Value>::new();
        for provider_id in &excluded_provider_ids {
            let Some(provider) = providers_snapshot
                .iter()
                .find(|provider| provider.id == *provider_id)
            else {
                continue;
            };
            if let Some(throttle) =
                probe_throttle_decision(provider, model.as_str(), &recent_events, ignore_throttle)
            {
                attempts.push(json!({
                    "ok": false,
                    "status": 429,
                    "marker_found": false,
                    "latency_ms": 0,
                    "route": probe_route_stub(&provider.id, model.as_str()),
                    "error": throttle.message,
                    "outcome_class": "probe_throttled",
                    "throttle": throttle.to_json(),
                }));
            }
        }

        for _ in 0..max_attempts {
            let mut attempt_body = body.clone();
            if !excluded_provider_ids.is_empty() {
                attempt_body["exclude_provider_ids"] = json!(excluded_provider_ids);
            }
            let envelope = openai_body_to_envelope(&service, &attempt_body).await?;
            let started = std::time::Instant::now();
            let proxy_outcome = service.proxy_openai_passthrough(envelope, attempt_body).await;
            let elapsed_ms = started.elapsed().as_millis() as u64;
            let (status, response) = match proxy_outcome {
                Ok(outcome) => outcome,
                Err(err) => {
                    let error = err.to_string();
                    let outcome_class = classify_probe_failure_text(&error);
                    let failed_provider_id = probe_error_provider_id(&error, &providers_snapshot);
                    let route = failed_provider_id
                        .as_deref()
                        .map(|provider_id| probe_route_stub(provider_id, model.as_str()))
                        .unwrap_or_else(|| json!({}));
                    attempts.push(json!({
                        "ok": false,
                        "status": null,
                        "marker_found": false,
                        "latency_ms": elapsed_ms,
                        "route": route,
                        "error": error,
                        "outcome_class": outcome_class,
                    }));
                    if let Some(provider_id) = failed_provider_id {
                        if !forced_probe && !excluded_provider_ids.contains(&provider_id) {
                            excluded_provider_ids.push(provider_id);
                            continue;
                        }
                    }
                    let final_attempt = attempts.last().cloned().unwrap_or_else(|| json!({}));
                    service.record_probe_result(probe_result_payload(
                        false,
                        &outcome_class,
                        marker,
                        lane,
                        final_attempt.clone(),
                        &attempts,
                    ))?;
                    return Ok(json!({
                        "ok": false,
                        "status": null,
                        "marker": marker,
                        "marker_found": false,
                        "content": "",
                        "latency_ms": elapsed_ms,
                        "route": {},
                        "error": final_attempt.get("error").cloned().unwrap_or_else(|| json!("")),
                        "outcome_class": outcome_class,
                        "attempts": attempts,
                    }));
                }
            };
            let content = probe_response_text(&response);
            let marker_found = probe_marker_found(&content, marker);
            let route = response
                .get("_manwe_route")
                .cloned()
                .unwrap_or_else(|| json!({}));
            let provider_id = route
                .get("provider_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let model_id = route
                .get("model_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let attempt_ok = status < 500 && marker_found;
            let outcome_class = probe_attempt_outcome_class(status, marker_found);
            attempts.push(json!({
                "ok": attempt_ok,
                "status": status,
                "marker_found": marker_found,
                "latency_ms": elapsed_ms,
                "route": route,
                "content": content,
                "outcome_class": outcome_class,
            }));

            if attempt_ok {
                service.record_probe_result(probe_result_payload(
                    true,
                    outcome_class,
                    marker,
                    lane,
                    attempts.last().cloned().unwrap_or_else(|| json!({})),
                    &attempts,
                ))?;
                return Ok(json!({
                    "ok": true,
                    "status": status,
                    "marker": marker,
                    "marker_found": marker_found,
                    "content": attempts.last().and_then(|attempt| attempt.get("content")).cloned().unwrap_or_else(|| json!("")),
                    "latency_ms": elapsed_ms,
                    "route": attempts.last().and_then(|attempt| attempt.get("route")).cloned().unwrap_or_else(|| json!({})),
                    "outcome_class": outcome_class,
                    "attempts": attempts,
                    "response": response,
                }));
            }

            if !provider_id.is_empty() {
                if probe_attempt_should_mark_health_failure(status, marker_found) {
                    let error = format!("probe failed with outcome_class={outcome_class}");
                    let _ = service
                        .mark_provider_result(
                            &provider_id,
                            false,
                            Some(elapsed_ms),
                            Some(error.clone()),
                        )
                        .await;
                    if !model_id.is_empty() {
                        let _ = service
                            .mark_model_result(
                                &provider_id,
                                &model_id,
                                false,
                                Some(elapsed_ms),
                                Some(error),
                            )
                            .await;
                    }
                }
                if !forced_probe && !excluded_provider_ids.contains(&provider_id) {
                    excluded_provider_ids.push(provider_id);
                    continue;
                }
            }
            break;
        }

        let final_attempt = attempts.last().cloned().unwrap_or_else(|| json!({}));
        let outcome_class = final_attempt
            .get("outcome_class")
            .and_then(Value::as_str)
            .unwrap_or("probe_failed");
        service.record_probe_result(probe_result_payload(
            false,
            outcome_class,
            marker,
            lane,
            final_attempt.clone(),
            &attempts,
        ))?;
        Ok(json!({
            "ok": false,
            "status": final_attempt.get("status").cloned().unwrap_or(Value::Null),
            "marker": marker,
            "marker_found": final_attempt.get("marker_found").cloned().unwrap_or(Value::Bool(false)),
            "content": final_attempt.get("content").cloned().unwrap_or_else(|| json!("")),
            "latency_ms": final_attempt.get("latency_ms").cloned().unwrap_or(Value::Null),
            "route": final_attempt.get("route").cloned().unwrap_or_else(|| json!({})),
            "outcome_class": outcome_class,
            "attempts": attempts,
        }))
    })
    .await
}

#[derive(Debug, Clone)]
struct ProbeThrottle {
    provider_id: String,
    model_id: Option<String>,
    reason: String,
    message: String,
    last_failure_class: Option<String>,
    remaining_seconds: Option<i64>,
    cooldown_until_utc: Option<String>,
}

impl ProbeThrottle {
    fn to_json(&self) -> Value {
        json!({
            "provider_id": self.provider_id,
            "model_id": self.model_id,
            "reason": self.reason,
            "message": self.message,
            "last_failure_class": self.last_failure_class,
            "remaining_seconds": self.remaining_seconds,
            "cooldown_until_utc": self.cooldown_until_utc,
        })
    }
}

fn probe_route_stub(provider_id: &str, model: &str) -> Value {
    json!({
        "provider_id": provider_id,
        "model_id": if model == "auto" {
            Value::Null
        } else {
            Value::String(model.to_string())
        },
    })
}

fn probe_error_provider_id(error: &str, providers: &[ProviderState]) -> Option<String> {
    let lowered = error.to_ascii_lowercase();
    providers
        .iter()
        .filter(|provider| !provider.id.trim().is_empty())
        .find(|provider| {
            let id = provider.id.to_ascii_lowercase();
            lowered.contains(&format!("provider {id} "))
                || lowered.contains(&format!("provider `{id}`"))
                || lowered.contains(&format!("provider '{id}'"))
        })
        .map(|provider| provider.id.clone())
}

fn initial_probe_exclusions(
    providers: &[ProviderState],
    model: &str,
    recent_events: &[Value],
    ignore_throttle: bool,
) -> Vec<String> {
    providers
        .iter()
        .filter(|provider| {
            probe_throttle_decision(provider, model, recent_events, ignore_throttle).is_some()
        })
        .map(|provider| provider.id.clone())
        .collect()
}

fn probe_throttle_decision(
    provider: &ProviderState,
    model: &str,
    recent_events: &[Value],
    ignore_throttle: bool,
) -> Option<ProbeThrottle> {
    if ignore_throttle {
        return None;
    }
    let model_id = if model == "auto" {
        None
    } else {
        Some(model.to_string())
    };
    if provider.in_cooldown {
        return Some(ProbeThrottle {
            provider_id: provider.id.clone(),
            model_id,
            reason: "provider_cooldown".to_string(),
            message: format!(
                "probe skipped because provider `{}` is in cooldown",
                provider.id
            ),
            last_failure_class: None,
            remaining_seconds: cooldown_remaining_seconds(provider.cooldown_until_utc.as_deref()),
            cooldown_until_utc: provider.cooldown_until_utc.clone(),
        });
    }
    if provider
        .requests_per_minute
        .is_some_and(|max| provider.requests_used_minute >= max)
    {
        return Some(ProbeThrottle {
            provider_id: provider.id.clone(),
            model_id,
            reason: "minute_quota_exhausted".to_string(),
            message: format!(
                "probe skipped because provider `{}` reached its minute quota",
                provider.id
            ),
            last_failure_class: None,
            remaining_seconds: Some(60),
            cooldown_until_utc: None,
        });
    }
    if provider
        .requests_per_day
        .is_some_and(|max| provider.requests_used_day >= max)
    {
        return Some(ProbeThrottle {
            provider_id: provider.id.clone(),
            model_id,
            reason: "day_quota_exhausted".to_string(),
            message: format!(
                "probe skipped because provider `{}` reached its day quota",
                provider.id
            ),
            last_failure_class: None,
            remaining_seconds: Some(86_400),
            cooldown_until_utc: None,
        });
    }
    if provider.access_tier == "local" {
        return None;
    }
    if provider.consecutive_failures > 0 {
        if let Some(error) = provider.last_error.as_deref() {
            let class = classify_probe_failure_text(error);
            if probe_failure_class_triggers_throttle(&class) {
                let throttle_seconds = probe_failure_throttle_seconds(provider);
                return Some(ProbeThrottle {
                    provider_id: provider.id.clone(),
                    model_id,
                    reason: "provider_failure_memory".to_string(),
                    message: format!(
                        "probe skipped because provider `{}` has unresolved {} failure memory",
                        provider.id, class
                    ),
                    last_failure_class: Some(class),
                    remaining_seconds: Some(throttle_seconds),
                    cooldown_until_utc: None,
                });
            }
        }
    }
    let recent_failure = recent_probe_failure(&provider.id, model, recent_events)?;
    if !probe_failure_class_triggers_throttle(&recent_failure.class) {
        return None;
    }
    let throttle_seconds = probe_failure_throttle_seconds(provider);
    let remaining = recent_failure
        .ts_utc
        .and_then(|ts| {
            let elapsed = chrono::Utc::now()
                .signed_duration_since(ts)
                .num_seconds()
                .max(0);
            let remaining = throttle_seconds - elapsed;
            (remaining > 0).then_some(remaining)
        })
        .unwrap_or(throttle_seconds);
    if remaining <= 0 {
        return None;
    }
    Some(ProbeThrottle {
        provider_id: provider.id.clone(),
        model_id,
        reason: "recent_probe_failure".to_string(),
        message: format!(
            "probe skipped because provider `{}` had a recent {} failure",
            provider.id, recent_failure.class
        ),
        last_failure_class: Some(recent_failure.class),
        remaining_seconds: Some(remaining),
        cooldown_until_utc: None,
    })
}

#[derive(Debug, Clone)]
struct RecentProbeFailure {
    class: String,
    ts_utc: Option<chrono::DateTime<chrono::Utc>>,
}

fn recent_probe_failure(
    provider_id: &str,
    model: &str,
    recent_events: &[Value],
) -> Option<RecentProbeFailure> {
    for event in recent_events.iter().rev() {
        let event_kind = event
            .get("event")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !matches!(
            event_kind,
            "provider_result" | "provider_client_error" | "model_result" | "tool_fit_observation"
        ) {
            continue;
        }
        let payload = event.get("payload").unwrap_or(event);
        if payload.get("provider_id").and_then(Value::as_str) != Some(provider_id) {
            continue;
        }
        if model != "auto" {
            if let Some(event_model) = payload.get("model_id").and_then(Value::as_str) {
                if event_model != model {
                    continue;
                }
            }
        }
        if payload.get("ok").and_then(Value::as_bool) == Some(true) {
            return None;
        }
        let class = payload
            .get("outcome_class")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| {
                payload
                    .get("error")
                    .and_then(Value::as_str)
                    .map(classify_probe_failure_text)
            })
            .unwrap_or_else(|| "unknown_failure".to_string());
        return Some(RecentProbeFailure {
            class,
            ts_utc: event_timestamp_utc(event),
        });
    }
    None
}

fn event_timestamp_utc(event: &Value) -> Option<chrono::DateTime<chrono::Utc>> {
    event
        .get("ts_utc")
        .or_else(|| event.get("ts"))
        .and_then(Value::as_str)
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|ts| ts.with_timezone(&chrono::Utc))
}

fn cooldown_remaining_seconds(cooldown_until_utc: Option<&str>) -> Option<i64> {
    let until = cooldown_until_utc
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())?
        .with_timezone(&chrono::Utc);
    Some(
        until
            .signed_duration_since(chrono::Utc::now())
            .num_seconds()
            .max(0),
    )
}

fn probe_failure_throttle_seconds(provider: &ProviderState) -> i64 {
    let env_key = if provider.access_tier.contains("paid") {
        "ARDA_MANWE_PROBE_PAID_FAILURE_THROTTLE_SECONDS"
    } else {
        "ARDA_MANWE_PROBE_FAILURE_THROTTLE_SECONDS"
    };
    std::env::var(env_key)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| {
            if provider.access_tier.contains("paid") {
                900
            } else {
                300
            }
        })
}

fn probe_recent_event_limit() -> usize {
    std::env::var("ARDA_MANWE_PROBE_RECENT_EVENT_LIMIT")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1_000)
}

fn probe_failure_class_triggers_throttle(class: &str) -> bool {
    matches!(
        class,
        "auth_error"
            | "billing_or_credit_error"
            | "provider_server_error"
            | "rate_or_retry_error"
            | "spend_blocked"
            | "timeout"
            | "transport_failure"
    )
}

fn probe_attempt_outcome_class(status: u16, marker_found: bool) -> &'static str {
    if marker_found && status < 500 {
        return "success";
    }
    if !marker_found && (200..300).contains(&status) {
        return "marker_missing";
    }
    match status {
        401 | 403 => "auth_error",
        402 => "billing_or_credit_error",
        404 => "not_found",
        408 | 409 | 413 | 425 | 429 => "rate_or_retry_error",
        500..=599 => "provider_server_error",
        _ => "provider_http_error",
    }
}

fn probe_attempt_should_mark_health_failure(status: u16, marker_found: bool) -> bool {
    status >= 300 || marker_found
}

fn probe_result_payload(
    ok: bool,
    outcome_class: &str,
    marker: &str,
    lane: &str,
    final_attempt: Value,
    attempts: &[Value],
) -> Value {
    let route = final_attempt
        .get("route")
        .cloned()
        .unwrap_or_else(|| json!({}));
    json!({
        "ok": ok,
        "outcome_class": outcome_class,
        "marker": marker,
        "marker_found": final_attempt
            .get("marker_found")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        "status": final_attempt.get("status").cloned().unwrap_or(Value::Null),
        "latency_ms": final_attempt.get("latency_ms").cloned().unwrap_or(Value::Null),
        "provider_id": route.get("provider_id").cloned().unwrap_or(Value::Null),
        "model_id": route.get("model_id").cloned().unwrap_or(Value::Null),
        "route": route,
        "lane": lane,
        "attempt_count": attempts.len(),
        "attempts": attempts,
    })
}

fn classify_probe_failure_text(error: &str) -> String {
    let lowered = error.to_ascii_lowercase();
    if lowered.contains("billing")
        || lowered.contains("credit")
        || lowered.contains("balance")
        || lowered.contains("recharge")
        || lowered.contains("payment")
    {
        "billing_or_credit_error".to_string()
    } else if lowered.contains("429") || lowered.contains("rate") || lowered.contains("quota") {
        "rate_or_retry_error".to_string()
    } else if lowered.contains("401") || lowered.contains("403") || lowered.contains("auth") {
        "auth_error".to_string()
    } else if lowered.contains("timeout") {
        "timeout".to_string()
    } else if lowered.contains("transport") || lowered.contains("connection") {
        "transport_failure".to_string()
    } else if lowered.contains("server") || lowered.contains("502") || lowered.contains("503") {
        "provider_server_error".to_string()
    } else {
        "unknown_failure".to_string()
    }
}

async fn route_history(State(service): State<ManweService>) -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "routes": service.route_history(100).await
    }))
}

async fn route(
    State(service): State<ManweService>,
    Json(req): Json<ManweRequestEnvelope>,
) -> impl IntoResponse {
    map_result_async(async move {
        Ok(json!({"ok": true, "decision": service.route_preview(req).await?}))
    })
    .await
}

async fn proxy(
    State(service): State<ManweService>,
    Json(req): Json<ManweRequestEnvelope>,
) -> impl IntoResponse {
    map_result_async(async move { service.proxy_openai(req).await }).await
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ProviderResultRequest {
    provider_id: String,
    ok: bool,
    #[serde(default)]
    latency_ms: Option<u64>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct ModelStreamingValidationRequest {
    provider_id: String,
    model_id: String,
    streaming_validated: bool,
    #[serde(default)]
    error: Option<String>,
}

async fn provider_result(
    State(service): State<ManweService>,
    headers: HeaderMap,
    Json(req): Json<ProviderResultRequest>,
) -> impl IntoResponse {
    if !authorize_mutation(&headers) {
        return openai_error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid Authorization header for mutation",
        );
    }
    map_result_async(async move {
        service
            .mark_provider_result(&req.provider_id, req.ok, req.latency_ms, req.error)
            .await?;
        Ok(json!({"ok": true}))
    })
    .await
    .into_response()
}

async fn model_streaming_validation(
    State(service): State<ManweService>,
    headers: HeaderMap,
    Json(req): Json<ModelStreamingValidationRequest>,
) -> impl IntoResponse {
    if !authorize_mutation(&headers) {
        return openai_error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid Authorization header for mutation",
        );
    }
    map_result_async(async move {
        service
            .mark_model_streaming_validation(
                &req.provider_id,
                &req.model_id,
                req.streaming_validated,
                req.error,
            )
            .await?;
        Ok(json!({"ok": true}))
    })
    .await
    .into_response()
}

async fn reload_config(
    State(service): State<ManweService>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if !authorize_mutation(&headers) {
        return openai_error(
            StatusCode::UNAUTHORIZED,
            "missing or invalid Authorization header for mutation",
        );
    }
    map_result_async(async move { service.reload_provider_config().await })
        .await
        .into_response()
}

async fn reconcile_catalogs(State(service): State<ManweService>) -> impl IntoResponse {
    map_result_async(async move { service.reconcile_provider_catalogs().await }).await
}

async fn observability(State(service): State<ManweService>) -> impl IntoResponse {
    map_result_async(async move { service.route_observability_rollup().await }).await
}

async fn paths(State(service): State<ManweService>) -> impl IntoResponse {
    Json(json!({"ok": true, "paths": service.paths()}))
}

async fn events(
    State(service): State<ManweService>,
) -> Sse<impl Stream<Item = std::result::Result<Event, Infallible>>> {
    let stream = IntervalStream::new(tokio::time::interval(std::time::Duration::from_secs(5))).map(
        move |_| {
            let service = service.clone();
            async move {
                let payload = build_event_payload(&service)
                    .await
                    .unwrap_or_else(|err| json!({"ok": false, "error": err.to_string()}));
                Ok(Event::default().event("status").data(payload.to_string()))
            }
        },
    );
    Sse::new(stream.then(|fut| fut)).keep_alive(KeepAlive::default())
}

async fn openai_models(State(service): State<ManweService>) -> impl IntoResponse {
    let providers = service
        .providers()
        .await
        .into_iter()
        .map(filter_provider_catalog_models)
        .collect::<Vec<_>>();
    let mut data = Vec::new();
    data.push(json!({
        "id": "auto",
        "object": "model",
        "owned_by": "manwe",
        "created": 0
    }));
    for provider in providers {
        for model in provider.models {
            data.push(json!({
                "id": advertised_model_id(&provider.id, &model.id),
                "object": "model",
                "owned_by": provider.id,
                "created": 0
            }));
        }
    }
    Json(json!({
        "object": "list",
        "data": data
    }))
}

fn advertised_model_id(provider_id: &str, model_id: &str) -> String {
    if model_id.starts_with(&format!("{provider_id}/")) {
        model_id.to_string()
    } else {
        format!("{provider_id}/{model_id}")
    }
}

fn filter_provider_catalog_models(mut provider: ProviderState) -> ProviderState {
    provider.models = visible_provider_catalog_models(&provider.id, &provider.models);
    provider
}

fn providers_query_ids(query: &ProvidersQuery) -> Vec<String> {
    query
        .ids
        .as_deref()
        .or(query.provider_ids.as_deref())
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn provider_row(
    provider: ProviderState,
    capacity_probe: Option<crate::adaptive::service::ProviderCapacityProbeRecord>,
    recent_events: &[Value],
    include_models: bool,
) -> Value {
    let operational = classify_provider_operational_state(&provider, chrono::Utc::now());
    let provider_id = provider.id.clone();
    let recent = recent_provider_result(recent_events, &provider.id);
    let supports_streaming = !matches!(
        provider.driver.as_str(),
        "hermes_agent_cli" | "codex_responses"
    ) && provider.models.iter().any(|model| {
        model.capabilities.streaming != Some(false) && model.streaming_validated != Some(false)
    });
    let probe_eligible = provider.enabled
        && provider.healthy
        && !provider.in_cooldown
        && !operational.blocked
        && provider.models.iter().any(|model| {
            model.healthy
                && !model.in_cooldown
                && model.capable_tasks.iter().any(|task| task == "chat")
        });
    let mut row = serde_json::Map::new();
    row.insert("id".to_string(), json!(provider_id.clone()));
    row.insert("provider_id".to_string(), json!(provider_id));
    row.insert("name".to_string(), json!(provider.name));
    row.insert("base_url".to_string(), json!(provider.base_url));
    row.insert("api_key_env".to_string(), json!(provider.api_key_env));
    row.insert("access_tier".to_string(), json!(provider.access_tier));
    row.insert("quality_band".to_string(), json!(provider.quality_band));
    row.insert("driver".to_string(), json!(provider.driver));
    row.insert("hermes_bin".to_string(), json!(provider.hermes_bin));
    row.insert(
        "hermes_provider".to_string(),
        json!(provider.hermes_provider),
    );
    row.insert(
        "hermes_toolsets".to_string(),
        json!(provider.hermes_toolsets),
    );
    row.insert(
        "hermes_bridge".to_string(),
        hermes_bridge_metadata(&provider),
    );
    row.insert("supports_tools".to_string(), json!(provider.supports_tools));
    row.insert(
        "supports_structured_output".to_string(),
        json!(provider.supports_structured_output),
    );
    row.insert("supports_streaming".to_string(), json!(supports_streaming));
    row.insert("probe_eligible".to_string(), json!(probe_eligible));
    row.insert(
        "intelligence_refreshed_at_utc".to_string(),
        json!(provider.intelligence_refreshed_at_utc),
    );
    row.insert("probe_model".to_string(), json!(provider.probe_model));
    row.insert("probe_profile".to_string(), json!(provider.probe_profile));
    row.insert("enabled".to_string(), json!(provider.enabled));
    row.insert("has_api_key".to_string(), json!(provider.has_api_key));
    row.insert("healthy".to_string(), json!(provider.healthy));
    row.insert("in_cooldown".to_string(), json!(provider.in_cooldown));
    row.insert(
        "cooldown_until_utc".to_string(),
        json!(provider.cooldown_until_utc),
    );
    row.insert(
        "cooldown_backoff_seconds".to_string(),
        json!(provider.cooldown_backoff_seconds),
    );
    row.insert(
        "requests_per_minute".to_string(),
        json!(provider.requests_per_minute),
    );
    row.insert(
        "requests_used_minute".to_string(),
        json!(provider.requests_used_minute),
    );
    row.insert(
        "minute_window_started_utc".to_string(),
        json!(provider.minute_window_started_utc),
    );
    row.insert(
        "requests_per_day".to_string(),
        json!(provider.requests_per_day),
    );
    row.insert(
        "requests_used_day".to_string(),
        json!(provider.requests_used_day),
    );
    row.insert(
        "day_window_started_utc".to_string(),
        json!(provider.day_window_started_utc),
    );
    row.insert("error_count".to_string(), json!(provider.error_count));
    row.insert(
        "consecutive_failures".to_string(),
        json!(provider.consecutive_failures),
    );
    row.insert(
        "consecutive_successes".to_string(),
        json!(provider.consecutive_successes),
    );
    row.insert("last_error".to_string(), json!(provider.last_error));
    row.insert("avg_latency_ms".to_string(), json!(provider.avg_latency_ms));
    row.insert(
        "active_connections".to_string(),
        json!(provider.active_connections),
    );
    row.insert(
        "last_reservation_utc".to_string(),
        json!(provider.last_reservation_utc),
    );
    row.insert(
        "last_success_utc".to_string(),
        json!(recent.last_success_utc),
    );
    row.insert(
        "last_failure_utc".to_string(),
        json!(recent.last_failure_utc),
    );
    row.insert(
        "last_failure_class".to_string(),
        json!(recent.last_failure_class),
    );
    row.insert("operational_state".to_string(), json!(operational.state));
    row.insert("operational_reason".to_string(), json!(operational.reason));
    row.insert(
        "operational_blocked".to_string(),
        json!(operational.blocked),
    );
    row.insert(
        "reset_seconds_estimate".to_string(),
        json!(operational.reset_seconds_estimate),
    );
    row.insert("capacity_probe".to_string(), json!(capacity_probe));
    if include_models {
        row.insert("models".to_string(), json!(provider.models));
        row.insert("model_count".to_string(), json!(provider.models.len()));
    } else {
        row.insert("model_count".to_string(), json!(provider.models.len()));
    }
    Value::Object(row)
}

fn compact_provider_row(row: Value) -> Value {
    const COMPACT_KEYS: &[&str] = &[
        "id",
        "provider_id",
        "name",
        "access_tier",
        "quality_band",
        "driver",
        "hermes_provider",
        "supports_tools",
        "supports_structured_output",
        "supports_streaming",
        "probe_eligible",
        "probe_model",
        "probe_profile",
        "enabled",
        "has_api_key",
        "healthy",
        "in_cooldown",
        "cooldown_until_utc",
        "last_success_utc",
        "last_failure_utc",
        "last_failure_class",
        "last_error",
        "avg_latency_ms",
        "operational_state",
        "operational_reason",
        "operational_blocked",
        "reset_seconds_estimate",
        "model_count",
    ];
    let Some(map) = row.as_object() else {
        return row;
    };
    let mut compact = serde_json::Map::new();
    for key in COMPACT_KEYS {
        if let Some(value) = map.get(*key) {
            compact.insert((*key).to_string(), value.clone());
        }
    }
    Value::Object(compact)
}

fn hermes_bridge_metadata(provider: &ProviderState) -> Value {
    let default_model = provider
        .models
        .iter()
        .find(|model| model.is_default)
        .or_else(|| provider.models.first())
        .map(|model| model.id.as_str());
    match provider.driver.as_str() {
        "hermes_agent_cli" => {
            let readiness = hermes_cli_readiness_summary(provider, default_model);
            json!({
                "type": "hermes_agent_cli",
                "persistent": false,
                "provider": provider.hermes_provider,
                "readiness": readiness,
                "latency_strategy": "fast-lane penalty plus cached blocked readiness"
            })
        }
        "hermes_proxy" => json!({
            "type": "hermes_proxy",
            "persistent": true,
            "provider": provider.hermes_provider,
            "base_url": hermes_proxy_base_url(provider),
            "readiness": if provider.enabled { "started_on_first_route" } else { "disabled" },
            "latency_strategy": "warm local OpenAI-compatible Hermes proxy process"
        }),
        "codex_responses" => json!({
            "type": "codex_responses",
            "persistent": true,
            "provider": provider.hermes_provider,
            "base_url": provider.base_url,
            "readiness": if provider.enabled { "uses_hermes_auth_store" } else { "disabled" },
            "latency_strategy": "Manwe pooled HTTP client to Codex Responses API; no hermes CLI subprocess"
        }),
        _ => json!({
            "type": provider.driver,
            "persistent": false,
        }),
    }
}

#[derive(Default)]
struct RecentProviderResult {
    last_success_utc: Option<String>,
    last_failure_utc: Option<String>,
    last_failure_class: Option<String>,
}

fn recent_provider_result(events: &[Value], provider_id: &str) -> RecentProviderResult {
    let mut result = RecentProviderResult::default();
    for event in events.iter().rev() {
        if event.get("event").and_then(Value::as_str) != Some("provider_result") {
            continue;
        }
        let payload = event.get("payload").unwrap_or(event);
        if payload.get("provider_id").and_then(Value::as_str) != Some(provider_id) {
            continue;
        }
        let ts = event
            .get("ts_utc")
            .or_else(|| event.get("ts"))
            .and_then(Value::as_str)
            .map(str::to_string);
        if payload.get("ok").and_then(Value::as_bool) == Some(true) {
            if result.last_success_utc.is_none() {
                result.last_success_utc = ts;
            }
        } else if result.last_failure_utc.is_none() {
            result.last_failure_utc = ts;
            let error = payload
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or_default();
            result.last_failure_class = Some(classify_failure_text(error).to_string());
        }
        if result.last_success_utc.is_some() && result.last_failure_utc.is_some() {
            break;
        }
    }
    result
}

fn classify_failure_text(error: &str) -> &'static str {
    let lowered = error.to_ascii_lowercase();
    if lowered.contains("billing")
        || lowered.contains("credit")
        || lowered.contains("balance")
        || lowered.contains("recharge")
        || lowered.contains("payment")
    {
        "billing_or_credit_error"
    } else if lowered.contains("unsupported") || lowered.contains("not supported") {
        "model_unavailable"
    } else if lowered.contains("429") || lowered.contains("rate") || lowered.contains("quota") {
        "rate_or_retry_error"
    } else if lowered.contains("401") || lowered.contains("403") || lowered.contains("auth") {
        "auth_error"
    } else if lowered.contains("timeout") {
        "timeout"
    } else if lowered.contains("transport") || lowered.contains("connection") {
        "transport_failure"
    } else if lowered.contains("context") || lowered.contains("too many tokens") {
        "context_overflow"
    } else {
        "unknown_error"
    }
}

fn probe_response_text(response: &Value) -> String {
    response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(Value::as_object)
        .and_then(|message| {
            ["content", "reasoning", "reasoning_content"]
                .iter()
                .find_map(|key| message.get(*key).and_then(Value::as_str))
        })
        .unwrap_or_default()
        .to_string()
}

fn probe_marker_found(content: &str, marker: &str) -> bool {
    let trimmed = content.trim();
    trimmed == marker || content.lines().any(|line| line.trim() == marker)
}

fn default_probe_model_for_provider(
    providers: &[ProviderState],
    provider_id: &str,
) -> Option<String> {
    let provider = providers
        .iter()
        .find(|provider| provider.id == provider_id)?;
    if let Some(probe_model) = provider.probe_model.as_deref() {
        if provider.models.iter().any(|model| {
            (model.id == probe_model || model.alias_matches(probe_model))
                && model.healthy
                && !model.in_cooldown
                && model.capable_tasks.iter().any(|task| task == "chat")
        }) {
            return Some(probe_model.to_string());
        }
    }
    provider
        .models
        .iter()
        .filter(|model| {
            model.healthy
                && !model.in_cooldown
                && model.capable_tasks.iter().any(|task| task == "chat")
        })
        .find(|model| model.is_default)
        .or_else(|| {
            provider.models.iter().find(|model| {
                model.healthy
                    && !model.in_cooldown
                    && model.capable_tasks.iter().any(|task| task == "chat")
            })
        })
        .map(|model| model.id.clone())
}

fn visible_provider_catalog_models(provider_id: &str, models: &[ModelState]) -> Vec<ModelState> {
    let _ = provider_id;
    // `/providers` should reflect the configured catalog truthfully. Agentic
    // tool-use eligibility is enforced later by route policy, not by hiding
    // configured models from the operator-facing catalog.
    models.to_vec()
}

async fn openai_chat_completions(
    State(service): State<ManweService>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let req = match openai_body_to_envelope_with_headers(&service, &body, Some(&headers)).await {
        Ok(req) => req,
        Err(err) => return openai_error(StatusCode::BAD_REQUEST, &err.to_string()),
    };

    if body.get("stream").and_then(Value::as_bool) == Some(true) {
        if should_emulate_streaming_tool_response(&req) {
            let mut non_stream_body = body.clone();
            if let Some(payload) = non_stream_body.as_object_mut() {
                payload.insert("stream".to_string(), Value::Bool(false));
            }
            let mut non_stream_req = req.clone();
            non_stream_req.options["stream"] = Value::Bool(false);
            return match service
                .proxy_openai_passthrough_with_route(non_stream_req, non_stream_body)
                .await
            {
                Ok(outcome) => {
                    let sse_body = openai_completion_to_sse(&outcome.response);
                    axum::response::Response::builder()
                        .status(StatusCode::OK)
                        .header(axum::http::header::CONTENT_TYPE, "text/event-stream")
                        .header(axum::http::header::CACHE_CONTROL, "no-cache")
                        .header(axum::http::header::CONNECTION, "keep-alive")
                        .header("x-manwe-route-id", outcome.route_id.as_str())
                        .header("x-manwe-provider-id", outcome.provider_id.as_str())
                        .header("x-manwe-model-id", outcome.model_id.as_str())
                        .header("x-manwe-route-class", outcome.route_class.as_str())
                        .header("x-manwe-execution-lane", outcome.execution_lane.as_str())
                        .header("x-manwe-stream-emulated", "true")
                        .body(Body::from(sse_body))
                        .unwrap_or_else(|_| {
                            openai_error(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "failed to build emulated streaming response",
                            )
                        })
                }
                Err(err) => openai_error(StatusCode::BAD_GATEWAY, &err.to_string()),
            };
        }
        let strip_reasoning = streaming_strip_reasoning_enabled(&body)
            .unwrap_or_else(|| streaming_strip_reasoning_default(&req));
        return match service.proxy_openai_streaming(req, body).await {
            Ok(outcome) => {
                let upstream = outcome.response;
                let upstream_status =
                    StatusCode::from_u16(upstream.status().as_u16()).unwrap_or(StatusCode::OK);
                let content_type = upstream
                    .headers()
                    .get(axum::http::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("text/event-stream")
                    .to_string();
                let service_for_stream = service.clone();
                let provider_id = outcome.provider_id;
                let model_id = outcome.model_id;
                let route_class = outcome.route_class;
                let execution_lane = outcome.execution_lane;
                let route_id = outcome.route_id;
                let marked = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let provider_header = provider_id.clone();
                let model_header = model_id.clone();
                let route_class_header = route_class.clone();
                let execution_lane_header = execution_lane.clone();
                let byte_stream = upstream.bytes_stream().map(move |result| {
                    streaming_upstream_chunk_to_downstream_chunk_with_feedback(
                        result,
                        &service_for_stream,
                        &provider_id,
                        &model_id,
                        &marked,
                        strip_reasoning,
                    )
                });
                axum::response::Response::builder()
                    .status(upstream_status)
                    .header(axum::http::header::CONTENT_TYPE, content_type)
                    .header(axum::http::header::CACHE_CONTROL, "no-cache")
                    .header(axum::http::header::CONNECTION, "keep-alive")
                    .header("x-manwe-route-id", route_id.as_str())
                    .header("x-manwe-provider-id", provider_header.as_str())
                    .header("x-manwe-model-id", model_header.as_str())
                    .header("x-manwe-route-class", route_class_header.as_str())
                    .header("x-manwe-execution-lane", execution_lane_header.as_str())
                    .body(Body::from_stream(byte_stream))
                    .unwrap_or_else(|_| {
                        openai_error(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "failed to build streaming response",
                        )
                    })
            }
            Err(err) => openai_error(StatusCode::BAD_GATEWAY, &err.to_string()),
        };
    }

    let strip_reasoning = streaming_strip_reasoning_enabled(&body)
        .unwrap_or_else(|| streaming_strip_reasoning_default(&req));

    match service.proxy_openai_passthrough_with_route(req, body).await {
        Ok(outcome) => {
            let mut response_payload = outcome.response;
            if strip_reasoning {
                strip_reasoning_fields(&mut response_payload);
            }
            let mut response = (
                StatusCode::from_u16(outcome.status).unwrap_or(StatusCode::OK),
                Json(response_payload),
            )
                .into_response();
            insert_manwe_route_header(&mut response, "x-manwe-route-id", &outcome.route_id);
            insert_manwe_route_header(&mut response, "x-manwe-provider-id", &outcome.provider_id);
            insert_manwe_route_header(&mut response, "x-manwe-model-id", &outcome.model_id);
            insert_manwe_route_header(&mut response, "x-manwe-route-class", &outcome.route_class);
            insert_manwe_route_header(
                &mut response,
                "x-manwe-execution-lane",
                &outcome.execution_lane,
            );
            response
        }
        Err(err) => openai_error(StatusCode::BAD_GATEWAY, &err.to_string()),
    }
}

fn should_emulate_streaming_tool_response(req: &ManweRequestEnvelope) -> bool {
    let enabled = std::env::var("ARDA_MANWE_EMULATE_TOOL_STREAMING")
        .ok()
        .map(|value| !matches!(value.trim(), "0" | "false" | "FALSE" | "off" | "OFF"))
        .unwrap_or(true);
    enabled
        && req
            .options
            .get("tool_use_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn openai_completion_to_sse(response: &Value) -> String {
    let id = response
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("chatcmpl-manwe-emulated");
    let model = response
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("manwe-emulated");
    let message = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .cloned()
        .unwrap_or_else(|| serde_json::json!({"role":"assistant","content":""}));
    let finish_reason = response
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("finish_reason"))
        .cloned()
        .unwrap_or_else(|| Value::String("stop".to_string()));
    let mut delta = serde_json::Map::new();
    delta.insert(
        "role".to_string(),
        message
            .get("role")
            .cloned()
            .unwrap_or_else(|| Value::String("assistant".to_string())),
    );
    for key in ["content", "tool_calls", "function_call"] {
        if let Some(value) = message.get(key) {
            delta.insert(key.to_string(), value.clone());
        }
    }
    let chunk = serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": Value::Object(delta),
            "finish_reason": null
        }]
    });
    let final_chunk = serde_json::json!({
        "id": id,
        "object": "chat.completion.chunk",
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": finish_reason
        }]
    });
    format!("data: {chunk}\n\ndata: {final_chunk}\n\ndata: [DONE]\n\n")
}

fn insert_manwe_route_header(response: &mut Response, name: &'static str, value: &str) {
    if value.trim().is_empty() {
        return;
    }
    if let Ok(value) = axum::http::HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    }
}

fn streaming_upstream_chunk_to_downstream_chunk_with_feedback(
    result: std::result::Result<Bytes, reqwest::Error>,
    service: &ManweService,
    provider_id: &str,
    model_id: &str,
    marked: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    strip_reasoning: bool,
) -> std::result::Result<Bytes, Infallible> {
    match result {
        Ok(chunk) => {
            if strip_reasoning {
                Ok(strip_reasoning_from_sse_chunk(chunk))
            } else {
                Ok(chunk)
            }
        }
        Err(err) => {
            let message = err.to_string();
            // First chunk error after a successful HTTP 200 → record a
            // provider/model failure so this upstream stops winning the
            // weighted-random pick until it cools down. Without this the
            // bytes_stream emits one Err per failed chunk and we'd report
            // the failure dozens of times in a single dead stream.
            if !marked.swap(true, std::sync::atomic::Ordering::SeqCst) {
                warn!(
                    provider = %provider_id,
                    model = %model_id,
                    error = %message,
                    "upstream streaming provider dropped connection"
                );
                service
                    .metrics()
                    .observe_streaming_chunk_error(provider_id, model_id);
                let service = service.clone();
                let provider_id = provider_id.to_string();
                let model_id = model_id.to_string();
                let err_message = message.clone();
                tokio::spawn(async move {
                    let _ = service
                        .mark_provider_result(
                            &provider_id,
                            false,
                            None,
                            Some(format!("streaming chunk decode failed: {err_message}")),
                        )
                        .await;
                    let _ = service
                        .mark_model_result(
                            &provider_id,
                            &model_id,
                            false,
                            None,
                            Some(format!("streaming chunk decode failed: {err_message}")),
                        )
                        .await;
                    let _ = service
                        .mark_model_streaming_validation(
                            &provider_id,
                            &model_id,
                            false,
                            Some(format!("streaming chunk decode failed: {err_message}")),
                        )
                        .await;
                });
            }
            Ok(Bytes::from(format!(
                "data: {}\n\ndata: [DONE]\n\n",
                json!({
                    "error": {
                        "message": format!("upstream streaming provider dropped connection: {message}"),
                        "type": "upstream_stream_error"
                    }
                })
            )))
        }
    }
}

fn streaming_strip_reasoning_enabled(body: &Value) -> Option<bool> {
    body.get("transform")
        .and_then(Value::as_object)
        .and_then(|transform| transform.get("strip_reasoning"))
        .and_then(Value::as_bool)
        .or_else(|| {
            body.get("extra_body")
                .and_then(Value::as_object)
                .and_then(|extra| extra.get("transform"))
                .and_then(Value::as_object)
                .and_then(|transform| transform.get("strip_reasoning"))
                .and_then(Value::as_bool)
        })
}

fn streaming_strip_reasoning_default(req: &ManweRequestEnvelope) -> bool {
    req.task_type == "code"
        || req
            .options
            .get("tool_use_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || req
            .options
            .get("tool_choice")
            .is_some_and(tool_choice_requires_tool_call)
        || req
            .options
            .get("workload_role")
            .and_then(Value::as_str)
            .is_some_and(|role| role == "execution")
        || req
            .options
            .get("execution_lane")
            .and_then(Value::as_str)
            .is_some_and(|lane| lane == "execution")
}

fn strip_reasoning_from_sse_chunk(chunk: Bytes) -> Bytes {
    let Ok(text) = std::str::from_utf8(&chunk) else {
        return chunk;
    };
    let mut changed = false;
    let mut out = String::with_capacity(text.len());
    for line in text.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let newline = if line.ends_with('\n') { "\n" } else { "" };
        let trimmed = line_without_newline.trim_start();
        let prefix_len = line_without_newline.len().saturating_sub(trimmed.len());
        if let Some(data) = trimmed.strip_prefix("data:") {
            let data = data.trim_start();
            if data != "[DONE]" {
                if let Ok(mut parsed) = serde_json::from_str::<Value>(data) {
                    strip_reasoning_fields(&mut parsed);
                    out.push_str(&line_without_newline[..prefix_len]);
                    out.push_str("data: ");
                    out.push_str(&parsed.to_string());
                    out.push_str(newline);
                    changed = true;
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    if changed {
        Bytes::from(out)
    } else {
        chunk
    }
}

fn strip_reasoning_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("reasoning");
            map.remove("reasoning_content");
            map.remove("reasoning_details");
            map.remove("thought_signature");
            map.remove("thought_signatures");
            map.remove("extra_content");
            if let Some(Value::String(content)) = map.get_mut("content") {
                let stripped = strip_visible_think_text(content);
                if stripped != *content {
                    *content = stripped;
                }
            }
            for child in map.values_mut() {
                strip_reasoning_fields(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_reasoning_fields(child);
            }
        }
        _ => {}
    }
}

fn strip_visible_think_text(content: &str) -> String {
    let trimmed_start = content.trim_start();
    if let Some(rest) = trimmed_start.strip_prefix("</think>") {
        return rest.trim_start().to_string();
    }

    let Some(after_open) = trimmed_start.strip_prefix("<think>") else {
        return content.to_string();
    };
    let Some((_, after_close)) = after_open.split_once("</think>") else {
        return String::new();
    };
    after_close.trim_start().to_string()
}

fn prometheus_escape_label_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('"', "\\\"")
}

async fn map_result_async<Fut>(fut: Fut) -> Json<Value>
where
    Fut: std::future::Future<Output = Result<Value>>,
{
    match fut.await {
        Ok(v) => Json(v),
        Err(err) => Json(json!({"ok": false, "error": err.to_string()})),
    }
}

async fn openai_body_to_envelope(
    service: &ManweService,
    body: &Value,
) -> Result<ManweRequestEnvelope> {
    openai_body_to_envelope_with_headers(service, body, None).await
}

async fn openai_body_to_envelope_with_headers(
    service: &ManweService,
    body: &Value,
    headers: Option<&HeaderMap>,
) -> Result<ManweRequestEnvelope> {
    let messages = body
        .get("messages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let requested_model = body.get("model").and_then(Value::as_str).unwrap_or("auto");
    let tool_request = request_has_tool_or_skill_intent(body, &messages);
    let tool_schema_count = request_tool_schema_count(body)
        + body
            .get("extra_body")
            .map(request_tool_schema_count)
            .unwrap_or(0);

    let mut options = serde_json::json!({
        "endpoint": "/chat/completions"
    });

    for key in [
        "max_tokens",
        "temperature",
        "top_p",
        "stream",
        "response_format",
        "tools",
        "tool_choice",
        "stop",
        "exclude_provider_ids",
        "excluded_provider_ids",
        "exclude_model_ids",
        "excluded_model_ids",
        "prefer_probe_model",
        "dry_run",
        "source_surface",
        "harness",
        "session_id",
        "conversation_id",
        "turn_id",
        "trace_id",
        "receipt_id",
        "skill",
        "skills",
        "toolset",
        "toolsets",
        "tool_mode",
        "tool_pool_strategy",
        "agent_mode",
        "allow_visible_reasoning",
        "allow_thinking_models",
        "require_reasoning_model",
    ] {
        if let Some(value) = body.get(key) {
            options[key] = value.clone();
        }
    }

    for key in [
        "workload_role",
        "context_priority",
        "quality_priority",
        "quality_tier",
        "cost_policy",
        "cost_tier",
        "privacy_requirement",
        "inference_origin",
        "origin_preference",
        "context_window_target",
        "execution_lane",
        "force_provider_id",
        "force_model_id",
        "allow_forced_provider_fallback",
        "exclude_provider_ids",
        "excluded_provider_ids",
        "exclude_model_ids",
        "excluded_model_ids",
        "dry_run",
        "tool_use_required",
        "source_surface",
        "harness",
        "skill",
        "skills",
        "toolset",
        "toolsets",
        "tool_mode",
        "agent_mode",
        "allow_visible_reasoning",
        "allow_thinking_models",
        "require_reasoning_model",
    ] {
        if let Some(value) = body.get(key) {
            options[key] = value.clone();
        }
    }

    if let Some(routing) = body.get("routing").and_then(Value::as_object) {
        for key in [
            "workload_role",
            "context_priority",
            "quality_priority",
            "quality_tier",
            "cost_policy",
            "cost_tier",
            "privacy_requirement",
            "inference_origin",
            "origin_preference",
            "context_window_target",
            "execution_lane",
            "force_provider_id",
            "force_model_id",
            "allow_forced_provider_fallback",
            "exclude_provider_ids",
            "excluded_provider_ids",
            "exclude_model_ids",
            "excluded_model_ids",
            "dry_run",
            "tool_use_required",
            "source_surface",
            "harness",
            "skill",
            "skills",
            "toolset",
            "toolsets",
            "tool_mode",
            "tool_pool_strategy",
            "agent_mode",
            "allow_visible_reasoning",
            "allow_thinking_models",
            "require_reasoning_model",
        ] {
            if let Some(value) = routing.get(key) {
                options[key] = value.clone();
            }
        }
    }

    if let Some(extra_body) = body.get("extra_body").and_then(Value::as_object) {
        for key in [
            "workload_role",
            "context_priority",
            "quality_priority",
            "quality_tier",
            "cost_policy",
            "cost_tier",
            "privacy_requirement",
            "inference_origin",
            "origin_preference",
            "context_window_target",
            "execution_lane",
            "force_provider_id",
            "force_model_id",
            "allow_forced_provider_fallback",
            "dry_run",
            "tool_use_required",
            "source_surface",
            "harness",
            "session_id",
            "conversation_id",
            "turn_id",
            "trace_id",
            "receipt_id",
            "skill",
            "skills",
            "toolset",
            "toolsets",
            "tool_mode",
            "tool_pool_strategy",
            "agent_mode",
            "allow_visible_reasoning",
            "allow_thinking_models",
            "require_reasoning_model",
        ] {
            if let Some(value) = extra_body.get(key) {
                options[key] = value.clone();
            }
        }

        if let Some(routing) = extra_body.get("routing").and_then(Value::as_object) {
            for key in [
                "workload_role",
                "context_priority",
                "quality_priority",
                "quality_tier",
                "cost_policy",
                "cost_tier",
                "privacy_requirement",
                "inference_origin",
                "origin_preference",
                "context_window_target",
                "execution_lane",
                "force_provider_id",
                "force_model_id",
                "allow_forced_provider_fallback",
                "exclude_provider_ids",
                "excluded_provider_ids",
                "exclude_model_ids",
                "excluded_model_ids",
                "dry_run",
                "tool_use_required",
                "source_surface",
                "harness",
                "skill",
                "skills",
                "toolset",
                "toolsets",
                "tool_mode",
                "tool_pool_strategy",
                "agent_mode",
                "allow_visible_reasoning",
                "allow_thinking_models",
                "require_reasoning_model",
                "governance_method",
                "philosopher_method",
                "governance_philosopher",
                "philosopher_lens",
                "philosopher",
                "governance_chain_id",
                "chain_id",
            ] {
                if let Some(value) = routing.get(key) {
                    options[key] = value.clone();
                }
            }
        }
    }

    if let Some(headers) = headers {
        apply_openai_route_headers(headers, &mut options);
    }

    if tool_schema_count > 0 {
        options["tools_available"] = Value::Bool(true);
        options["tool_schema_count"] = Value::Number((tool_schema_count as u64).into());
    }

    if tool_request {
        options["tool_use_required"] = Value::Bool(true);
        if options.get("workload_role").is_none() {
            options["workload_role"] = Value::String("execution".to_string());
        }
        if options.get("origin_preference").is_none()
            && options.get("inference_origin").is_none()
            && options.get("tool_pool_strategy").is_none()
            && !options
                .get("allow_free_tool_pool")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            && !options
                .get("free_tool_pool")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            options["origin_preference"] = Value::String("auto".to_string());
        }
        if options.get("cost_policy").is_none() && options.get("cost_tier").is_none() {
            options["cost_policy"] = Value::String("balanced".to_string());
        }
        if options.get("quality_priority").is_none() && options.get("quality_tier").is_none() {
            options["quality_priority"] = Value::String("high".to_string());
        }
    }

    let has_forced_route = options
        .get("force_provider_id")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        || options
            .get("force_model_id")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty());
    if !has_forced_route {
        if let Some((provider_id, model_id)) =
            resolve_requested_model(service, requested_model).await?
        {
            options["force_provider_id"] = Value::String(provider_id);
            options["force_model_id"] = Value::String(model_id);
        }
    }

    let agent_id = body
        .get("agent_id")
        .and_then(Value::as_str)
        .or_else(|| body.get("source_agent").and_then(Value::as_str))
        .or_else(|| {
            body.get("routing")
                .and_then(Value::as_object)
                .and_then(|routing| routing.get("agent_id"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            body.get("routing")
                .and_then(Value::as_object)
                .and_then(|routing| routing.get("source_agent"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            body.get("extra_body")
                .and_then(Value::as_object)
                .and_then(|extra| extra.get("agent_id").or_else(|| extra.get("source_agent")))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            body.get("extra_body")
                .and_then(Value::as_object)
                .and_then(|extra| extra.get("routing"))
                .and_then(Value::as_object)
                .and_then(|routing| {
                    routing
                        .get("agent_id")
                        .or_else(|| routing.get("source_agent"))
                })
                .and_then(Value::as_str)
        })
        .or_else(|| {
            headers.and_then(|headers| {
                header_str(headers, "x-manwe-agent-id")
                    .or_else(|| header_str(headers, "x-arda-agent-id"))
            })
        })
        .unwrap_or("openai_shim")
        .to_string();
    let task_type = headers
        .and_then(|headers| {
            header_str(headers, "x-manwe-task-type")
                .or_else(|| header_str(headers, "x-arda-task-type"))
        })
        .map(str::to_string)
        .unwrap_or_else(|| infer_task_type(body, &messages));

    Ok(ManweRequestEnvelope {
        agent_id,
        task_type,
        priority: "normal".to_string(),
        messages,
        options,
    })
}

fn apply_openai_route_headers(headers: &HeaderMap, options: &mut Value) {
    for (header, option) in [
        ("x-manwe-workload-role", "workload_role"),
        ("x-manwe-context-priority", "context_priority"),
        ("x-manwe-quality-priority", "quality_priority"),
        ("x-manwe-cost-policy", "cost_policy"),
        ("x-manwe-privacy-requirement", "privacy_requirement"),
        ("x-manwe-inference-origin", "inference_origin"),
        ("x-manwe-origin-preference", "origin_preference"),
        ("x-manwe-execution-lane", "execution_lane"),
        ("x-manwe-force-provider-id", "force_provider_id"),
        ("x-manwe-force-model-id", "force_model_id"),
        ("x-manwe-source-surface", "source_surface"),
        ("x-manwe-harness", "harness"),
        ("x-manwe-governance-method", "governance_method"),
        ("x-manwe-philosopher-method", "philosopher_method"),
        ("x-manwe-governance-philosopher", "governance_philosopher"),
        ("x-manwe-philosopher-lens", "philosopher_lens"),
        ("x-manwe-governance-chain-id", "governance_chain_id"),
    ] {
        if let Some(value) = header_str(headers, header) {
            options[option] = Value::String(value.to_string());
        }
    }

    if let Some(value) = header_str(headers, "x-manwe-route-class") {
        options["route_class"] = Value::String(value.to_string());
    }
    if let Some(value) = header_str(headers, "x-manwe-context-window-target")
        .or_else(|| header_str(headers, "x-arda-context-window-target"))
    {
        if let Ok(target) = value.parse::<u64>() {
            options["context_window_target"] = Value::Number(target.into());
        }
    }
    if let Some(value) = header_str(headers, "x-manwe-tool-use-required") {
        if let Ok(required) = value.parse::<bool>() {
            options["tool_use_required"] = Value::Bool(required);
        }
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn request_has_tool_or_skill_intent(body: &Value, messages: &[Value]) -> bool {
    explicit_tool_payload(body)
        || routing_value_requests_tools(body)
        || body.get("extra_body").is_some_and(|extra_body| {
            explicit_tool_payload(extra_body) || routing_value_requests_tools(extra_body)
        })
        || messages.iter().any(message_has_tool_history)
        || messages_imply_tool_intent(messages)
}

fn explicit_tool_payload(value: &Value) -> bool {
    value
        .get("tool_choice")
        .is_some_and(tool_choice_requires_tool_call)
        || non_empty_field(value, "skill")
        || non_empty_field(value, "skills")
        || non_empty_field(value, "toolset")
        || non_empty_field(value, "toolsets")
        || non_empty_field(value, "mcp_servers")
        || value
            .get("tool_mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| !matches!(mode, "none" | "off" | "auto"))
        || value
            .get("agent_mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| matches!(mode, "agentic" | "autonomous" | "tooling"))
        || value
            .get("tool_use_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn request_tool_schema_count(value: &Value) -> usize {
    ["tools", "available_tools", "enabled_tools"]
        .iter()
        .filter_map(|key| value.get(key).and_then(Value::as_array))
        .map(Vec::len)
        .sum()
}

fn tool_choice_requires_tool_call(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(required) => *required,
        Value::String(raw) => {
            let trimmed = raw.trim().to_ascii_lowercase();
            !trimmed.is_empty() && !matches!(trimmed.as_str(), "auto" | "none" | "off")
        }
        Value::Object(map) => !map.is_empty(),
        Value::Array(items) => !items.is_empty(),
        _ => true,
    }
}

fn routing_value_requests_tools(value: &Value) -> bool {
    let Some(routing) = value.get("routing").and_then(Value::as_object) else {
        return false;
    };
    routing
        .get("tool_use_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || routing
            .get("workload_role")
            .and_then(Value::as_str)
            .is_some_and(|role| matches!(role, "execution" | "subagent"))
        || routing
            .get("execution_lane")
            .and_then(Value::as_str)
            .is_some_and(|lane| lane == "execution")
        || routing
            .get("tool_mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| !matches!(mode, "none" | "off"))
        || routing
            .get("agent_mode")
            .and_then(Value::as_str)
            .is_some_and(|mode| matches!(mode, "agentic" | "autonomous" | "tooling"))
        || map_field_non_empty(routing, "skill")
        || map_field_non_empty(routing, "skills")
        || map_field_non_empty(routing, "toolset")
        || map_field_non_empty(routing, "toolsets")
}

fn messages_imply_tool_intent(messages: &[Value]) -> bool {
    let joined = messages
        .iter()
        .filter_map(|message| message.get("content").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    if joined.trim().is_empty() {
        return false;
    }
    tool_intent_regex().is_some_and(|regex| regex.is_match(&joined))
}

fn tool_intent_regex() -> Option<&'static regex::RegexSet> {
    static REGEX: OnceLock<Option<regex::RegexSet>> = OnceLock::new();
    REGEX.get_or_init(|| {
        regex::RegexSet::new([
            r"\b(run|execute|invoke|call)\s+(the\s+)?(tool|tools|command|terminal|shell|script|harness)\b",
            r"\b(use|with)\s+(the\s+)?(terminal|shell|filesystem|file tools|tools)\b",
            r"\b(inspect|scan|check|verify)\s+.*\b(repo|repository|file|files|local state|private local state|queue evidence)\b",
            r"\b(patch|fix|repair|modify|edit|update)\s+.*\b(bug|issue|file|files|repo|repository|queue|task)\b",
            r"\b(read|write|edit|patch|modify|update|create|delete)\s+((the\s+)?(file|files|repo|repository|queue|task queue|active queue)|[A-Za-z0-9_./-]+\.(rs|py|ts|tsx|js|json|toml|md|sh))\b",
            r"\b(cargo|npm|pnpm|git|hermes|bash|python3?|rg|jq)\s+[A-Za-z0-9_./:-]",
            r"\b(continue|work|begin|finish|complete)\s+.*\b(active[_ -]?queue|task queue|queued task|queue_active)\b",
            r"\b(apply|make)\s+.*\b(change|changes|fix|patch|commit|push)\b",
        ])
        .ok()
    })
    .as_ref()
}

fn message_has_tool_history(message: &Value) -> bool {
    (message.get("role").and_then(Value::as_str) == Some("tool")
        && message
            .get("tool_call_id")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty()))
        || message
            .get("tool_calls")
            .and_then(Value::as_array)
            .is_some_and(|arr| !arr.is_empty())
        || message
            .get("function_call")
            .is_some_and(|value| !value.is_null())
}

fn map_field_non_empty(map: &serde_json::Map<String, Value>, key: &str) -> bool {
    map.get(key).is_some_and(value_is_non_empty_intent)
}

fn non_empty_field(value: &Value, key: &str) -> bool {
    value.get(key).is_some_and(value_is_non_empty_intent)
}

fn value_is_non_empty_intent(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(false) => false,
        Value::String(raw) => {
            let trimmed = raw.trim();
            !trimmed.is_empty() && !matches!(trimmed, "none" | "off")
        }
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
        _ => true,
    }
}

fn infer_task_type(body: &Value, messages: &[Value]) -> String {
    if request_has_tool_or_skill_intent(body, messages) {
        return "code".to_string();
    }
    let joined = messages
        .iter()
        .filter_map(|message| message.get("content").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    if joined.contains("code")
        || joined.contains("rust")
        || joined.contains("python")
        || joined.contains("bash")
        || joined.contains("debug")
    {
        "code".to_string()
    } else {
        "chat".to_string()
    }
}

async fn resolve_requested_model(
    service: &ManweService,
    requested_model: &str,
) -> Result<Option<(String, String)>> {
    let trimmed = requested_model.trim();
    if trimmed.is_empty() || matches!(trimmed, "auto" | "default" | "manwe/auto") {
        return Ok(None);
    }

    let providers = service.providers().await;
    let mut advertised_matches = providers
        .iter()
        .flat_map(|provider| {
            provider.models.iter().filter_map(move |model| {
                let advertised = advertised_model_id(&provider.id, &model.id);
                if model.id == trimmed || advertised == trimmed {
                    Some((provider.id.clone(), model.id.clone()))
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    if advertised_matches.len() == 1 {
        return Ok(advertised_matches.pop());
    }
    if advertised_matches.len() > 1 {
        return Err(ArdaError::Agent {
            agent: "manwe".to_string(),
            message: format!(
                "requested model `{trimmed}` is ambiguous; use a more specific model id"
            ),
        });
    }

    if let Some((provider_id, model_id)) = trimmed.split_once('/') {
        let provider = providers
            .iter()
            .find(|provider| provider.id == provider_id)
            .ok_or_else(|| ArdaError::Agent {
                agent: "manwe".to_string(),
                message: format!("unknown provider `{provider_id}` in requested model `{trimmed}`"),
            })?;
        let model = provider
            .models
            .iter()
            .find(|model| model.id == model_id)
            .ok_or_else(|| ArdaError::Agent {
                agent: "manwe".to_string(),
                message: format!("unknown model `{model_id}` for provider `{provider_id}`"),
            })?;
        return Ok(Some((provider.id.clone(), model.id.clone())));
    }

    Err(ArdaError::Agent {
        agent: "manwe".to_string(),
        message: format!("requested model `{trimmed}` was not found"),
    })
}

fn openai_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "message": message,
                "type": "manwe_error"
            }
        })),
    )
        .into_response()
}

fn authorize_mutation(headers: &HeaderMap) -> bool {
    const ENV: &str = "ARDA_MANWE_API_KEY";
    let Some(expected) = std::env::var(ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
    else {
        return true;
    };
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|header| header == format!("Bearer {expected}"))
}

async fn build_event_payload(service: &ManweService) -> Result<Value> {
    let status = service.status().await?;
    let providers = service.providers().await;
    let recent_events = service.recent_state_events(12);
    let recent_governance_events = service.recent_governance_events(12);
    let provider_rows_source = providers
        .iter()
        .cloned()
        .map(filter_provider_catalog_models)
        .collect::<Vec<_>>();
    let mut provider_rows = Vec::new();
    for provider in provider_rows_source {
        let probe = service.capacity_probe_record(&provider.id).await;
        provider_rows.push(provider_row(provider, probe, &recent_events, true));
    }
    let cooldowns = provider_rows
        .iter()
        .filter(|provider| provider.get("in_cooldown").and_then(Value::as_bool) == Some(true))
        .cloned()
        .collect::<Vec<_>>();
    let exhausted = provider_rows
        .iter()
        .filter(|provider| {
            let used = provider
                .get("requests_used_day")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let max = provider.get("requests_per_day").and_then(Value::as_u64);
            max.is_some_and(|max| used >= max)
        })
        .cloned()
        .collect::<Vec<_>>();
    let recent_route_events = recent_events
        .iter()
        .filter(|event| {
            event
                .get("event")
                .and_then(Value::as_str)
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        "route_selected"
                            | "route_failed"
                            | "provider_result"
                            | "route_failed_policy"
                            | "route_cooldown_bypass"
                    )
                })
        })
        .cloned()
        .collect::<Vec<_>>();
    let recent_governance = recent_governance_events
        .iter()
        .filter(|event| {
            event
                .get("event")
                .and_then(Value::as_str)
                .is_some_and(|kind| {
                    matches!(
                        kind,
                        "echo_gate"
                            | "echo_gate_abort"
                            | "echo_gate_proxy_abort"
                            | "route_selected"
                            | "route_failed"
                            | "route_failed_policy"
                            | "route_cooldown_bypass"
                    )
                })
        })
        .cloned()
        .collect::<Vec<_>>();

    Ok(json!({
        "ok": true,
        "stream_version": "manwe.events.v2",
        "generated_at_utc": chrono::Utc::now().to_rfc3339(),
        "status": status,
        "routing": {
            "policy_defaults": {
                "privacy_tier": std::env::var("ARDA_ROUTE_PRIVACY_DEFAULT").unwrap_or_else(|_| "public".to_string()),
                "cost_tier": std::env::var("ARDA_ROUTE_COST_DEFAULT").unwrap_or_else(|_| "balanced".to_string()),
                "quality_tier": std::env::var("ARDA_ROUTE_QUALITY_DEFAULT").unwrap_or_else(|_| "balanced".to_string()),
                "origin_preference": std::env::var("ARDA_ROUTE_ORIGIN_DEFAULT").unwrap_or_else(|_| "auto".to_string()),
                "latency_sla_ms": std::env::var("ARDA_ROUTE_LATENCY_SLA_MS").ok().and_then(|v| v.parse::<u64>().ok())
            },
            "recent_events": recent_route_events
        },
        "governance": {
            "recent_events": recent_governance
        },
        "provider_pool": {
            "providers": provider_rows,
            "cooldowns": cooldowns,
            "exhausted": exhausted
        },
        "arda_hints": {
            "primary_panel": "inference_router",
            "world_overlay": "provider_health_and_origin",
            "alert_on_cooldown": !cooldowns.is_empty(),
            "alert_on_exhaustion": !exhausted.is_empty()
        }
    }))
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{
        build_event_payload, filter_provider_catalog_models, http_request_limit,
        openai_body_to_envelope, openai_body_to_envelope_with_headers, openai_chat_completions,
        openai_completion_to_sse, should_emulate_streaming_tool_response, strip_reasoning_fields,
        visible_provider_catalog_models,
    };
    use crate::adaptive::service::ManweService;
    use crate::types::{ManweRequestEnvelope, ModelState, ProviderState};
    use axum::body::to_bytes;
    use axum::extract::State;
    use axum::http::{HeaderMap, StatusCode};
    use axum::Json;
    use serde_json::{json, Value};
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn mutation_auth_is_optional_but_requires_exact_bearer_when_configured() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let mut headers = HeaderMap::new();

        std::env::remove_var("ARDA_MANWE_API_KEY");
        assert!(super::authorize_mutation(&headers));

        std::env::set_var("ARDA_MANWE_API_KEY", "test-secret");
        assert!(!super::authorize_mutation(&headers));

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer wrong-secret".parse().expect("header value"),
        );
        assert!(!super::authorize_mutation(&headers));

        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer test-secret".parse().expect("header value"),
        );
        assert!(super::authorize_mutation(&headers));

        std::env::remove_var("ARDA_MANWE_API_KEY");
    }

    #[test]
    fn nvidia_catalog_visibility_preserves_configured_models() {
        let models = vec![
            ModelState {
                aliases: vec![],
                id: "qwen/qwen3-coder-480b-a35b-instruct".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 256_000,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            },
            ModelState {
                aliases: vec![],
                id: "google/codegemma-7b".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 128_000,
                is_default: false,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            },
        ];

        let visible = visible_provider_catalog_models("nvidia", &models);
        assert_eq!(visible.len(), 2);
        assert_eq!(visible[0].id, "qwen/qwen3-coder-480b-a35b-instruct");
        assert_eq!(visible[1].id, "google/codegemma-7b");
    }

    #[test]
    fn provider_catalog_filter_preserves_non_nvidia_models() {
        let provider = ProviderState {
            id: "opencode".to_string(),
            name: "OpenCode Zen".to_string(),
            base_url: Some("https://opencode.ai/zen/v1".to_string()),
            api_key_env: Some("OPENCODE_API_KEY".to_string()),
            access_tier: "free_cloud".to_string(),
            quality_band: "high".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: Some(120),
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: Some(100_000),
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                aliases: vec![],
                id: "glm-5.1".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 128_000,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        };

        let filtered = filter_provider_catalog_models(provider);
        assert_eq!(filtered.models.len(), 1);
        assert_eq!(filtered.models[0].id, "glm-5.1");
    }

    #[test]
    fn providers_query_ids_accepts_comma_separated_ids() {
        let query = super::ProvidersQuery {
            ids: Some("openrouter, nvidia,,groq ".to_string()),
            provider_ids: None,
            compact: None,
            include_models: None,
        };

        assert_eq!(
            super::providers_query_ids(&query),
            vec![
                "openrouter".to_string(),
                "nvidia".to_string(),
                "groq".to_string()
            ]
        );
    }

    #[test]
    fn compact_provider_row_keeps_routing_metadata_and_drops_catalog() {
        let row = json!({
            "id": "openrouter",
            "provider_id": "openrouter",
            "name": "OpenRouter",
            "driver": "openai_compat",
            "hermes_provider": null,
            "supports_streaming": true,
            "probe_eligible": true,
            "last_success_utc": "2026-03-10T00:00:00Z",
            "last_failure_class": "rate_or_retry_error",
            "model_count": 2,
            "models": [{"id": "large"}],
            "base_url": "https://openrouter.ai/api/v1",
            "api_key_env": "OPENROUTER_API_KEY"
        });

        let compact = super::compact_provider_row(row);

        assert_eq!(compact["provider_id"], "openrouter");
        assert_eq!(compact["driver"], "openai_compat");
        assert_eq!(compact["supports_streaming"], true);
        assert_eq!(compact["probe_eligible"], true);
        assert_eq!(compact["last_failure_class"], "rate_or_retry_error");
        assert_eq!(compact["model_count"], 2);
        assert!(compact.get("models").is_none());
        assert!(compact.get("base_url").is_none());
        assert!(compact.get("api_key_env").is_none());
    }

    fn probe_test_provider(id: &str, access_tier: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            api_key_env: None,
            access_tier: access_tier.to_string(),
            quality_band: "medium".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: Some(60),
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: Some(10_000),
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                aliases: vec![],
                id: "test-model".to_string(),
                capable_tasks: vec!["chat".to_string()],
                context_window: 8192,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: crate::types::ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            active_connections: 0,
            last_reservation_utc: None,
            supports_tools: true,
            supports_structured_output: true,
            driver: "openai_compat".to_string(),
            hermes_bin: None,
            hermes_provider: None,
            hermes_toolsets: None,
        }
    }

    #[test]
    fn probe_throttle_skips_recent_cloud_rate_failure() {
        let provider = probe_test_provider("openrouter", "mixed");
        let events = vec![json!({
            "event": "tool_fit_observation",
            "ts": chrono::Utc::now().to_rfc3339(),
            "payload": {
                "provider_id": "openrouter",
                "model_id": "test-model",
                "ok": false,
                "outcome_class": "rate_or_retry_error"
            }
        })];

        let throttle = super::probe_throttle_decision(&provider, "test-model", &events, false)
            .expect("throttle");

        assert_eq!(throttle.reason, "recent_probe_failure");
        assert_eq!(
            throttle.last_failure_class.as_deref(),
            Some("rate_or_retry_error")
        );
        assert!(throttle.remaining_seconds.unwrap_or_default() > 0);
    }

    #[test]
    fn probe_throttle_can_be_explicitly_bypassed() {
        let provider = probe_test_provider("openrouter", "mixed");
        let events = vec![json!({
            "event": "tool_fit_observation",
            "ts": chrono::Utc::now().to_rfc3339(),
            "payload": {
                "provider_id": "openrouter",
                "model_id": "test-model",
                "ok": false,
                "outcome_class": "rate_or_retry_error"
            }
        })];

        assert!(super::probe_throttle_decision(&provider, "test-model", &events, true).is_none());
    }

    #[test]
    fn probe_throttle_does_not_skip_local_recent_failure() {
        let provider = probe_test_provider("local_fallback", "local");
        let events = vec![json!({
            "event": "tool_fit_observation",
            "ts": chrono::Utc::now().to_rfc3339(),
            "payload": {
                "provider_id": "local_fallback",
                "model_id": "test-model",
                "ok": false,
                "outcome_class": "rate_or_retry_error"
            }
        })];

        assert!(super::probe_throttle_decision(&provider, "test-model", &events, false).is_none());
    }

    #[test]
    fn probe_throttle_success_clears_recent_failure_window() {
        let provider = probe_test_provider("openrouter", "mixed");
        let events = vec![
            json!({
                "event": "tool_fit_observation",
                "ts": chrono::Utc::now().to_rfc3339(),
                "payload": {
                    "provider_id": "openrouter",
                    "model_id": "test-model",
                    "ok": false,
                    "outcome_class": "rate_or_retry_error"
                }
            }),
            json!({
                "event": "provider_result",
                "ts": chrono::Utc::now().to_rfc3339(),
                "payload": {
                    "provider_id": "openrouter",
                    "ok": true
                }
            }),
        ];

        assert!(super::probe_throttle_decision(&provider, "test-model", &events, false).is_none());
    }

    #[test]
    fn probe_throttle_uses_provider_failure_memory() {
        let mut provider = probe_test_provider("openai_sub", "paid_cloud");
        provider.consecutive_failures = 1;
        provider.last_error = Some(
            "codex_responses openai_sub HTTP 429 rate_or_retry_error: upstream error".to_string(),
        );

        let throttle =
            super::probe_throttle_decision(&provider, "test-model", &[], false).expect("throttle");

        assert_eq!(throttle.reason, "provider_failure_memory");
        assert_eq!(
            throttle.last_failure_class.as_deref(),
            Some("rate_or_retry_error")
        );
    }

    #[test]
    fn probe_attempt_outcome_class_separates_marker_missing_from_http_failure() {
        assert_eq!(super::probe_attempt_outcome_class(200, true), "success");
        assert_eq!(
            super::probe_attempt_outcome_class(200, false),
            "marker_missing"
        );
        assert_eq!(
            super::probe_attempt_outcome_class(429, false),
            "rate_or_retry_error"
        );
        assert_eq!(
            super::probe_attempt_outcome_class(503, false),
            "provider_server_error"
        );
    }

    #[test]
    fn probe_error_provider_id_extracts_proxy_provider_failure() {
        let providers = vec![
            probe_test_provider("opencode", "free_cloud"),
            probe_test_provider("openrouter", "mixed"),
        ];
        let error = "Agent error: manwe — provider opencode HTTP 401: invalid credentials";

        let provider_id = super::probe_error_provider_id(error, &providers);

        assert_eq!(provider_id.as_deref(), Some("opencode"));
    }

    #[test]
    fn probe_health_mutation_skips_marker_missing_successful_http_response() {
        assert!(!super::probe_attempt_should_mark_health_failure(200, false));
        assert!(!super::probe_attempt_should_mark_health_failure(299, false));
        assert!(super::probe_attempt_should_mark_health_failure(429, false));
        assert!(super::probe_attempt_should_mark_health_failure(503, false));
    }

    #[test]
    fn probe_result_payload_records_structured_route_receipt() {
        let attempts = vec![json!({
            "ok": false,
            "status": 200,
            "marker_found": false,
            "latency_ms": 42,
            "outcome_class": "marker_missing",
            "route": {
                "provider_id": "openrouter",
                "model_id": "nvidia/nemotron-nano-9b-v2:free"
            }
        })];

        let payload = super::probe_result_payload(
            false,
            "marker_missing",
            "ANKH",
            "interactive",
            attempts[0].clone(),
            &attempts,
        );

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["outcome_class"], "marker_missing");
        assert_eq!(payload["provider_id"], "openrouter");
        assert_eq!(payload["model_id"], "nvidia/nemotron-nano-9b-v2:free");
        assert_eq!(payload["lane"], "interactive");
        assert_eq!(payload["attempt_count"], 1);
    }

    #[tokio::test]
    async fn event_payload_includes_provider_and_route_sections() {
        let dir = tempdir().expect("tempdir");
        fs::write(
            dir.path().join("state.jsonl"),
            concat!(
                "{\"ts\":\"2026-03-09T11:00:00Z\",\"event\":\"route_selected\",\"payload\":{\"provider_id\":\"local_fallback\"}}\n",
                "{\"ts\":\"2026-03-09T11:01:00Z\",\"event\":\"provider_result\",\"payload\":{\"provider_id\":\"local_fallback\",\"ok\":true}}\n"
            ),
        )
        .expect("state write");
        let service = ManweService::new(dir.path()).expect("service");

        let payload = build_event_payload(&service).await.expect("payload");
        assert_eq!(payload["stream_version"], "manwe.events.v2");
        assert!(payload["provider_pool"]["providers"]
            .as_array()
            .is_some_and(|providers| !providers.is_empty()));
        assert!(payload["routing"]["recent_events"]
            .as_array()
            .is_some_and(|events| !events.is_empty()));
        assert_eq!(payload["arda_hints"]["primary_panel"], "inference_router");
        assert!(payload["provider_pool"]["providers"][0]
            .get("operational_state")
            .is_some());
        assert!(payload["provider_pool"]["providers"][0]
            .get("driver")
            .is_some());
        assert!(payload["provider_pool"]["providers"][0]
            .get("supports_streaming")
            .is_some());
        assert!(payload["provider_pool"]["providers"][0]
            .get("probe_eligible")
            .is_some());
    }

    #[tokio::test]
    async fn openai_tool_requests_default_to_execution_and_balanced_cost() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "agent_id": "hermes",
                "model": "auto",
                "messages": [{"role":"user","content":"inspect the repo and patch the bug"}],
                "tools": [{"type":"function","function":{"name":"read_file"}}],
                "tool_choice": "auto"
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.agent_id, "hermes");
        assert_eq!(req.task_type, "code");
        assert_eq!(req.options["workload_role"], "execution");
        assert_eq!(req.options["origin_preference"], "auto");
        assert_eq!(req.options["cost_policy"], "balanced");
        assert_eq!(req.options["quality_priority"], "high");
        assert_eq!(req.options["tool_use_required"], true);
    }

    #[tokio::test]
    async fn openai_available_tools_do_not_force_execution_route() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "agent_id": "hermes",
                "model": "auto",
                "dry_run": true,
                "messages": [{"role":"user","content":"What is the current active queue health?"}],
                "tools": [
                    {"type":"function","function":{"name":"read_file"}},
                    {"type":"function","function":{"name":"terminal"}}
                ],
                "tool_choice": "auto"
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.agent_id, "hermes");
        assert_eq!(req.task_type, "chat");
        assert_eq!(req.options["tools_available"], true);
        assert_eq!(req.options["tool_schema_count"], 2);
        assert_eq!(req.options["dry_run"], true);
        assert!(req.options.get("tool_use_required").is_none());
        assert!(req.options.get("workload_role").is_none());
        assert!(req.options.get("origin_preference").is_none());
    }

    #[test]
    fn streaming_tool_turn_uses_emulation_gate() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::remove_var("ARDA_MANWE_EMULATE_TOOL_STREAMING");
        let req = ManweRequestEnvelope {
            agent_id: "hermes".to_string(),
            task_type: "code".to_string(),
            priority: "normal".to_string(),
            messages: vec![
                json!({"role":"assistant","tool_calls":[{"id":"call_1","type":"function","function":{"name":"read_file","arguments":"{}"}}]}),
                json!({"role":"tool","tool_call_id":"call_1","content":"queue evidence"}),
            ],
            options: json!({
                "endpoint": "/chat/completions",
                "stream": true,
                "tool_use_required": true
            }),
        };

        assert!(should_emulate_streaming_tool_response(&req));

        std::env::set_var("ARDA_MANWE_EMULATE_TOOL_STREAMING", "false");
        assert!(!should_emulate_streaming_tool_response(&req));
        std::env::remove_var("ARDA_MANWE_EMULATE_TOOL_STREAMING");
    }

    #[test]
    fn openai_completion_to_sse_preserves_tool_calls() {
        let sse = openai_completion_to_sse(&json!({
            "id": "chatcmpl-test",
            "model": "gpt-5.5",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {
                            "name": "terminal",
                            "arguments": "{\"command\":\"hermes queue list\"}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        }));

        assert!(sse.contains("data: "));
        assert!(sse.contains("\"object\":\"chat.completion.chunk\""));
        assert!(sse.contains("\"tool_calls\""));
        assert!(sse.contains("\"finish_reason\":\"tool_calls\""));
        assert!(sse.ends_with("data: [DONE]\n\n"));
    }

    #[tokio::test]
    async fn openai_tool_requests_preserve_explicit_local_origin() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "agent_id": "hermes",
                "model": "auto",
                "messages": [{"role":"user","content":"inspect private local state"}],
                "tools": [{"type":"function","function":{"name":"read_file"}}],
                "extra_body": {
                    "routing": {
                        "origin_preference": "local"
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["origin_preference"], "local");
        assert_eq!(req.options["tool_use_required"], true);
    }

    #[tokio::test]
    async fn openai_tool_requests_keep_explicit_free_pool_strategy() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "agent_id": "hermes",
                "model": "auto",
                "messages": [{"role":"user","content":"inspect the repo and patch the bug"}],
                "tools": [{"type":"function","function":{"name":"read_file"}}],
                "tool_pool_strategy": "free_first"
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["tool_use_required"], true);
        assert_eq!(req.options["workload_role"], "execution");
        assert_eq!(req.options["tool_pool_strategy"], "free_first");
        assert!(req.options.get("origin_preference").is_none());
    }

    #[tokio::test]
    async fn openai_tool_requests_preserve_governance_method_metadata() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "agent_id": "hermes",
                "model": "auto",
                "messages": [{"role":"user","content":"verify queue evidence before acting"}],
                "tools": [{"type":"function","function":{"name":"terminal"}}],
                "extra_body": {
                    "routing": {
                        "governance_method": "single",
                        "governance_philosopher": "bacon",
                        "governance_chain_id": "default_triad"
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["tool_use_required"], true);
        assert_eq!(req.options["governance_method"], "single");
        assert_eq!(req.options["governance_philosopher"], "bacon");
        assert_eq!(req.options["governance_chain_id"], "default_triad");
    }

    #[tokio::test]
    async fn openai_nested_routing_preserves_exclusion_metadata() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "messages": [{"role":"user","content":"run tools"}],
                "tools": [{"type":"function","function":{"name":"read_file"}}],
                "extra_body": {
                    "routing": {
                        "excluded_provider_ids": ["openai_sub", "mistral"],
                        "excluded_model_ids": ["bad-model"]
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(
            req.options["excluded_provider_ids"],
            json!(["openai_sub", "mistral"])
        );
        assert_eq!(req.options["excluded_model_ids"], json!(["bad-model"]));
    }

    #[test]
    fn strip_reasoning_fields_removes_non_stream_visible_thinking() {
        let mut value = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "reasoning": "hidden chain",
                    "reasoning_content": "hidden chain",
                    "extra_content": {
                        "google": {
                            "thought_signature": "hidden signature"
                        }
                    },
                    "content": "<think>hidden</think>visible answer"
                }
            }],
            "reasoning_details": [{"text": "hidden"}]
        });

        strip_reasoning_fields(&mut value);

        let message = &value["choices"][0]["message"];
        assert!(message.get("reasoning").is_none());
        assert!(message.get("reasoning_content").is_none());
        assert!(message.get("extra_content").is_none());
        assert!(value.get("reasoning_details").is_none());
        assert_eq!(message["content"], "visible answer");
    }

    #[tokio::test]
    async fn openai_hermes_skill_metadata_defaults_to_execution_and_balanced_cost() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "messages": [{"role":"user","content":"continue the queued work"}],
                "extra_body": {
                    "routing": {
                        "agent_id": "hermes",
                        "source_surface": "hermes_agent_gateway",
                        "skill": "repo-maintenance",
                        "toolsets": ["filesystem", "terminal"],
                        "agent_mode": "agentic"
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.agent_id, "hermes");
        assert_eq!(req.task_type, "code");
        assert_eq!(req.options["workload_role"], "execution");
        assert_eq!(req.options["cost_policy"], "balanced");
        assert_eq!(req.options["quality_priority"], "high");
        assert_eq!(req.options["tool_use_required"], true);
        assert_eq!(req.options["source_surface"], "hermes_agent_gateway");
        assert_eq!(req.options["skill"], "repo-maintenance");
    }

    #[tokio::test]
    async fn openai_extra_body_routing_metadata_overrides_tool_defaults() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "messages": [{"role":"user","content":"run tools and continue"}],
                "tools": [{"type":"function","function":{"name":"terminal"}}],
                "tool_choice": "auto",
                "extra_body": {
                    "routing": {
                        "workload_role": "orchestrator",
                        "context_priority": "high",
                        "context_window_target": 128000,
                        "cost_policy": "free_first",
                        "quality_priority": "high"
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["workload_role"], "orchestrator");
        assert_eq!(req.options["context_priority"], "high");
        assert_eq!(req.options["context_window_target"], 128000);
        assert_eq!(req.options["cost_policy"], "free_first");
        assert_eq!(req.options["quality_priority"], "high");
    }

    #[tokio::test]
    async fn openai_manwe_headers_set_route_metadata() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");
        let mut headers = HeaderMap::new();
        headers.insert("x-manwe-agent-id", "hermes".parse().unwrap());
        headers.insert("x-manwe-task-type", "code".parse().unwrap());
        headers.insert("x-manwe-execution-lane", "execution".parse().unwrap());
        headers.insert("x-manwe-context-window-target", "64000".parse().unwrap());
        headers.insert("x-manwe-tool-use-required", "true".parse().unwrap());
        headers.insert("x-manwe-governance-method", "chain".parse().unwrap());
        headers.insert(
            "x-manwe-governance-chain-id",
            "default_triad".parse().unwrap(),
        );

        let req = openai_body_to_envelope_with_headers(
            &service,
            &json!({
                "model": "auto",
                "messages": [{"role":"user","content":"run tools and continue"}],
            }),
            Some(&headers),
        )
        .await
        .expect("envelope");

        assert_eq!(req.agent_id, "hermes");
        assert_eq!(req.options["governance_method"], "chain");
        assert_eq!(req.options["governance_chain_id"], "default_triad");
        assert_eq!(req.task_type, "code");
        assert_eq!(req.options["execution_lane"], "execution");
        assert_eq!(req.options["context_window_target"], 64000);
        assert_eq!(req.options["tool_use_required"], true);
    }

    #[tokio::test]
    async fn openai_extra_body_preserves_forced_provider_and_model() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "messages": [{"role":"user","content":"health probe"}],
                "extra_body": {
                    "routing": {
                        "force_provider_id": "nvidia",
                        "force_model_id": "meta/llama-3.1-8b-instruct",
                        "allow_forced_provider_fallback": true
                    }
                }
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["force_provider_id"], "nvidia");
        assert_eq!(req.options["force_model_id"], "meta/llama-3.1-8b-instruct");
        assert_eq!(req.options["allow_forced_provider_fallback"], true);
    }

    #[tokio::test]
    async fn openai_forced_route_skips_global_model_ambiguity() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "gpt-5.5",
                "messages": [{"role":"user","content":"health probe"}],
                "force_provider_id": "openai_sub",
                "force_model_id": "gpt-5.5"
            }),
        )
        .await
        .expect("forced route should not globally resolve model");

        assert_eq!(req.options["force_provider_id"], "openai_sub");
        assert_eq!(req.options["force_model_id"], "gpt-5.5");
    }

    #[test]
    fn http_request_limit_uses_env_when_valid_and_falls_back_otherwise() {
        let _guard = ENV_LOCK.lock().expect("env lock");

        std::env::remove_var("ARDA_MANWE_HTTP_MAX_CONCURRENCY");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ARDA_MANWE_HTTP_MAX_CONCURRENCY", "64");
        assert_eq!(http_request_limit(), 64);

        std::env::set_var("ARDA_MANWE_HTTP_MAX_CONCURRENCY", "0");
        assert_eq!(http_request_limit(), 24);

        std::env::set_var("ARDA_MANWE_HTTP_MAX_CONCURRENCY", "invalid");
        assert_eq!(http_request_limit(), 24);

        std::env::remove_var("ARDA_MANWE_HTTP_MAX_CONCURRENCY");
    }

    #[tokio::test]
    async fn openai_requested_model_reports_not_found_clearly() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let err = openai_body_to_envelope(
            &service,
            &json!({
                "model": "unknown_provider/unknown_model",
                "messages": [{"role":"user","content":"hello"}]
            }),
        )
        .await
        .expect_err("unknown provider/model should fail");

        let message = err.to_string();
        assert!(message.contains("unknown provider `unknown_provider`"));
        assert!(message.contains("unknown_provider/unknown_model"));
    }

    #[tokio::test]
    async fn openai_requested_model_sets_forced_provider_and_model_for_exact_match() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "google/gemini-2.0-flash",
                "messages": [{"role":"user","content":"hello"}]
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["force_provider_id"], "google");
        assert_eq!(req.options["force_model_id"], "gemini-2.0-flash");
    }

    #[tokio::test]
    async fn openai_body_to_envelope_preserves_stream_for_route_policy() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "stream": true,
                "messages": [{"role":"user","content":"hello"}]
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["stream"], true);
    }

    #[tokio::test]
    async fn openai_body_to_envelope_preserves_probe_model_preference() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let req = openai_body_to_envelope(
            &service,
            &json!({
                "model": "auto",
                "prefer_probe_model": true,
                "execution_lane": "interactive",
                "messages": [{"role":"user","content":"health probe"}]
            }),
        )
        .await
        .expect("envelope");

        assert_eq!(req.options["prefer_probe_model"], true);
    }

    #[tokio::test]
    async fn chat_completions_returns_bad_request_json_for_invalid_model() {
        let dir = tempdir().expect("tempdir");
        let service = ManweService::new(dir.path()).expect("service");

        let response = openai_chat_completions(
            State(service),
            HeaderMap::new(),
            Json(json!({
                "model": "unknown_provider/unknown_model",
                "messages": [{"role":"user","content":"hello"}]
            })),
        )
        .await;

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["error"]["type"], "manwe_error");
        assert!(payload["error"]["message"]
            .as_str()
            .expect("error message")
            .contains("unknown provider `unknown_provider`"));
    }

    #[tokio::test]
    async fn chat_completions_forced_model_passthrough_returns_bad_gateway_on_upstream_failure() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let config_path = dir.path().join("manwe.providers.toml");
        fs::write(
            &config_path,
            r#"
[[provider]]
id = "local_test"
name = "Local Test"
base_url = "http://127.0.0.1:9/v1"
enabled = true
healthy = true
access_tier = "local"
quality_band = "high"

  [[provider.model]]
  id = "test-model"
  capable_tasks = ["chat", "code"]
  context_window = 32768
  is_default = true
"#,
        )
        .expect("config write");

        std::env::set_var("ARDA_MANWE_PROVIDER_CONFIG", &config_path);
        let service = ManweService::new(dir.path()).expect("service");

        let response = openai_chat_completions(
            State(service),
            HeaderMap::new(),
            Json(json!({
                "model": "local_test/test-model",
                "messages": [{"role":"user","content":"hello"}]
            })),
        )
        .await;

        std::env::remove_var("ARDA_MANWE_PROVIDER_CONFIG");

        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body bytes");
        let payload: Value = serde_json::from_slice(&body).expect("json body");
        assert_eq!(payload["error"]["type"], "manwe_error");
        assert!(
            payload["error"]["message"]
                .as_str()
                .expect("error message")
                .contains("connection")
                || payload["error"]["message"]
                    .as_str()
                    .expect("error message")
                    .contains("error sending request")
        );
    }
}
