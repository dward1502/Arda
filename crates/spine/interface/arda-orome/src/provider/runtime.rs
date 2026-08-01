// sigil: REPAIR
//! Provider runtime types, kept in their own module to avoid graph cycles.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(all(test, feature = "service-runtime"))]
use std::collections::HashMap;

#[cfg(all(test, feature = "service-runtime"))]
use crate::mcp::McpChannel;

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
    /// Provider-assigned receipt ID. `None` means live delivery is unproven.
    #[serde(default)]
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
    pub timed_out: bool,
}

impl DispatchReceipt {
    pub fn delivery_proven(&self) -> bool {
        self.dispatched && self.provider_message_id.is_some()
    }
}

#[derive(Clone)]
pub struct ProviderRuntime {
    pub providers: Vec<ProviderConfig>,
    pub(crate) dispatch_policy: DispatchPolicy,
    pub(crate) edge_policy: EdgeCommunicationPolicy,
    pub(crate) dispatch_metrics: Arc<DispatchMetrics>,
    #[cfg(all(test, feature = "service-runtime"))]
    pub(crate) test_channels: HashMap<String, Arc<dyn McpChannel>>,
}

impl std::fmt::Debug for ProviderRuntime {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ProviderRuntime")
            .field("providers", &self.providers)
            .field("dispatch_policy", &self.dispatch_policy)
            .field("edge_policy", &self.edge_policy)
            .finish_non_exhaustive()
    }
}

impl ProviderRuntime {
    pub fn new(providers: Vec<ProviderConfig>) -> Self {
        Self {
            providers,
            dispatch_policy: DispatchPolicy::default(),
            edge_policy: EdgeCommunicationPolicy::default(),
            dispatch_metrics: Arc::new(DispatchMetrics::default()),
            #[cfg(all(test, feature = "service-runtime"))]
            test_channels: HashMap::new(),
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
            #[cfg(all(test, feature = "service-runtime"))]
            test_channels: HashMap::new(),
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
