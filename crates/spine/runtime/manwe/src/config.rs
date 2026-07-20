//! Static provider catalog for `uinen` (the local inference gateway).
//!
//! The catalog is intentionally thin and data-driven: a `[providers.*]` table in
//! `uinen.toml`. No runtime adaptive routing, no quota mesh — the daemon picks a
//! provider by model prefix or `default_provider` and forwards the request. This
//! matches the frozen refactor contract (REFACTOR_PLAN.md §2).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

/// One upstream OpenAI-compatible endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct ProviderConfig {
    /// OpenAI-compatible base URL — must end in `/v1` (e.g. Ollama's
    /// `http://127.0.0.1:11434/v1`).
    pub base_url: String,
    #[serde(default)]
    pub api_key: Option<String>,
    /// Models this provider serves, used to populate `/v1/models`.
    #[serde(default)]
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ManweConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
}

fn default_bind() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    7171
}

impl ManweConfig {
    /// Sensible default so the gateway runs with zero config: local Ollama.
    pub fn embedded() -> ManweConfig {
        let mut providers = HashMap::new();
        providers.insert(
            "ollama".to_string(),
            ProviderConfig {
                base_url: "http://127.0.0.1:11434/v1".to_string(),
                api_key: None,
                models: vec!["llama3".to_string(), "mistral".to_string()],
            },
        );
        ManweConfig {
            bind: default_bind(),
            port: default_port(),
            default_provider: Some("ollama".to_string()),
            providers,
        }
    }

    /// Load from `path`; on any failure fall back to [`ManweConfig::embedded`]
    /// so the daemon always has a working gateway definition.
    pub fn load(path: &Path) -> ManweConfig {
        match std::fs::read_to_string(path) {
            Ok(text) => match toml::from_str::<ManweConfig>(&text) {
                Ok(cfg) => {
                    tracing::info!(
                        "manwe: loaded config from {path:?} ({providers} providers)",
                        providers = cfg.providers.len()
                    );
                    cfg
                }
                Err(e) => {
                    tracing::warn!("manwe: failed to parse {path:?} ({e}); using embedded default");
                    ManweConfig::embedded()
                }
            },
            Err(_) => {
                tracing::info!(
                    "uinen: no config at {path:?}; using embedded default (Ollama @11434)"
                );
                ManweConfig::embedded()
            }
        }
    }

    /// Resolve which provider handles `model`. Supports an explicit
    /// `"provider/model"` prefix; otherwise uses `default_provider`, then any
    /// provider as a last resort. Returns the provider name + config.
    pub fn resolve_provider<'a>(&'a self, model: &'a str) -> Option<(&'a str, &'a ProviderConfig)> {
        if let Some((prov, _)) = model.split_once('/') {
            if let Some(p) = self.providers.get(prov) {
                return Some((prov, p));
            }
        }
        if let Some(name) = &self.default_provider {
            if let Some(p) = self.providers.get(name) {
                return Some((name.as_str(), p));
            }
        }
        self.providers.iter().next().map(|(k, v)| (k.as_str(), v))
    }
}
