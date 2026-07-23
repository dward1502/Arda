use crate::adaptive::service::types::ModelState;
use crate::adaptive::service::types::{CharonService, ProviderState};
use arda_core::error::Result;
use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use std::collections::BTreeSet;
use std::time::Duration as StdDuration;

pub(super) fn spawn(service: CharonService) {
    if !catalog_reconciliation_enabled() {
        return;
    }
    tokio::spawn(async move {
        tokio::time::sleep(StdDuration::from_secs(initial_delay_seconds())).await;
        let mut interval = tokio::time::interval(StdDuration::from_secs(interval_seconds()));
        loop {
            interval.tick().await;
            if let Err(err) = service.reconcile_provider_catalogs().await {
                tracing::warn!(error = %err, "CHARON provider catalog reconciliation failed");
            }
        }
    });
}

impl CharonService {
    pub async fn reconcile_provider_catalogs(&self) -> Result<JsonValue> {
        let providers = self.providers().await;
        let mut receipts = Vec::new();
        for provider in providers.iter() {
            let receipt = self.reconcile_one_provider_catalog(provider).await?;
            receipts.push(receipt);
        }
        let summary = json!({
            "ok": true,
            "generated_at_utc": Utc::now().to_rfc3339(),
            "providers_checked": receipts.len(),
            "providers_with_live_catalog": receipts.iter().filter(|receipt| receipt.get("live_catalog").and_then(JsonValue::as_bool) == Some(true)).count(),
            "receipts": receipts,
        });
        self.append_state_event(
            "provider_catalog_reconciliation_complete",
            json!({
                "providers_checked": summary["providers_checked"],
                "providers_with_live_catalog": summary["providers_with_live_catalog"],
            }),
        )?;
        Ok(summary)
    }

    async fn reconcile_one_provider_catalog(&self, provider: &ProviderState) -> Result<JsonValue> {
        let checked_at = Utc::now();
        let mut receipt = json!({
            "provider_id": provider.id,
            "checked_at_utc": checked_at.to_rfc3339(),
            "live_catalog": false,
            "configured_model_count": provider.models.len(),
            "live_model_count": null,
            "stale_configured_models": [],
            "new_live_models": [],
            "selected_probe_model": null,
            "selected_probe_profile": null,
            "selected_default_model": null,
            "error": null,
        });

        let live_ids = match fetch_live_model_ids(&provider).await {
            Ok(Some(ids)) => ids,
            Ok(None) => {
                receipt["error"] = json!("provider does not expose a reconcilable /models catalog");
                if let Some((model, profile)) = self
                    .persist_configured_catalog_probe_choice(&provider, checked_at)
                    .await?
                {
                    receipt["selected_probe_model"] = json!(model);
                    receipt["selected_probe_profile"] = json!(profile);
                    receipt["probe_model_source"] = json!("configured_catalog");
                }
                self.append_state_event("provider_catalog_reconciled", receipt.clone())?;
                return Ok(receipt);
            }
            Err(err) => {
                receipt["error"] = json!(err);
                if let Some((model, profile)) = self
                    .persist_configured_catalog_probe_choice(&provider, checked_at)
                    .await?
                {
                    receipt["selected_probe_model"] = json!(model);
                    receipt["selected_probe_profile"] = json!(profile);
                    receipt["probe_model_source"] = json!("configured_catalog_after_fetch_error");
                }
                self.append_state_event("provider_catalog_reconciled", receipt.clone())?;
                return Ok(receipt);
            }
        };

        let configured_ids = provider
            .models
            .iter()
            .map(|model| model.id.clone())
            .collect::<BTreeSet<_>>();
        let live_set = live_ids.iter().cloned().collect::<BTreeSet<_>>();
        let stale = configured_ids
            .iter()
            .filter(|model_id| !model_is_live(model_id, &live_set))
            .cloned()
            .collect::<Vec<_>>();
        let new_live = live_set
            .iter()
            .filter(|live_id| {
                !configured_ids
                    .iter()
                    .any(|model_id| model_ids_equivalent(model_id, live_id))
            })
            .take(25)
            .cloned()
            .collect::<Vec<_>>();
        let probe_choice = select_probe_model(&provider, &live_set);
        let default_choice = select_default_replacement(&provider, &live_set);

        let mut mutated = false;
        let mut snapshot = None;
        {
            let mut providers = self.providers.write().await;
            if let Some(current) = providers.iter_mut().find(|item| item.id == provider.id) {
                current.intelligence_refreshed_at_utc = Some(checked_at.to_rfc3339());
                current.probe_model = probe_choice.as_ref().map(|(model, _)| model.clone());
                current.probe_profile = probe_choice.as_ref().map(|(_, profile)| profile.clone());
                mutated = true;

                if !live_set.is_empty() {
                    for model in &mut current.models {
                        if !model_is_live(&model.id, &live_set) {
                            model.healthy = false;
                            model.in_cooldown = true;
                            model.last_error = Some(
                                "catalog reconciliation: missing from live /models".to_string(),
                            );
                        } else {
                            clear_catalog_missing_quarantine_if_live(model, &live_set);
                        }
                    }
                    if let Some(default_model) = default_choice.as_deref() {
                        for model in &mut current.models {
                            model.is_default = model.id == default_model;
                            if model.id == default_model {
                                model.healthy = true;
                                model.in_cooldown = false;
                                model.last_error = None;
                            }
                        }
                    }
                }
                snapshot = Some(providers.clone());
            }
        }
        if mutated {
            if let Some(providers) = snapshot.as_deref() {
                self.persist_provider_runtime_state_snapshot(providers)
                    .await?;
            }
        }

        receipt["live_catalog"] = json!(true);
        receipt["live_model_count"] = json!(live_ids.len());
        receipt["stale_configured_models"] = json!(stale);
        receipt["new_live_models"] = json!(new_live);
        if let Some((model, profile)) = probe_choice {
            receipt["selected_probe_model"] = json!(model);
            receipt["selected_probe_profile"] = json!(profile);
        }
        if let Some(default_model) = default_choice {
            receipt["selected_default_model"] = json!(default_model);
        }
        self.append_state_event("provider_catalog_reconciled", receipt.clone())?;
        Ok(receipt)
    }

    async fn persist_configured_catalog_probe_choice(
        &self,
        provider: &ProviderState,
        checked_at: chrono::DateTime<Utc>,
    ) -> Result<Option<(String, String)>> {
        let configured_set = provider
            .models
            .iter()
            .map(|model| model.id.clone())
            .collect::<BTreeSet<_>>();
        let Some((model, profile)) = select_probe_model(provider, &configured_set) else {
            return Ok(None);
        };

        let mut snapshot = None;
        {
            let mut providers = self.providers.write().await;
            if let Some(current) = providers.iter_mut().find(|item| item.id == provider.id) {
                current.intelligence_refreshed_at_utc = Some(checked_at.to_rfc3339());
                current.probe_model = Some(model.clone());
                current.probe_profile = Some(profile.clone());
                snapshot = Some(providers.clone());
            }
        }
        if let Some(providers) = snapshot.as_deref() {
            self.persist_provider_runtime_state_snapshot(providers)
                .await?;
        }

        Ok(Some((model, profile)))
    }
}

async fn fetch_live_model_ids(
    provider: &ProviderState,
) -> std::result::Result<Option<Vec<String>>, String> {
    if provider.driver != "openai_compat" {
        return Ok(None);
    }
    let Some(base_url) = provider.base_url.as_deref() else {
        return Ok(None);
    };
    let Some(api_key_env) = provider.api_key_env.as_deref() else {
        return Ok(None);
    };
    let api_key = std::env::var(api_key_env).map_err(|_| format!("{api_key_env} not set"))?;
    if api_key.trim().is_empty() {
        return Ok(None);
    }
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(8))
        .build()
        .map_err(|err| format!("failed to build catalog client: {err}"))?;
    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|err| format!("catalog fetch transport error: {err}"))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "catalog fetch returned HTTP {}: {}",
            status.as_u16(),
            text
        ));
    }
    let parsed = serde_json::from_str::<JsonValue>(&text)
        .map_err(|err| format!("catalog response was not JSON: {err}"))?;
    let ids = parsed
        .get("data")
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get("id").and_then(JsonValue::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(Some(ids))
}

fn select_probe_model(
    provider: &ProviderState,
    live_set: &BTreeSet<String>,
) -> Option<(String, String)> {
    if let Some(existing) = provider.probe_model.as_deref() {
        if model_is_live(existing, live_set) {
            return Some((existing.to_string(), "configured_probe_model".to_string()));
        }
    }

    provider
        .models
        .iter()
        .filter(|model| model_is_live(&model.id, live_set))
        .filter(|model| model.capable_tasks.iter().any(|task| task == "chat"))
        .max_by_key(|model| probe_model_score(&model.id, model.avg_latency_ms))
        .map(|model| (model.id.clone(), "low_latency_terse".to_string()))
}

fn select_default_replacement(
    provider: &ProviderState,
    live_set: &BTreeSet<String>,
) -> Option<String> {
    let default_still_live = provider
        .models
        .iter()
        .any(|model| model.is_default && model_is_live(&model.id, live_set));
    if default_still_live {
        return None;
    }
    provider
        .models
        .iter()
        .filter(|model| model_is_live(&model.id, live_set))
        .filter(|model| default_replacement_candidate(provider, model))
        .filter(|model| model.capable_tasks.iter().any(|task| task == "chat"))
        .max_by_key(|model| {
            (
                usize::from(model.capable_tasks.iter().any(|task| task == "code")),
                model.context_window,
            )
        })
        .map(|model| model.id.clone())
}

fn default_replacement_candidate(provider: &ProviderState, model: &ModelState) -> bool {
    if provider.id == "openrouter" {
        return model.id == "openrouter/free" || model.id.ends_with(":free");
    }
    true
}

fn model_is_live(model_id: &str, live_set: &BTreeSet<String>) -> bool {
    live_set.is_empty()
        || live_set
            .iter()
            .any(|live_id| model_ids_equivalent(model_id, live_id))
}

fn model_ids_equivalent(configured_id: &str, live_id: &str) -> bool {
    configured_id == live_id
        || live_id
            .strip_prefix("models/")
            .is_some_and(|stripped| stripped == configured_id)
        || configured_id
            .strip_prefix("models/")
            .is_some_and(|stripped| stripped == live_id)
}

fn clear_catalog_missing_quarantine_if_live(model: &mut ModelState, live_set: &BTreeSet<String>) {
    if model_is_live(&model.id, live_set)
        && model
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("missing from live /models"))
    {
        model.healthy = true;
        model.in_cooldown = false;
        model.last_error = None;
    }
}

fn probe_model_score(model_id: &str, latency_ms: Option<u64>) -> i64 {
    let id = model_id.to_ascii_lowercase();
    let mut score = 0_i64;
    for needle in [
        "flash", "instant", "nano", "haiku", "mini", "8b", "7b", "free",
    ] {
        if id.contains(needle) {
            score += 20;
        }
    }
    for needle in [
        "reasoning",
        "coder",
        "ultra",
        "large",
        "120b",
        "405b",
        "opus",
    ] {
        if id.contains(needle) {
            score -= 18;
        }
    }
    if let Some(latency) = latency_ms {
        score -= (latency / 250) as i64;
    }
    score
}

fn catalog_reconciliation_enabled() -> bool {
    std::env::var("ARDA_MANWE_CATALOG_RECONCILE_ENABLED")
        .map(|value| !matches!(value.to_ascii_lowercase().as_str(), "0" | "false" | "no"))
        .unwrap_or(true)
}

fn initial_delay_seconds() -> u64 {
    std::env::var("ARDA_MANWE_CATALOG_RECONCILE_INITIAL_DELAY_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(300)
}

fn interval_seconds() -> u64 {
    std::env::var("ARDA_MANWE_CATALOG_RECONCILE_INTERVAL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(21_600)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adaptive::service::types::{ModelCapabilities, ModelState};
    use tempfile::tempdir;

    fn model(id: &str, is_default: bool) -> ModelState {
        ModelState {
            id: id.to_string(),
            aliases: vec![],
            capable_tasks: vec!["chat".to_string()],
            context_window: 128_000,
            is_default,
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_error: None,
            avg_latency_ms: None,
            cost_per_million_tokens_in: None,
            cost_per_million_tokens_out: None,
            capabilities: ModelCapabilities::default(),
            streaming_validated: None,
        }
    }

    fn provider(models: Vec<ModelState>) -> ProviderState {
        ProviderState {
            id: "openrouter".to_string(),
            name: "OpenRouter".to_string(),
            base_url: Some("https://openrouter.ai/api/v1".to_string()),
            api_key_env: Some("OPENROUTER_API_KEY".to_string()),
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
            cooldown_backoff_seconds: 0,
            requests_per_minute: None,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: None,
            requests_used_day: 0,
            day_window_started_utc: None,
            models,
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
    fn select_probe_model_prefers_existing_live_probe_choice() {
        let mut provider = provider(vec![
            model("nvidia/nemotron-nano-9b-v2:free", false),
            model("openai/gpt-oss-120b", true),
        ]);
        provider.probe_model = Some("openai/gpt-oss-120b".to_string());
        let live_set = BTreeSet::from([
            "nvidia/nemotron-nano-9b-v2:free".to_string(),
            "openai/gpt-oss-120b".to_string(),
        ]);

        let selected = select_probe_model(&provider, &live_set).expect("probe choice");

        assert_eq!(selected.0, "openai/gpt-oss-120b");
        assert_eq!(selected.1, "configured_probe_model");
    }

    #[test]
    fn select_probe_model_prefers_terse_free_model_over_large_reasoner() {
        let provider = provider(vec![
            model("provider/reasoning-ultra-120b", true),
            model("nvidia/nemotron-nano-9b-v2:free", false),
        ]);
        let live_set = BTreeSet::from([
            "provider/reasoning-ultra-120b".to_string(),
            "nvidia/nemotron-nano-9b-v2:free".to_string(),
        ]);

        let selected = select_probe_model(&provider, &live_set).expect("probe choice");

        assert_eq!(selected.0, "nvidia/nemotron-nano-9b-v2:free");
        assert_eq!(selected.1, "low_latency_terse");
    }

    #[test]
    fn model_is_live_accepts_google_models_prefix() {
        let live_set = BTreeSet::from(["models/gemini-2.5-flash".to_string()]);

        assert!(model_is_live("gemini-2.5-flash", &live_set));
        assert!(!model_is_live("gemini-2.5-pro", &live_set));
    }

    #[test]
    fn live_catalog_match_clears_previous_catalog_missing_quarantine() {
        let live_set = BTreeSet::from(["models/gemini-2.5-flash".to_string()]);
        let mut model = model("gemini-2.5-flash", true);
        model.healthy = false;
        model.in_cooldown = true;
        model.last_error = Some("catalog reconciliation: missing from live /models".to_string());

        clear_catalog_missing_quarantine_if_live(&mut model, &live_set);

        assert!(model.healthy);
        assert!(!model.in_cooldown);
        assert!(model.last_error.is_none());
    }

    #[tokio::test]
    async fn configured_catalog_reconciliation_persists_probe_choice_without_live_catalog() {
        let dir = tempdir().expect("tempdir");
        let service = CharonService::new(dir.path());
        let provider = provider(vec![
            model("provider/reasoning-ultra-120b", true),
            model("llama-3.1-8b-instant", false),
        ]);

        {
            let mut providers = service.providers.write().await;
            providers.clear();
            providers.push(provider.clone());
        }

        let selected = service
            .persist_configured_catalog_probe_choice(&provider, Utc::now())
            .await
            .expect("persisted")
            .expect("selected");

        assert_eq!(selected.0, "llama-3.1-8b-instant");
        assert_eq!(selected.1, "low_latency_terse");
        let providers = service.providers().await;
        assert_eq!(
            providers[0].probe_model.as_deref(),
            Some("llama-3.1-8b-instant")
        );
        assert_eq!(
            providers[0].probe_profile.as_deref(),
            Some("low_latency_terse")
        );
    }

    #[test]
    fn select_default_replacement_only_changes_missing_default() {
        let mut provider = provider(vec![
            model("stale-default", true),
            model("live-small", false),
            model("live-large", false),
        ]);
        provider.id = "generic".to_string();
        let live_set = BTreeSet::from(["live-small".to_string(), "live-large".to_string()]);

        let selected = select_default_replacement(&provider, &live_set).expect("replacement");

        assert_eq!(selected, "live-large");
    }

    #[test]
    fn select_default_replacement_skips_openrouter_auto() {
        let mut provider = provider(vec![
            model("stale-default", true),
            model("openrouter/auto", false),
            model("nvidia/nemotron-3-super-120b-a12b:free", false),
        ]);
        provider.id = "openrouter".to_string();
        let live_set = BTreeSet::from([
            "openrouter/auto".to_string(),
            "nvidia/nemotron-3-super-120b-a12b:free".to_string(),
        ]);

        let selected = select_default_replacement(&provider, &live_set).expect("replacement");

        assert_eq!(selected, "nvidia/nemotron-3-super-120b-a12b:free");
    }
}
