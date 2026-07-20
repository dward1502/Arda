//! Static provider catalog for `manwe` (the local inference gateway).
//!
//! The catalog is data-driven: it can load from `manwe.toml` or from
//! `config/fleet.toml` `[[nodes]]` entries marked `active` / `active_staging`.
//! Fleet sources are preferred when `config/fleet.toml` exists.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

    /// Load from `path`; on any failure fall back to [`ManweConfig::embedded`]
    /// so the daemon always has a working gateway definition.
    /// When `config/fleet.toml` exists, active fleet nodes are preferred.
    pub fn load(path: &Path) -> ManweConfig {
        match Self::load_providers(path) {
            Ok(providers) => {
                let default_provider = providers.keys().next().map(|value| value.to_owned());
                let cfg = ManweConfig {
                    bind: default_bind(),
                    port: default_port(),
                    default_provider,
                    providers,
                };
                cfg
            }
            Err(err) => {
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

impl ManweConfig {
    fn load_providers(path: &Path) -> anyhow::Result<HashMap<String, ProviderConfig>> {
        if !path.exists() {
            return Ok(Self::embedded().providers);
        }

        let raw = std::fs::read_to_string(path)?;
        let cfg: ManweConfig = toml::from_str(&raw)?;
        if cfg.providers.is_empty() {
            return Ok(Self::embedded().providers);
        }
        Ok(cfg.providers)
    }

    fn default_fleet_path() -> PathBuf {
        PathBuf::from(
            std::env::var("MANWE_FLEET_CONFIG")
                .unwrap_or_else(|_| "config/fleet.toml".to_string()),
        )
    }

    fn load_fleet_providers(path: &Path) -> anyhow::Result<HashMap<String, ProviderConfig>> {
        let text = std::fs::read_to_string(path)?;
        let mut providers = HashMap::new();
        let mut count = 0usize;
        for (index, node) in parse_fleet_nodes(&text).into_iter().enumerate() {
            let status = node
                .enrollment_status
                .clone()
                .unwrap_or_default()
                .to_lowercase();
            if status != "active" && status != "active_staging" {
                continue;
            }
            let runtime_status = node.llm_runtime.as_deref().unwrap_or_default();
            if runtime_status.contains("inactive") {
                continue;
            }
            let key = format!("fleet_{index}");
            let base_url = normalize_runtime_url(node.base_url.as_deref().unwrap_or_default(), node.runtime_port);
            let model = node
                .runtime_model_alias
                .or_else(|| node.expected_models.first().cloned())
                .unwrap_or_default();
            let provider = ProviderConfig {
                base_url,
                api_key: None,
                models: if model.is_empty() { vec![] } else { vec![model] },
            };
            providers.insert(key, provider);
            count += 1;
        }
        if providers.is_empty() {
            anyhow::bail!("no active fleet nodes found");
        }
        tracing::info!("manwe: loaded {count} fleet providers");
        Ok(providers)
    }
}

fn parse_fleet_nodes(text: &str) -> Vec<FleetNode> {
    let mut out = Vec::new();
    let mut current = None;
    for raw in text.split('\n') {
        let trimmed = raw.trim();
        if trimmed == "[nodes]" || trimmed == "[[nodes]]" {
            if let Some(node) = current.take() {
                out.push(node);
            }
            current = Some(FleetNode::default());
            continue;
        }
        let Some(node) = current.as_mut() else {
            continue;
        };
        let Some((left, right)) = split_first_eq(trimmed) else {
            continue;
        };
        let value = right.trim().trim_matches('"');
        match left {
            "charon_provider_id" if !value.is_empty() => node.charon_provider_id = value.into(),
            "enrollment_status" if !value.is_empty() => node.enrollment_status = Some(value.into()),
            "base_url" if !value.is_empty() => node.base_url = Some(value.into()),
            "runtime_port" if !value.is_empty() => node.runtime_port = value.parse().ok(),
            "runtime_model_alias" if !value.is_empty() => node.runtime_model_alias = Some(value.into()),
            "expected_models" if value.starts_with('[') => node.expected_models = parse_string_array(value),
            "llm_runtime" if !value.is_empty() => node.llm_runtime = Some(value.into()),
            _ => {}
        }
    }
    if let Some(node) = current.take() {
        if !node.charon_provider_id.is_empty() {
            out.push(node);
        }
    }
    out
}

fn split_first_eq(value: &str) -> Option<(&str, &str)> {
    let mut iter = value.splitn(2, '=');
    let left = iter.next()?.trim();
    let right = iter.next()?;
    Some((left, right))
}

fn parse_string_array(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for token in value
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
    {
        let item = token.trim().trim_matches('"');
        if !item.is_empty() {
            out.push(item.into());
        }
    }
    out
}

fn normalize_base_url(base_url: Option<&str>, runtime_port: Option<u16>) -> String {
    let trimmed = base_url.map(|value| value.trim_end_matches('/')).unwrap_or_default();
    if trimmed.is_empty() {
        if let Some(port) = runtime_port {
            return format!("http://127.0.0.1:{port}");
        }
        return String::new();
    }
    if trimmed.contains(':') {
        return format!("http://{trimmed}");
    }
    trimmed.to_string()
}

fn resolve_models(alias: Option<String>, expected: Vec<String>) -> Vec<String> {
    alias.into_iter().chain(expected.into_iter()).collect()
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetConfig {
    #[serde(default)]
    nodes: Vec<FleetNode>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetNode {
    #[serde(default)]
    enrollment_status: Option<String>,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    runtime_port: Option<u16>,
    #[serde(default)]
    llm_runtime: Option<String>,
    #[serde(default)]
    expected_models: Vec<String>,
    #[serde(default)]
    runtime_model_alias: Option<String>,
}
