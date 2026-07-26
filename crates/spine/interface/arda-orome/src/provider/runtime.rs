// sigil: REPAIR
//! Provider runtime types, kept in their own module to avoid graph cycles.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use super::orchestration::{DispatchMetrics, DispatchPolicy, EdgeCommunicationPolicy};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchReceipt {
    pub dispatched: bool,
    pub attempts: usize,
    pub streaming: bool,
    pub chunks_sent: usize,
    pub provider_id: String,
    pub error: Option<String>,
    pub timed_out: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderRuntime {
    pub providers: Vec<ProviderConfig>,
    pub(crate) dispatch_policy: DispatchPolicy,
    pub(crate) edge_policy: EdgeCommunicationPolicy,
    pub(crate) dispatch_metrics: Arc<DispatchMetrics>,
}

impl ProviderRuntime {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        Self {
            providers,
            dispatch_policy: DispatchPolicy::default(),
            edge_policy: EdgeCommunicationPolicy::default(),
            dispatch_metrics: Arc::new(DispatchMetrics::default()),
        }
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
            dispatch_policy: DispatchPolicy::default(),
            edge_policy: EdgeCommunicationPolicy::default(),
            dispatch_metrics: Arc::new(DispatchMetrics::default()),
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
