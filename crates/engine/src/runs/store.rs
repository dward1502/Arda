use super::RecoveredRun;
use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunGraphError, RunId};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEventKind {
    Planned {
        project_id: String,
        approval_id: String,
    },
    NodeTransition {
        state: NodeState,
    },
    Cancelled {
        reason: String,
    },
    EvidenceLinked {
        evidence_id: String,
        evidence_path: String,
        authority: String,
    },
    ResultProjected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub schema_version: String,
    pub sequence: u64,
    pub run_id: RunId,
    pub node_id: NodeId,
    pub idempotency_key: String,
    pub kind: RunEventKind,
    pub receipt_digest: Option<String>,
    pub recorded_at_unix_ms: u128,
}

impl RunEvent {
    pub const SCHEMA_VERSION: &'static str = "arda.run-event.v1";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEventDraft {
    pub node_id: NodeId,
    pub idempotency_key: String,
    pub kind: RunEventKind,
    pub receipt_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendOutcome {
    Appended { sequence: u64 },
    AlreadyApplied { sequence: u64 },
}

#[derive(Debug, Clone)]
pub struct RunStore {
    run_id: RunId,
    directory: PathBuf,
}

impl RunStore {
    pub fn open(root: impl AsRef<Path>, run_id: RunId) -> Result<Self, RunStoreError> {
        let directory = root.as_ref().join("data/runs").join(run_id.as_str());
        fs::create_dir_all(&directory).map_err(|source| RunStoreError::Io {
            path: directory.clone(),
            source,
        })?;
        Ok(Self { run_id, directory })
    }

    pub fn events_path(&self) -> PathBuf {
        self.directory.join("events.jsonl")
    }

    pub fn checkpoint_path(&self) -> PathBuf {
        self.directory.join("checkpoint.json")
    }

    pub fn result_path(&self) -> PathBuf {
        self.directory.join("result.json")
    }

    pub fn execution_receipt_path(&self, node_id: &NodeId) -> PathBuf {
        self.directory
            .join("execution-receipts")
            .join(format!("{}.json", node_id.as_str()))
    }

    pub fn append(&self, draft: RunEventDraft) -> Result<AppendOutcome, RunStoreError> {
        if draft.idempotency_key.trim().is_empty() {
            return Err(RunStoreError::EmptyIdempotencyKey);
        }
        let recovered = self.recover()?;
        if let Some(sequence) = recovered
            .applied_idempotency_keys
            .get(&draft.idempotency_key)
        {
            return Ok(AppendOutcome::AlreadyApplied {
                sequence: *sequence,
            });
        }

        let sequence = recovered.events.len() as u64 + 1;
        let event = RunEvent {
            schema_version: RunEvent::SCHEMA_VERSION.to_string(),
            sequence,
            run_id: self.run_id.clone(),
            node_id: draft.node_id,
            idempotency_key: draft.idempotency_key,
            kind: draft.kind,
            receipt_digest: draft.receipt_digest,
            recorded_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        };
        let mut bytes = serde_json::to_vec(&event).map_err(RunStoreError::Serialize)?;
        bytes.push(b'\n');
        let path = self.events_path();
        let mut journal = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| RunStoreError::Io {
                path: path.clone(),
                source,
            })?;
        journal
            .write_all(&bytes)
            .and_then(|_| journal.sync_all())
            .map_err(|source| RunStoreError::Io { path, source })?;
        Ok(AppendOutcome::Appended { sequence })
    }

    pub fn recover(&self) -> Result<RecoveredRun, RunStoreError> {
        let path = self.events_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(RunStoreError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };
        if !raw.is_empty() && !raw.ends_with('\n') {
            return Err(RunStoreError::CorruptJournal {
                line: raw.lines().count(),
                message: "journal tail is not newline-terminated".to_string(),
            });
        }

        let mut events = Vec::new();
        for (index, line) in raw.lines().enumerate() {
            let line_number = index + 1;
            let event: RunEvent =
                serde_json::from_str(line).map_err(|error| RunStoreError::CorruptJournal {
                    line: line_number,
                    message: error.to_string(),
                })?;
            let expected = line_number as u64;
            if event.sequence != expected {
                return Err(RunStoreError::SequenceGap {
                    expected,
                    actual: event.sequence,
                });
            }
            if event.schema_version != RunEvent::SCHEMA_VERSION {
                return Err(RunStoreError::UnsupportedEventVersion(event.schema_version));
            }
            if event.run_id != self.run_id {
                return Err(RunStoreError::RunIdMismatch {
                    expected: self.run_id.clone(),
                    actual: event.run_id,
                });
            }
            events.push(event);
        }

        let checkpoint_path = self.checkpoint_path();
        let checkpoint = match fs::read_to_string(&checkpoint_path) {
            Ok(raw) => Some(serde_json::from_str::<RunGraph>(&raw).map_err(|error| {
                RunStoreError::CorruptCheckpoint {
                    path: checkpoint_path,
                    message: error.to_string(),
                }
            })?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(RunStoreError::Io {
                    path: checkpoint_path,
                    source,
                });
            }
        };
        Ok(RecoveredRun::from_parts(events, checkpoint))
    }

    pub fn write_checkpoint(&self, graph: &RunGraph) -> Result<(), RunStoreError> {
        graph.validate().map_err(RunStoreError::Graph)?;
        let bytes = serde_json::to_vec_pretty(graph).map_err(RunStoreError::Serialize)?;
        atomic_write(&self.checkpoint_path(), &bytes)
    }

    pub fn write_result(&self, result: &serde_json::Value) -> Result<(), RunStoreError> {
        let bytes = serde_json::to_vec_pretty(result).map_err(RunStoreError::Serialize)?;
        atomic_write(&self.result_path(), &bytes)
    }

    pub fn read_result(&self) -> Result<Option<serde_json::Value>, RunStoreError> {
        let path = self.result_path();
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(RunStoreError::Serialize),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(RunStoreError::Io { path, source }),
        }
    }

    pub fn write_execution_receipt(
        &self,
        node_id: &NodeId,
        receipt: &serde_json::Value,
    ) -> Result<(), RunStoreError> {
        let path = self.execution_receipt_path(node_id);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RunStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let bytes = serde_json::to_vec_pretty(receipt).map_err(RunStoreError::Serialize)?;
        atomic_write(&path, &bytes)
    }

    pub fn read_execution_receipt(
        &self,
        node_id: &NodeId,
    ) -> Result<Option<serde_json::Value>, RunStoreError> {
        let path = self.execution_receipt_path(node_id);
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(RunStoreError::Serialize),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(RunStoreError::Io { path, source }),
        }
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), RunStoreError> {
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|source| RunStoreError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|source| RunStoreError::Io {
                path: temporary.clone(),
                source,
            })?;
        drop(file);
        fs::rename(&temporary, path).map_err(|source| RunStoreError::Io {
            path: path.to_path_buf(),
            source,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug, thiserror::Error)]
pub enum RunStoreError {
    #[error("run store I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize run data: {0}")]
    Serialize(serde_json::Error),
    #[error("corrupt run journal at line {line}: {message}")]
    CorruptJournal { line: usize, message: String },
    #[error("run journal sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    #[error("unsupported run event version: {0}")]
    UnsupportedEventVersion(String),
    #[error("journal run id mismatch: expected {expected:?}, got {actual:?}")]
    RunIdMismatch { expected: RunId, actual: RunId },
    #[error("corrupt checkpoint at {path}: {message}")]
    CorruptCheckpoint { path: PathBuf, message: String },
    #[error("idempotency key cannot be empty")]
    EmptyIdempotencyKey,
    #[error("run graph validation failed: {0}")]
    Graph(#[source] RunGraphError),
}
