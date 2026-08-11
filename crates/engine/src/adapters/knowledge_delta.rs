//! Governed external finding -> Vairë promotion -> named-consumer outcome loop.

use arda_vaire::{ApprovedKnowledgeDelta, GovernedKnowledgeReceipt, MnemosyneService};
use arda_varda::{EvaluationDecision, ExternalEvaluationReceipt};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::os::{fd::AsRawFd, unix::fs::PermissionsExt};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GovernedKnowledgeDelta {
    pub delta_id: String,
    pub source_reference: String,
    pub source_digest: String,
    pub confidence: f64,
    pub scope: String,
    pub consumer_id: String,
    pub retention_policy: String,
    pub revocation_operation: String,
    pub content: String,
    pub correction_of: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeConsumerOutcome {
    Used,
    Rejected,
    Superseded,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgePromotionReceipt {
    pub schema_version: String,
    pub delta_id: String,
    pub observation_id: String,
    pub evaluation_reference: String,
    pub approval_reference: String,
    pub source_reference: String,
    pub source_digest: String,
    pub confidence_millionths: u32,
    pub scope: String,
    pub consumer_id: String,
    pub retention_policy: String,
    pub revocation_operation: String,
    pub content_digest: String,
    pub vaire_receipt_id: String,
    pub promoted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KnowledgeOutcomeReceipt {
    pub schema_version: String,
    pub delta_id: String,
    pub consumer_id: String,
    pub outcome: KnowledgeConsumerOutcome,
    pub rationale: String,
    pub retrieved_memory_id: Option<String>,
    pub recorded_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct KnowledgeDeltaLoop {
    root: PathBuf,
    memory: MnemosyneService,
}

impl KnowledgeDeltaLoop {
    pub fn new(root: &Path) -> Result<Self, KnowledgeDeltaError> {
        Ok(Self {
            root: root.to_path_buf(),
            memory: MnemosyneService::new(root.join("data/vaire"))?,
        })
    }

    pub fn promote(
        &self,
        evaluation: &ExternalEvaluationReceipt,
        delta: GovernedKnowledgeDelta,
        promoted_at: DateTime<Utc>,
    ) -> Result<KnowledgePromotionReceipt, KnowledgeDeltaError> {
        validate_delta(&delta)?;
        if evaluation.decision != EvaluationDecision::ApprovedSafeLocal {
            return Err(KnowledgeDeltaError::EvaluationNotApproved);
        }
        let approval_reference = evaluation
            .approval_reference
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .ok_or(KnowledgeDeltaError::MissingApproval)?;
        if evaluation.content_hash != digest_hex(delta.content.as_bytes())
            || evaluation.normalized_url != delta.source_reference
        {
            return Err(KnowledgeDeltaError::SourceMismatch);
        }

        let path = self
            .root
            .join("data/assimilation/knowledge_promotions.jsonl");
        let mut ledger = LockedLedger::open(&path)?;
        if let Some(existing) = read_jsonl::<KnowledgePromotionReceipt>(&mut ledger.file)?
            .into_iter()
            .find(|receipt| receipt.delta_id == delta.delta_id)
        {
            if existing.source_digest == delta.source_digest
                && existing.consumer_id == delta.consumer_id
            {
                return Ok(existing);
            }
            return Err(KnowledgeDeltaError::DeltaConflict(delta.delta_id));
        }

        let vaire: GovernedKnowledgeReceipt =
            self.memory.ingest_approved_delta(ApprovedKnowledgeDelta {
                delta_id: delta.delta_id.clone(),
                source_reference: format!(
                    "{}#approval={approval_reference}",
                    delta.source_reference
                ),
                warden_observation_id: evaluation.observation_id.clone(),
                varda_evaluation_id: evaluation_reference(evaluation),
                approval_reference: approval_reference.to_string(),
                content: delta.content.clone(),
                correction_of: delta.correction_of,
            })?;
        let receipt = KnowledgePromotionReceipt {
            schema_version: "arda.knowledge-promotion-receipt.v1".to_string(),
            delta_id: delta.delta_id,
            observation_id: evaluation.observation_id.clone(),
            evaluation_reference: evaluation_reference(evaluation),
            approval_reference: approval_reference.to_string(),
            source_reference: delta.source_reference,
            source_digest: delta.source_digest,
            confidence_millionths: (delta.confidence * 1_000_000.0).round() as u32,
            scope: delta.scope,
            consumer_id: delta.consumer_id,
            retention_policy: delta.retention_policy,
            revocation_operation: delta.revocation_operation,
            content_digest: format!("sha256:{}", digest_hex(delta.content.as_bytes())),
            vaire_receipt_id: vaire.receipt_id,
            promoted_at,
        };
        append_jsonl(&mut ledger.file, &receipt)?;
        Ok(receipt)
    }

    pub fn consume(
        &self,
        delta_id: &str,
        consumer_id: &str,
        query: &str,
        outcome: KnowledgeConsumerOutcome,
        rationale: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<KnowledgeOutcomeReceipt, KnowledgeDeltaError> {
        let promotions = self.promotions()?;
        let promotion = promotions
            .iter()
            .find(|receipt| receipt.delta_id == delta_id)
            .ok_or_else(|| KnowledgeDeltaError::UnknownDelta(delta_id.to_string()))?;
        if promotion.consumer_id != consumer_id {
            return Err(KnowledgeDeltaError::WrongConsumer {
                expected: promotion.consumer_id.clone(),
                actual: consumer_id.to_string(),
            });
        }
        if rationale.trim().is_empty() {
            return Err(KnowledgeDeltaError::MissingField("rationale"));
        }
        let retrieved_memory_id = if outcome == KnowledgeConsumerOutcome::Used {
            self.memory
                .recall_relevant(query, 24 * 365, Some("athena"), None, 20)?
                .into_iter()
                .find(|entry| {
                    entry
                        .tags
                        .iter()
                        .any(|tag| tag == &format!("delta_id:{delta_id}"))
                })
                .map(|entry| entry.memory_id)
                .ok_or_else(|| KnowledgeDeltaError::NotRetrieved(delta_id.to_string()))?
                .into()
        } else {
            None
        };
        let path = self.root.join("data/assimilation/knowledge_outcomes.jsonl");
        let mut ledger = LockedLedger::open(&path)?;
        if let Some(existing) = read_jsonl::<KnowledgeOutcomeReceipt>(&mut ledger.file)?
            .into_iter()
            .find(|receipt| receipt.delta_id == delta_id && receipt.consumer_id == consumer_id)
        {
            if existing.outcome == outcome && existing.rationale == rationale {
                return Ok(existing);
            }
            return Err(KnowledgeDeltaError::OutcomeConflict(delta_id.to_string()));
        }
        let receipt = KnowledgeOutcomeReceipt {
            schema_version: "arda.knowledge-consumer-outcome.v1".to_string(),
            delta_id: delta_id.to_string(),
            consumer_id: consumer_id.to_string(),
            outcome,
            rationale: rationale.to_string(),
            retrieved_memory_id,
            recorded_at,
        };
        append_jsonl(&mut ledger.file, &receipt)?;
        Ok(receipt)
    }

    pub fn quarantine_evaluation(
        &self,
        evaluation: &ExternalEvaluationReceipt,
        consumer_id: &str,
        rationale: &str,
        recorded_at: DateTime<Utc>,
    ) -> Result<KnowledgeOutcomeReceipt, KnowledgeDeltaError> {
        if evaluation.decision == EvaluationDecision::ApprovedSafeLocal {
            return Err(KnowledgeDeltaError::EvaluationApproved);
        }
        if consumer_id.trim().is_empty() {
            return Err(KnowledgeDeltaError::MissingField("consumer_id"));
        }
        if rationale.trim().is_empty() {
            return Err(KnowledgeDeltaError::MissingField("rationale"));
        }
        let delta_id = format!("quarantine:{}", evaluation.observation_id);
        let path = self.root.join("data/assimilation/knowledge_outcomes.jsonl");
        let mut ledger = LockedLedger::open(&path)?;
        let receipt = KnowledgeOutcomeReceipt {
            schema_version: "arda.knowledge-consumer-outcome.v1".to_string(),
            delta_id,
            consumer_id: consumer_id.to_string(),
            outcome: KnowledgeConsumerOutcome::Quarantined,
            rationale: rationale.to_string(),
            retrieved_memory_id: None,
            recorded_at,
        };
        if let Some(existing) = read_jsonl::<KnowledgeOutcomeReceipt>(&mut ledger.file)?
            .into_iter()
            .find(|candidate| candidate.delta_id == receipt.delta_id)
        {
            return Ok(existing);
        }
        append_jsonl(&mut ledger.file, &receipt)?;
        Ok(receipt)
    }

    pub fn promotions(&self) -> Result<Vec<KnowledgePromotionReceipt>, KnowledgeDeltaError> {
        read_path(
            self.root
                .join("data/assimilation/knowledge_promotions.jsonl"),
        )
    }

    pub fn outcomes(&self) -> Result<Vec<KnowledgeOutcomeReceipt>, KnowledgeDeltaError> {
        read_path(self.root.join("data/assimilation/knowledge_outcomes.jsonl"))
    }

    pub fn learning_count(&self) -> Result<usize, KnowledgeDeltaError> {
        Ok(self
            .outcomes()?
            .into_iter()
            .filter(|receipt| receipt.outcome == KnowledgeConsumerOutcome::Used)
            .count())
    }
}

fn validate_delta(delta: &GovernedKnowledgeDelta) -> Result<(), KnowledgeDeltaError> {
    for (field, value) in [
        ("delta_id", delta.delta_id.as_str()),
        ("source_reference", delta.source_reference.as_str()),
        ("source_digest", delta.source_digest.as_str()),
        ("scope", delta.scope.as_str()),
        ("consumer_id", delta.consumer_id.as_str()),
        ("retention_policy", delta.retention_policy.as_str()),
        ("revocation_operation", delta.revocation_operation.as_str()),
        ("content", delta.content.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(KnowledgeDeltaError::MissingField(field));
        }
    }
    if !delta.confidence.is_finite() || !(0.0..=1.0).contains(&delta.confidence) {
        return Err(KnowledgeDeltaError::InvalidConfidence);
    }
    if !is_digest(&delta.source_digest) {
        return Err(KnowledgeDeltaError::InvalidDigest);
    }
    Ok(())
}

fn evaluation_reference(evaluation: &ExternalEvaluationReceipt) -> String {
    format!(
        "varda-evaluation:{}:{}",
        evaluation.suggestion_id, evaluation.observation_id
    )
}

fn digest_hex(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

fn is_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn read_path<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Result<Vec<T>, KnowledgeDeltaError> {
    let mut file = match std::fs::OpenOptions::new().read(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let _lock = FileLock::shared(&file)?;
    read_jsonl(&mut file)
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(
    file: &mut std::fs::File,
) -> Result<Vec<T>, KnowledgeDeltaError> {
    file.seek(SeekFrom::Start(0))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some(serde_json::from_str(&line).map_err(|error| {
                KnowledgeDeltaError::CorruptEntry {
                    line: index + 1,
                    message: error.to_string(),
                }
            })),
            Err(error) => Some(Err(error.into())),
        })
        .collect()
}

fn append_jsonl(
    value: &mut std::fs::File,
    record: &impl Serialize,
) -> Result<(), KnowledgeDeltaError> {
    value.seek(SeekFrom::End(0))?;
    serde_json::to_writer(&mut *value, record)?;
    writeln!(value)?;
    value.sync_all()?;
    Ok(())
}

struct LockedLedger {
    file: std::fs::File,
    _lock: FileLock,
}

impl LockedLedger {
    fn open(path: &Path) -> Result<Self, KnowledgeDeltaError> {
        let parent = path.parent().expect("knowledge ledger path has parent");
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        let lock = FileLock::exclusive(&file)?;
        Ok(Self { file, _lock: lock })
    }
}

struct FileLock {
    fd: std::os::fd::RawFd,
}

impl FileLock {
    fn shared(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_SH)
    }

    fn exclusive(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_EX)
    }

    fn acquire(file: &std::fs::File, operation: libc::c_int) -> std::io::Result<Self> {
        let fd = file.as_raw_fd();
        // SAFETY: `fd` belongs to the live file held by the caller.
        if unsafe { libc::flock(fd, operation) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: the locked file outlives this guard.
        let _ = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KnowledgeDeltaError {
    #[error("knowledge delta I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("knowledge delta serialization error: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("knowledge delta memory error: {0}")]
    Memory(#[from] arda_core::error::ArdaError),
    #[error("knowledge delta is missing {0}")]
    MissingField(&'static str),
    #[error("knowledge delta confidence must be between zero and one")]
    InvalidConfidence,
    #[error("knowledge delta source digest must be SHA-256")]
    InvalidDigest,
    #[error("external evaluation did not approve promotion")]
    EvaluationNotApproved,
    #[error("approved external evaluation is missing approval")]
    MissingApproval,
    #[error("knowledge delta source does not match evaluated canonical evidence")]
    SourceMismatch,
    #[error("knowledge delta {0} conflicts with its durable promotion")]
    DeltaConflict(String),
    #[error("knowledge delta {0} was not promoted")]
    UnknownDelta(String),
    #[error("knowledge delta consumer mismatch: expected {expected}, got {actual}")]
    WrongConsumer { expected: String, actual: String },
    #[error("promoted knowledge delta {0} was not retrieved by the named consumer")]
    NotRetrieved(String),
    #[error("knowledge outcome conflicts for delta {0}")]
    OutcomeConflict(String),
    #[error("approved evaluations cannot be quarantined")]
    EvaluationApproved,
    #[error("corrupt knowledge ledger entry at line {line}: {message}")]
    CorruptEntry { line: usize, message: String },
}
