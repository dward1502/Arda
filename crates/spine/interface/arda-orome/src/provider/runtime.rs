// sigil: REPAIR
//! Provider runtime types, kept in their own module to avoid graph cycles.

use serde::{Deserialize, Serialize};

/// Canonical provider family used by service surfaces.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderType {
    #[default]
    Discord,
    Slack,
    Http,
    Email,
    Matrix,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub kind: ProviderType,
    pub name: String,
    pub endpoint: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct DispatchReceipt {
    pub dispatched: bool,
    pub attempts: usize,
    pub streaming: bool,
    pub chunks_sent: usize,
    pub provider_id: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderRuntime {
    pub providers: Vec<ProviderConfig>,
}

impl ProviderRuntime {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        Self { providers }
    }

    pub fn from_defaults() -> Self {
        Self {
            providers: vec![ProviderConfig {
                id: "discord".into(),
                kind: ProviderType::Discord,
                name: "Discord".into(),
                endpoint: "".into(),
                capabilities: vec![],
            }],
        }
    }

    pub fn select(&self, provider_id: &str) -> Option<&ProviderRuntime> {
        self.providers
            .iter()
            .any(|p| p.id == provider_id || p.id == "default")
            .then_some(self)
    }
}

impl Default for ProviderRuntime {
    fn default() -> Self {
        Self::from_defaults()
    }
}
