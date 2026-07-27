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
use std::path::{Path, PathBuf};

/// Resolve the canonical Arda workspace root without depending on the process
/// working directory. Operators can override the compiled workspace location
/// with `ARDA_ROOT`.
pub fn arda_root() -> PathBuf {
    if let Some(path) = std::env::var_os("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

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

    /// Load static configuration and retain a credential-free description of
    /// the source selected by the fallback contract.
    pub fn load_with_source(path: &Path) -> (ManweConfig, StaticConfigSource) {
        match std::fs::exists(path) {
            Ok(true) => match std::fs::read_to_string(path) {
                Ok(raw) => match toml::from_str::<ManweConfig>(&raw) {
                    Ok(cfg) if !cfg.providers.is_empty() => (cfg, StaticConfigSource::File),
                    Ok(_) => (Self::embedded(), StaticConfigSource::EmbeddedEmpty),
                    Err(_) => (Self::embedded(), StaticConfigSource::EmbeddedMalformed),
                },
                Err(_) => (Self::embedded(), StaticConfigSource::EmbeddedUnreadable),
            },
            Ok(false) => (Self::embedded(), StaticConfigSource::EmbeddedMissing),
            Err(_) => (Self::embedded(), StaticConfigSource::EmbeddedUnreadable),
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

/// Credential-free static configuration provenance exposed by health and
/// capabilities responses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StaticConfigSource {
    File,
    EmbeddedMissing,
    EmbeddedUnreadable,
    EmbeddedMalformed,
    EmbeddedEmpty,
}

/// Resolve Manwe's adaptive state directory. Explicit Manwe variables own the
/// path, followed by `ARDA_ROOT`, the `ARDA_HOME` compatibility root supplied
/// by the caller, and finally the compiled workspace root.
#[cfg(feature = "adaptive")]
pub fn adaptive_state_dir(arda_home: Option<&Path>) -> PathBuf {
    if let Some(path) = std::env::var_os("ARDA_MANWE_STATE_DIR") {
        return PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("ARDA_MANWE_HOME") {
        return PathBuf::from(path);
    }
    if std::env::var_os("ARDA_ROOT").is_some() {
        return arda_root().join("data/manwe");
    }
    arda_home
        .map(|root| root.join("data/manwe"))
        .unwrap_or_else(|| arda_root().join("data/manwe"))
}

/// Resolve the static fleet catalog path. The canonical variable wins over
/// the retired Annunimas/Charon compatibility alias and workspace default.
pub fn static_fleet_config_path() -> PathBuf {
    std::env::var_os("ARDA_MANWE_FLEET_CONFIG")
        .or_else(|| std::env::var_os("ANNUNIMAS_CHARON_FLEET_CONFIG"))
        .map(PathBuf::from)
        .unwrap_or_else(|| arda_root().join("config/fleet.toml"))
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn load_reports_file_and_embedded_fallback_sources() {
        let dir = tempdir().expect("tempdir");
        let missing = dir.path().join("missing.toml");
        let (_, source) = ManweConfig::load_with_source(&missing);
        assert_eq!(source, StaticConfigSource::EmbeddedMissing);

        let malformed = dir.path().join("malformed.toml");
        std::fs::write(&malformed, "[providers\n").expect("write malformed config");
        let (_, source) = ManweConfig::load_with_source(&malformed);
        assert_eq!(source, StaticConfigSource::EmbeddedMalformed);

        let valid = dir.path().join("valid.toml");
        std::fs::write(
            &valid,
            r#"
default_provider = "local"
[providers.local]
base_url = "http://127.0.0.1:11434/v1"
models = ["test"]
"#,
        )
        .expect("write valid config");
        let (config, source) = ManweConfig::load_with_source(&valid);
        assert_eq!(source, StaticConfigSource::File);
        assert!(config.providers.contains_key("local"));
    }
}
