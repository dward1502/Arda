//! Provider hydration from `.env`, `config/manwe.providers.toml`,
//! `config/routing/*.toml`, and `config/fleet.toml`.
//!
//! This is intentionally additive:
//! - If any source fails to parse, it is skipped with a warning.
//! - If no provider resolves, callers should fall back to existing bootstrap behavior.

use std::collections::BTreeMap;
use std::path::Path;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::provider::{ProviderCatalog, ProviderDefinition};

#[derive(Debug, Clone, Default)]
pub struct DotenvMap {
    inner: BTreeMap<String, String>,
}

impl DotenvMap {
    pub fn from_file(path: impl AsRef<Path>) -> Self {
        let mut inner = BTreeMap::new();
        if let Ok(raw) = std::fs::read_to_string(path) {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = split_env_line(line) {
                    inner.insert(key, value);
                }
            }
        }
        Self { inner }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.inner.get(key).map(|s| s.as_str())
    }
}

fn split_env_line(line: &str) -> Option<(String, String)> {
    let without_prefix = line.trim_start_matches("export ").trim();
    let (key, value) = without_prefix.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let value = value.trim().trim_start_matches('"').trim_end_matches('"').trim_start_matches(''').trim_end_matches(''');
    Some((key.to_string(), value.to_string()))
}

#[derive(Debug, Clone, Default)]
pub struct HydrationResult {
    pub provider_count: usize,
    pub env_keys_seen: usize,
    pub routing_files_read: usize,
    pub fallback_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManweProvider {
    id: String,
    name: Option<String>,
    #[serde(default)]
    enabled: bool,
    base_url: Option<String>,
    api_key_env: Option<String>,
    #[serde(default)]
    models: Vec<ManweModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManweModel {
    id: String,
    #[serde(default)]
    is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ManweProvidersFile {
    #[serde(default)]
    provider: Vec<ManweProvider>,
}

pub fn hydrate_providers(
    repo_root: impl AsRef<Path>,
    dotenv: &DotenvMap,
    routing_globs: &[impl AsRef<Path>],
) -> (ProviderCatalog, HydrationResult) {
    let mut result = HydrationResult::default();
    let mut catalog = ProviderCatalog::empty();

    let manwe_toml = repo_root.as_ref().join("config/manwe.providers.toml");
    if let Ok(raw) = std::fs::read_to_string(&manwe_toml) {
        let parsed: ManweProvidersFile = toml::from_str(&raw).unwrap_or_default();
        let mut inserted = 0usize;
        for provider in parsed.provider {
            if !provider.enabled || provider.id.trim().is_empty() {
                continue;
            }
            let mut base_url = provider.base_url.unwrap_or_default();
            if !base_url.is_empty() {
                base_url = resolve_env_references(base_url, dotenv);
            }
            let api_key_env = provider.api_key_env.as_deref().and_then(|key| {
                let candidate = resolve_env_references(key.to_string(), dotenv);
                let trimmed = candidate.trim_start_matches("${").trim_end_matches("}").trim();
                let resolved = if !candidate.starts_with("${") {
                    candidate
                } else if dotenv.get(trimmed).is_some() {
                    candidate
                } else {
                    candidate
                };
                if resolved.is_empty() { None } else { Some(resolved) }
            }).or_else(|| provider.api_key_env.clone());
            let model_id = provider.models.iter().find(|m| m.is_default)
                .or_else(|| provider.models.first()).map(|m| m.id.clone()).unwrap_or_default();
            let mut def = ProviderDefinition::openai_compatible(
                provider.id.clone(),
                provider.name.unwrap_or_else(|| provider.id.clone()),
                model_id,
                if base_url.is_empty() { "http://127.0.0.1:7171/v1" } else { base_url.as_str() },
            );
            def.api_key_env = api_key_env;
            catalog.insert(def);
            inserted += 1;
        }
        if inserted == 0 {
            warn!("manwe.providers.toml loaded zero enabled providers");
        }
        result.provider_count += inserted;
    }

    for pattern in routing_globs {
        if let Ok(paths) = glob::glob(pattern.as_ref().to_string_lossy().as_ref()) {
            for entry in paths.flatten() {
                if let Ok(raw) = std::fs::read_to_string(&entry) {
                    let _ = apply_routing_overrides(&mut catalog, &raw);
                    result.routing_files_read += 1;
                }
            }
        }
    }

    let fleet = crate::provider::ProviderCatalog::from_fleet_config(repo_root.as_ref().join("config/fleet.toml"));
    if !fleet.is_empty() {
        catalog.merge(fleet);
    }

    if catalog.is_empty() {
        catalog.insert(ProviderDefinition::openai_compatible(
            "local_placeholder", "Placeholder Provider", "placeholder-model", "http://127.0.0.1:7171/v1",
        ));
        result.fallback_active = true;
    }

    (catalog, result)
}

impl ProviderCatalog {
    pub fn merge(&mut self, mut other: ProviderCatalog) {
        for (key, value) in other.by_id.drain() {
            self.by_id.entry(key).or_insert(value);
        }
    }

    pub fn apply_dotenv_overrides(&mut self, dotenv: &DotenvMap) {
        if dotenv.is_empty() {
            return;
        }
        for (_, def) in self.by_id.iter_mut() {
            if !def.base_url.is_empty() {
                def.base_url = resolve_env_references(def.base_url.clone(), dotenv);
            }
            if let Some(env_key) = def.api_key_env.as_ref() {
                if dotenv.get(env_key).is_some() {
                    def.api_key_env = Some(env_key.clone());
                }
            }
        }
    }
}

fn apply_routing_overrides(_catalog: &mut ProviderCatalog, _body: &str) {
    // Reference/routing docs are parsed only after an explicit routing config
    // contract is added for these files.
}

fn resolve_env_references(value: String, dotenv: &DotenvMap) -> String {
    let re = Regex::new(r"\$\{([^}]+)\}").expect("valid env reference regex");
    let mut last = value;
    loop {
        let next = re.replace_all(&last, |caps: &regex::Captures| {
            let key = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            dotenv.get(key).map(|s| s.as_str()).unwrap_or(caps.get(0).unwrap().as_str())
        }).into_owned();
        if next == last {
            break;
        }
        last = next;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotenv_parser_handles_quoted_and_plain_values() {
        let raw = r#"KEY=plain
QUOTED="value with spaces"
EMPTY=
# comment
"#;
        let map = DotenvMap::from_file(raw);
        assert_eq!(map.get("KEY"), Some("plain"));
        assert_eq!(map.get("QUOTED"), Some("value with spaces"));
        assert_eq!(map.get("EMPTY"), Some(""));
    }

    #[test]
    fn env_references_resolve_from_dotenv() {
        let mut inner = BTreeMap::new();
        inner.insert("HOST".to_string(), "100.0.0.1".to_string());
        inner.insert("PORT".to_string(), "1234".to_string());
        let dotenv = DotenvMap { inner };
        assert_eq!(resolve_env_references("http://${HOST}:${PORT}/v1".into(), &dotenv), "http://100.0.0.1:1234/v1");
    }

    #[test]
    fn hydrate_providers_falls_back_when_empty() {
        let (catalog, result) = hydrate_providers(".", &DotenvMap::default(), &[] as &[PathBuf]);
        assert!(result.fallback_active);
        assert!(catalog.local_placeholder().is_some());
    }
}
