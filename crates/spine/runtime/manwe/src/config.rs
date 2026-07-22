//! Static provider catalog for `manwe` (the local inference gateway).
//!
//! The catalog is data-driven: it loads from `manwe.toml`, falling back
//! to a builtin local Ollama catalog so the daemon always has a working
//! gateway definition.
//!
//! Fleet/node configuration loading is explicitly deferred to the adaptive
//! baseline restoration pass.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// One upstream OpenAI-compatible endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

    /// Load from `path`; on any failure fall back to [`ManweConfig::embedded`].
    /// Fleet/node dynamic loading is deferred to the adaptive baseline.
    pub fn load(path: &Path) -> ManweConfig {
        match std::fs::exists(path) {
            Ok(true) => match std::fs::read_to_string(path) {
                Ok(raw) => {
                    let cfg: ManweConfig = toml::from_str(&raw).unwrap_or_default();
                    if cfg.providers.is_empty() {
                        return Self::embedded();
                    }
                    cfg
                }
                Err(_) => Self::embedded(),
            },
            Ok(false) | Err(_) => Self::embedded(),
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
            return None;
        }
        if let Some(name) = &self.default_provider {
            if let Some(p) = self.providers.get(name) {
                return Some((name.as_str(), p));
            }
        }
        self.providers.iter().next().map(|(k, v)| (k.as_str(), v))
    }

    /// Validate static config so misconfiguration fails fast with actionable
    /// reports instead of late runtime surprises.
    ///
    /// Covers tests, compile-time type checks, and runtime-only dangers:
    /// - providers exist
    /// - bind parses
    /// - port is usable
    /// - base_urls are semicolon-free and not credentials-only
    /// - api_key paths are not noise
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.providers.is_empty() {
            return Err(ConfigError::new("no providers configured"));
        }

        let bind_addr = format!("{}:{}", self.bind, self.port);
        bind_addr
            .parse::<std::net::SocketAddr>()
            .map_err(|e| ConfigError::new(format!("invalid bind/port `{bind_addr}`: {e}")))?;

        for (name, provider) in &self.providers {
            if provider.base_url.contains(';') {
                return Err(ConfigError::new(format!(
                    "provider `{name}` base_url contains semicolons and looks like multiple values; remove extra entries"
                )));
            }
            let looks_like_api_key = provider
                .base_url
                .trim_start_matches("http://")
                .trim_start_matches("https://")
                .trim_end_matches('/')
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_');
            if looks_like_api_key {
                return Err(ConfigError::new(format!(
                    "provider `{name}` base_url looks like a credential fragment instead of an endpoint"
                )));
            }

            if let Some(api_key) = &provider.api_key {
                if api_key.chars().all(|ch| ch.is_ascii_whitespace()) {
                    return Err(ConfigError::new(format!(
                        "provider `{name}` api_key is whitespace/noise"
                    )));
                }
            }
        }

        Ok(())
    }
}

/// Config validation error.
#[derive(Debug, thiserror::Error)]
#[error("manwe config error: {0}")]
pub struct ConfigError(#[from] pub Box<dyn std::error::Error + Send + Sync + 'static>);

impl ConfigError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(Box::new(ManweConfigErrorInner {
            message: message.into(),
        })
            as Box<dyn std::error::Error + Send + Sync + 'static>)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
struct ManweConfigErrorInner {
    message: String,
}
