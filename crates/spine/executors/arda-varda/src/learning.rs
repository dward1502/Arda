use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const KNOWLEDGE_DELTA_SCHEMA_VERSION: &str = "arda.athena.knowledge_delta.v1";
pub const KNOWLEDGE_DELTA_RELATIVE_PATH: &str = "data/athena/knowledge_deltas.jsonl";

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct KnowledgeDelta {
    pub schema_version: String,
    pub source_path: String,
    pub confidence: f32,
    pub uncertainty: f32,
    pub created_at_unix: u64,
    pub expires_at_unix: u64,
    pub delta_content: String,
}

impl KnowledgeDelta {
    pub fn new(
        source: &str,
        confidence: f32,
        uncertainty: f32,
        content: &str,
        ttl_secs: u64,
    ) -> Self {
        let now = unix_now();

        Self {
            schema_version: KNOWLEDGE_DELTA_SCHEMA_VERSION.to_string(),
            source_path: source.to_string(),
            confidence,
            uncertainty,
            created_at_unix: now,
            expires_at_unix: now.saturating_add(ttl_secs),
            delta_content: content.to_string(),
        }
    }

    pub fn is_expired(&self) -> bool {
        unix_now() > self.expires_at_unix
    }

    pub fn is_valid_contract_shape(&self) -> bool {
        self.schema_version == KNOWLEDGE_DELTA_SCHEMA_VERSION
            && !self.source_path.trim().is_empty()
            && !self.delta_content.trim().is_empty()
            && (0.0..=1.0).contains(&self.confidence)
            && (0.0..=1.0).contains(&self.uncertainty)
            && self.expires_at_unix >= self.created_at_unix
    }
}

pub fn emit_delta(delta: &KnowledgeDelta) -> Result<(), std::io::Error> {
    emit_delta_to_root(delta, default_workspace_root())
}

pub fn emit_delta_to_root(
    delta: &KnowledgeDelta,
    root: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    emit_delta_to_file(delta, root.as_ref().join(KNOWLEDGE_DELTA_RELATIVE_PATH))
}

pub fn emit_delta_to_file(
    delta: &KnowledgeDelta,
    file_path: impl AsRef<Path>,
) -> Result<(), std::io::Error> {
    if !delta.is_valid_contract_shape() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "knowledge delta does not satisfy arda.athena.knowledge_delta.v1",
        ));
    }

    let file_path = file_path.as_ref();
    if let Some(parent) = file_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(file_path)?;

    let json = serde_json::to_string(delta).map_err(std::io::Error::other)?;
    writeln!(file, "{}", json)?;

    Ok(())
}

fn default_workspace_root() -> PathBuf {
    std::env::var_os("ARDA_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
