// sigil: REPAIR
use serde_json::Value;
use std::fmt;
use thiserror::Error;

/// Top-level error type for `arda-rumil`. All variants are review/audit scoped;
/// no variant grants or implies execution authority.
#[derive(Debug, Error)]
pub enum RumilError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),

    #[error("provider failed: {0}")]
    ProviderFailed(String),

    #[error("provider timed out after {timeout_seconds}s")]
    ProviderTimedOut { timeout_seconds: u64 },

    #[error("malformed output: {0}")]
    MalformedOutput(String),

    #[error("denied by budget: {0}")]
    DeniedByBudget(String),

    #[error("path rejected: {0}")]
    PathRejected(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("toml error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("unsupported contract version: {0}")]
    UnsupportedVersion(String),

    #[error("packet validation failed: {0}")]
    PacketValidation(String),

    #[error("walk error: {0}")]
    Walk(String),

    #[error("hash error: {0}")]
    #[cfg(feature = "crypto")]
    Hash(String),
}

pub type Result<T> = std::result::Result<T, RumilError>;

/// A non-empty packet kind tag, e.g. `arda.rumil.audit-request.v1`.
/// Used for envelope validation without coupling to the full struct set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PacketKind(pub String);

impl fmt::Display for PacketKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl PacketKind {
    pub fn from_parts(domain: &str, name: &str, version: &str) -> Self {
        Self(format!("{domain}.{name}.{version}"))
    }

    pub fn domain(&self) -> Option<String> {
        let parts: Vec<&str> = self.0.split('.').collect();
        if parts.len() >= 3 {
            Some(parts[0..parts.len() - 2].join("."))
        } else {
            None
        }
    }

    pub fn name(&self) -> Option<&str> {
        let parts: Vec<&str> = self.0.split('.').collect();
        if parts.len() >= 3 {
            Some(parts[parts.len() - 2])
        } else {
            None
        }
    }

    pub fn version(&self) -> Option<&str> {
        let parts: Vec<&str> = self.0.split('.').collect();
        if let Some(last) = parts.last() {
            Some(last)
        } else {
            None
        }
    }
}

/// Lightweight envelope wrapper for round-trip tests. The real packets
/// live in `contracts.rs`; this helper lets tests verify envelope validation
/// without pulling in every struct.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvelopeProbe {
    pub kind: String,
    pub payload: Value,
}

impl EnvelopeProbe {
    pub fn probe(value: &Value) -> Option<Self> {
        let kind = value.get("kind")?.as_str()?.to_string();
        let payload = value.get("payload")?.clone();
        Some(Self { kind, payload })
    }
}
