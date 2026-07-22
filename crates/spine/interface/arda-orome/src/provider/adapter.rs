// sigil: REPAIR
//! Provider adapter trait and concrete capability surface.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Discord,
    Slack,
    Http,
    Email,
    Matrix,
    Custom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProviderCapabilities {
    pub streaming: bool,
    pub message_limit_chars: Option<usize>,
    pub supports_channels: bool,
    pub supports_direct: bool,
    pub supports_embeds: bool,
}

impl ProviderCapabilities {
    pub const fn new(streaming: bool) -> Self {
        Self {
            streaming,
            message_limit_chars: None,
            supports_channels: true,
            supports_direct: true,
            supports_embeds: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAdapter {
    pub kind: ProviderKind,
    pub id: String,
    pub name: String,
    pub capabilities: ProviderCapabilities,
    pub endpoint: String,
    pub metadata: serde_json::Value,
}

impl ProviderAdapter {
    pub fn new(
        kind: ProviderKind,
        id: impl Into<String>,
        name: impl Into<String>,
        capabilities: ProviderCapabilities,
        endpoint: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            id: id.into(),
            name: name.into(),
            capabilities,
            endpoint: endpoint.into(),
            metadata: serde_json::json!({}),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("provider adapter error: {message}")]
pub struct ProviderAdapterError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl ProviderAdapterError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }
}
