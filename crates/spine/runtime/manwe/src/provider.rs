//! Minimal provider catalog for the gateway bootstrap.
//!
//! This is intentionally small: keep `ProviderCatalog` to one upstream per
//! placeholder slot so it reassembles the real config later.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderDefinition {
    pub id: String,
    pub name: String,
    pub model_id: String,
    pub base_url: String,
    pub api_key_env: Option<String>,
    pub transport: ProviderTransport,
    pub capabilities: ProviderCapabilities,
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
        api_key_env: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            model_id: model_id.into(),
            base_url: base_url.into(),
            api_key_env: Some(api_key_env.into()),
            transport: ProviderTransport::OpenAICompatible,
            capabilities: ProviderCapabilities {
                streaming: true,
                tool_calls: true,
                structured_output: true,
            },
        }
    }
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
        Self::new(vec![
            ProviderDefinition::openai_compatible(
                "local_placeholder",
                "Placeholder Provider",
                "placeholder-model",
                "http://127.0.0.1:7171/v1",
                "ARDA_MANWE_PLACEHOLDER_API_KEY",
            ),
        ])
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

    pub fn local_placeholder(&self) -> Option<&ProviderDefinition> {
        self.get("local_placeholder")
    }
}
