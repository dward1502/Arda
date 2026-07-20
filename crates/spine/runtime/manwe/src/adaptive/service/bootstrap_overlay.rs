use crate::adaptive::service::bootstrap_defaults::{default_providers, env_key_present};
use crate::adaptive::service::types::{ModelCapabilities, ModelState, ProviderState};
use arda_core::error::{ArdaError, Result};
use chrono::{Duration, Utc};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

pub(super) fn default_provider_intelligence_path() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_PROVIDER_INTELLIGENCE_PATH") {
        return PathBuf::from(path);
    }
    crate::adaptive::service::paths::arda_root().join("core/state/provider_intelligence.json")
}

pub(super) fn default_tool_fit_model_intelligence_path() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_TOOL_FIT_MODEL_INTELLIGENCE_PATH") {
        return PathBuf::from(path);
    }
    super::paths::arda_root().join("core/state/tool_fit_model_intelligence.json")
}

#[derive(Debug, Deserialize)]
struct ProviderConfigFile {
    #[serde(default)]
    provider: Vec<ProviderConfig>,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct FleetBootstrapFile {
    #[serde(default)]
    pub(super) generated_at_utc: Option<String>,
    #[serde(default)]
    pub(super) targets: Vec<FleetBootstrapTarget>,
}

#[derive(Debug, Deserialize)]
pub(super) struct FleetBootstrapTarget {
    #[serde(default)]
    target_id: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    charon_provider_id: Option<String>,
    #[serde(default)]
    configured_base_url: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    has_live_endpoint: Option<bool>,
    #[serde(default)]
    expected_models: Vec<String>,
    #[serde(default)]
    observed_models: Vec<String>,
    #[serde(default)]
    intentional_offline: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ProviderConfig {
    id: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    healthy: Option<bool>,
    #[serde(default)]
    api_key_env: Option<String>,
    #[serde(default)]
    access_tier: Option<String>,
    #[serde(default)]
    quality_band: Option<String>,
    #[serde(default)]
    probe_model: Option<String>,
    #[serde(default)]
    probe_profile: Option<String>,
    #[serde(default)]
    requests_per_minute: Option<u64>,
    #[serde(default)]
    requests_per_day: Option<u64>,
    #[serde(default)]
    supports_tools: Option<bool>,
    #[serde(default)]
    supports_structured_output: Option<bool>,
    #[serde(default)]
    driver: Option<String>,
    #[serde(default)]
    hermes_bin: Option<String>,
    #[serde(default)]
    hermes_provider: Option<String>,
    #[serde(default)]
    hermes_toolsets: Option<String>,
    #[serde(default)]
    model: Vec<ModelConfig>,
}

#[derive(Debug, Deserialize)]
struct ModelConfig {
    id: String,
    #[serde(default)]
    capable_tasks: Vec<String>,
    #[serde(default)]
    context_window: Option<usize>,
    #[serde(default)]
    is_default: Option<bool>,
    #[serde(default)]
    cost_per_million_tokens_in: Option<f64>,
    #[serde(default)]
    cost_per_million_tokens_out: Option<f64>,
    #[serde(default)]
    capabilities: ModelCapabilities,
    #[serde(default)]
    streaming_validated: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct ProviderIntelligenceFile {
    #[serde(default)]
    providers: BTreeMap<String, ProviderIntelligenceOverlay>,
}

#[derive(Debug, Deserialize, Default)]
struct ProviderIntelligenceOverlay {
    #[serde(default)]
    access_tier: Option<String>,
    #[serde(default)]
    quality_band: Option<String>,
    #[serde(default)]
    requests_per_minute: Option<u64>,
    #[serde(default)]
    requests_per_day: Option<u64>,
    #[serde(default)]
    requests_used_minute: Option<u64>,
    #[serde(default)]
    requests_used_day: Option<u64>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    healthy: Option<bool>,
    #[serde(default)]
    refreshed_at_utc: Option<String>,
    #[serde(default)]
    models: Vec<ModelConfig>,
    #[serde(default)]
    metadata: ProviderIntelligenceMetadata,
}

#[derive(Debug, Deserialize, Default)]
struct ProviderIntelligenceMetadata {
    #[serde(default)]
    stale_configured_models: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ToolFitModelIntelligenceFile {
    #[serde(default)]
    models: Vec<ToolFitModelRecommendation>,
}

#[derive(Debug, Deserialize, Default)]
struct ToolFitModelRecommendation {
    #[serde(default)]
    provider_id: String,
    #[serde(default)]
    model_id: String,
    #[serde(default)]
    recommendation: String,
    #[serde(default)]
    attempts: u64,
    #[serde(default)]
    success_rate: Option<f64>,
}

pub(super) fn load_providers_from_config(
    path: &Path,
    bootstrap_path: &Path,
) -> Result<Vec<ProviderState>> {
    if !path.exists() {
        return Ok(default_providers());
    }
    let raw = fs::read_to_string(path)?;
    let cfg: ProviderConfigFile = toml::from_str(&raw).map_err(|e| ArdaError::Agent {
        agent: "charon".to_string(),
        message: format!("failed to parse provider config {}: {e}", path.display()),
    })?;
    if cfg.provider.is_empty() {
        return Err(ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "provider config {} has no [[provider]] blocks",
                path.display()
            ),
        });
    }
    let mut configured = Vec::new();
    for provider in cfg.provider {
        if provider.id.trim().is_empty() {
            continue;
        }
        let mut models = provider
            .model
            .into_iter()
            .filter(|m| !m.id.trim().is_empty())
            .map(|m| ModelState {
                aliases: vec![],
                id: m.id,
                capable_tasks: if m.capable_tasks.is_empty() {
                    vec!["chat".to_string()]
                } else {
                    m.capable_tasks
                },
                context_window: m.context_window.unwrap_or(8192),
                is_default: m.is_default.unwrap_or(false),
                healthy: true,
                in_cooldown: false,
                cooldown_until_utc: None,
                consecutive_failures: 0,
                consecutive_successes: 0,
                last_error: None,
                avg_latency_ms: None,
                cost_per_million_tokens_in: m.cost_per_million_tokens_in,
                cost_per_million_tokens_out: m.cost_per_million_tokens_out,
                capabilities: m.capabilities,
                streaming_validated: m.streaming_validated,
            })
            .collect::<Vec<_>>();
        if models.is_empty() {
            models.push(ModelState {
                aliases: vec![],
                id: "default".to_string(),
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
                capabilities: ModelCapabilities::default(),
                streaming_validated: None,
            });
        } else if !models.iter().any(|m| m.is_default) {
            if let Some(first) = models.first_mut() {
                first.is_default = true;
            }
        }
        let has_api_key = provider
            .api_key_env
            .as_deref()
            .map(env_key_present)
            .unwrap_or(true);
        if let Some(base) = provider.base_url.as_deref() {
            warn_if_base_url_suspicious(&provider.id, provider.driver.as_deref(), base);
        }
        if provider.healthy.is_some() {
            tracing::warn!(
                provider_id = %provider.id,
                "the `healthy` field in charon.providers.toml is deprecated and ignored; the runtime probe owns provider health. Remove the field from the [[provider]] block."
            );
        }
        configured.push(ProviderState {
            id: provider.id.clone(),
            name: provider.name.unwrap_or(provider.id),
            base_url: provider.base_url,
            api_key_env: provider.api_key_env.clone(),
            access_tier: provider.access_tier.unwrap_or_else(|| "mixed".to_string()),
            quality_band: provider
                .quality_band
                .unwrap_or_else(|| "medium".to_string()),
            intelligence_refreshed_at_utc: None,
            probe_model: provider.probe_model,
            probe_profile: provider.probe_profile,
            enabled: provider.enabled.unwrap_or(true),
            has_api_key,
            // Always start optimistic; the runtime probe corrects within seconds.
            // The static config no longer drives this — see deprecation warning above.
            healthy: true,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: provider.requests_per_minute,
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: provider.requests_per_day,
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
            supports_tools: provider.supports_tools.unwrap_or(true),
            supports_structured_output: provider.supports_structured_output.unwrap_or(true),
            driver: provider
                .driver
                .unwrap_or_else(|| "openai_compat".to_string()),
            hermes_bin: provider.hermes_bin,
            hermes_provider: provider.hermes_provider,
            hermes_toolsets: provider.hermes_toolsets,
        });
    }
    if configured.is_empty() {
        return Err(ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "provider config {} produced no valid providers",
                path.display()
            ),
        });
    }
    warn_on_base_url_collisions(&configured);
    let merged = merge_with_default_providers(configured);
    let merged = apply_fleet_bootstrap_overlay(merged, bootstrap_path)?;
    let merged =
        apply_provider_intelligence_overlay(merged, &default_provider_intelligence_path())?;
    apply_tool_fit_model_intelligence_overlay(merged, &default_tool_fit_model_intelligence_path())
}

pub(super) fn merge_with_default_providers(configured: Vec<ProviderState>) -> Vec<ProviderState> {
    let mut seen = BTreeSet::new();
    for provider in &configured {
        seen.insert(provider.id.clone());
    }

    let mut merged = configured;
    for provider in default_providers() {
        if seen.insert(provider.id.clone()) {
            merged.push(provider);
        }
    }
    merged
}

fn apply_fleet_bootstrap_overlay(
    mut providers: Vec<ProviderState>,
    bootstrap_path: &Path,
) -> Result<Vec<ProviderState>> {
    let Some(bootstrap) = load_fleet_bootstrap_state(bootstrap_path)? else {
        return Ok(providers);
    };
    let bootstrap_fresh = fleet_bootstrap_is_fresh(&bootstrap);

    for provider in &mut providers {
        let bootstrap_target = bootstrap.targets.iter().find(|target| {
            target
                .charon_provider_id
                .as_deref()
                .is_some_and(|id| id == provider.id)
                || target
                    .configured_base_url
                    .as_deref()
                    .zip(provider.base_url.as_deref())
                    .is_some_and(|(left, right)| normalize_url(left) == normalize_url(right))
        });

        let Some(target) = bootstrap_target else {
            continue;
        };

        let intentional_offline = target.intentional_offline.unwrap_or(false);
        let has_live_endpoint = target.has_live_endpoint.unwrap_or(false);
        let observed_models = target.observed_models.clone();
        let expected_models = target.expected_models.clone();
        let status = target.status.as_deref().unwrap_or("unknown");
        let bootstrap_healthy =
            has_live_endpoint && matches!(status, "online" | "degraded") && !intentional_offline;

        if bootstrap_fresh {
            if let Some(base_url) = target.configured_base_url.clone() {
                provider.base_url = Some(base_url);
            }
            if let Some(display_name) = target.display_name.clone() {
                provider.name = display_name;
            }
        }
        if bootstrap_fresh {
            if intentional_offline {
                provider.enabled = false;
                provider.healthy = false;
                provider.last_error = Some(format!(
                    "fleet bootstrap marked provider unavailable (status={status}, target_id={})",
                    target.target_id.as_deref().unwrap_or("unknown")
                ));
            } else if bootstrap_healthy {
                provider.healthy = true;
                provider.last_error = None;
            }
        }

        if !observed_models.is_empty() {
            let configured_models = provider.models.clone();
            provider.models = observed_models
                .iter()
                .enumerate()
                .map(|(index, model_id)| ModelState {
                    aliases: vec![],
                    id: model_id.clone(),
                    capable_tasks: infer_tasks_for_observed_model(
                        &provider.id,
                        model_id,
                        &configured_models,
                    ),
                    context_window: find_configured_model(model_id, &configured_models)
                        .map(|model| model.context_window)
                        .or_else(|| configured_models.first().map(|model| model.context_window))
                        .unwrap_or(8192),
                    is_default: index == 0,
                    healthy: true,
                    in_cooldown: false,
                    cooldown_until_utc: None,
                    consecutive_failures: 0,
                    consecutive_successes: 0,
                    last_error: None,
                    avg_latency_ms: None,
                    cost_per_million_tokens_in: find_configured_model(model_id, &configured_models)
                        .and_then(|model| model.cost_per_million_tokens_in),
                    cost_per_million_tokens_out: find_configured_model(
                        model_id,
                        &configured_models,
                    )
                    .and_then(|model| model.cost_per_million_tokens_out),
                    capabilities: infer_capabilities_for_observed_model(
                        &provider.id,
                        model_id,
                        &configured_models,
                    ),
                    streaming_validated: find_configured_model(model_id, &configured_models)
                        .and_then(|model| model.streaming_validated),
                })
                .collect();
        } else if !expected_models.is_empty() && !bootstrap_healthy {
            for model in &mut provider.models {
                model.is_default = expected_models
                    .first()
                    .is_some_and(|expected| expected == &model.id);
            }
        }
    }

    Ok(providers)
}

fn apply_provider_intelligence_overlay(
    mut providers: Vec<ProviderState>,
    intelligence_path: &Path,
) -> Result<Vec<ProviderState>> {
    if !intelligence_path.exists() {
        return Ok(providers);
    }
    let raw = fs::read_to_string(intelligence_path)?;
    let parsed: ProviderIntelligenceFile =
        serde_json::from_str(&raw).map_err(|e| ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "failed to parse provider intelligence {}: {e}",
                intelligence_path.display()
            ),
        })?;

    let quarantine_enabled = std::env::var("ARDA_ENABLE_QUARANTINE")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    for provider in &mut providers {
        let Some(overlay) = parsed.providers.get(&provider.id) else {
            continue;
        };
        if let Some(access_tier) = &overlay.access_tier {
            provider.access_tier = access_tier.to_ascii_lowercase();
        }
        if let Some(quality_band) = &overlay.quality_band {
            provider.quality_band = quality_band.to_ascii_lowercase();
        }
        if let Some(requests_per_minute) = overlay.requests_per_minute {
            provider.requests_per_minute = Some(requests_per_minute);
        }
        if let Some(requests_per_day) = overlay.requests_per_day {
            provider.requests_per_day = if provider.id == "openrouter" {
                Some(
                    provider
                        .requests_per_day
                        .unwrap_or(requests_per_day)
                        .max(requests_per_day),
                )
            } else {
                Some(requests_per_day)
            };
        }
        if let Some(requests_used_minute) = overlay.requests_used_minute {
            provider.requests_used_minute = requests_used_minute;
        }
        if let Some(requests_used_day) = overlay.requests_used_day {
            provider.requests_used_day = requests_used_day;
        }
        if let Some(enabled) = overlay.enabled {
            // Operator config is authoritative for opt-in. Intelligence may
            // disable a provider when discovery proves it unusable, but it must
            // not re-enable paid/future providers the config explicitly turned off.
            if !enabled {
                provider.enabled = false;
            }
        }
        if let Some(healthy) = overlay.healthy {
            provider.healthy = healthy;
        }
        if let Some(refreshed_at_utc) = &overlay.refreshed_at_utc {
            provider.intelligence_refreshed_at_utc = Some(refreshed_at_utc.clone());
            if provider.requests_used_minute > 0 && provider.minute_window_started_utc.is_none() {
                provider.minute_window_started_utc = Some(refreshed_at_utc.clone());
            }
            if provider.requests_used_day > 0 && provider.day_window_started_utc.is_none() {
                provider.day_window_started_utc = Some(refreshed_at_utc.clone());
            }
        }
        if !overlay.models.is_empty() {
            let configured_models = provider.models.clone();
            provider.models = overlay
                .models
                .iter()
                .filter(|model| !model.id.trim().is_empty())
                .map(|model| {
                    let configured_model = configured_models.iter().find(|configured| {
                        configured.id == model.id || configured.alias_matches(&model.id)
                    });
                    let mut capabilities = merge_model_capabilities(
                        &model.capabilities,
                        configured_model.map(|configured| &configured.capabilities),
                    );
                    if configured_model.is_none() && local_provider_id(&provider.id) {
                        capabilities = merge_model_capabilities(
                            &capabilities,
                            Some(&infer_capabilities_for_observed_model(
                                &provider.id,
                                &model.id,
                                &configured_models,
                            )),
                        );
                    }
                    ModelState {
                        aliases: vec![],
                        id: model.id.clone(),
                        capable_tasks: if model.capable_tasks.is_empty() {
                            configured_model
                                .map(|configured| configured.capable_tasks.clone())
                                .filter(|tasks| !tasks.is_empty())
                                .unwrap_or_else(|| vec!["chat".to_string()])
                        } else {
                            model.capable_tasks.clone()
                        },
                        context_window: model.context_window.unwrap_or_else(|| {
                            configured_model
                                .map(|configured| configured.context_window)
                                .unwrap_or(8192)
                        }),
                        is_default: model.is_default.unwrap_or(false),
                        healthy: true,
                        in_cooldown: false,
                        cooldown_until_utc: None,
                        consecutive_failures: 0,
                        consecutive_successes: 0,
                        last_error: None,
                        avg_latency_ms: None,
                        cost_per_million_tokens_in: model.cost_per_million_tokens_in.or_else(
                            || {
                                configured_model
                                    .and_then(|configured| configured.cost_per_million_tokens_in)
                            },
                        ),
                        cost_per_million_tokens_out: model.cost_per_million_tokens_out.or_else(
                            || {
                                configured_model
                                    .and_then(|configured| configured.cost_per_million_tokens_out)
                            },
                        ),
                        capabilities,
                        streaming_validated: model.streaming_validated.or_else(|| {
                            configured_model.and_then(|configured| configured.streaming_validated)
                        }),
                    }
                })
                .collect::<Vec<_>>();
            if provider.models.is_empty() {
                provider.models.push(ModelState {
                    aliases: vec![],
                    id: "default".to_string(),
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
                    capabilities: ModelCapabilities::default(),
                    streaming_validated: None,
                });
            } else if !provider.models.iter().any(|model| model.is_default) {
                if let Some(first) = provider.models.first_mut() {
                    first.is_default = true;
                }
            }
        }
        if quarantine_enabled {
            quarantine_stale_configured_models(provider, overlay);
        } else if !overlay.metadata.stale_configured_models.is_empty() {
            warn!(
                provider_id = %provider.id,
                stale_count = overlay.metadata.stale_configured_models.len(),
                "skipping provider intelligence quarantine because ARDA_ENABLE_QUARANTINE is not set"
            );
        }
    }

    Ok(providers)
}

fn merge_model_capabilities(
    overlay: &ModelCapabilities,
    configured: Option<&ModelCapabilities>,
) -> ModelCapabilities {
    let Some(configured) = configured else {
        return overlay.clone();
    };
    ModelCapabilities {
        tools: configured.tools.or(overlay.tools),
        streaming: configured.streaming.or(overlay.streaming),
        structured_output: configured.structured_output.or(overlay.structured_output),
        visible_reasoning: configured.visible_reasoning.or(overlay.visible_reasoning),
    }
}

fn quarantine_stale_configured_models(
    provider: &mut ProviderState,
    overlay: &ProviderIntelligenceOverlay,
) {
    if overlay.metadata.stale_configured_models.is_empty() {
        return;
    }

    let stale_ids = overlay
        .metadata
        .stale_configured_models
        .iter()
        .map(|model_id| model_id.trim().to_ascii_lowercase())
        .filter(|model_id| !model_id.is_empty())
        .collect::<BTreeSet<_>>();
    if stale_ids.is_empty() {
        return;
    }

    let cooldown_until = (Utc::now() + Duration::hours(24)).to_rfc3339();
    let mut default_needs_repair = false;
    for model in &mut provider.models {
        if !stale_ids.contains(&model.id.trim().to_ascii_lowercase()) {
            continue;
        }
        model.healthy = false;
        model.in_cooldown = true;
        model.cooldown_until_utc = Some(cooldown_until.clone());
        model.consecutive_failures = model.consecutive_failures.max(1);
        model.consecutive_successes = 0;
        model.last_error = Some(
            "model quarantined by provider intelligence: missing from live catalog".to_string(),
        );
        if model.is_default {
            model.is_default = false;
            default_needs_repair = true;
        }
    }

    if default_needs_repair || !provider.models.iter().any(|model| model.is_default) {
        if let Some(index) = default_repair_candidate_index(provider) {
            provider.models[index].is_default = true;
        }
    }
}

fn apply_tool_fit_model_intelligence_overlay(
    mut providers: Vec<ProviderState>,
    intelligence_path: &Path,
) -> Result<Vec<ProviderState>> {
    if !intelligence_path.exists() {
        return Ok(providers);
    }
    let raw = fs::read_to_string(intelligence_path)?;
    let parsed: ToolFitModelIntelligenceFile =
        serde_json::from_str(&raw).map_err(|e| ArdaError::Agent {
            agent: "charon".to_string(),
            message: format!(
                "failed to parse tool-fit model intelligence {}: {e}",
                intelligence_path.display()
            ),
        })?;

    for recommendation in parsed.models {
        if !tool_fit_recommendation_should_quarantine(&recommendation) {
            continue;
        }
        let Some(provider) = providers
            .iter_mut()
            .find(|provider| provider.id == recommendation.provider_id)
        else {
            continue;
        };
        let Some(model) = provider
            .models
            .iter_mut()
            .find(|model| model.id == recommendation.model_id)
        else {
            continue;
        };
        model.healthy = false;
        model.in_cooldown = true;
        model.cooldown_until_utc = Some((Utc::now() + Duration::hours(24)).to_rfc3339());
        model.consecutive_failures = model.consecutive_failures.max(1);
        model.consecutive_successes = 0;
        model.last_error = Some(format!(
            "model quarantined by tool-fit intelligence: recommendation={} attempts={} success_rate={}",
            recommendation.recommendation,
            recommendation.attempts,
            recommendation
                .success_rate
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
        if model.is_default {
            model.is_default = false;
            if let Some(index) = default_repair_candidate_index(provider) {
                provider.models[index].is_default = true;
            }
        }
    }

    Ok(providers)
}

fn default_repair_candidate_index(provider: &ProviderState) -> Option<usize> {
    if provider.id == "openrouter" {
        if let Some(index) = provider
            .models
            .iter()
            .position(|model| default_repair_candidate(provider, model))
        {
            return Some(index);
        }
    }
    provider
        .models
        .iter()
        .position(|model| model.healthy && !model.in_cooldown)
}

fn default_repair_candidate(provider: &ProviderState, model: &ModelState) -> bool {
    if !model.healthy || model.in_cooldown {
        return false;
    }
    if provider.id == "openrouter" {
        return model.id == "openrouter/auto";
    }
    true
}

fn tool_fit_recommendation_should_quarantine(recommendation: &ToolFitModelRecommendation) -> bool {
    recommendation.recommendation == "quarantine_candidate" && recommendation.attempts >= 3
}

pub(super) fn fleet_bootstrap_is_fresh(bootstrap: &FleetBootstrapFile) -> bool {
    let Some(generated_at) = bootstrap.generated_at_utc.as_deref() else {
        return false;
    };
    let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(generated_at) else {
        return false;
    };
    let max_age_seconds = std::env::var("ARDA_FLEET_BOOTSTRAP_MAX_AGE_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1800);
    (Utc::now() - parsed.with_timezone(&Utc)).num_seconds() <= max_age_seconds
}

fn find_configured_model<'a>(model_id: &str, existing: &'a [ModelState]) -> Option<&'a ModelState> {
    existing
        .iter()
        .find(|model| model.id == model_id || model.alias_matches(model_id))
}

fn infer_tasks_for_observed_model(
    provider_id: &str,
    model_id: &str,
    existing: &[ModelState],
) -> Vec<String> {
    find_configured_model(model_id, existing)
        .map(|model| model.capable_tasks.clone())
        .or_else(|| {
            local_provider_id(provider_id).then(|| {
                vec![
                    "code".to_string(),
                    "research".to_string(),
                    "reasoning".to_string(),
                    "chat".to_string(),
                    "summary".to_string(),
                    "background".to_string(),
                ]
            })
        })
        .or_else(|| existing.first().map(|model| model.capable_tasks.clone()))
        .unwrap_or_else(|| vec!["chat".to_string()])
}

fn infer_capabilities_for_observed_model(
    provider_id: &str,
    model_id: &str,
    existing: &[ModelState],
) -> ModelCapabilities {
    if let Some(configured) = find_configured_model(model_id, existing) {
        return configured.capabilities.clone();
    }
    if local_provider_id(provider_id) {
        return ModelCapabilities {
            tools: Some(true),
            streaming: Some(true),
            structured_output: None,
            visible_reasoning: None,
        };
    }
    ModelCapabilities::default()
}

fn local_provider_id(provider_id: &str) -> bool {
    provider_id.starts_with("edge_")
        || matches!(
            provider_id,
            "local_fallback" | "mesh_local" | "local_llamacpp"
        )
}

fn load_fleet_bootstrap_state(path: &Path) -> Result<Option<FleetBootstrapFile>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)?;
    let parsed = serde_json::from_str(&raw).map_err(|e| ArdaError::Agent {
        agent: "charon".to_string(),
        message: format!(
            "failed to parse fleet bootstrap state {}: {e}",
            path.display()
        ),
    })?;
    Ok(Some(parsed))
}

fn normalize_url(url: &str) -> String {
    url.trim_end_matches('/').to_ascii_lowercase()
}

/// Warn when two or more providers share the same base_url. This catches
/// the duplicate-provider mistake where one physical upstream is exposed
/// twice (e.g. `mesh_local` + `edge_backbone` both pointing at the same
/// llama-server) — which silently inflates routing weight for that node
/// and makes failover behavior inconsistent.
fn warn_on_base_url_collisions(providers: &[ProviderState]) {
    use std::collections::BTreeMap;
    let mut by_url: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for p in providers {
        if let Some(base) = p.base_url.as_deref() {
            let key = normalize_url(base);
            if key.is_empty() {
                continue;
            }
            by_url.entry(key).or_default().push(p.id.as_str());
        }
    }
    for (base_url, ids) in by_url {
        if ids.len() < 2 {
            continue;
        }
        tracing::warn!(
            base_url = %base_url,
            providers = %ids.join(", "),
            "charon: {} providers share the same base_url ({}); this double-counts the upstream in routing decisions. Collapse to a single provider entry or change one base_url.",
            ids.len(),
            base_url
        );
    }
}

/// Emit a tracing warning when a provider's base_url looks like it will
/// fail OpenAI-compat path resolution. Charon appends `/chat/completions`
/// directly to base_url, so URLs missing the standard `/v1` segment will
/// 404 or 400 silently.
fn warn_if_base_url_suspicious(provider_id: &str, driver: Option<&str>, base_url: &str) {
    if !matches!(
        driver.unwrap_or("openai_compat"),
        "openai_compat" | "hermes_proxy"
    ) {
        return;
    }
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        tracing::warn!(provider_id, "charon: base_url is empty");
        return;
    }
    // Surfaces with their own well-known conventions.
    // - litellm / ollama: aggregators using their own paths
    // - /zen/: OpenCode Zen lane
    // - /openai: Google's OpenAI-compat suffix (e.g. /v1beta/openai)
    // - /api/: OpenRouter and similar
    let known_non_v1_surfaces = ["litellm", "ollama", "/zen/", "/api/", "/openai"];
    let lowered = trimmed.to_ascii_lowercase();
    if known_non_v1_surfaces
        .iter()
        .any(|needle| lowered.contains(needle))
    {
        return;
    }
    let last_segment = trimmed.rsplit('/').next().unwrap_or("");
    if last_segment != "v1" {
        tracing::warn!(
            provider_id,
            base_url = trimmed,
            "charon: base_url does not end in /v1 — most OpenAI-compat \
             servers (LM Studio, llama.cpp, vLLM) need it. Charon appends \
             /chat/completions directly. Add /v1 or set supports_tools=false \
             if this is a non-OpenAI surface."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn loopback_url(port: u16) -> String {
        format!("http://{}:{port}/v1", "127.0.0.1")
    }

    fn private_lan_host(last_octet: u8) -> String {
        format!("{}.{}.{}.{}", 10, 0, 0, last_octet)
    }

    fn lan_url(host: &str) -> String {
        format!("http://{host}:{}/v1", 11434)
    }

    fn provider(id: &str) -> ProviderState {
        ProviderState {
            id: id.to_string(),
            name: format!("{id} name"),
            base_url: Some(loopback_url(1234)),
            api_key_env: None,
            access_tier: "mixed".to_string(),
            quality_band: "medium".to_string(),
            intelligence_refreshed_at_utc: None,
            probe_model: None,
            probe_profile: None,
            enabled: true,
            has_api_key: true,
            healthy: false,
            in_cooldown: false,
            cooldown_until_utc: None,
            cooldown_backoff_seconds: 120,
            requests_per_minute: Some(60),
            requests_used_minute: 0,
            minute_window_started_utc: None,
            requests_per_day: Some(1_000),
            requests_used_day: 0,
            day_window_started_utc: None,
            models: vec![ModelState {
                aliases: vec![],
                id: "baseline-model".to_string(),
                capable_tasks: vec!["code".to_string()],
                context_window: 32_768,
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
            last_error: Some("old error".to_string()),
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
    fn warn_on_base_url_collisions_groups_by_normalized_url() {
        // Doesn't assert on tracing output (would need a log subscriber);
        // exercises the code path so a future regression can't no-op it
        // and so we get coverage of the BTreeMap collision branch.
        let p1 = ProviderState {
            base_url: Some(lan_url(&private_lan_host(8))),
            ..provider("alpha")
        };
        let p2 = ProviderState {
            // Same URL but trailing slash + uppercase host — exercises normalize_url.
            base_url: Some(format!("http://{}:{}/V1/", private_lan_host(8), 11434)),
            ..provider("beta")
        };
        let p3 = ProviderState {
            base_url: Some(lan_url(&private_lan_host(99))),
            ..provider("gamma")
        };
        super::warn_on_base_url_collisions(&[p1, p2, p3]);
    }

    #[test]
    fn openrouter_default_repair_prefers_auto_over_explicit_free_model() {
        let mut provider = provider("openrouter");
        provider.models = vec![
            ModelState {
                id: "openrouter/auto".to_string(),
                is_default: false,
                ..provider.models[0].clone()
            },
            ModelState {
                id: "qwen/qwen3-coder:free".to_string(),
                is_default: false,
                healthy: false,
                in_cooldown: true,
                ..provider.models[0].clone()
            },
            ModelState {
                id: "nvidia/nemotron-3-super-120b-a12b:free".to_string(),
                is_default: false,
                ..provider.models[0].clone()
            },
        ];

        let index = default_repair_candidate_index(&provider).expect("repair candidate");

        assert_eq!(provider.models[index].id, "openrouter/auto");
    }

    #[test]
    fn fresh_fleet_bootstrap_overrides_provider_identity_and_models() {
        let dir = tempdir().expect("tempdir");
        let bootstrap_path = dir.path().join("fleet_bootstrap.json");
        let generated_at = Utc::now().to_rfc3339();
        let host = private_lan_host(8);
        fs::write(
            &bootstrap_path,
            format!(
                r#"{{
  "generated_at_utc": "{generated_at}",
  "targets": [
    {{
      "target_id": "edge-1",
      "display_name": "Edge Worker One",
      "charon_provider_id": "edge_worker_light",
      "configured_base_url": "http://{host}:11434/v1",
      "status": "online",
      "has_live_endpoint": true,
      "observed_models": ["qwen3:32b", "qwen2.5-coder:14b"]
    }}
  ]
}}"#
            ),
        )
        .expect("bootstrap write");

        let providers =
            apply_fleet_bootstrap_overlay(vec![provider("edge_worker_light")], &bootstrap_path)
                .expect("overlay");
        let provider = &providers[0];

        assert_eq!(provider.name, "Edge Worker One");
        assert_eq!(provider.base_url.as_deref(), Some(lan_url(&host).as_str()));
        assert!(provider.healthy);
        assert!(provider.last_error.is_none());
        assert_eq!(provider.models.len(), 2);
        assert_eq!(provider.models[0].id, "qwen3:32b");
        assert!(provider.models[0].is_default);
        assert_eq!(provider.models[0].capabilities.tools, Some(true));
        assert!(provider.models[0]
            .capable_tasks
            .iter()
            .any(|task| task == "code"));
    }

    #[test]
    fn provider_intelligence_overlay_populates_windows_and_default_model() {
        let dir = tempdir().expect("tempdir");
        let intelligence_path = dir.path().join("provider_intelligence.json");
        let refreshed_at = Utc::now().to_rfc3339();
        fs::write(
            &intelligence_path,
            format!(
                r#"{{
  "providers": {{
    "edge_worker_light": {{
      "access_tier": "LOCAL",
      "quality_band": "HIGH",
      "requests_per_minute": 90,
      "requests_per_day": 2000,
      "requests_used_minute": 12,
      "requests_used_day": 144,
      "enabled": true,
      "healthy": true,
      "refreshed_at_utc": "{refreshed_at}",
      "models": [
        {{
          "id": "qwen3:32b",
          "capable_tasks": ["code", "reasoning"],
          "context_window": 131072,
          "capabilities": {{"tools": false}}
        }}
      ]
    }}
  }}
}}"#
            ),
        )
        .expect("intelligence write");

        let providers = apply_provider_intelligence_overlay(
            vec![provider("edge_worker_light")],
            &intelligence_path,
        )
        .expect("overlay");
        let provider = &providers[0];

        assert_eq!(provider.access_tier, "local");
        assert_eq!(provider.quality_band, "high");
        assert_eq!(provider.requests_per_minute, Some(90));
        assert_eq!(provider.requests_used_minute, 12);
        assert_eq!(
            provider.minute_window_started_utc.as_deref(),
            Some(refreshed_at.as_str())
        );
        assert_eq!(
            provider.day_window_started_utc.as_deref(),
            Some(refreshed_at.as_str())
        );
        assert_eq!(provider.models.len(), 1);
        assert_eq!(provider.models[0].id, "qwen3:32b");
        assert!(provider.models[0].is_default);
        assert_eq!(provider.models[0].capable_tasks, vec!["code", "reasoning"]);
        assert_eq!(provider.models[0].capabilities.tools, Some(true));
    }

    #[test]
    fn provider_intelligence_overlay_preserves_configured_model_capability_denials() {
        let dir = tempdir().expect("tempdir");
        let intelligence_path = dir.path().join("provider_intelligence.json");
        fs::write(
            &intelligence_path,
            r#"{
  "providers": {
    "cerebras": {
      "models": [
        {
          "id": "gpt-oss-120b",
          "capable_tasks": ["code", "reasoning"],
          "context_window": 131072,
          "capabilities": {
            "structured_output": true
          }
        }
      ]
    }
  }
}"#,
        )
        .expect("intelligence write");

        let mut provider = provider("cerebras");
        provider.models = vec![ModelState {
            id: "gpt-oss-120b".to_string(),
            capabilities: ModelCapabilities {
                tools: Some(false),
                streaming: Some(true),
                structured_output: Some(true),
                visible_reasoning: None,
            },
            ..provider.models[0].clone()
        }];

        let providers = apply_provider_intelligence_overlay(vec![provider], &intelligence_path)
            .expect("overlay");
        let model = &providers[0].models[0];

        assert_eq!(model.id, "gpt-oss-120b");
        assert_eq!(model.capabilities.tools, Some(false));
        assert_eq!(model.capabilities.streaming, Some(true));
        assert_eq!(model.capabilities.structured_output, Some(true));
    }

    #[test]
    fn provider_intelligence_overlay_does_not_lower_openrouter_daily_cap() {
        let dir = tempdir().expect("tempdir");
        let intelligence_path = dir.path().join("provider_intelligence.json");
        fs::write(
            &intelligence_path,
            r#"{
  "providers": {
    "openrouter": {
      "requests_per_day": 50,
      "requests_per_minute": 20,
      "models": [
        {
          "id": "openrouter/auto",
          "capable_tasks": ["code", "chat"],
          "context_window": 1048576
        }
      ]
    }
  }
}"#,
        )
        .expect("intelligence write");

        let mut openrouter = provider("openrouter");
        openrouter.requests_per_day = Some(2_000);

        let providers = apply_provider_intelligence_overlay(vec![openrouter], &intelligence_path)
            .expect("overlay");

        assert_eq!(providers[0].requests_per_day, Some(2_000));
        assert_eq!(providers[0].requests_per_minute, Some(20));
    }

    #[test]
    fn provider_intelligence_quarantines_stale_configured_models() {
        let dir = tempdir().expect("tempdir");
        let intelligence_path = dir.path().join("provider_intelligence.json");
        fs::write(
            &intelligence_path,
            r#"{
  "providers": {
    "openrouter": {
      "metadata": {
        "stale_configured_models": ["retired/model:free"]
      }
    }
  }
}"#,
        )
        .expect("intelligence write");

        let mut provider = provider("openrouter");
        provider.models = vec![
            ModelState {
                id: "retired/model:free".to_string(),
                is_default: true,
                ..provider.models[0].clone()
            },
            ModelState {
                id: "live/model:free".to_string(),
                is_default: false,
                ..provider.models[0].clone()
            },
        ];

        let providers = apply_provider_intelligence_overlay(vec![provider], &intelligence_path)
            .expect("overlay");
        let provider = &providers[0];
        let stale = provider
            .models
            .iter()
            .find(|model| model.id == "retired/model:free")
            .expect("stale model");
        let live = provider
            .models
            .iter()
            .find(|model| model.id == "live/model:free")
            .expect("live model");

        assert!(!stale.healthy);
        assert!(stale.in_cooldown);
        assert!(!stale.is_default);
        assert!(stale
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("missing from live catalog")));
        assert!(live.healthy);
        assert!(!live.in_cooldown);
        assert!(live.is_default);
    }

    #[test]
    fn tool_fit_intelligence_quarantines_repeated_bad_model() {
        let dir = tempdir().expect("tempdir");
        let intelligence_path = dir.path().join("tool_fit_model_intelligence.json");
        fs::write(
            &intelligence_path,
            r#"{
  "models": [
    {
      "provider_id": "opencode",
      "model_id": "retired-model",
      "recommendation": "quarantine_candidate",
      "attempts": 3,
      "success_rate": 0.0
    },
    {
      "provider_id": "opencode",
      "model_id": "live-model",
      "recommendation": "promote_for_tools",
      "attempts": 5,
      "success_rate": 1.0
    }
  ]
}"#,
        )
        .expect("intelligence write");

        let mut provider = provider("opencode");
        provider.models = vec![
            ModelState {
                id: "retired-model".to_string(),
                is_default: true,
                ..provider.models[0].clone()
            },
            ModelState {
                id: "live-model".to_string(),
                is_default: false,
                ..provider.models[0].clone()
            },
        ];

        let providers =
            apply_tool_fit_model_intelligence_overlay(vec![provider], &intelligence_path)
                .expect("overlay");
        let provider = &providers[0];
        let retired = provider
            .models
            .iter()
            .find(|model| model.id == "retired-model")
            .expect("retired model");
        let live = provider
            .models
            .iter()
            .find(|model| model.id == "live-model")
            .expect("live model");

        assert!(!retired.healthy);
        assert!(retired.in_cooldown);
        assert!(!retired.is_default);
        assert!(retired
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("tool-fit intelligence")));
        assert!(live.healthy);
        assert!(!live.in_cooldown);
        assert!(live.is_default);
    }

    #[test]
    fn fleet_bootstrap_freshness_uses_positive_env_override() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        std::env::set_var("ARDA_FLEET_BOOTSTRAP_MAX_AGE_SECONDS", "1");
        let stale = FleetBootstrapFile {
            generated_at_utc: Some((Utc::now() - chrono::Duration::seconds(5)).to_rfc3339()),
            targets: Vec::new(),
        };
        assert!(!fleet_bootstrap_is_fresh(&stale));
        std::env::remove_var("ARDA_FLEET_BOOTSTRAP_MAX_AGE_SECONDS");
    }
}
