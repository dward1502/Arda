use super::{AppendOutcome, RunEventKind, RunStore, RunStoreError};
use arda_core::run_graph::RunId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceMeasurementSource {
    Observed,
    DefaultFallback,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceUsageDraft {
    pub idempotency_key: String,
    pub source: ResourceMeasurementSource,
    pub provider_id: Option<String>,
    pub local_joulework: f64,
    pub hosted_cost_usd: f64,
    pub hosted_requests: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes: Option<String>,
}

impl ResourceUsageDraft {
    fn validate(&self) -> Result<(), ResourceLedgerError> {
        if self.idempotency_key.trim().is_empty() {
            return Err(ResourceLedgerError::EmptyIdempotencyKey);
        }
        if !self.local_joulework.is_finite()
            || !self.hosted_cost_usd.is_finite()
            || self.local_joulework < 0.0
            || self.hosted_cost_usd < 0.0
        {
            return Err(ResourceLedgerError::InvalidAmount);
        }
        if self.provider_id.as_deref().is_some_and(str::is_empty) {
            return Err(ResourceLedgerError::EmptyProviderId);
        }
        if self.hosted_requests > 0 && self.provider_id.is_none() {
            return Err(ResourceLedgerError::ProviderRequired);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLedgerEntry {
    pub schema_version: String,
    pub sequence: u64,
    pub run_id: RunId,
    pub idempotency_key: String,
    pub source: ResourceMeasurementSource,
    pub provider_id: Option<String>,
    pub local_joulework: f64,
    pub hosted_cost_usd: f64,
    pub hosted_requests: u64,
    pub supersedes: Option<String>,
    pub recorded_after_run_completion: bool,
    pub recorded_at_unix_ms: u128,
}

impl ResourceLedgerEntry {
    pub const SCHEMA_VERSION: &'static str = "arda.resource-ledger-entry.v1";

    fn matches_draft(&self, run_id: &RunId, draft: &ResourceUsageDraft) -> bool {
        self.run_id == *run_id
            && self.idempotency_key == draft.idempotency_key
            && self.source == draft.source
            && self.provider_id == draft.provider_id
            && self.local_joulework == draft.local_joulework
            && self.hosted_cost_usd == draft.hosted_cost_usd
            && self.hosted_requests == draft.hosted_requests
            && self.supersedes == draft.supersedes
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResourceRollup {
    pub local_joulework: f64,
    pub hosted_cost_usd: f64,
    pub hosted_requests: u64,
    pub observed_entries: usize,
    pub default_entries: usize,
}

impl RunStore {
    pub fn append_resource_usage(
        &self,
        draft: ResourceUsageDraft,
    ) -> Result<AppendOutcome, ResourceLedgerError> {
        draft.validate()?;
        let entries = self.read_resource_ledger()?;
        if let Some(existing) = entries
            .iter()
            .find(|entry| entry.idempotency_key == draft.idempotency_key)
        {
            if !existing.matches_draft(self.run_id(), &draft) {
                return Err(ResourceLedgerError::IdempotencyConflict {
                    key: draft.idempotency_key,
                });
            }
            return Ok(AppendOutcome::AlreadyApplied {
                sequence: existing.sequence,
            });
        }
        if let Some(superseded_key) = &draft.supersedes {
            let superseded = entries
                .iter()
                .find(|entry| entry.idempotency_key == *superseded_key)
                .ok_or_else(|| ResourceLedgerError::UnknownSupersededEntry {
                    key: superseded_key.clone(),
                })?;
            if superseded.run_id != *self.run_id() {
                return Err(ResourceLedgerError::CrossRunSupersession);
            }
            if entries
                .iter()
                .any(|entry| entry.supersedes.as_deref() == Some(superseded_key))
            {
                return Err(ResourceLedgerError::AlreadySuperseded {
                    key: superseded_key.clone(),
                });
            }
        }

        let recorded_after_run_completion = self
            .recover()?
            .events
            .iter()
            .any(|event| matches!(event.kind, RunEventKind::ResultProjected));
        let entry = ResourceLedgerEntry {
            schema_version: ResourceLedgerEntry::SCHEMA_VERSION.to_string(),
            sequence: entries.len() as u64 + 1,
            run_id: self.run_id().clone(),
            idempotency_key: draft.idempotency_key,
            source: draft.source,
            provider_id: draft.provider_id,
            local_joulework: draft.local_joulework,
            hosted_cost_usd: draft.hosted_cost_usd,
            hosted_requests: draft.hosted_requests,
            supersedes: draft.supersedes,
            recorded_after_run_completion,
            recorded_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        };
        append_json_line(self.resource_ledger_path(), &entry)?;
        Ok(AppendOutcome::Appended {
            sequence: entry.sequence,
        })
    }

    pub fn read_resource_ledger(&self) -> Result<Vec<ResourceLedgerEntry>, ResourceLedgerError> {
        let path = self.resource_ledger_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => return Err(ResourceLedgerError::Io { path, source }),
        };
        if !raw.is_empty() && !raw.ends_with('\n') {
            return Err(ResourceLedgerError::CorruptTail);
        }
        raw.lines()
            .enumerate()
            .map(|(index, line)| {
                let entry: ResourceLedgerEntry = serde_json::from_str(line).map_err(|error| {
                    ResourceLedgerError::CorruptEntry {
                        line: index + 1,
                        message: error.to_string(),
                    }
                })?;
                let expected = index as u64 + 1;
                if entry.sequence != expected {
                    return Err(ResourceLedgerError::SequenceGap {
                        expected,
                        actual: entry.sequence,
                    });
                }
                if entry.schema_version != ResourceLedgerEntry::SCHEMA_VERSION {
                    return Err(ResourceLedgerError::UnsupportedVersion(
                        entry.schema_version,
                    ));
                }
                Ok(entry)
            })
            .collect()
    }

    pub fn resource_rollup_since(
        &self,
        cutoff_unix_ms: u128,
        provider_id: Option<&str>,
    ) -> Result<ResourceRollup, ResourceLedgerError> {
        let entries = self.read_resource_ledger()?;
        let superseded = entries
            .iter()
            .filter_map(|entry| entry.supersedes.clone())
            .collect::<BTreeSet<_>>();
        let mut rollup = ResourceRollup::default();
        for entry in entries.iter().filter(|entry| {
            entry.recorded_at_unix_ms >= cutoff_unix_ms
                && !superseded.contains(&entry.idempotency_key)
                && provider_id.is_none_or(|provider| entry.provider_id.as_deref() == Some(provider))
        }) {
            rollup.local_joulework += entry.local_joulework;
            rollup.hosted_cost_usd += entry.hosted_cost_usd;
            rollup.hosted_requests += entry.hosted_requests;
            match entry.source {
                ResourceMeasurementSource::Observed => rollup.observed_entries += 1,
                ResourceMeasurementSource::DefaultFallback => rollup.default_entries += 1,
            }
        }
        Ok(rollup)
    }
}

fn append_json_line(path: PathBuf, value: &impl Serialize) -> Result<(), ResourceLedgerError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ResourceLedgerError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let mut bytes = serde_json::to_vec(value).map_err(ResourceLedgerError::Serialize)?;
    bytes.push(b'\n');
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| ResourceLedgerError::Io {
            path: path.clone(),
            source,
        })?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|source| ResourceLedgerError::Io { path, source })
}

#[derive(Debug, thiserror::Error)]
pub enum ResourceLedgerError {
    #[error("resource ledger I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize resource ledger entry: {0}")]
    Serialize(serde_json::Error),
    #[error("resource ledger tail is not newline-terminated")]
    CorruptTail,
    #[error("corrupt resource ledger entry at line {line}: {message}")]
    CorruptEntry { line: usize, message: String },
    #[error("resource ledger sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    #[error("unsupported resource ledger version: {0}")]
    UnsupportedVersion(String),
    #[error("resource idempotency key cannot be empty")]
    EmptyIdempotencyKey,
    #[error("resource idempotency key {key:?} conflicts with an existing entry")]
    IdempotencyConflict { key: String },
    #[error("resource amounts must be finite and non-negative")]
    InvalidAmount,
    #[error("provider id cannot be empty")]
    EmptyProviderId,
    #[error("hosted requests require a provider id")]
    ProviderRequired,
    #[error("superseded resource entry {key:?} does not exist")]
    UnknownSupersededEntry { key: String },
    #[error("resource entries cannot supersede usage from another run")]
    CrossRunSupersession,
    #[error("resource entry {key:?} was already superseded")]
    AlreadySuperseded { key: String },
    #[error("run store failed while recording resource usage: {0}")]
    Store(#[from] RunStoreError),
}
