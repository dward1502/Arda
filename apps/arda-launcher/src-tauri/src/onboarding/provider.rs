use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
use toml::Value as TomlValue;

use crate::onboarding::helpers::{
    infer_payment_status, infer_provider_class, infer_provider_profile, make_signup_hint, now_utc,
};
use crate::onboarding::types::*;

pub fn provider_checklist(root: &Path) -> ProviderChecklist {
    let providers_path = root.join("config/charon.providers.toml");
    let mut provider_entries = Vec::new();
    let mut providers = Vec::new();
    let mut suggested_signatures = Vec::new();

    if let Ok(raw) = fs::read_to_string(&providers_path) {
        if let Ok(parsed) = raw.parse::<TomlValue>() {
            if let Some(arr) = parsed.get("provider").and_then(TomlValue::as_array) {
                for provider_value in arr {
                    if let Some(provider) = provider_value.as_table() {
                        let id = provider
                            .get("id")
                            .and_then(TomlValue::as_str)
                            .unwrap_or("unknown")
                            .to_string();
                        let name = provider
                            .get("name")
                            .and_then(TomlValue::as_str)
                            .unwrap_or(&id)
                            .to_string();
                        let enabled = provider
                            .get("enabled")
                            .and_then(TomlValue::as_bool)
                            .unwrap_or(false);
                        let access_tier = provider
                            .get("access_tier")
                            .and_then(TomlValue::as_str)
                            .map(str::to_string);
                        let base_url = provider
                            .get("base_url")
                            .and_then(TomlValue::as_str)
                            .map(str::to_string);
                        let models = provider
                            .get("model")
                            .and_then(TomlValue::as_array)
                            .map(std::vec::Vec::as_slice)
                            .unwrap_or(&[]);
                        let env_key = provider
                            .get("api_key_env")
                            .and_then(TomlValue::as_str)
                            .map(str::to_string);
                        let missing_env = env_key
                            .as_deref()
                            .filter(|env_key| env::var(env_key).ok().filter(|v| !v.trim().is_empty()).is_none())
                            .into_iter()
                            .map(|k| {
                                suggested_signatures.push(format!(
                                    "provider={id}: set {k} in your local env file (never commit secrets)"
                                ));
                                k.to_string()
                            })
                            .collect::<Vec<_>>();
                        let mut model_ids = Vec::new();
                        let mut default_model = None;
                        let mut has_default_model = false;
                        for model in models {
                            if let Some(model_id) = model.get("id").and_then(TomlValue::as_str) {
                                model_ids.push(model_id.to_string());
                                if model
                                    .as_table()
                                    .and_then(|m| m.get("is_default"))
                                    .and_then(TomlValue::as_bool)
                                    .unwrap_or(false)
                                {
                                    has_default_model = true;
                                    default_model = Some(model_id.to_string());
                                }
                            }
                        }
                        let requires_key = !missing_env.is_empty();
                        let payment_class =
                            infer_payment_status(access_tier.as_deref(), requires_key, &model_ids);
                        let (locality, route_class) = infer_provider_profile(
                            &id,
                            access_tier.as_deref(),
                            base_url.as_deref(),
                        );
                        let provider_class =
                            infer_provider_class(&locality, access_tier.as_deref());
                        let mut route_hints = BTreeSet::new();
                        route_hints.insert(format!("provider:{provider_class}"));
                        route_hints.insert(format!("locality:{locality}"));
                        route_hints.insert(format!("route:{route_class}"));
                        route_hints.insert(format!("payment:{payment_class}"));

                        provider_entries.push(ParsedProviderEntry {
                            provider_id: id,
                            provider_name: name,
                            enabled,
                            access_tier,
                            base_url,
                            missing_env,
                            has_default_model,
                            model_count: models.len(),
                            model_ids,
                            default_model,
                            env_key,
                            route_hints,
                            payment_class,
                        });
                    }
                }
            }
        }
    }

    let local_fallback_candidates: Vec<String> = provider_entries
        .iter()
        .filter(|entry| {
            entry.enabled
                && entry.has_default_model
                && entry
                    .route_hints
                    .iter()
                    .any(|hint| hint == "locality:local")
        })
        .map(|entry| entry.provider_id.clone())
        .collect();

    let local_default_model = provider_entries
        .iter()
        .find(|entry| {
            entry.enabled
                && entry.has_default_model
                && entry
                    .route_hints
                    .iter()
                    .any(|hint| hint == "locality:local")
        })
        .and_then(|entry| entry.default_model.clone());

    for parsed in provider_entries {
        let route_hints: Vec<String> = parsed.route_hints.iter().cloned().collect();
        let requires_key = !parsed.missing_env.is_empty();
        let has_local_fallback = requires_key && !local_fallback_candidates.is_empty();
        let fallback_routes = if has_local_fallback {
            local_fallback_candidates.clone()
        } else {
            Vec::new()
        };
        let missing_env = parsed.missing_env.clone();

        providers.push(ProviderInfo {
            provider_id: parsed.provider_id.clone(),
            provider_name: parsed.provider_name,
            enabled: parsed.enabled,
            access_tier: parsed.access_tier,
            route_hints: route_hints.clone(),
            provider_profile: {
                let mut provider_class = "unknown".to_string();
                let mut locality = "unknown".to_string();
                let mut route_class = "unknown".to_string();
                for hint in route_hints.iter() {
                    if let Some(value) = hint.strip_prefix("provider:") {
                        provider_class = value.to_string();
                    }
                    if let Some(value) = hint.strip_prefix("locality:") {
                        locality = value.to_string();
                    }
                    if let Some(value) = hint.strip_prefix("route:") {
                        route_class = value.to_string();
                    }
                }

                Some(ProviderCheckProfile {
                    provider_class: Some(provider_class),
                    locality: Some(locality),
                    route_class: Some(route_class),
                    source_hint: parsed.base_url.clone(),
                })
            },
            signup_hint: Some(make_signup_hint(
                &parsed.provider_id,
                parsed.env_key.as_deref(),
            )),
            action_hint: Some(ProviderActionHint {
                description: Some(format!(
                    "{} route is configured as {} ({}).",
                    parsed.provider_id,
                    route_hints
                        .iter()
                        .find(|hint| hint.starts_with("provider:"))
                        .unwrap_or(&"provider:unknown".to_string()),
                    parsed.payment_class
                )),
                requires_key,
                no_key_fallback_available: has_local_fallback,
            }),
            readiness_hint: Some(ProviderReadinessHint {
                route_target: parsed.base_url.clone(),
                local_default_model: local_default_model.clone(),
                has_local_fallback,
            }),
            has_default_model: parsed.has_default_model,
            model_count: parsed.model_count,
            fallback_routes,
            model_ids: parsed.model_ids,
            missing_env: missing_env.clone(),
        });

        if missing_env.is_empty() && parsed.enabled {
            suggested_signatures.push(format!(
                "provider={} ready for {} lane",
                parsed.provider_id,
                if parsed.route_hints.iter().any(|hint| hint == "payment:free") {
                    "free"
                } else {
                    "paid"
                }
            ));
        }
    }

    if providers.is_empty() {
        suggested_signatures.push(
            "Check config/charon.providers.toml; if missing, run from a repo-root with that file present."
                .to_string(),
        );
    }

    ProviderChecklist {
        generated_at_utc: now_utc(),
        profile: "local".to_string(),
        providers_path: providers_path.to_string_lossy().to_string(),
        providers,
        suggested_signatures,
    }
}

pub(crate) fn provider_env_keys(root: &Path) -> BTreeSet<String> {
    let providers_path = root.join("config/charon.providers.toml");
    let mut keys = BTreeSet::new();
    if let Ok(raw) = fs::read_to_string(&providers_path) {
        if let Ok(parsed) = raw.parse::<TomlValue>() {
            if let Some(arr) = parsed.get("provider").and_then(TomlValue::as_array) {
                for provider_value in arr {
                    if let Some(provider) = provider_value.as_table() {
                        if let Some(key) = provider.get("api_key_env").and_then(TomlValue::as_str) {
                            keys.insert(key.to_string());
                        }
                    }
                }
            }
        }
    }
    keys
}
