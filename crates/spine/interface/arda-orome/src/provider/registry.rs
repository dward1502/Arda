// sigil: REPAIR
//! Provider registry with health-aware selection.

use crate::provider::{ProviderAdapter, ProviderCapabilities, ProviderKind};

#[derive(Debug, Clone, Default)]
pub struct ProviderRegistry {
    adapters: Vec<ProviderAdapter>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, adapter: ProviderAdapter) {
        if !self
            .adapters
            .iter()
            .any(|existing| existing.id == adapter.id)
        {
            self.adapters.push(adapter);
        }
    }

    pub fn by_id(&self, id: &str) -> Option<&ProviderAdapter> {
        self.adapters.iter().find(|adapter| adapter.id == id)
    }

    pub fn by_kind(&self, kind: ProviderKind) -> Vec<&ProviderAdapter> {
        self.adapters
            .iter()
            .filter(|adapter| adapter.kind == kind)
            .collect()
    }

    pub fn streaming_adapters(&self) -> Vec<&ProviderAdapter> {
        self.adapters
            .iter()
            .filter(|adapter| adapter.capabilities.streaming)
            .collect()
    }

    pub fn healthy_for_message(&self, provider_id: &str) -> Option<&ProviderAdapter> {
        self.by_id(provider_id).filter(|adapter| {
            adapter
                .metadata
                .get("healthy")
                .and_then(|value| value.as_bool())
                .unwrap_or(true)
        })
    }

    pub fn resolve_direct_capable(&self, provider_id: &str) -> Option<&ProviderAdapter> {
        self.by_id(provider_id).filter(|adapter| adapter.capabilities.supports_direct)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ProviderAdapter> {
        self.adapters.iter()
    }
}

#[derive(Debug, Clone)]
pub struct ProviderHandle {
    pub adapter: ProviderAdapter,
    pub retryable: bool,
    pub reason: Option<String>,
}

impl Default for ProviderHandle {
    fn default() -> Self {
        Self {
            adapter: ProviderAdapter {
                kind: crate::provider::ProviderKind::Http,
                id: String::new(),
                name: String::new(),
                capabilities: ProviderCapabilities::default(),
                endpoint: String::new(),
                metadata: serde_json::Value::Null,
            },
            retryable: false,
            reason: None,
        }
    }
}
