use super::adaptive_routing::{capability_truth_rows, pacing_state_for_provider};
use super::route_policy::{configured_local_device_pressure, derive_route_execution_profile};
use super::{classify_provider_operational_state, CharonService};
use arda_core::error::Result;
use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeMap;

impl CharonService {
    pub async fn operator_route_summary(&self) -> Result<JsonValue> {
        let providers = self.providers().await;
        let now = Utc::now();
        let mut provider_rows = Vec::new();
        let mut cooldown_rows = Vec::new();
        let mut failure_rows = Vec::new();
        let mut configured_models = 0usize;
        let mut enabled_healthy_models = 0usize;
        let mut routable_models = 0usize;
        let mut routable_tool_models = 0usize;
        let mut high_context_tool_models = 0usize;

        for provider in &providers {
            configured_models += provider.models.len();
            let provider_routable = provider.enabled
                && provider.healthy
                && provider.has_api_key
                && !provider.in_cooldown
                && provider
                    .requests_per_day
                    .is_none_or(|max| provider.requests_used_day < max)
                && provider
                    .requests_per_minute
                    .is_none_or(|max| provider.requests_used_minute < max);
            let operational = classify_provider_operational_state(provider, now).state;
            let healthy_model_count = provider
                .models
                .iter()
                .filter(|model| model.healthy && !model.in_cooldown)
                .count();
            let provider_tool_models = provider
                .models
                .iter()
                .filter(|model| {
                    model.healthy && !model.in_cooldown && model.capabilities.tools == Some(true)
                })
                .count();
            let provider_high_context_tool_models = provider
                .models
                .iter()
                .filter(|model| {
                    model.healthy
                        && !model.in_cooldown
                        && model.capabilities.tools == Some(true)
                        && model.context_window >= 64_000
                })
                .count();

            if provider.enabled && provider.healthy {
                enabled_healthy_models += provider.models.len();
            }
            if provider_routable {
                routable_models += healthy_model_count;
                routable_tool_models += provider_tool_models;
                high_context_tool_models += provider_high_context_tool_models;
            }
            if provider.in_cooldown || operational == "rate_limited" {
                cooldown_rows.push(json!({
                    "provider_id": provider.id,
                    "state": operational,
                    "cooldown_until_utc": provider.cooldown_until_utc,
                    "requests_used_day": provider.requests_used_day,
                    "requests_per_day": provider.requests_per_day,
                    "requests_used_minute": provider.requests_used_minute,
                    "requests_per_minute": provider.requests_per_minute,
                    "last_error": provider.last_error,
                }));
            }
            if provider.last_error.is_some() || provider.error_count > 0 {
                failure_rows.push(json!({
                    "provider_id": provider.id,
                    "last_error": provider.last_error,
                    "error_count": provider.error_count,
                    "consecutive_failures": provider.consecutive_failures,
                }));
            }
            provider_rows.push(json!({
                "provider_id": provider.id,
                "name": provider.name,
                "tier": provider.access_tier,
                "state": operational,
                "enabled": provider.enabled,
                "healthy": provider.healthy,
                "routable": provider_routable,
                "models": provider.models.len(),
                "healthy_models": healthy_model_count,
                "tool_models": provider_tool_models,
                "high_context_tool_models": provider_high_context_tool_models,
                "avg_latency_ms": provider.avg_latency_ms,
                "requests_used_day": provider.requests_used_day,
                "requests_per_day": provider.requests_per_day,
                "requests_used_minute": provider.requests_used_minute,
                "requests_per_minute": provider.requests_per_minute,
                "cooldown_until_utc": provider.cooldown_until_utc,
                "last_error": provider.last_error,
            }));
        }

        provider_rows.sort_by(|a, b| {
            let ar = a
                .get("routable")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            let br = b
                .get("routable")
                .and_then(JsonValue::as_bool)
                .unwrap_or(false);
            br.cmp(&ar).then_with(|| {
                a.get("provider_id")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .cmp(
                        b.get("provider_id")
                            .and_then(JsonValue::as_str)
                            .unwrap_or(""),
                    )
            })
        });
        failure_rows.sort_by_key(|row| {
            std::cmp::Reverse(
                row.get("error_count")
                    .and_then(JsonValue::as_u64)
                    .unwrap_or(0),
            )
        });
        failure_rows.truncate(20);

        Ok(json!({
            "ok": true,
            "generated_at_utc": now.to_rfc3339(),
            "summary": {
                "providers_total": providers.len(),
                "providers_enabled": providers.iter().filter(|provider| provider.enabled).count(),
                "providers_currently_routable": provider_rows.iter().filter(|row| row.get("routable").and_then(JsonValue::as_bool) == Some(true)).count(),
                "models_configured": configured_models,
                "models_on_enabled_healthy_providers": enabled_healthy_models,
                "models_currently_routable": routable_models,
                "tool_models_currently_routable": routable_tool_models,
                "high_context_tool_models_currently_routable": high_context_tool_models,
                "providers_in_cooldown_or_rate_limited": cooldown_rows.len(),
            },
            "providers": provider_rows,
            "cooldowns_and_limits": cooldown_rows,
            "last_failures": failure_rows,
        }))
    }

    pub async fn route_observability_rollup(&self) -> Result<JsonValue> {
        let providers = self.providers().await;
        let recent_events = self.recent_state_events(1_000);
        let route_history = self.route_history(100).await;
        let capability_truth = capability_truth_rows(&providers);
        let semantic_failures = self.recent_semantic_failures(50);
        let local_device_pressure = configured_local_device_pressure();

        let mut failures: BTreeMap<String, FailureBucket> = BTreeMap::new();
        let mut task_models: BTreeMap<String, TaskModelBucket> = BTreeMap::new();
        let mut fallback_chains = Vec::new();
        let mut legacy_route_failures = Vec::new();
        let mut catalog_reconciliations = Vec::new();

        for event in &recent_events {
            let kind = event.get("event").and_then(JsonValue::as_str).unwrap_or("");
            let payload = event.get("payload").unwrap_or(event);
            match kind {
                "provider_result" => {
                    if payload.get("ok").and_then(JsonValue::as_bool) == Some(false) {
                        add_failure(&mut failures, payload, None);
                    }
                }
                "model_result" => {
                    if payload.get("ok").and_then(JsonValue::as_bool) == Some(false) {
                        add_failure(
                            &mut failures,
                            payload,
                            payload.get("model_id").and_then(JsonValue::as_str),
                        );
                    }
                }
                "tool_fit_observation" => {
                    let provider_id = payload.get("provider_id").and_then(JsonValue::as_str);
                    let model_id = payload.get("model_id").and_then(JsonValue::as_str);
                    let task_type = payload
                        .get("task_type")
                        .and_then(JsonValue::as_str)
                        .unwrap_or("unknown");
                    let ok = payload
                        .get("ok")
                        .and_then(JsonValue::as_bool)
                        .unwrap_or(false);
                    let latency_ms = payload.get("latency_ms").and_then(JsonValue::as_u64);
                    if !ok {
                        add_failure(&mut failures, payload, model_id);
                    }
                    if let (Some(provider_id), Some(model_id)) = (provider_id, model_id) {
                        let key = format!("{task_type}|{provider_id}|{model_id}");
                        let bucket = task_models.entry(key).or_insert_with(|| TaskModelBucket {
                            task_type: task_type.to_string(),
                            provider_id: provider_id.to_string(),
                            model_id: model_id.to_string(),
                            successes: 0,
                            failures: 0,
                            avg_latency_ms: None,
                        });
                        if ok {
                            bucket.successes += 1;
                            bucket.avg_latency_ms =
                                merge_latency(bucket.avg_latency_ms, latency_ms);
                        } else {
                            bucket.failures += 1;
                        }
                    }
                }
                "route_fallback_chain" => {
                    fallback_chains.push(payload.clone());
                }
                "provider_catalog_reconciled" => {
                    catalog_reconciliations.push(payload.clone());
                }
                "route_cooldown_bypass" | "route_failed" => {
                    legacy_route_failures.push(json!({
                        "reason": kind,
                        "payload": payload,
                    }));
                }
                _ => {}
            }
        }

        let mut top_failures = failures
            .into_values()
            .map(|bucket| {
                json!({
                    "provider_id": bucket.provider_id,
                    "model_id": bucket.model_id,
                    "outcome_class": bucket.outcome_class,
                    "count": bucket.count,
                    "last_error": bucket.last_error,
                })
            })
            .collect::<Vec<_>>();
        top_failures.sort_by_key(|item| {
            std::cmp::Reverse(item.get("count").and_then(JsonValue::as_u64).unwrap_or(0))
        });
        top_failures.truncate(20);

        let mut best_by_task = task_models
            .into_values()
            .filter(|bucket| bucket.successes > 0)
            .map(|bucket| {
                let reliability =
                    bucket.successes as f64 / (bucket.successes + bucket.failures).max(1) as f64;
                json!({
                    "task_type": bucket.task_type,
                    "provider_id": bucket.provider_id,
                    "model_id": bucket.model_id,
                    "successes": bucket.successes,
                    "failures": bucket.failures,
                    "reliability": reliability,
                    "avg_latency_ms": bucket.avg_latency_ms,
                })
            })
            .collect::<Vec<_>>();
        best_by_task.sort_by(|a, b| {
            let at = a.get("task_type").and_then(JsonValue::as_str).unwrap_or("");
            let bt = b.get("task_type").and_then(JsonValue::as_str).unwrap_or("");
            at.cmp(bt).then_with(|| {
                let ar = a
                    .get("reliability")
                    .and_then(JsonValue::as_f64)
                    .unwrap_or(0.0);
                let br = b
                    .get("reliability")
                    .and_then(JsonValue::as_f64)
                    .unwrap_or(0.0);
                br.partial_cmp(&ar).unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        let mut slowest = providers
            .iter()
            .filter_map(|provider| {
                Some(json!({
                    "provider_id": provider.id,
                    "avg_latency_ms": provider.avg_latency_ms?,
                    "active_connections": provider.active_connections,
                    "operational_state": classify_provider_operational_state(provider, Utc::now()).state,
                }))
            })
            .collect::<Vec<_>>();
        slowest.sort_by_key(|item| {
            std::cmp::Reverse(
                item.get("avg_latency_ms")
                    .and_then(JsonValue::as_u64)
                    .unwrap_or(0),
            )
        });
        slowest.truncate(15);

        let billing_quota_risk = providers
            .iter()
            .filter(|provider| {
                provider.in_cooldown
                    || provider
                        .last_error
                        .as_deref()
                        .is_some_and(is_billing_or_quota_text)
                    || provider.requests_per_day.is_some_and(|max| {
                        max > 0 && provider.requests_used_day >= max.saturating_mul(9) / 10
                    })
                    || provider.requests_per_minute.is_some_and(|max| {
                        max > 0 && provider.requests_used_minute >= max.saturating_mul(9) / 10
                    })
            })
            .map(|provider| {
                json!({
                    "provider_id": provider.id,
                    "last_error": provider.last_error,
                    "in_cooldown": provider.in_cooldown,
                    "cooldown_until_utc": provider.cooldown_until_utc,
                    "requests_used_day": provider.requests_used_day,
                    "requests_per_day": provider.requests_per_day,
                    "requests_used_minute": provider.requests_used_minute,
                    "requests_per_minute": provider.requests_per_minute,
                })
            })
            .collect::<Vec<_>>();
        let free_provider_pool = free_provider_pool_rollup(&providers);

        fallback_chains.reverse();
        fallback_chains.truncate(30);
        legacy_route_failures.reverse();
        legacy_route_failures.truncate(30);
        catalog_reconciliations.reverse();
        catalog_reconciliations.truncate(30);

        Ok(json!({
            "ok": true,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "local_device_pressure": local_device_pressure,
            "low_power_route_governor": {
                "pressure_source": if local_device_pressure.is_some() { "env" } else { "not_configured" },
                "compression_lane": "auto_origin_pressure_aware",
                "execution_lane": "auto_origin_latency_and_pressure_aware",
                "budget_governor": "cost_tier_quota_and_pacing",
            },
            "top_failures": top_failures,
            "slowest_active_providers": slowest,
            "best_model_per_task_observed": best_by_task,
            "free_provider_pool": free_provider_pool,
            "capability_truth": capability_truth,
            "recent_semantic_failures": semantic_failures,
            "providers_in_billing_or_quota_risk": billing_quota_risk,
            "recent_fallback_chains": fallback_chains,
            "recent_legacy_route_failures": legacy_route_failures,
            "recent_catalog_reconciliations": catalog_reconciliations,
            "recent_routes": route_history,
        }))
    }
}

fn free_provider_pool_rollup(providers: &[crate::types::ProviderState]) -> JsonValue {
    let default_pool = default_free_pool_provider_ids();
    let mut rows = providers
        .iter()
        .filter(|provider| provider_is_free_pool_member(provider, &default_pool))
        .map(|provider| {
            let free_model_count = provider
                .models
                .iter()
                .filter(|model| model_is_free(model))
                .count();
            let healthy_model_count = provider
                .models
                .iter()
                .filter(|model| model.healthy && !model.in_cooldown)
                .count();
            let skip_reasons = free_pool_skip_reasons(provider, healthy_model_count);
            let profile = derive_route_execution_profile(
                &crate::types::CharonRequestEnvelope {
                    agent_id: "observability".to_string(),
                    task_type: "code".to_string(),
                    priority: "normal".to_string(),
                    messages: vec![],
                    options: serde_json::json!({"workload_role": "execution"}),
                },
                "normal",
            );
            json!({
                "provider_id": provider.id,
                "access_tier": provider.access_tier,
                "driver": provider.driver,
                "in_pool": skip_reasons.is_empty(),
                "skip_reasons": skip_reasons,
                "probe_model": provider.probe_model,
                "probe_profile": provider.probe_profile,
                "avg_latency_ms": provider.avg_latency_ms,
                "consecutive_failures": provider.consecutive_failures,
                "consecutive_successes": provider.consecutive_successes,
                "last_failure_class": provider.last_error.as_deref().map(classify_failure_text),
                "free_model_count": free_model_count,
                "healthy_model_count": healthy_model_count,
                "requests_used_minute": provider.requests_used_minute,
                "requests_per_minute": provider.requests_per_minute,
                "requests_used_day": provider.requests_used_day,
                "requests_per_day": provider.requests_per_day,
                "cooldown_until_utc": provider.cooldown_until_utc,
                "pacing_state_execution": pacing_state_for_provider(provider, "normal", &profile),
            })
        })
        .collect::<Vec<_>>();

    rows.sort_by(|a, b| {
        let a_in_pool = a
            .get("in_pool")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        let b_in_pool = b
            .get("in_pool")
            .and_then(JsonValue::as_bool)
            .unwrap_or(false);
        b_in_pool.cmp(&a_in_pool).then_with(|| {
            let a_latency = a
                .get("avg_latency_ms")
                .and_then(JsonValue::as_u64)
                .unwrap_or(u64::MAX);
            let b_latency = b
                .get("avg_latency_ms")
                .and_then(JsonValue::as_u64)
                .unwrap_or(u64::MAX);
            a_latency.cmp(&b_latency)
        })
    });

    json!({
        "default_provider_ids": default_pool,
        "providers": rows,
    })
}

fn provider_is_free_pool_member(
    provider: &crate::types::ProviderState,
    default_pool: &[&'static str],
) -> bool {
    provider
        .access_tier
        .trim()
        .eq_ignore_ascii_case("free_cloud")
        || default_pool.iter().any(|id| *id == provider.id)
        || provider.models.iter().any(model_is_free)
}

fn free_pool_skip_reasons(
    provider: &crate::types::ProviderState,
    healthy_model_count: usize,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if !provider.enabled {
        reasons.push("disabled".to_string());
    }
    if !provider.has_api_key {
        reasons.push("missing_api_key".to_string());
    }
    if !provider.healthy {
        reasons.push("unhealthy".to_string());
    }
    if provider.in_cooldown {
        reasons.push("provider_cooldown".to_string());
    }
    if provider
        .last_error
        .as_deref()
        .is_some_and(is_billing_or_quota_text)
    {
        reasons.push("billing_or_quota_failure_memory".to_string());
    }
    if provider
        .requests_per_minute
        .is_some_and(|max| max > 0 && provider.requests_used_minute >= max)
    {
        reasons.push("minute_quota_exhausted".to_string());
    } else if provider
        .requests_per_minute
        .is_some_and(|max| max > 0 && provider.requests_used_minute >= max.saturating_mul(9) / 10)
    {
        reasons.push("minute_quota_risk".to_string());
    }
    if provider
        .requests_per_day
        .is_some_and(|max| max > 0 && provider.requests_used_day >= max)
    {
        reasons.push("day_quota_exhausted".to_string());
    } else if provider
        .requests_per_day
        .is_some_and(|max| max > 0 && provider.requests_used_day >= max.saturating_mul(9) / 10)
    {
        reasons.push("day_quota_risk".to_string());
    }
    if healthy_model_count == 0 {
        reasons.push("no_healthy_models".to_string());
    }
    reasons
}

fn default_free_pool_provider_ids() -> Vec<&'static str> {
    vec![
        "openrouter",
        "nvidia",
        "groq",
        "cerebras",
        "google",
        "opencode",
    ]
}

fn model_is_free(model: &crate::types::ModelState) -> bool {
    let id = model.id.to_ascii_lowercase();
    id.ends_with(":free")
        || id.ends_with("-free")
        || id.contains("/free")
        || matches!(
            (
                model.cost_per_million_tokens_in,
                model.cost_per_million_tokens_out
            ),
            (Some(input), Some(output)) if input <= 0.0 && output <= 0.0
        )
}

#[derive(Default)]
struct FailureBucket {
    provider_id: String,
    model_id: Option<String>,
    outcome_class: String,
    count: u64,
    last_error: Option<String>,
}

struct TaskModelBucket {
    task_type: String,
    provider_id: String,
    model_id: String,
    successes: u64,
    failures: u64,
    avg_latency_ms: Option<u64>,
}

fn add_failure(
    failures: &mut BTreeMap<String, FailureBucket>,
    payload: &JsonValue,
    model_id: Option<&str>,
) {
    let provider_id = payload
        .get("provider_id")
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    let model_id = model_id.or_else(|| payload.get("model_id").and_then(JsonValue::as_str));
    let error = payload
        .get("error")
        .and_then(JsonValue::as_str)
        .map(str::to_string);
    let outcome_class = payload
        .get("outcome_class")
        .and_then(JsonValue::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| classify_failure_text(error.as_deref().unwrap_or("")));
    let key = format!(
        "{}|{}|{}",
        provider_id,
        model_id.unwrap_or("*"),
        outcome_class
    );
    let bucket = failures.entry(key).or_insert_with(|| FailureBucket {
        provider_id: provider_id.to_string(),
        model_id: model_id.map(str::to_string),
        outcome_class,
        count: 0,
        last_error: None,
    });
    bucket.count += 1;
    if error.is_some() {
        bucket.last_error = error;
    }
}

fn classify_failure_text(error: &str) -> String {
    let lowered = error.to_ascii_lowercase();
    if is_billing_or_quota_text(&lowered) {
        "billing_or_quota_risk".to_string()
    } else if lowered.contains("timeout") {
        "timeout".to_string()
    } else if lowered.contains("transport") || lowered.contains("connection") {
        "transport_failure".to_string()
    } else if lowered.contains("model") && lowered.contains("not") {
        "model_unavailable".to_string()
    } else {
        "unknown_failure".to_string()
    }
}

fn is_billing_or_quota_text(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "billing",
        "quota",
        "credit",
        "balance",
        "payment",
        "recharge",
        "rate limit",
        "resource package",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

fn merge_latency(existing: Option<u64>, latency: Option<u64>) -> Option<u64> {
    match (existing, latency) {
        (Some(a), Some(b)) => Some(((a as f64 * 0.7) + (b as f64 * 0.3)).round() as u64),
        (None, Some(b)) => Some(b),
        (Some(a), None) => Some(a),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ModelCapabilities, ModelState, ProviderState};

    fn provider(id: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
            base_url: Some("https://example.invalid/v1".to_string()),
            api_key_env: Some("EXAMPLE_API_KEY".to_string()),
            access_tier: "mixed".to_string(),
            quality_band: "high".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: Some("provider/healthy-free".to_string()),
            probe_profile: Some("low_latency_terse".to_string()),
            enabled: true,
            has_api_key: true,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 0,
            requests_per_minute: Some(60),
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: Some(1_000),
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                id: "provider/healthy-free".to_string(),
                aliases: vec![],
                capable_tasks: vec!["chat".to_string()],
                context_window: 128_000,
                is_default: true,
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: Some(1_000),
                cost_per_million_tokens_in: Some(0.0),
                cost_per_million_tokens_out: Some(0.0),
                capabilities: ModelCapabilities::default(),
                streaming_validated: None,
            }],
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 3,
            last_error: None,
            avg_latency_ms: Some(1_500),
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
    fn free_provider_pool_rollup_reports_pool_and_skip_reasons() {
        let ready = provider("openrouter");
        let mut blocked = provider("nvidia");
        blocked.in_cooldown = true;
        blocked.last_error = Some("quota exceeded".to_string());
        blocked.requests_used_minute = 54;

        let rollup = free_provider_pool_rollup(&[ready, blocked]);
        let ids = rollup["default_provider_ids"]
            .as_array()
            .expect("default ids")
            .iter()
            .filter_map(JsonValue::as_str)
            .collect::<Vec<_>>();
        assert!(ids.contains(&"openrouter"));
        assert!(ids.contains(&"nvidia"));
        assert!(ids.contains(&"groq"));
        assert!(ids.contains(&"cerebras"));
        assert!(ids.contains(&"google"));
        assert!(ids.contains(&"opencode"));

        let providers = rollup["providers"].as_array().expect("providers");
        let ready_row = providers
            .iter()
            .find(|row| row["provider_id"] == "openrouter")
            .expect("ready row");
        assert_eq!(ready_row["in_pool"], true);
        assert_eq!(ready_row["free_model_count"], 1);

        let blocked_row = providers
            .iter()
            .find(|row| row["provider_id"] == "nvidia")
            .expect("blocked row");
        assert_eq!(blocked_row["in_pool"], false);
        let reasons = blocked_row["skip_reasons"]
            .as_array()
            .expect("skip reasons")
            .iter()
            .filter_map(JsonValue::as_str)
            .collect::<Vec<_>>();
        assert!(reasons.contains(&"provider_cooldown"));
        assert!(reasons.contains(&"billing_or_quota_failure_memory"));
        assert!(reasons.contains(&"minute_quota_risk"));
    }
}
