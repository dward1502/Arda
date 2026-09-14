//! Varda evaluation of terminal outcome evidence before placement learning.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const OUTCOME_LEARNING_EVALUATION_SCHEMA_VERSION: &str =
    "arda.varda.outcome-learning-evaluation.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeLearningDecision {
    ApprovedSafeLocal,
    ReviewRequired,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeLearningEvidence {
    pub learning_id: String,
    pub objective_id: String,
    pub task_kind: String,
    pub role: String,
    pub node_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub terminal_receipt_refs: Vec<String>,
    pub acceptance_conditions: Vec<String>,
    pub satisfied_conditions: Vec<String>,
    pub proposed_score_adjustment_millionths: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutcomeLearningEvaluationReceipt {
    pub schema_version: String,
    pub evaluation_id: String,
    pub evaluation_digest: String,
    pub learning_id: String,
    pub objective_id: String,
    pub decision: OutcomeLearningDecision,
    pub terminal_receipt_refs: Vec<String>,
    pub acceptance_conditions: Vec<String>,
    pub satisfied_conditions: Vec<String>,
    pub proposed_score_adjustment_millionths: i32,
    pub rationale: String,
}

pub fn evaluate_outcome_learning(
    evidence: &OutcomeLearningEvidence,
) -> OutcomeLearningEvaluationReceipt {
    let required = evidence
        .acceptance_conditions
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let satisfied = evidence
        .satisfied_conditions
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let complete = !required.is_empty() && required.is_subset(&satisfied);
    let valid_identity = [
        evidence.learning_id.as_str(),
        evidence.objective_id.as_str(),
        evidence.task_kind.as_str(),
        evidence.role.as_str(),
        evidence.node_id.as_str(),
        evidence.provider_id.as_str(),
        evidence.model_id.as_str(),
    ]
    .iter()
    .all(|value| !value.trim().is_empty());
    let bounded = (-500_000..=500_000).contains(&evidence.proposed_score_adjustment_millionths);
    let (decision, rationale) =
        if !valid_identity || evidence.terminal_receipt_refs.is_empty() || !bounded {
            (
                OutcomeLearningDecision::Rejected,
                "missing lineage or unbounded placement adjustment",
            )
        } else if complete {
            (
                OutcomeLearningDecision::ApprovedSafeLocal,
                "terminal receipts satisfy every named acceptance condition",
            )
        } else {
            (
                OutcomeLearningDecision::ReviewRequired,
                "terminal evidence does not satisfy every named acceptance condition",
            )
        };
    let identity = serde_json::to_vec(evidence).expect("outcome learning evidence serializes");
    let mut receipt = OutcomeLearningEvaluationReceipt {
        schema_version: OUTCOME_LEARNING_EVALUATION_SCHEMA_VERSION.into(),
        evaluation_id: format!("varda-learning:{}", hex_digest(&identity)),
        evaluation_digest: String::new(),
        learning_id: evidence.learning_id.clone(),
        objective_id: evidence.objective_id.clone(),
        decision,
        terminal_receipt_refs: evidence.terminal_receipt_refs.clone(),
        acceptance_conditions: evidence.acceptance_conditions.clone(),
        satisfied_conditions: evidence.satisfied_conditions.clone(),
        proposed_score_adjustment_millionths: evidence.proposed_score_adjustment_millionths,
        rationale: rationale.into(),
    };
    receipt.evaluation_digest = evaluation_digest(&receipt);
    receipt
}

impl OutcomeLearningEvaluationReceipt {
    pub fn has_valid_digest(&self) -> bool {
        self.schema_version == OUTCOME_LEARNING_EVALUATION_SCHEMA_VERSION
            && evaluation_digest(self) == self.evaluation_digest
    }
}

fn evaluation_digest(receipt: &OutcomeLearningEvaluationReceipt) -> String {
    let mut unsigned = receipt.clone();
    unsigned.evaluation_digest.clear();
    format!(
        "sha256:{}",
        hex_digest(&serde_json::to_vec(&unsigned).expect("evaluation serializes"))
    )
}

fn hex_digest(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}
