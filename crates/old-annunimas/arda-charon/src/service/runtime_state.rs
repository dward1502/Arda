use super::{status::classify_provider_operational_state, CharonService};
use crate::types::ProviderState;
use arda_core::error::Result;
use chrono::{Duration, Utc};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderRuntimeSnapshot {
    schema_version: u32,
    generated_at_utc: String,
    providers: Vec<ProviderState>,
}

pub(super) fn merge_runtime_state(
    current: Vec<ProviderState>,
    mut loaded: Vec<ProviderState>,
) -> Vec<ProviderState> {
    for provider in &mut loaded {
        if let Some(existing) = current.iter().find(|p| p.id == provider.id) {
            provider.intelligence_refreshed_at_utc = existing.intelligence_refreshed_at_utc.clone();
            provider.probe_model = existing.probe_model.clone();
            provider.probe_profile = existing.probe_profile.clone();
            provider.requests_used_minute = existing.requests_used_minute;
            provider.requests_used_day = existing.requests_used_day;
            provider.in_cooldown = existing.in_cooldown;
            provider.cooldown_until_utc = existing.cooldown_until_utc.clone();
            provider.cooldown_backoff_seconds = existing.cooldown_backoff_seconds;
            provider.error_count = existing.error_count;
            provider.consecutive_failures = existing.consecutive_failures;
            provider.consecutive_successes = existing.consecutive_successes;
            provider.last_error = existing.last_error.clone();
            provider.avg_latency_ms = existing.avg_latency_ms;
            provider.minute_window_started_utc = existing.minute_window_started_utc.clone();
            provider.day_window_started_utc = existing.day_window_started_utc.clone();
            for model in &mut provider.models {
                if let Some(existing_model) = existing.models.iter().find(|m| m.id == model.id) {
                    model.healthy = existing_model.healthy;
                    model.in_cooldown = existing_model.in_cooldown;
                    model.cooldown_until_utc = existing_model.cooldown_until_utc.clone();
                    model.consecutive_failures = existing_model.consecutive_failures;
                    model.consecutive_successes = existing_model.consecutive_successes;
                    model.last_error = existing_model.last_error.clone();
                    model.avg_latency_ms = existing_model.avg_latency_ms;
                    if model.cost_per_million_tokens_in.is_none() {
                        model.cost_per_million_tokens_in =
                            existing_model.cost_per_million_tokens_in;
                    }
                    if model.cost_per_million_tokens_out.is_none() {
                        model.cost_per_million_tokens_out =
                            existing_model.cost_per_million_tokens_out;
                    }
                    if model.capabilities.tools.is_none() {
                        model.capabilities.tools = existing_model.capabilities.tools;
                    }
                    if model.capabilities.streaming.is_none() {
                        model.capabilities.streaming = existing_model.capabilities.streaming;
                    }
                    if model.capabilities.structured_output.is_none() {
                        model.capabilities.structured_output =
                            existing_model.capabilities.structured_output;
                    }
                    model.streaming_validated = existing_model.streaming_validated;
                }
            }
        }
    }
    loaded
}

pub(super) fn merge_persisted_runtime_state(
    path: &Path,
    loaded: Vec<ProviderState>,
) -> Result<Vec<ProviderState>> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(loaded),
        Err(err) => return Err(err.into()),
    };
    let snapshot = serde_json::from_str::<ProviderRuntimeSnapshot>(&content)?;
    if snapshot.schema_version != 1 {
        return Ok(loaded);
    }
    Ok(merge_runtime_state(snapshot.providers, loaded))
}

pub(super) fn persist_runtime_state_snapshot(
    path: &Path,
    providers: &[ProviderState],
) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let snapshot = ProviderRuntimeSnapshot {
        schema_version: 1,
        generated_at_utc: Utc::now().to_rfc3339(),
        providers: providers.to_vec(),
    };
    let bytes = serde_json::to_vec_pretty(&snapshot)?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, bytes)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

pub(super) fn refresh_provider_windows(
    providers: &mut [ProviderState],
    now: chrono::DateTime<Utc>,
) {
    for p in providers {
        refresh_model_windows(&mut p.models, now);
        reap_stale_provider_reservations(p, now);
        if let Some(until) = p.cooldown_until_utc.as_deref() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(until) {
                if dt.with_timezone(&Utc) <= now {
                    p.in_cooldown = false;
                    p.cooldown_until_utc = None;
                    if p.consecutive_failures >= 3 {
                        p.consecutive_successes = 0;
                    }
                }
            }
        }
        if let Some(start) = p.minute_window_started_utc.as_deref() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start) {
                if (now - dt.with_timezone(&Utc)).num_seconds() >= 60 {
                    p.requests_used_minute = 0;
                    p.minute_window_started_utc = Some(now.to_rfc3339());
                }
            }
        }
        if let Some(start) = p.day_window_started_utc.as_deref() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(start) {
                if (now - dt.with_timezone(&Utc)).num_seconds() >= 86_400 {
                    p.requests_used_day = 0;
                    p.day_window_started_utc = Some(now.to_rfc3339());
                }
            }
        }
    }
}

fn refresh_model_windows(models: &mut [crate::types::ModelState], now: chrono::DateTime<Utc>) {
    for model in models {
        if let Some(until) = model.cooldown_until_utc.as_deref() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(until) {
                if dt.with_timezone(&Utc) <= now {
                    model.in_cooldown = false;
                    model.cooldown_until_utc = None;
                    model.healthy = true;
                    model.last_error = None;
                }
            }
        }
    }
}

fn reap_stale_provider_reservations(p: &mut ProviderState, now: chrono::DateTime<Utc>) {
    if p.active_connections == 0 {
        p.last_reservation_utc = None;
        return;
    }
    let ttl_seconds = stale_reservation_ttl_seconds();
    let Some(last_reserved) = p.last_reservation_utc.as_deref() else {
        p.active_connections = 0;
        return;
    };
    let Ok(dt) = chrono::DateTime::parse_from_rfc3339(last_reserved) else {
        p.active_connections = 0;
        p.last_reservation_utc = None;
        return;
    };
    if (now - dt.with_timezone(&Utc)).num_seconds() >= ttl_seconds {
        p.active_connections = 0;
        p.last_reservation_utc = None;
    }
}

fn stale_reservation_ttl_seconds() -> i64 {
    std::env::var("ARDA_CHARON_RESERVATION_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(900)
}

pub(super) fn failure_backoff_seconds(consecutive_failures: u32) -> i64 {
    let exponent = consecutive_failures.saturating_sub(3).min(4);
    let factor = 2u64.saturating_pow(exponent);
    (120u64.saturating_mul(factor).min(1_800)) as i64
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

pub(super) fn provider_unavailable_reason(
    p: &ProviderState,
    priority: &str,
    strict: bool,
    now: chrono::DateTime<Utc>,
) -> Option<serde_json::Value> {
    let operational = classify_provider_operational_state(p, now);
    if !p.enabled {
        return Some(serde_json::json!({"provider_id": p.id, "reason": "disabled"}));
    }
    if !p.has_api_key {
        return Some(serde_json::json!({"provider_id": p.id, "reason": "missing_api_key"}));
    }
    if !p.healthy {
        return Some(serde_json::json!({
            "provider_id": p.id,
            "reason": operational.state,
            "detail": operational.reason
        }));
    }
    if p.in_cooldown {
        return Some(serde_json::json!({
            "provider_id": p.id,
            "reason": operational.state,
            "detail": operational.reason,
            "cooldown_until_utc": p.cooldown_until_utc,
            "cooldown_backoff_seconds": p.cooldown_backoff_seconds,
            "reset_seconds_estimate": operational.reset_seconds_estimate
        }));
    }
    if super::provider_in_half_open(p) {
        return Some(serde_json::json!({
            "provider_id": p.id,
            "reason": "half_open_probe_throttled",
            "detail": "provider cooldown expired; routing only a probe fraction until a request succeeds",
            "probe_stride": std::env::var("ARDA_CHARON_HALF_OPEN_PROBE_STRIDE")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
                .filter(|value| *value > 0)
                .unwrap_or(10)
                .min(1_000),
        }));
    }
    if p.requests_per_minute
        .is_some_and(|max| p.requests_used_minute >= max)
    {
        return Some(serde_json::json!({
            "provider_id": p.id,
            "reason": operational.state,
            "detail": operational.reason,
            "reset_seconds_estimate": estimate_reset_seconds(p.minute_window_started_utc.as_deref(), 60, now),
        }));
    }
    if p.requests_per_day
        .is_some_and(|max| p.requests_used_day >= max)
    {
        return Some(serde_json::json!({
            "provider_id": p.id,
            "reason": operational.state,
            "detail": operational.reason,
            "reset_seconds_estimate": estimate_reset_seconds(p.day_window_started_utc.as_deref(), 86_400, now),
        }));
    }
    if strict && super::near_day_quota(p, 0.85) {
        return Some(serde_json::json!({"provider_id": p.id, "reason": "strict_near_day_quota"}));
    }
    if super::is_high_priority(priority) && p.consecutive_failures >= 2 {
        return Some(
            serde_json::json!({"provider_id": p.id, "reason": "high_priority_failure_streak"}),
        );
    }
    None
}

impl CharonService {
    pub(super) async fn refresh_local_provider_health(&self) -> Result<()> {
        let now = Utc::now();
        let providers_to_probe = {
            let providers = self.providers.read().await;
            providers
                .iter()
                .filter(|provider| should_probe_local_provider(provider, now))
                .map(|provider| {
                    (
                        provider.id.clone(),
                        provider.base_url.clone().unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>()
        };

        if providers_to_probe.is_empty() {
            return Ok(());
        }

        let client = reqwest::Client::builder()
            .timeout(StdDuration::from_secs(local_health_probe_timeout_seconds()))
            .build()
            .map_err(|err| arda_core::error::ArdaError::Agent {
                agent: "charon".to_string(),
                message: format!("failed to build local health probe client: {err}"),
            })?;

        for (provider_id, base_url) in providers_to_probe {
            let (healthy, latency_ms, error) = probe_local_provider(&client, &base_url).await;
            let model_probe_results = if healthy && provider_id == "edge_backbone" {
                let model_ids = {
                    let providers = self.providers.read().await;
                    providers
                        .iter()
                        .find(|provider| provider.id == provider_id)
                        .map(|provider| {
                            provider
                                .models
                                .iter()
                                .map(|model| model.id.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default()
                };
                let mut results = Vec::with_capacity(model_ids.len());
                for model_id in model_ids {
                    let (model_healthy, model_latency_ms, model_error) =
                        probe_local_model(&base_url, &model_id).await;
                    results.push((model_id, model_healthy, model_latency_ms, model_error));
                }
                results
            } else {
                Vec::new()
            };
            let mut providers = self.providers.write().await;
            if let Some(provider) = providers
                .iter_mut()
                .find(|provider| provider.id == provider_id)
            {
                provider.intelligence_refreshed_at_utc = Some(now.to_rfc3339());
                provider.avg_latency_ms = super::merge_latency(provider.avg_latency_ms, latency_ms);
                if healthy {
                    provider.healthy = true;
                    provider.last_error = None;
                } else {
                    provider.healthy = false;
                    provider.last_error = error;
                }

                if provider.healthy && provider.id == "edge_backbone" {
                    for (model_id, model_healthy, model_latency_ms, model_error) in
                        model_probe_results
                    {
                        let Some(model) = provider
                            .models
                            .iter_mut()
                            .find(|model| model.id == model_id)
                        else {
                            continue;
                        };
                        model.avg_latency_ms =
                            super::merge_latency(model.avg_latency_ms, model_latency_ms);
                        if model_healthy {
                            model.healthy = true;
                            model.in_cooldown = false;
                            model.cooldown_until_utc = None;
                            model.last_error = None;
                            model.consecutive_successes += 1;
                            model.consecutive_failures = 0;
                        } else {
                            model.healthy = false;
                            model.in_cooldown = true;
                            model.consecutive_successes = 0;
                            model.consecutive_failures += 1;
                            model.cooldown_until_utc = Some(
                                (Utc::now()
                                    + Duration::seconds(failure_backoff_seconds(
                                        model.consecutive_failures.max(3),
                                    )))
                                .to_rfc3339(),
                            );
                            model.last_error = model_error;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

fn should_probe_local_provider(provider: &ProviderState, now: chrono::DateTime<Utc>) -> bool {
    if !provider.enabled {
        return false;
    }
    if provider.access_tier != "local" {
        return false;
    }
    if provider
        .base_url
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        return false;
    }
    let min_interval = local_health_probe_interval_seconds();
    match provider.intelligence_refreshed_at_utc.as_deref() {
        Some(value) => chrono::DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|dt| (now - dt.with_timezone(&Utc)).num_seconds() >= min_interval)
            .unwrap_or(true),
        None => true,
    }
}

fn local_health_probe_interval_seconds() -> i64 {
    std::env::var("ARDA_CHARON_LOCAL_HEALTH_PROBE_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(45)
}

fn local_health_probe_timeout_seconds() -> u64 {
    std::env::var("ARDA_CHARON_LOCAL_HEALTH_PROBE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(2)
}

async fn probe_local_provider(
    client: &reqwest::Client,
    base_url: &str,
) -> (bool, Option<u64>, Option<String>) {
    let normalized = base_url.trim_end_matches('/');
    let candidates = if normalized.ends_with("/v1") {
        vec![
            format!("{normalized}/models"),
            format!("{}{}", normalized.trim_end_matches("/v1"), "/health"),
        ]
    } else {
        vec![
            format!("{normalized}/health"),
            format!("{normalized}/v1/models"),
        ]
    };

    let last_index = candidates.len().saturating_sub(1);
    for (index, url) in candidates.iter().enumerate() {
        let start = std::time::Instant::now();
        match client.get(url).send().await {
            Ok(response)
                if response.status().is_success()
                    || response.status() == StatusCode::UNAUTHORIZED =>
            {
                return (true, Some(start.elapsed().as_millis() as u64), None);
            }
            Ok(response) => {
                let err = format!(
                    "local health probe failed for {url}: HTTP {}",
                    response.status()
                );
                if index == last_index {
                    return (false, Some(start.elapsed().as_millis() as u64), Some(err));
                }
            }
            Err(err) => {
                let err_msg = format!("local health probe failed for {url}: {err}");
                if index == last_index {
                    return (
                        false,
                        Some(start.elapsed().as_millis() as u64),
                        Some(err_msg),
                    );
                }
            }
        }
    }

    (
        false,
        None,
        Some("local health probe failed with no probe candidates".to_string()),
    )
}

async fn probe_local_model(base_url: &str, model_id: &str) -> (bool, Option<u64>, Option<String>) {
    let normalized = base_url.trim_end_matches('/');
    let url = if normalized.ends_with("/v1") {
        format!("{normalized}/chat/completions")
    } else {
        format!("{normalized}/v1/chat/completions")
    };
    let client = match reqwest::Client::builder()
        .timeout(StdDuration::from_secs(local_model_probe_timeout_seconds()))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return (
                false,
                None,
                Some(format!("failed to build local model probe client: {err}")),
            )
        }
    };
    let start = std::time::Instant::now();
    let payload = serde_json::json!({
        "model": model_id,
        "messages": [{"role":"user","content":"ok"}],
        "max_tokens": 1
    });
    match client.post(&url).json(&payload).send().await {
        Ok(response) => {
            let latency = Some(start.elapsed().as_millis() as u64);
            if response.status().is_success() {
                (true, latency, None)
            } else {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                (
                    false,
                    latency,
                    Some(format!(
                        "local model probe failed for {model_id}: HTTP {status} {body}"
                    )),
                )
            }
        }
        Err(err) => (
            false,
            Some(start.elapsed().as_millis() as u64),
            Some(format!("local model probe failed for {model_id}: {err}")),
        ),
    }
}

fn local_model_probe_timeout_seconds() -> u64 {
    std::env::var("ARDA_CHARON_LOCAL_MODEL_PROBE_TIMEOUT_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn provider(id: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: id.to_string(),
            base_url: Some(format!("http://{}:{}/v1", "127.0.0.1", 1234)),
            api_key_env: None,
            access_tier: "mixed".to_string(),
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
            minute_window_started_utc: Some(Utc::now().to_rfc3339()),
            requests_per_day: Some(1_000),
            requests_used_day: 0,
            day_window_started_utc: Some(Utc::now().to_rfc3339()),
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
            models: vec![],
        }
    }

    #[test]
    fn merge_runtime_state_preserves_runtime_counters_for_matching_provider() {
        let current = vec![ProviderState {
            intelligence_refreshed_at_utc: Some("2026-03-30T04:00:00Z".to_string()),
            probe_model: Some("llama-3.1-8b-instant".to_string()),
            probe_profile: Some("low_latency_terse".to_string()),
            requests_per_day: Some(50),
            requests_used_minute: 7,
            requests_used_day: 42,
            in_cooldown: true,
            cooldown_until_utc: Some("2026-03-30T05:00:00Z".to_string()),
            error_count: 3,
            consecutive_failures: 2,
            consecutive_successes: 4,
            avg_latency_ms: Some(900),
            ..provider("groq")
        }];
        let loaded = vec![ProviderState {
            requests_per_day: Some(2_000),
            requests_used_minute: 0,
            requests_used_day: 0,
            in_cooldown: false,
            cooldown_until_utc: None,
            error_count: 0,
            consecutive_failures: 0,
            consecutive_successes: 0,
            avg_latency_ms: None,
            ..provider("groq")
        }];

        let merged = merge_runtime_state(current, loaded);
        let groq = &merged[0];
        assert_eq!(groq.requests_per_day, Some(2_000));
        assert_eq!(groq.requests_used_minute, 7);
        assert_eq!(groq.requests_used_day, 42);
        assert!(groq.in_cooldown);
        assert_eq!(groq.error_count, 3);
        assert_eq!(groq.consecutive_successes, 4);
        assert_eq!(groq.avg_latency_ms, Some(900));
        assert_eq!(
            groq.intelligence_refreshed_at_utc.as_deref(),
            Some("2026-03-30T04:00:00Z")
        );
        assert_eq!(groq.probe_model.as_deref(), Some("llama-3.1-8b-instant"));
        assert_eq!(groq.probe_profile.as_deref(), Some("low_latency_terse"));
    }

    #[test]
    fn persisted_runtime_snapshot_rehydrates_probe_and_model_memory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("provider_runtime_state.json");
        let current = vec![ProviderState {
            intelligence_refreshed_at_utc: Some("2026-03-30T04:00:00Z".to_string()),
            probe_model: Some("nvidia/nemotron-nano-9b-v2:free".to_string()),
            probe_profile: Some("low_latency_terse".to_string()),
            models: vec![crate::types::ModelState {
                id: "nvidia/nemotron-nano-9b-v2:free".to_string(),
                aliases: vec![],
                capable_tasks: vec!["chat".to_string()],
                context_window: 131_072,
                is_default: true,
                healthy: false,
                in_cooldown: true,
                cooldown_until_utc: Some("2026-03-30T05:00:00Z".to_string()),
                consecutive_failures: 3,
                consecutive_successes: 0,
                last_error: Some("catalog reconciliation: missing from live /models".to_string()),
                avg_latency_ms: Some(321),
                cost_per_million_tokens_in: None,
                cost_per_million_tokens_out: None,
                capabilities: Default::default(),
                streaming_validated: Some(false),
            }],
            ..provider("openrouter")
        }];
        persist_runtime_state_snapshot(&path, &current).expect("persist snapshot");

        let loaded = vec![ProviderState {
            models: vec![crate::types::ModelState {
                id: "nvidia/nemotron-nano-9b-v2:free".to_string(),
                aliases: vec![],
                capable_tasks: vec!["chat".to_string()],
                context_window: 131_072,
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
                capabilities: Default::default(),
                streaming_validated: None,
            }],
            ..provider("openrouter")
        }];

        let merged = merge_persisted_runtime_state(&path, loaded).expect("merge persisted");
        let provider = &merged[0];
        let model = &provider.models[0];
        assert_eq!(
            provider.probe_model.as_deref(),
            Some("nvidia/nemotron-nano-9b-v2:free")
        );
        assert_eq!(provider.probe_profile.as_deref(), Some("low_latency_terse"));
        assert!(!model.healthy);
        assert!(model.in_cooldown);
        assert_eq!(model.consecutive_failures, 3);
        assert_eq!(model.avg_latency_ms, Some(321));
        assert_eq!(model.streaming_validated, Some(false));
    }

    #[test]
    fn refresh_provider_windows_clears_expired_cooldown_and_stale_reservations() {
        let now = Utc::now();
        let mut providers = vec![ProviderState {
            in_cooldown: true,
            cooldown_until_utc: Some((now - Duration::seconds(5)).to_rfc3339()),
            active_connections: 2,
            last_reservation_utc: Some((now - Duration::seconds(1200)).to_rfc3339()),
            ..provider("local_fallback")
        }];

        refresh_provider_windows(&mut providers, now);

        let provider = &providers[0];
        assert!(!provider.in_cooldown);
        assert!(provider.cooldown_until_utc.is_none());
        assert_eq!(provider.active_connections, 0);
        assert!(provider.last_reservation_utc.is_none());
    }

    #[test]
    fn provider_unavailable_reason_reports_quota_exhaustion() {
        let now = Utc::now();
        let provider = ProviderState {
            requests_per_minute: Some(10),
            requests_used_minute: 10,
            minute_window_started_utc: Some(now.to_rfc3339()),
            ..provider("groq")
        };

        let reason = provider_unavailable_reason(&provider, "normal", false, now).expect("reason");

        assert_eq!(
            reason.get("reason").and_then(|value| value.as_str()),
            Some("minute_quota_exhausted")
        );
    }

    #[test]
    fn should_probe_enabled_local_provider_when_refresh_is_stale() {
        let now = Utc::now();
        let provider = ProviderState {
            access_tier: "local".to_string(),
            enabled: true,
            intelligence_refreshed_at_utc: Some((now - Duration::seconds(120)).to_rfc3339()),
            ..provider("edge_laptop")
        };

        assert!(should_probe_local_provider(&provider, now));
    }

    #[test]
    fn should_not_probe_recently_refreshed_local_provider() {
        let now = Utc::now();
        let provider = ProviderState {
            access_tier: "local".to_string(),
            enabled: true,
            intelligence_refreshed_at_utc: Some((now - Duration::seconds(5)).to_rfc3339()),
            ..provider("edge_laptop")
        };

        assert!(!should_probe_local_provider(&provider, now));
    }

    #[test]
    fn refresh_provider_windows_rolls_expired_quota_windows_forward() {
        let now = Utc::now();
        let mut providers = vec![ProviderState {
            requests_used_minute: 9,
            minute_window_started_utc: Some((now - Duration::seconds(61)).to_rfc3339()),
            requests_used_day: 99,
            day_window_started_utc: Some((now - Duration::seconds(86_405)).to_rfc3339()),
            ..provider("groq")
        }];

        refresh_provider_windows(&mut providers, now);

        let provider = &providers[0];
        assert_eq!(provider.requests_used_minute, 0);
        assert_eq!(provider.requests_used_day, 0);
        assert_eq!(
            provider.minute_window_started_utc.as_deref(),
            Some(now.to_rfc3339().as_str())
        );
        assert_eq!(
            provider.day_window_started_utc.as_deref(),
            Some(now.to_rfc3339().as_str())
        );
    }

    #[test]
    fn refresh_provider_windows_clears_invalid_reservation_timestamp() {
        let now = Utc::now();
        let mut providers = vec![ProviderState {
            active_connections: 3,
            last_reservation_utc: Some("not-a-timestamp".to_string()),
            ..provider("edge_backbone")
        }];

        refresh_provider_windows(&mut providers, now);

        let provider = &providers[0];
        assert_eq!(provider.active_connections, 0);
        assert!(provider.last_reservation_utc.is_none());
    }

    #[test]
    fn stale_reservation_ttl_uses_positive_env_override() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("ARDA_CHARON_RESERVATION_TTL_SECONDS", "30");
        assert_eq!(stale_reservation_ttl_seconds(), 30);
        std::env::set_var("ARDA_CHARON_RESERVATION_TTL_SECONDS", "0");
        assert_eq!(stale_reservation_ttl_seconds(), 900);
        std::env::remove_var("ARDA_CHARON_RESERVATION_TTL_SECONDS");
    }
}
