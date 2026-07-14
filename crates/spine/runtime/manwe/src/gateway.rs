//! Gateway span + provider record types.
//!
//! Frozen contract: `manwe` is the static local root gateway. These types
//! describe the gateway endpoint and the provider catalog records without
//! pulling in any adaptive-routing / quota-mesh machinery. They exist so the
//! charon→gateway bridge (`charon_remote`) can name a concrete gateway and its
//! providers while the real authority wiring is ported incrementally.

use serde::{Deserialize, Serialize};

/// A reference to a running `manwe` gateway endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpannedManweGateway {
    /// Full chat-completions endpoint, e.g.
    /// `http://127.0.0.1:7171/v1/chat/completions`.
    pub endpoint: String,
}

impl SpannedManweGateway {
    /// The frozen default: local gateway on `127.0.0.1:7171`.
    pub fn local() -> Self {
        Self {
            endpoint: "http://127.0.0.1:7171/v1/chat/completions".to_string(),
        }
    }
}

/// A single provider entry in the gateway catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRecord {
    pub id: String,
    pub name: String,
    pub base_url: String,
    #[serde(default)]
    pub models: Vec<String>,
}

impl ProviderRecord {
    pub fn openai_compatible(
        id: impl Into<String>,
        name: impl Into<String>,
        base_url: impl Into<String>,
        models: Vec<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            base_url: base_url.into(),
            models,
        }
    }
}
