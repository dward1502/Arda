//! Approved, replay-safe placement learning derived from terminal outcome evidence.

use arda_vaire::GovernedKnowledgeReceipt;
use arda_varda::outcome_learning::{
    OutcomeLearningDecision, OutcomeLearningEvaluationReceipt, OutcomeLearningEvidence,
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const PLACEMENT_LEARNING_SCHEMA_VERSION: &str = "arda.placement-learning-receipt.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlacementLearningReceipt {
    pub schema_version: String,
    pub receipt_id: String,
    pub receipt_digest: String,
    pub learning_id: String,
    pub objective_id: String,
    pub task_kind: String,
    pub role: String,
    pub node_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub score_adjustment_millionths: i32,
    pub varda_evaluation_id: String,
    pub varda_evaluation_digest: String,
    pub vaire_receipt_id: String,
    pub approval_reference: String,
    pub terminal_receipt_refs: Vec<String>,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct PlacementLearningStore {
    path: PathBuf,
}

impl PlacementLearningStore {
    pub fn new(workbench_root: &Path) -> Self {
        Self {
            path: workbench_root.join("data/placement_learning/receipts.jsonl"),
        }
    }

    pub fn approve(
        &self,
        evidence: &OutcomeLearningEvidence,
        evaluation: &OutcomeLearningEvaluationReceipt,
        memory_receipt: &GovernedKnowledgeReceipt,
        applied_at: DateTime<Utc>,
    ) -> Result<PlacementLearningReceipt, PlacementLearningError> {
        if evaluation.decision != OutcomeLearningDecision::ApprovedSafeLocal
            || !evaluation.has_valid_digest()
        {
            return Err(PlacementLearningError::EvaluationNotApproved);
        }
        if evaluation.learning_id != evidence.learning_id
            || evaluation.objective_id != evidence.objective_id
            || evaluation.terminal_receipt_refs != evidence.terminal_receipt_refs
            || evaluation.proposed_score_adjustment_millionths
                != evidence.proposed_score_adjustment_millionths
            || memory_receipt.varda_evaluation_id != evaluation.evaluation_id
            || memory_receipt.delta_id != evidence.learning_id
            || memory_receipt.approval_reference.trim().is_empty()
        {
            return Err(PlacementLearningError::LineageMismatch);
        }
        let identity = format!("{}\0{}", evidence.learning_id, evaluation.evaluation_id);
        let mut receipt = PlacementLearningReceipt {
            schema_version: PLACEMENT_LEARNING_SCHEMA_VERSION.into(),
            receipt_id: format!("placement-learning:{}", hex_digest(identity.as_bytes())),
            receipt_digest: String::new(),
            learning_id: evidence.learning_id.clone(),
            objective_id: evidence.objective_id.clone(),
            task_kind: evidence.task_kind.clone(),
            role: evidence.role.clone(),
            node_id: evidence.node_id.clone(),
            provider_id: evidence.provider_id.clone(),
            model_id: evidence.model_id.clone(),
            score_adjustment_millionths: evidence.proposed_score_adjustment_millionths,
            varda_evaluation_id: evaluation.evaluation_id.clone(),
            varda_evaluation_digest: evaluation.evaluation_digest.clone(),
            vaire_receipt_id: memory_receipt.receipt_id.clone(),
            approval_reference: memory_receipt.approval_reference.clone(),
            terminal_receipt_refs: evidence.terminal_receipt_refs.clone(),
            applied_at,
        };
        receipt.receipt_digest = receipt_digest(&receipt)?;

        let mut ledger = LockedLedger::open(&self.path)?;
        if let Some(existing) = read_receipts(&mut ledger.file)?
            .into_iter()
            .find(|item| item.receipt_id == receipt.receipt_id)
        {
            if existing == receipt {
                return Ok(existing);
            }
            return Err(PlacementLearningError::ReplayConflict);
        }
        serde_json::to_writer(&mut ledger.file, &receipt)?;
        writeln!(ledger.file)?;
        ledger.file.sync_all()?;
        Ok(receipt)
    }

    pub fn receipts(&self) -> Result<Vec<PlacementLearningReceipt>, PlacementLearningError> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        let mut file = std::fs::OpenOptions::new().read(true).open(&self.path)?;
        file.lock_shared()?;
        let result = read_receipts(&mut file);
        file.unlock()?;
        result
    }

    pub fn adjustment(
        &self,
        task_kind: &str,
        role: &str,
        node_id: &str,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(f64, Vec<String>), PlacementLearningError> {
        let matching = self
            .receipts()?
            .into_iter()
            .filter(|item| {
                item.task_kind == task_kind
                    && item.role == role
                    && item.node_id == node_id
                    && item.provider_id == provider_id
                    && item.model_id == model_id
            })
            .collect::<Vec<_>>();
        let adjustment = matching
            .iter()
            .map(|item| f64::from(item.score_adjustment_millionths) / 1_000_000.0)
            .sum();
        Ok((
            adjustment,
            matching.into_iter().map(|item| item.receipt_id).collect(),
        ))
    }
}

fn read_receipts(
    file: &mut std::fs::File,
) -> Result<Vec<PlacementLearningReceipt>, PlacementLearningError> {
    file.seek(SeekFrom::Start(0))?;
    BufReader::new(file)
        .lines()
        .enumerate()
        .filter_map(|(index, line)| match line {
            Ok(line) if line.trim().is_empty() => None,
            Ok(line) => Some(
                serde_json::from_str::<PlacementLearningReceipt>(&line)
                    .map_err(PlacementLearningError::Json)
                    .and_then(|receipt| {
                        if receipt.schema_version != PLACEMENT_LEARNING_SCHEMA_VERSION
                            || receipt_digest(&receipt)? != receipt.receipt_digest
                        {
                            return Err(PlacementLearningError::CorruptEntry(index + 1));
                        }
                        Ok(receipt)
                    }),
            ),
            Err(error) => Some(Err(error.into())),
        })
        .collect()
}

fn receipt_digest(receipt: &PlacementLearningReceipt) -> Result<String, PlacementLearningError> {
    let mut unsigned = receipt.clone();
    unsigned.receipt_digest.clear();
    Ok(format!(
        "sha256:{}",
        hex_digest(&serde_json::to_vec(&unsigned)?)
    ))
}

fn hex_digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

struct LockedLedger {
    file: std::fs::File,
}

impl LockedLedger {
    fn open(path: &Path) -> Result<Self, PlacementLearningError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlacementLearningError {
    #[error("Varda outcome evaluation is not approved safe-local")]
    EvaluationNotApproved,
    #[error("Varda, Vairë, and terminal evidence lineage do not match")]
    LineageMismatch,
    #[error("conflicting replay for placement learning receipt")]
    ReplayConflict,
    #[error("corrupt placement learning entry at line {0}")]
    CorruptEntry(usize),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
