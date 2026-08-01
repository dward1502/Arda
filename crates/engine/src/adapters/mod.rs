//! Bounded project-adapter process boundary.

mod hermes;
mod jsonl;

pub use hermes::{
    HermesAdapter, HermesAdapterConfig, HermesAdapterError, HermesArtifactEvidence,
    HermesExecutionReceipt, HermesNodeTask, HermesReceiptStatus, HermesTestEvidence,
    HermesToolEvidence, HermesToolsets, NormalizedHermesUsage,
};
pub use jsonl::JsonlAdapter;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::Duration;
use tokio::sync::watch;

pub const ADAPTER_SCHEMA_VERSION: &str = "arda.project-adapter.v1";

#[derive(Clone, Debug)]
pub struct AdapterProcessConfig {
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub expected_adapter: String,
    pub expected_adapter_version: String,
    pub project_root: PathBuf,
    pub cwd: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub environment_allowlist: BTreeSet<String>,
    pub capabilities: BTreeSet<String>,
    pub timeout: Duration,
    pub cancellation_grace: Duration,
    pub max_line_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct AdapterRequest {
    pub id: String,
    pub operation: String,
    pub arguments: Value,
    pub timeout: Duration,
    pub required_capabilities: BTreeSet<String>,
    pub idempotency_key: String,
    pub recovery_token: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AdapterStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct AdapterProvenance {
    pub adapter: String,
    pub adapter_version: String,
    pub cwd: PathBuf,
    pub started_at: String,
    pub finished_at: String,
    pub request_digest: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AdapterResult {
    pub status: AdapterStatus,
    pub output: Value,
    pub provenance: AdapterProvenance,
    pub recovery_token: Option<String>,
}

#[derive(Clone, Debug)]
pub struct AdapterCancellation {
    sender: watch::Sender<bool>,
}

impl AdapterCancellation {
    pub fn new() -> Self {
        let (sender, _) = watch::channel(false);
        Self { sender }
    }

    pub fn cancel(&self) {
        let _ = self.sender.send(true);
    }

    pub(crate) fn subscribe(&self) -> watch::Receiver<bool> {
        self.sender.subscribe()
    }
}

impl Default for AdapterCancellation {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("adapter executable must be absolute: {0}")]
    ExecutableNotAbsolute(PathBuf),
    #[error("adapter executable is not a regular file: {0}")]
    InvalidExecutable(PathBuf),
    #[error("invalid adapter project root: {0}")]
    InvalidProjectRoot(PathBuf),
    #[error("invalid adapter working directory: {0}")]
    InvalidCwd(PathBuf),
    #[error("adapter working directory {cwd} is outside project root {project_root}")]
    CwdOutsideProject { cwd: PathBuf, project_root: PathBuf },
    #[error("adapter environment key is not allowlisted: {0}")]
    EnvironmentDenied(String),
    #[error("invalid adapter configuration: {0}")]
    InvalidConfig(String),
    #[error("failed to spawn adapter: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("adapter I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("adapter protocol violation: {0}")]
    Protocol(String),
    #[error("adapter denied capability {capability}: {reason}")]
    DeniedCapability { capability: String, reason: String },
    #[error("adapter timed out")]
    Timeout,
    #[error("adapter was cancelled")]
    Cancelled,
}
