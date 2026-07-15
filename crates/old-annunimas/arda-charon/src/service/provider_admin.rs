use super::{
    classify_models_probe_status, decay_lane_fitness_snapshot, merge_latency, CharonService,
    LaneFitnessSnapshot, ProviderCapacityProbeRecord, ProviderState,
};
use arda_core::error::Result;
use chrono::Utc;
use serde_json::Value as JsonValue;
use std::fs;
use std::time::Duration as StdDuration;

impl CharonService {
    pub(super) fn read_lane_fitness_snapshot(&self) -> LaneFitnessSnapshot {
        let path = self.lane_fitness_path();
        let mut snapshot = fs::read_to_string(&path)
            .ok()
            .and_then(|content| serde_json::from_str::<LaneFitnessSnapshot>(&content).ok())
            .unwrap_or_else(|| LaneFitnessSnapshot {
                generated_at_utc: Utc::now().to_rfc3339(),
                lanes: std::collections::BTreeMap::new(),
            });
        if decay_lane_fitness_snapshot(&mut snapshot) {
            let _ = fs::write(
                &path,
                serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
            );
        }
        snapshot
    }

    pub(super) fn update_lane_fitness(
        &self,
        lane: &str,
        provider_id: &str,
        ok: bool,
        latency_ms: Option<u64>,
    ) -> Result<()> {
        let mut snapshot = self.read_lane_fitness_snapshot();
        let lane_entry = snapshot.lanes.entry(lane.to_string()).or_default();
        let provider_entry = lane_entry.entry(provider_id.to_string()).or_default();
        provider_entry.last_result_utc = Some(Utc::now().to_rfc3339());
        if ok {
            provider_entry.success_count += 1;
            provider_entry.avg_latency_ms =
                merge_latency(provider_entry.avg_latency_ms, latency_ms);
        } else {
            provider_entry.failure_count += 1;
        }
        snapshot.generated_at_utc = Utc::now().to_rfc3339();
        fs::write(
            self.lane_fitness_path(),
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string()) + "\n",
        )?;
        Ok(())
    }

    pub(super) async fn maybe_refresh_provider_capacity_probe(
        &self,
        provider: &ProviderState,
    ) -> Result<Option<ProviderCapacityProbeRecord>> {
        let now = Utc::now();
        if let Some(existing) = self
            .capacity_probe_cache
            .read()
            .await
            .get(&provider.id)
            .cloned()
        {
            if chrono::DateTime::parse_from_rfc3339(&existing.next_refresh_at_utc)
                .ok()
                .is_some_and(|next| next.with_timezone(&Utc) > now)
            {
                return Ok(Some(existing));
            }
        }

        let Some(api_key_env) = provider.api_key_env.as_deref() else {
            return Ok(None);
        };
        let Ok(api_key) = std::env::var(api_key_env) else {
            return Ok(None);
        };

        let probe = match provider.id.as_str() {
            "openrouter" | "openrouter_free" => {
                probe_openrouter_key_capacity(&provider.id, &api_key).await
            }
            "opencode" | "openai" | "google" | "mistral" | "zai" | "nvidia" | "cerebras"
            | "groq" => {
                let Some(base_url) = provider.base_url.as_deref() else {
                    return Ok(None);
                };
                probe_openai_models_capacity(&provider.id, base_url, &api_key).await
            }
            _ => return Ok(None),
        };
        self.apply_capacity_probe_record(provider, &probe).await?;
        Ok(Some(probe))
    }

    pub(super) async fn apply_capacity_probe_record(
        &self,
        provider: &ProviderState,
        probe: &ProviderCapacityProbeRecord,
    ) -> Result<()> {
        {
            let linked_ids = {
                let providers = self.providers.read().await;
                providers
                    .iter()
                    .filter(|candidate| candidate.api_key_env == provider.api_key_env)
                    .map(|candidate| candidate.id.clone())
                    .collect::<Vec<_>>()
            };
            let mut cache = self.capacity_probe_cache.write().await;
            for provider_id in linked_ids {
                let mut linked_probe = probe.clone();
                linked_probe.provider_id = provider_id.clone();
                cache.insert(provider_id, linked_probe);
            }
        }

        if !probe.blocked {
            return Ok(());
        }

        let cooldown_seconds = chrono::DateTime::parse_from_rfc3339(&probe.next_refresh_at_utc)
            .ok()
            .map(|next| {
                (next.with_timezone(&Utc) - Utc::now())
                    .num_seconds()
                    .max(60)
            })
            .unwrap_or(300);
        let mut providers = self.providers.write().await;
        for candidate in providers
            .iter_mut()
            .filter(|candidate| candidate.api_key_env == provider.api_key_env)
        {
            candidate.in_cooldown = true;
            candidate.cooldown_backoff_seconds = cooldown_seconds as u64;
            candidate.cooldown_until_utc =
                Some((Utc::now() + chrono::Duration::seconds(cooldown_seconds)).to_rfc3339());
            candidate.last_error = Some(format!(
                "[capacity_probe:{}] {}",
                probe.source, probe.reason
            ));
        }
        Ok(())
    }
}

pub(super) async fn probe_openrouter_key_capacity(
    provider_id: &str,
    api_key: &str,
) -> ProviderCapacityProbeRecord {
    let checked_at = Utc::now();
    let url = "https://openrouter.ai/api/v1/key";
    let client = match reqwest::Client::builder()
        .timeout(StdDuration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return ProviderCapacityProbeRecord {
                provider_id: provider_id.to_string(),
                source: "openrouter_key".to_string(),
                state: "probe_error".to_string(),
                reason: format!("failed to build probe client: {err}"),
                blocked: false,
                checked_at_utc: checked_at.to_rfc3339(),
                next_refresh_at_utc: (checked_at + chrono::Duration::minutes(2)).to_rfc3339(),
                meta: serde_json::json!({}),
            };
        }
    };

    let response = match client.get(url).bearer_auth(api_key).send().await {
        Ok(response) => response,
        Err(err) => {
            return ProviderCapacityProbeRecord {
                provider_id: provider_id.to_string(),
                source: "openrouter_key".to_string(),
                state: "probe_error".to_string(),
                reason: format!("capacity probe transport error: {err}"),
                blocked: false,
                checked_at_utc: checked_at.to_rfc3339(),
                next_refresh_at_utc: (checked_at + chrono::Duration::minutes(2)).to_rfc3339(),
                meta: serde_json::json!({}),
            };
        }
    };

    let status = response.status().as_u16();
    let parsed = response
        .json::<JsonValue>()
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    let data = parsed
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let limit_remaining = data.get("limit_remaining").and_then(JsonValue::as_f64);
    let limit = data.get("limit").and_then(JsonValue::as_f64);
    let usage = data.get("usage").and_then(JsonValue::as_f64);
    let limit_reset = data
        .get("limit_reset")
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");

    let (state, reason, blocked, ttl_minutes) = match status {
        200 => {
            if limit_remaining.is_some_and(|remaining| remaining <= 0.0) {
                (
                    "spend_blocked".to_string(),
                    format!("OpenRouter key has no remaining credit (limit_reset={limit_reset})"),
                    true,
                    10,
                )
            } else {
                (
                    "ready".to_string(),
                    format!(
                        "OpenRouter key capacity available (remaining={}, reset={})",
                        limit_remaining
                            .map(|value| format!("{value:.2}"))
                            .unwrap_or_else(|| "unknown".to_string()),
                        limit_reset
                    ),
                    false,
                    5,
                )
            }
        }
        401 | 403 => (
            "auth_failed".to_string(),
            "OpenRouter key probe was unauthorized".to_string(),
            true,
            15,
        ),
        429 => (
            "rate_limited".to_string(),
            "OpenRouter key probe hit rate limits".to_string(),
            true,
            5,
        ),
        _ => (
            "probe_error".to_string(),
            format!("OpenRouter key probe returned HTTP {status}"),
            false,
            2,
        ),
    };

    ProviderCapacityProbeRecord {
        provider_id: provider_id.to_string(),
        source: "openrouter_key".to_string(),
        state,
        reason,
        blocked,
        checked_at_utc: checked_at.to_rfc3339(),
        next_refresh_at_utc: (checked_at + chrono::Duration::minutes(ttl_minutes)).to_rfc3339(),
        meta: serde_json::json!({
            "status": status,
            "limit": limit,
            "limit_remaining": limit_remaining,
            "usage": usage,
            "limit_reset": limit_reset,
            "raw": data,
        }),
    }
}

pub(super) async fn probe_openai_models_capacity(
    provider_id: &str,
    base_url: &str,
    api_key: &str,
) -> ProviderCapacityProbeRecord {
    let checked_at = Utc::now();
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(StdDuration::from_secs(5))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            return ProviderCapacityProbeRecord {
                provider_id: provider_id.to_string(),
                source: "models_probe".to_string(),
                state: "probe_error".to_string(),
                reason: format!("failed to build probe client: {err}"),
                blocked: false,
                checked_at_utc: checked_at.to_rfc3339(),
                next_refresh_at_utc: (checked_at + chrono::Duration::minutes(2)).to_rfc3339(),
                meta: serde_json::json!({ "url": url }),
            };
        }
    };

    let response = match client.get(&url).bearer_auth(api_key).send().await {
        Ok(response) => response,
        Err(err) => {
            return ProviderCapacityProbeRecord {
                provider_id: provider_id.to_string(),
                source: "models_probe".to_string(),
                state: "probe_error".to_string(),
                reason: format!("capacity probe transport error: {err}"),
                blocked: false,
                checked_at_utc: checked_at.to_rfc3339(),
                next_refresh_at_utc: (checked_at + chrono::Duration::minutes(2)).to_rfc3339(),
                meta: serde_json::json!({ "url": url }),
            };
        }
    };

    let status = response.status().as_u16();
    let raw_text = response.text().await.unwrap_or_default();
    let parsed =
        serde_json::from_str::<JsonValue>(&raw_text).unwrap_or_else(|_| serde_json::json!({}));
    let model_count = parsed
        .get("data")
        .and_then(JsonValue::as_array)
        .map(std::vec::Vec::len);
    let (state, reason, blocked, ttl_minutes) =
        classify_models_probe_status(provider_id, status, &raw_text, model_count);

    ProviderCapacityProbeRecord {
        provider_id: provider_id.to_string(),
        source: "models_probe".to_string(),
        state,
        reason,
        blocked,
        checked_at_utc: checked_at.to_rfc3339(),
        next_refresh_at_utc: (checked_at + chrono::Duration::minutes(ttl_minutes)).to_rfc3339(),
        meta: serde_json::json!({
            "status": status,
            "url": url,
            "model_count": model_count,
        }),
    }
}
