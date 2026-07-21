//! Minimal provider catalog for the gateway bootstrap.
//!
//! This is intentionally small: keep `ProviderCatalog` to one upstream per
//! placeholder slot so it reassembles the real config later.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

pub type ProviderId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub id: ProviderId,
    pub name: String,
    pub node_id: Option<String>,
    pub model_id: String,
    pub base_url: String,
    pub api_key_env: Option<String>,
    pub transport: ProviderTransport,
    pub capabilities: ProviderCapabilities,
    pub health_url: Option<String>,
    pub models_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTransport {
    OpenAICompatible,
    AnthropicMessages,
    LocalHttp,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub tool_calls: bool,
    pub structured_output: bool,
}

impl ProviderDefinition {
    pub fn openai_compatible(
        id: impl Into<String>,
        name: impl Into<String>,
        model_id: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            node_id: None,
            model_id: model_id.into(),
            base_url: base_url.into(),
            api_key_env: Some("ARDA_MANWE_PLACEHOLDER_API_KEY".into()),
            transport: ProviderTransport::OpenAICompatible,
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: true,
            },
            health_url: None,
            models_url: None,
        }
    }

    pub fn from_fleet_node(node_id: impl Into<String>, node: &FleetNode) -> Self {
        let id = node.manwe_provider_id.clone();
        Self {
            id: id.clone(),
            name: node.display_name.clone().unwrap_or_else(|| id.clone()),
            node_id: Some(node_id.into()),
            model_id: node
                .runtime_model_alias
                .clone()
                .or_else(|| node.expected_models.first().cloned())
                .unwrap_or_default(),
            base_url: normalize_runtime_url(
                node.base_url.as_deref().unwrap_or_default(),
                node.runtime_port,
            ),
            api_key_env: None,
            transport: ProviderTransport::OpenAICompatible,
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: false,
            },
            health_url: node.health_url.clone(),
            models_url: node.models_url.clone(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetNode {
    manwe_provider_id: String,
    display_name: Option<String>,
    enrollment_status: Option<String>,
    base_url: Option<String>,
    runtime_port: Option<u16>,
    runtime_backend: Option<String>,
    runtime_host: Option<String>,
    models_url: Option<String>,
    health_url: Option<String>,
    expected_models: Vec<String>,
    runtime_model_alias: Option<String>,
    llm_runtime: Option<String>,
}

fn normalize_runtime_url(base_url: &str, runtime_port: Option<u16>) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(port) = runtime_port {
        let candidate = trimmed.trim_start_matches("http://").trim_start_matches("https://");
        if candidate.contains(':') {
            return format!("http://{candidate}");
        }
        return format!("http://127.0.0.1:{port}");
    }
    trimmed.to_string()
}

#[derive(Debug, Clone, Default)]
pub struct ProviderCatalog {
    by_id: HashMap<String, ProviderDefinition>,
}

impl ProviderCatalog {
    pub fn new(injected: Vec<ProviderDefinition>) -> Self {
        let mut by_id = HashMap::new();
        for entry in injected {
            by_id.insert(entry.id.clone(), entry);
        }
        Self { by_id }
    }

    pub fn default_bootstrap() -> Self {
        Self::new(vec![ProviderDefinition::openai_compatible(
            "local_placeholder",
            "Placeholder Provider",
            "placeholder-model",
            "http://127.0.0.1:7171/v1",
        )])
    }

    pub fn empty() -> Self {
        Self {
            by_id: HashMap::new(),
        }
    }

    pub fn insert(&mut self, provider: ProviderDefinition) {
        self.by_id.insert(provider.id.clone(), provider);
    }

    pub fn get(&self, id: &str) -> Option<&ProviderDefinition> {
        self.by_id.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ProviderDefinition)> {
        self.by_id.iter()
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    pub fn local_placeholder(&self) -> Option<&ProviderDefinition> {
        self.get("local_placeholder")
    }
        pub fn from_fleet_config(path: impl AsRef<Path>) -> Self {
            let mut catalog = Self::empty();
            if let Ok(text) = fs::read_to_string(path) {
                let nodes = parse_fleet_nodes(&text);
                for (index, node) in nodes.into_iter().enumerate() {
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
                    let provider = ProviderDefinition::from_fleet_node(key, &node);
                    catalog.insert(provider);
                }
            }
            catalog
        }

        pub fn from_fleet_config_direct(text: impl AsRef<str>) -> Self {
            let nodes = parse_fleet_nodes(text.as_ref());
            let mut catalog = Self::empty();
            for (index, node) in nodes.into_iter().enumerate() {
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
                catalog.insert(ProviderDefinition::from_fleet_node(key, &node));
            }
            catalog
        }

        pub fn refresh(&mut self, path: impl AsRef<Path>) {
            *self = Self::from_fleet_config(path);
        }
    }

#[derive(Debug, Clone, Deserialize, Default)]
struct FleetConfig {
    nodes: Vec<FleetNode>,
}

fn parse_fleet_nodes(text: &str) -> Vec<FleetNode> {
    let mut out = Vec::new();
    let mut current = None;
    let mut key: Option<String> = None;
    for raw in text.split('\n') {
        let trimmed = raw.trim();
        if trimmed == "[nodes]" {
            if let Some(node) = current.take() {
                out.push(node);
            }
            current = Some(FleetNode::default());
            key = None;
            continue;
        }
        if trimmed.starts_with("[[") && trimmed.ends_with("]]") {
            if trimmed.starts_with("[[nodes]]") {
                if let Some(node) = current.take() {
                    out.push(node);
                }
                current = Some(FleetNode::default());
                key = None;
                continue;
            }
            key = None;
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
            "manwe_provider_id" if !value.is_empty() => node.manwe_provider_id = value.into(),
            "display_name" if !value.is_empty() => node.display_name = Some(value.into()),
            "enrollment_status" if !value.is_empty() => {
                node.enrollment_status = Some(value.into());
            }
            "base_url" if !value.is_empty() => node.base_url = Some(value.into()),
            "runtime_port" if !value.is_empty() => {
                node.runtime_port = value.parse().ok();
            }
            "runtime_backend" if !value.is_empty() => {
                node.runtime_backend = Some(value.into());
            }
            "runtime_host" if !value.is_empty() => node.runtime_host = Some(value.into()),
            "models_url" if !value.is_empty() => node.models_url = Some(value.into()),
            "health_url" if !value.is_empty() => node.health_url = Some(value.into()),
            "expected_models" if value.starts_with('[') => {
                node.expected_models = parse_string_array(value);
            }
            "runtime_model_alias" if !value.is_empty() => {
                node.runtime_model_alias = Some(value.into());
            }
            "llm_runtime" if !value.is_empty() => node.llm_runtime = Some(value.into()),
            _ => {}
        }
    }
    if let Some(node) = current.take() {
        if !node.manwe_provider_id.is_empty() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_active_fleet_nodes() {
        let sample = r#"
[[nodes]]
manwe_provider_id = "edge_core"
display_name = "Core Hub"
enrollment_status = "active"
base_url = "http://core:9337/v1"
runtime_port = 9337
expected_models = ["LFM"]

[[nodes]]
manwe_provider_id = "edge_guardhouse"
enrollment_status = "inactive"
base_url = "http://warden:1234/v1"

[[nodes]]
manwe_provider_id = "edge_laptop"
llm_runtime = "local_voice_stt_operator"
base_url = "http://laptop:1234/v1"
"#;
        let nodes = parse_fleet_nodes(sample);
        let ids: Vec<_> = nodes.iter().map(|n| n.manwe_provider_id.as_str()).collect();
        assert_eq!(ids, vec!["edge_core"]);
    }

    #[test]
    fn builds_catalog_from_fleet_text() {
        let sample = r#"
[[nodes]]
manwe_provider_id = "edge_core"
enrollment_status = "active"
base_url = "http://core:9337/v1"
runtime_port = 9337
expected_models = ["LFM"]
"#;
        let catalog = ProviderCatalog::from_fleet_config_direct(sample);
        assert_eq!(catalog.len(), 1);
        let provider = catalog.get("edge_core").expect("edge_core catalog");
        assert_eq!(provider.model_id, "LFM");
        assert_eq!(provider.base_url, "http://core/v1");
        assert_eq!(provider.health_url, None);
    }

    #[test]
    fn falls_back_to_local_placeholder_when_empty_fleet() {
        let catalog = ProviderCatalog::from_fleet_config_direct("");
        assert_eq!(catalog.len(), 1);
        assert!(catalog.local_placeholder().is_some());
    }
}
