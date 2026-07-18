use crate::adaptive::service::capabilities::ProviderCapabilitySummary;
use crate::adaptive::service::types::{CharonService, ProviderState};
use crate::adaptive::service::bootstrap_runtime::collect_package_runtime_signals;
use crate::adaptive::service::state_io::{count_malformed_jsonl, read_recent_jsonl, runtime_build_cache_autorun_enabled, runtime_build_cache_command_args, runtime_build_cache_command_program, runtime_build_cache_state_path};
use crate::adaptive::service::runtime_state::refresh_provider_windows;
use arda_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderBudgetPressure {
    pub provider_id: String,
    pub provider_name: String,
    pub level: String,
    pub minute_usage_ratio: Option<f64>,
    pub day_usage_ratio: Option<f64>,
    pub in_cooldown: bool,
    pub cooldown_until_utc: Option<String>,
    pub exhausted_minute: bool,
    pub exhausted_day: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetPressureSummary {
    pub providers_total: usize,
    pub warning_total: usize,
    pub critical_total: usize,
    pub cooldown_total: usize,
    pub exhausted_total: usize,
    pub highest_level: String,
    pub providers: Vec<ProviderBudgetPressure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharonAlert {
    pub level: String,
    pub provider_id: String,
    pub provider_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharonStatus {
    pub charon_version: String,
    pub providers_total: usize,
    pub providers_enabled: usize,
    pub providers_ready: usize,
    pub providers_healthy: usize,
    pub providers_exhausted: usize,
    pub providers_degraded: usize,
    pub providers_in_cooldown: usize,
    pub provider_state_counts: BTreeMap<String, usize>,
    pub malformed_state_events: usize,
    pub malformed_governance_events: usize,
    pub recent_route_failures: usize,
    pub recent_route_successes: usize,
    pub recent_local_fallback_routes: usize,
    pub capability_summary: ProviderCapabilitySummary,
    pub budget_pressure: BudgetPressureSummary,
    pub route_guardrails: RouteGuardrailSummary,
    pub alerts: Vec<CharonAlert>,
    pub llmfit_backend: String,
    pub llmfit_recommendation_count: usize,
    pub nanoclaw_runtime_ready: bool,
    pub nanoclaw_probe_state: String,
    pub runtime_build_cache_status: String,
    pub runtime_build_cache_observed_bytes: u64,
    pub runtime_build_cache_removed_bytes: u64,
    pub state_path: String,
    pub governance_events_path: String,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteGuardrailSummary {
    pub tool_execution_min_context_window: usize,
    pub low_context_tool_model_total: usize,
    pub visible_reasoning_model_total: usize,
    pub tool_incompatible_model_total: usize,
    pub hermes_tool_routing: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderOperationalState {
    pub state: String,
    pub reason: String,
    pub blocked: bool,
    pub reset_seconds_estimate: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PackageRuntimeSignals {
    pub generated_at_utc: String,
    pub llmfit_backend: String,
    pub llmfit_recommendation_count: usize,
    pub llmfit_local_max_params_b: Option<f64>,
    pub llmfit_top_model_names: Vec<String>,
    pub nanoclaw_binary_present: bool,
    pub nanoclaw_runtime_ready: bool,
    pub nanoclaw_probe_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RuntimeBuildCacheSignals {
    pub generated_at_utc: String,
    pub authority: String,
    pub build_root: String,
    pub target_dir: String,
    pub observed_bytes: u64,
    pub target_bytes: u64,
    pub removed_bytes: u64,
    pub status: String,
}

impl CharonService {
    pub async fn status(&self) -> Result<CharonStatus> {
        let mut providers = self.providers.write().await;
        refresh_provider_windows(&mut providers, Utc::now());
        let recent_events = read_recent_jsonl(&self.state_path, 48);
        let package_runtime = self.read_package_runtime_signals();
        let build_cache = self.read_runtime_build_cache_signals();
        let providers_total = providers.len();
        let providers_enabled = providers.iter().filter(|p| p.enabled).count();
        let providers_ready = providers
            .iter()
            .filter(|p| p.enabled && p.has_api_key)
            .count();
        let providers_healthy = providers
            .iter()
            .filter(|p| p.healthy && p.enabled && p.has_api_key && !p.in_cooldown)
            .count();
        let providers_degraded = providers
            .iter()
            .filter(|p| p.consecutive_failures >= 2 || p.error_count >= 5)
            .count();
        let providers_exhausted = providers
            .iter()
            .filter(|p| {
                p.requests_per_day
                    .is_some_and(|max| p.requests_used_day >= max)
            })
            .count();
        let providers_in_cooldown = providers.iter().filter(|p| p.in_cooldown).count();
        let provider_state_counts = provider_state_counts(&providers, Utc::now());
        let recent_route_failures = recent_events
            .iter()
            .filter(|value| value.get("event").and_then(|v| v.as_str()) == Some("route_failed"))
            .count();
        let recent_route_successes = recent_events
            .iter()
            .filter(|value| value.get("event").and_then(|v| v.as_str()) == Some("route_selected"))
            .count();
        let recent_local_fallback_routes = recent_events
            .iter()
            .filter(|value| {
                value.get("event").and_then(|v| v.as_str()) == Some("route_selected")
                    && value
                        .get("payload")
                        .and_then(|v| v.get("provider_id"))
                        .and_then(|v| v.as_str())
                        == Some("local_fallback")
            })
            .count();
        let budget_pressure = build_budget_pressure_summary(&providers);
        let capability_summary = self.provider_capability_summary();
        let route_guardrails = build_route_guardrail_summary(&providers);
        let alerts = build_budget_alerts(&budget_pressure);
        Ok(CharonStatus {
            charon_version: "0.1.0".to_string(),
            providers_total,
            providers_enabled,
            providers_ready,
            providers_healthy,
            providers_exhausted,
            providers_degraded,
            providers_in_cooldown,
            provider_state_counts,
            malformed_state_events: count_malformed_jsonl(&self.state_path),
            malformed_governance_events: count_malformed_jsonl(&self.governance_events_path),
            recent_route_failures,
            recent_route_successes,
            recent_local_fallback_routes,
            capability_summary,
            budget_pressure,
            route_guardrails,
            alerts,
            llmfit_backend: package_runtime.llmfit_backend,
            llmfit_recommendation_count: package_runtime.llmfit_recommendation_count,
            nanoclaw_runtime_ready: package_runtime.nanoclaw_runtime_ready,
            nanoclaw_probe_state: package_runtime.nanoclaw_probe_state,
            runtime_build_cache_status: build_cache.status,
            runtime_build_cache_observed_bytes: build_cache.observed_bytes,
            runtime_build_cache_removed_bytes: build_cache.removed_bytes,
            state_path: self.state_path.display().to_string(),
            governance_events_path: self.governance_events_path.display().to_string(),
            socket_path: self.socket_path().display().to_string(),
        })
    }

    pub(super) fn read_package_runtime_signals(&self) -> PackageRuntimeSignals {
        let path = self.package_runtime_signals_path();
        let signals = collect_package_runtime_signals();
        let _ = fs::write(
            &path,
            serde_json::to_string_pretty(&signals).unwrap_or_else(|_| "{}".to_string()) + "\n",
        );
        signals
    }

    pub(super) fn read_runtime_build_cache_signals(&self) -> RuntimeBuildCacheSignals {
        let path = runtime_build_cache_state_path();
        if runtime_build_cache_autorun_enabled() {
            let mut command = Command::new(runtime_build_cache_command_program());
            command.args(runtime_build_cache_command_args());
            let _ = command.output();
        }
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(value) = serde_json::from_str::<RuntimeBuildCacheSignals>(&content) {
                return value;
            }
        }
        RuntimeBuildCacheSignals {
            generated_at_utc: Utc::now().to_rfc3339(),
            authority: "charon_housekeeping_fallback".to_string(),
            build_root: std::env::var("ARDA_BUILD_CACHE_ROOT")
                .or_else(|_| std::env::var("ARDA_RUNTIME_BUILD_ROOT"))
                .unwrap_or_else(|_| "/tmp/arda-build".to_string()),
            target_dir: std::env::var("CARGO_TARGET_DIR")
                .unwrap_or_else(|_| "/tmp/arda-build/target".to_string()),
            observed_bytes: 0,
            target_bytes: 0,
            removed_bytes: 0,
            status: "unknown".to_string(),
        }
    }
}

fn build_route_guardrail_summary(providers: &[ProviderState]) -> RouteGuardrailSummary {
    let tool_floor = super::route_policy::tool_execution_min_context_window();
    let mut low_context_tool_model_total = 0;
    let mut visible_reasoning_model_total = 0;
    let mut tool_incompatible_model_total = 0;

    for provider in providers {
        for model in &provider.models {
            if model.context_window < tool_floor {
                low_context_tool_model_total += 1;
            }
            if super::route_policy::model_has_visible_reasoning_surface(model) {
                visible_reasoning_model_total += 1;
            }
            if model.capabilities.tools == Some(false) || !provider.supports_tools {
                tool_incompatible_model_total += 1;
            }
        }
    }

    RouteGuardrailSummary {
        tool_execution_min_context_window: tool_floor,
        low_context_tool_model_total,
        visible_reasoning_model_total,
        tool_incompatible_model_total,
        hermes_tool_routing: "tool/code routes require tool-capable non-visible-reasoning models at or above the context floor unless an explicit emergency low-context fallback flag is present".to_string(),
    }
}

pub(crate) fn classify_provider_operational_state(
    provider: &ProviderState,
    now: chrono::DateTime<Utc>,
) -> ProviderOperationalState {
    if !provider.enabled {
        return ProviderOperationalState {
            state: "disabled".to_string(),
            reason: "provider disabled in config".to_string(),
            blocked: true,
            reset_seconds_estimate: None,
        };
    }
    if !provider.has_api_key {
        return ProviderOperationalState {
            state: "missing_api_key".to_string(),
            reason: "provider missing API key".to_string(),
            blocked: true,
            reset_seconds_estimate: None,
        };
    }

    let last_error = provider
        .last_error
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let minute_reset =
        estimate_reset_seconds(provider.minute_window_started_utc.as_deref(), 60, now);
    let day_reset = estimate_reset_seconds(provider.day_window_started_utc.as_deref(), 86_400, now);

    if provider
        .requests_per_day
        .is_some_and(|max| provider.requests_used_day >= max)
    {
        return ProviderOperationalState {
            state: "rate_limited".to_string(),
            reason: "daily request budget exhausted".to_string(),
            blocked: true,
            reset_seconds_estimate: day_reset,
        };
    }
    if provider
        .requests_per_minute
        .is_some_and(|max| provider.requests_used_minute >= max)
    {
        return ProviderOperationalState {
            state: "minute_quota_exhausted".to_string(),
            reason: "minute request budget exhausted".to_string(),
            blocked: true,
            reset_seconds_estimate: minute_reset,
        };
    }
    if provider.in_cooldown {
        let (state, reason) = classify_error_surface(&last_error);
        return ProviderOperationalState {
            state: if state == "ready" {
                "cooldown".to_string()
            } else {
                state.to_string()
            },
            reason: if reason.is_empty() {
                format!(
                    "provider cooling down until {}",
                    provider.cooldown_until_utc.as_deref().unwrap_or("unknown")
                )
            } else {
                reason.to_string()
            },
            blocked: true,
            reset_seconds_estimate: provider
                .cooldown_until_utc
                .as_deref()
                .and_then(|until| chrono::DateTime::parse_from_rfc3339(until).ok())
                .map(|until| (until.with_timezone(&Utc) - now).num_seconds().max(0)),
        };
    }
    if !provider.healthy {
        let (state, reason) = classify_error_surface(&last_error);
        return ProviderOperationalState {
            state: if state == "ready" {
                "unhealthy".to_string()
            } else {
                state.to_string()
            },
            reason: if reason.is_empty() {
                provider
                    .last_error
                    .clone()
                    .unwrap_or_else(|| "provider health probe failed".to_string())
            } else {
                reason.to_string()
            },
            blocked: true,
            reset_seconds_estimate: None,
        };
    }
    if provider.consecutive_failures >= 2 {
        return ProviderOperationalState {
            state: "degraded".to_string(),
            reason: "provider is on a recent failure streak".to_string(),
            blocked: false,
            reset_seconds_estimate: None,
        };
    }

    ProviderOperationalState {
        state: "ready".to_string(),
        reason: "provider available for routing".to_string(),
        blocked: false,
        reset_seconds_estimate: None,
    }
}

fn classify_error_surface(last_error: &str) -> (&'static str, &'static str) {
    if last_error.is_empty() {
        return ("ready", "");
    }
    if [
        "insufficient balance",
        "insufficient credits",
        "requires more credits",
        "creditserror",
        "billing",
        "out of credits",
        "spend limit",
        "quota exceeded",
    ]
    .iter()
    .any(|needle| last_error.contains(needle))
    {
        return (
            "spend_blocked",
            "provider balance or credit limit is exhausted",
        );
    }
    if [
        "rate limited",
        "rate_limit_exceeded",
        "too many requests",
        "tokens per minute",
        "tpm",
        "daily limit",
    ]
    .iter()
    .any(|needle| last_error.contains(needle))
    {
        return ("rate_limited", "provider is currently rate limited");
    }
    if [
        "unauthorized",
        "invalid api key",
        "authentication",
        "auth",
        "forbidden",
        "permission denied",
        "api key",
    ]
    .iter()
    .any(|needle| last_error.contains(needle))
    {
        return (
            "auth_failed",
            "provider authentication or authorization failed",
        );
    }
    if [
        "extra inputs are not permitted",
        "invalid_function_call",
        "input should be 'function'",
        "not found for account",
        "validation errors",
        "tool call id",
    ]
    .iter()
    .any(|needle| last_error.contains(needle))
    {
        return (
            "schema_incompatible",
            "provider rejected the request schema for this call shape",
        );
    }
    ("ready", "")
}

fn estimate_reset_seconds(
    started_at_utc: Option<&str>,
    window_seconds: i64,
    now: chrono::DateTime<Utc>,
) -> Option<i64> {
    let started = chrono::DateTime::parse_from_rfc3339(started_at_utc?).ok()?;
    let elapsed = (now - started.with_timezone(&Utc)).num_seconds();
    Some((window_seconds - elapsed).max(0))
}

fn provider_state_counts(
    providers: &[ProviderState],
    now: chrono::DateTime<Utc>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for provider in providers {
        let state = classify_provider_operational_state(provider, now).state;
        *counts.entry(state).or_insert(0) += 1;
    }
    counts
}

pub(super) fn build_budget_pressure_summary(providers: &[ProviderState]) -> BudgetPressureSummary {
    let mut rows = providers
        .iter()
        .filter(|provider| provider.enabled)
        .map(provider_budget_pressure)
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        pressure_rank(&b.level)
            .cmp(&pressure_rank(&a.level))
            .then_with(|| a.provider_id.cmp(&b.provider_id))
    });

    let warning_total = rows.iter().filter(|row| row.level == "warning").count();
    let critical_total = rows.iter().filter(|row| row.level == "critical").count();
    let cooldown_total = rows.iter().filter(|row| row.in_cooldown).count();
    let exhausted_total = rows
        .iter()
        .filter(|row| row.exhausted_minute || row.exhausted_day)
        .count();
    let highest_level = rows
        .iter()
        .map(|row| row.level.as_str())
        .max_by_key(|level| pressure_rank(level))
        .unwrap_or("ok")
        .to_string();

    BudgetPressureSummary {
        providers_total: rows.len(),
        warning_total,
        critical_total,
        cooldown_total,
        exhausted_total,
        highest_level,
        providers: rows,
    }
}

pub(super) fn build_budget_alerts(summary: &BudgetPressureSummary) -> Vec<CharonAlert> {
    let mut alerts = Vec::new();
    for row in &summary.providers {
        if row.level == "ok" {
            continue;
        }
        let message = if row.exhausted_day {
            format!(
                "{} exhausted its daily request budget and should be deprioritized until reset",
                row.provider_name
            )
        } else if row.exhausted_minute {
            format!(
                "{} exhausted its minute request budget and is under immediate pressure",
                row.provider_name
            )
        } else if row.in_cooldown {
            format!(
                "{} is in cooldown until {}",
                row.provider_name,
                row.cooldown_until_utc.as_deref().unwrap_or("unknown")
            )
        } else {
            let minute = row
                .minute_usage_ratio
                .map(|value| format!("{:.1}% minute", value * 100.0));
            let day = row
                .day_usage_ratio
                .map(|value| format!("{:.1}% day", value * 100.0));
            let joined = [minute, day]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" / ");
            format!("{} is under budget pressure ({joined})", row.provider_name)
        };
        alerts.push(CharonAlert {
            level: row.level.clone(),
            provider_id: row.provider_id.clone(),
            provider_name: row.provider_name.clone(),
            message,
        });
    }
    alerts
}

fn provider_budget_pressure(provider: &ProviderState) -> ProviderBudgetPressure {
    let minute_usage_ratio =
        quota_ratio(provider.requests_used_minute, provider.requests_per_minute);
    let day_usage_ratio = quota_ratio(provider.requests_used_day, provider.requests_per_day);
    let exhausted_minute = provider
        .requests_per_minute
        .is_some_and(|max| provider.requests_used_minute >= max);
    let exhausted_day = provider
        .requests_per_day
        .is_some_and(|max| provider.requests_used_day >= max);
    let critical_pressure = exhausted_minute
        || exhausted_day
        || provider.in_cooldown
        || minute_usage_ratio.is_some_and(|value| value >= 0.90)
        || day_usage_ratio.is_some_and(|value| value >= 0.90);
    let warning_pressure = minute_usage_ratio.is_some_and(|value| value >= 0.75)
        || day_usage_ratio.is_some_and(|value| value >= 0.75);
    let level = if critical_pressure {
        "critical"
    } else if warning_pressure {
        "warning"
    } else {
        "ok"
    };

    ProviderBudgetPressure {
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        level: level.to_string(),
        minute_usage_ratio,
        day_usage_ratio,
        in_cooldown: provider.in_cooldown,
        cooldown_until_utc: provider.cooldown_until_utc.clone(),
        exhausted_minute,
        exhausted_day,
    }
}

fn quota_ratio(used: u64, max: Option<u64>) -> Option<f64> {
    max.filter(|value| *value > 0)
        .map(|value| (used as f64 / value as f64).clamp(0.0, 1.0))
}

fn pressure_rank(level: &str) -> u8 {
    match level {
        "critical" => 3,
        "warning" => 2,
        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::classify_provider_operational_state;
    use crate::adaptive::service::types::{ModelState, ProviderState};
    use chrono::Utc;

    fn provider() -> ProviderState {
        ProviderState {
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
            minute_window_started_utc: Some(Utc::now().to_rfc3339()),
            requests_per_day: Some(10_000),
            requests_used_day: 0,
            day_window_started_utc: Some(Utc::now().to_rfc3339()),
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
        }
    }

    #[test]
    fn classifies_opencode_credit_errors_as_spend_blocked() {
        let mut provider = provider();
        provider.in_cooldown = true;
        provider.cooldown_until_utc =
            Some((Utc::now() + chrono::Duration::minutes(30)).to_rfc3339());
        provider.last_error = Some("Insufficient balance. Manage your billing here: https://opencode.ai/workspace/x/billing".to_string());

        let state = classify_provider_operational_state(&provider, Utc::now());
        assert_eq!(state.state, "spend_blocked");
        assert!(state.blocked);
    }

    #[test]
    fn classifies_mistral_schema_errors_as_schema_incompatible() {
        let mut provider = provider();
        provider.healthy = false;
        provider.last_error =
            Some("provider mistral HTTP 422: extra inputs are not permitted".to_string());

        let state = classify_provider_operational_state(&provider, Utc::now());
        assert_eq!(state.state, "schema_incompatible");
        assert!(state.blocked);
    }
}