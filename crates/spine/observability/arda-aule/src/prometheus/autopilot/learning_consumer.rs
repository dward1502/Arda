//! Observe-only Aulë consumption of governed Varda deltas.

use arda_outpost_protocol::{ResearchSuggestion, ResearchSuggestionLedger};
use arda_vaire::{ApprovedKnowledgeDelta, GovernedKnowledgeReceipt, MnemosyneService};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LearningDisposition {
    Informational,
    ResearchFollowup,
    SafeLocalProposalCandidate,
    GovernedReviewCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningConsumptionReceipt {
    pub schema_version: String,
    pub consumption_id: String,
    pub delta_id: String,
    pub vaire_receipt_id: String,
    pub source_reference: String,
    pub warden_observation_id: String,
    pub varda_evaluation_id: String,
    pub approval_reference: String,
    pub disposition: LearningDisposition,
    pub task_promotion_allowed: bool,
    pub consumed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningLoopSwitches {
    pub observe_only: bool,
    pub intake_enabled: bool,
    pub evaluation_enabled: bool,
    pub safe_local_knowledge_enabled: bool,
    pub proposal_activation_enabled: bool,
    pub queue_mutation_enabled: bool,
    pub execution_enabled: bool,
}

impl Default for LearningLoopSwitches {
    fn default() -> Self {
        Self {
            observe_only: true,
            intake_enabled: false,
            evaluation_enabled: false,
            safe_local_knowledge_enabled: false,
            proposal_activation_enabled: false,
            queue_mutation_enabled: false,
            execution_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningLoopPolicy {
    pub per_cycle_cap: usize,
    pub max_input_age_seconds: i64,
}

impl Default for LearningLoopPolicy {
    fn default() -> Self {
        Self {
            per_cycle_cap: 1,
            max_input_age_seconds: 86_400,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningCycleInput {
    pub cycle_id: String,
    pub observed_at_utc: String,
    pub delta: ApprovedKnowledgeDelta,
    pub disposition: LearningDisposition,
    pub rollback_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningCycleReceipt {
    pub schema_version: String,
    pub cycle_id: String,
    pub delta_id: String,
    pub status: String,
    pub stale_input: bool,
    pub rollback_applied: bool,
    pub memory_receipt_id: Option<String>,
    pub proposal_receipt_id: Option<String>,
    pub task_promotion_allowed: bool,
    pub queue_mutation_performed: bool,
    pub execution_performed: bool,
    pub source_reference: String,
    pub warden_observation_id: String,
    pub varda_evaluation_id: String,
    pub approval_reference: String,
    pub completed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct LearningCycleMetrics {
    pub observed: usize,
    pub intake_eligible: usize,
    pub evaluation_eligible: usize,
    pub safe_local_knowledge_written: usize,
    pub proposals_activated: usize,
    pub stale_blocked: usize,
    pub rolled_back: usize,
    pub duplicate_replays: usize,
    pub queue_mutations: usize,
    pub executions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LearningCycleReport {
    pub cycle_id: String,
    pub receipts: Vec<LearningCycleReceipt>,
    pub metrics: LearningCycleMetrics,
}

#[derive(Debug, thiserror::Error)]
pub enum LearningConsumerError {
    #[error("Vairë governed ingest failed: {0}")]
    Vaire(#[from] arda_core::error::ArdaError),
    #[error("learning consumption ledger error: {0}")]
    Io(#[from] std::io::Error),
    #[error("research suggestion ledger error: {0}")]
    Research(#[from] arda_outpost_protocol::ResearchReceiptError),
    #[error("learning cycle input timestamp is invalid: {0}")]
    InvalidTimestamp(String),
}

/// Emit an advisory-only bounded research request for the Scout runtime.
/// This writes no task, approval, or execution record.
pub fn emit_research_suggestion(
    ledger_path: impl AsRef<Path>,
    query: impl Into<String>,
    idempotency_key: impl Into<String>,
    expires_at_utc: chrono::DateTime<chrono::Utc>,
    max_results: usize,
    budget_bytes: usize,
) -> Result<ResearchSuggestion, LearningConsumerError> {
    let now = Utc::now();
    let suggestion = ResearchSuggestion::new(
        query,
        idempotency_key,
        now,
        expires_at_utc,
        max_results,
        budget_bytes,
    )?;
    Ok(ResearchSuggestionLedger::open(ledger_path)?.append(&suggestion)?)
}

pub fn consume_approved_delta(
    service: &MnemosyneService,
    delta: ApprovedKnowledgeDelta,
    disposition: LearningDisposition,
    ledger_path: impl AsRef<Path>,
) -> Result<LearningConsumptionReceipt, LearningConsumerError> {
    if let Some(existing) = read_consumption_receipts(ledger_path.as_ref())?
        .into_iter()
        .find(|receipt| receipt.delta_id == delta.delta_id)
    {
        return Ok(existing);
    }
    let vaire_receipt: GovernedKnowledgeReceipt = service.ingest_approved_delta(delta.clone())?;
    if let Some(parent) = ledger_path.as_ref().parent() {
        std::fs::create_dir_all(parent)?;
    }
    let receipt = LearningConsumptionReceipt {
        schema_version: "arda.aule.learning_consumption.v1".to_owned(),
        consumption_id: format!("cons_{}", uuid::Uuid::new_v4().simple()),
        delta_id: delta.delta_id,
        vaire_receipt_id: vaire_receipt.receipt_id,
        source_reference: delta.source_reference,
        warden_observation_id: delta.warden_observation_id,
        varda_evaluation_id: delta.varda_evaluation_id,
        approval_reference: delta.approval_reference,
        disposition,
        task_promotion_allowed: false,
        consumed_at_utc: Utc::now().to_rfc3339(),
    };
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path)?;
    serde_json::to_writer(&mut file, &receipt).map_err(std::io::Error::other)?;
    writeln!(file)?;
    file.sync_data()?;
    Ok(receipt)
}

/// Run one bounded, replay-safe learning cycle. Proposal activation writes
/// only an append-only proposal receipt; queue mutation and execution remain
/// hard-disabled in this stage.
pub fn run_learning_cycle(
    service: &MnemosyneService,
    inputs: &[LearningCycleInput],
    switches: &LearningLoopSwitches,
    policy: &LearningLoopPolicy,
    consumption_path: impl AsRef<Path>,
    cycle_path: impl AsRef<Path>,
    proposal_path: impl AsRef<Path>,
) -> Result<LearningCycleReport, LearningConsumerError> {
    let cycle_path = cycle_path.as_ref();
    let mut existing = read_cycle_receipts(cycle_path)?;
    let cycle_id = inputs
        .first()
        .map(|input| input.cycle_id.clone())
        .unwrap_or_else(|| "empty-cycle".to_owned());
    let mut receipts = Vec::new();
    let mut metrics = LearningCycleMetrics {
        observed: inputs.len().min(policy.per_cycle_cap),
        ..LearningCycleMetrics::default()
    };

    for input in inputs.iter().take(policy.per_cycle_cap) {
        if let Some(receipt) = existing
            .iter()
            .find(|receipt| {
                receipt.cycle_id == input.cycle_id && receipt.delta_id == input.delta.delta_id
            })
            .cloned()
        {
            metrics.duplicate_replays += 1;
            receipts.push(receipt);
            continue;
        }

        let observed_at = chrono::DateTime::parse_from_rfc3339(&input.observed_at_utc)
            .map_err(|error| LearningConsumerError::InvalidTimestamp(error.to_string()))?
            .with_timezone(&Utc);
        let stale = Utc::now().signed_duration_since(observed_at).num_seconds()
            > policy.max_input_age_seconds;
        let mut receipt = LearningCycleReceipt {
            schema_version: "arda.aule.learning_cycle.v1".to_owned(),
            cycle_id: input.cycle_id.clone(),
            delta_id: input.delta.delta_id.clone(),
            status: "observed".to_owned(),
            stale_input: stale,
            rollback_applied: input.rollback_requested,
            memory_receipt_id: None,
            proposal_receipt_id: None,
            task_promotion_allowed: false,
            queue_mutation_performed: false,
            execution_performed: false,
            source_reference: input.delta.source_reference.clone(),
            warden_observation_id: input.delta.warden_observation_id.clone(),
            varda_evaluation_id: input.delta.varda_evaluation_id.clone(),
            approval_reference: input.delta.approval_reference.clone(),
            completed_at_utc: Utc::now().to_rfc3339(),
        };

        if stale {
            metrics.stale_blocked += 1;
            receipt.status = "blocked_stale".to_owned();
        } else if input.rollback_requested {
            metrics.rolled_back += 1;
            receipt.status = "rolled_back".to_owned();
        } else if switches.observe_only {
            receipt.status = "observe_only".to_owned();
        } else if !switches.intake_enabled {
            receipt.status = "blocked_intake_switch".to_owned();
        } else {
            metrics.intake_eligible += 1;
            if !switches.evaluation_enabled {
                receipt.status = "blocked_evaluation_switch".to_owned();
            } else {
                metrics.evaluation_eligible += 1;
                if !switches.safe_local_knowledge_enabled
                    || input.disposition != LearningDisposition::SafeLocalProposalCandidate
                {
                    receipt.status = "blocked_safe_local_switch".to_owned();
                } else {
                    let memory = consume_approved_delta(
                        service,
                        input.delta.clone(),
                        input.disposition.clone(),
                        consumption_path.as_ref(),
                    )?;
                    receipt.memory_receipt_id = Some(memory.vaire_receipt_id);
                    metrics.safe_local_knowledge_written += 1;
                    receipt.status = "safe_local_knowledge".to_owned();
                    if switches.proposal_activation_enabled {
                        let proposal_id = append_proposal_receipt(proposal_path.as_ref(), input)?;
                        receipt.proposal_receipt_id = Some(proposal_id);
                        metrics.proposals_activated += 1;
                        receipt.status = "proposal_activated".to_owned();
                    }
                }
            }
        }
        append_learning_jsonl(cycle_path, &receipt)?;
        existing.push(receipt.clone());
        receipts.push(receipt);
    }

    Ok(LearningCycleReport {
        cycle_id,
        receipts,
        metrics,
    })
}

fn append_proposal_receipt(
    path: &Path,
    input: &LearningCycleInput,
) -> Result<String, LearningConsumerError> {
    let proposal_id = format!("proposal_{}", input.delta.delta_id);
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    if existing
        .lines()
        .any(|line| line.contains(&format!("\"proposal_id\":\"{proposal_id}\"")))
    {
        return Ok(proposal_id);
    }
    let value = serde_json::json!({
        "schema_version": "arda.aule.learning_proposal.v1",
        "proposal_id": proposal_id,
        "cycle_id": input.cycle_id,
        "delta_id": input.delta.delta_id,
        "source_reference": input.delta.source_reference,
        "approval_reference": input.delta.approval_reference,
        "task_promotion_allowed": false,
        "queue_mutation_performed": false,
        "execution_performed": false,
        "created_at_utc": Utc::now().to_rfc3339(),
    });
    append_learning_jsonl(path, &value)?;
    Ok(proposal_id)
}

fn append_learning_jsonl<T: Serialize>(
    path: &Path,
    value: &T,
) -> Result<(), LearningConsumerError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    serde_json::to_writer(&mut file, value).map_err(std::io::Error::other)?;
    writeln!(file)?;
    file.sync_data()?;
    Ok(())
}

fn read_cycle_receipts(path: &Path) -> Result<Vec<LearningCycleReceipt>, LearningConsumerError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(std::io::Error::other))
        .collect::<Result<Vec<_>, _>>()
        .map_err(LearningConsumerError::Io)
}

fn read_consumption_receipts(
    path: &Path,
) -> Result<Vec<LearningConsumptionReceipt>, LearningConsumerError> {
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(LearningConsumerError::Io(error)),
    };
    contents
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).map_err(std::io::Error::other))
        .collect::<Result<Vec<_>, _>>()
        .map_err(LearningConsumerError::Io)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_outpost_protocol::{
        AcknowledgementReceipt, ExternalObservationReceipt, ResearchDispatch,
        ResearchReceiptLedger, ResearchSuggestion,
    };
    use arda_varda::{approved_delta, import_next_canonical_result};
    use chrono::Duration;
    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    #[test]
    fn consumption_is_provenanced_and_never_allows_task_promotion() {
        let dir = tempdir().unwrap();
        let service = MnemosyneService::new(dir.path().join("memory")).unwrap();
        let delta = ApprovedKnowledgeDelta {
            delta_id: "delta-1".into(),
            source_reference: "https://example.com#approval=ap-1".into(),
            warden_observation_id: "obs-1".into(),
            varda_evaluation_id: "eval-1".into(),
            approval_reference: "ap-1".into(),
            content: "approved knowledge".into(),
            correction_of: None,
        };
        let receipt = consume_approved_delta(
            &service,
            delta.clone(),
            LearningDisposition::SafeLocalProposalCandidate,
            dir.path().join("consumption.jsonl"),
        )
        .unwrap();
        assert!(!receipt.vaire_receipt_id.is_empty());
        assert!(!receipt.task_promotion_allowed);
        let replay = consume_approved_delta(
            &service,
            delta,
            LearningDisposition::Informational,
            dir.path().join("consumption.jsonl"),
        )
        .unwrap();
        assert_eq!(replay.consumption_id, receipt.consumption_id);
        assert_eq!(
            std::fs::read_to_string(dir.path().join("consumption.jsonl"))
                .unwrap()
                .lines()
                .count(),
            1
        );
    }

    #[test]
    fn persisted_chain_replays_through_varda_vaire_and_aule_once() {
        let dir = tempdir().unwrap();
        let now = Utc::now();
        let suggestion = ResearchSuggestion::new(
            "end to end governed learning",
            "e2e:suggestion",
            now,
            now + Duration::minutes(10),
            2,
            4096,
        )
        .unwrap();
        let dispatch = ResearchDispatch::accepted(&suggestion, "e2e:dispatch", now, 1).unwrap();
        let observation = ExternalObservationReceipt::completed(
            &suggestion,
            &dispatch,
            "https://example.com/e2e",
            hex_digest(b"canonical fetched content"),
            hex_digest(b"https://example.com/e2e"),
            now,
        )
        .unwrap();
        let acknowledgement =
            AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now).unwrap();
        let research_path = dir.path().join("warden.jsonl");
        let research = ResearchReceiptLedger::open(&research_path).unwrap();
        research
            .append_complete_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
            .unwrap();

        let evaluation = import_next_canonical_result(
            &research_path,
            dir.path().join("varda-evaluations.jsonl"),
            &observation.normalized_url,
            "canonical fetched content",
            now,
            now,
        )
        .unwrap()
        .unwrap();
        assert!(import_next_canonical_result(
            &research_path,
            dir.path().join("varda-evaluations.jsonl"),
            &observation.normalized_url,
            "canonical fetched content",
            now,
            now,
        )
        .unwrap()
        .is_none());

        let delta = approved_delta(&evaluation, "canonical fetched content").unwrap();
        let governed = arda_vaire::ApprovedKnowledgeDelta {
            delta_id: "e2e:delta".into(),
            source_reference: delta.source_path,
            warden_observation_id: observation.observation_id,
            varda_evaluation_id: evaluation.observation_id.clone(),
            approval_reference: evaluation.approval_reference.unwrap(),
            content: delta.delta_content,
            correction_of: None,
        };
        let memory = arda_vaire::MnemosyneService::new(dir.path().join("memory")).unwrap();
        let first = consume_approved_delta(
            &memory,
            governed.clone(),
            LearningDisposition::SafeLocalProposalCandidate,
            dir.path().join("aule-consumption.jsonl"),
        )
        .unwrap();
        let replay = consume_approved_delta(
            &memory,
            governed,
            LearningDisposition::Informational,
            dir.path().join("aule-consumption.jsonl"),
        )
        .unwrap();
        assert_eq!(first.consumption_id, replay.consumption_id);
        assert!(!replay.task_promotion_allowed);
    }

    #[test]
    fn low_risk_canary_survives_restart_and_replay_without_duplicate_side_effects() {
        let dir = tempdir().unwrap();
        let memory = MnemosyneService::new(dir.path().join("memory")).unwrap();
        let now = Utc::now();
        let input = LearningCycleInput {
            cycle_id: "canary-cycle-1".into(),
            observed_at_utc: now.to_rfc3339(),
            delta: ApprovedKnowledgeDelta {
                delta_id: "canary-delta-1".into(),
                source_reference: "https://example.com/canary".into(),
                warden_observation_id: "canary-observation-1".into(),
                varda_evaluation_id: "canary-evaluation-1".into(),
                approval_reference: "canary-approval-1".into(),
                content: "low-risk local canary knowledge".into(),
                correction_of: None,
            },
            disposition: LearningDisposition::SafeLocalProposalCandidate,
            rollback_requested: false,
        };
        let switches = LearningLoopSwitches {
            observe_only: false,
            intake_enabled: true,
            evaluation_enabled: true,
            safe_local_knowledge_enabled: true,
            proposal_activation_enabled: true,
            queue_mutation_enabled: false,
            execution_enabled: false,
        };
        let policy = LearningLoopPolicy {
            per_cycle_cap: 1,
            max_input_age_seconds: 60,
        };
        let paths = (
            dir.path().join("consumption.jsonl"),
            dir.path().join("cycles.jsonl"),
            dir.path().join("proposals.jsonl"),
        );

        let first = run_learning_cycle(
            &memory,
            std::slice::from_ref(&input),
            &switches,
            &policy,
            &paths.0,
            &paths.1,
            &paths.2,
        )
        .unwrap();
        assert_eq!(first.metrics.safe_local_knowledge_written, 1);
        assert_eq!(first.metrics.proposals_activated, 1);
        assert!(!first.receipts[0].task_promotion_allowed);
        assert!(!first.receipts[0].queue_mutation_performed);
        assert!(!first.receipts[0].execution_performed);

        let restarted = MnemosyneService::new(dir.path().join("memory")).unwrap();
        let replay = run_learning_cycle(
            &restarted,
            std::slice::from_ref(&input),
            &switches,
            &policy,
            &paths.0,
            &paths.1,
            &paths.2,
        )
        .unwrap();
        assert_eq!(replay.metrics.duplicate_replays, 1);
        assert_eq!(
            std::fs::read_to_string(&paths.0).unwrap().lines().count(),
            1
        );
        assert_eq!(
            std::fs::read_to_string(&paths.1).unwrap().lines().count(),
            1
        );
        assert_eq!(
            std::fs::read_to_string(&paths.2).unwrap().lines().count(),
            1
        );
    }

    #[test]
    fn cycle_switches_cap_stale_inputs_and_rollback_are_receipted() {
        let dir = tempdir().unwrap();
        let memory = MnemosyneService::new(dir.path().join("memory")).unwrap();
        let stale = LearningCycleInput {
            cycle_id: "stale-cycle".into(),
            observed_at_utc: (Utc::now() - Duration::hours(2)).to_rfc3339(),
            delta: ApprovedKnowledgeDelta {
                delta_id: "stale-delta".into(),
                source_reference: "https://example.com/stale".into(),
                warden_observation_id: "stale-observation".into(),
                varda_evaluation_id: "stale-evaluation".into(),
                approval_reference: "stale-approval".into(),
                content: "stale".into(),
                correction_of: None,
            },
            disposition: LearningDisposition::SafeLocalProposalCandidate,
            rollback_requested: false,
        };
        let rollback = LearningCycleInput {
            cycle_id: "rollback-cycle".into(),
            observed_at_utc: Utc::now().to_rfc3339(),
            rollback_requested: true,
            ..stale.clone()
        };
        let report = run_learning_cycle(
            &memory,
            &[stale, rollback.clone()],
            &LearningLoopSwitches::default(),
            &LearningLoopPolicy {
                per_cycle_cap: 1,
                max_input_age_seconds: 60,
            },
            dir.path().join("consumption.jsonl"),
            dir.path().join("cycles.jsonl"),
            dir.path().join("proposals.jsonl"),
        )
        .unwrap();
        assert_eq!(report.receipts.len(), 1);
        assert_eq!(report.metrics.stale_blocked, 1);
        assert_eq!(report.metrics.safe_local_knowledge_written, 0);
        assert_eq!(report.receipts[0].status, "blocked_stale");

        let rollback_report = run_learning_cycle(
            &memory,
            std::slice::from_ref(&rollback),
            &LearningLoopSwitches {
                observe_only: false,
                intake_enabled: true,
                evaluation_enabled: true,
                safe_local_knowledge_enabled: true,
                proposal_activation_enabled: true,
                queue_mutation_enabled: false,
                execution_enabled: false,
            },
            &LearningLoopPolicy {
                per_cycle_cap: 1,
                max_input_age_seconds: 60,
            },
            dir.path().join("consumption.jsonl"),
            dir.path().join("cycles.jsonl"),
            dir.path().join("proposals.jsonl"),
        )
        .unwrap();
        assert_eq!(rollback_report.metrics.rolled_back, 1);
        assert_eq!(rollback_report.receipts[0].status, "rolled_back");
    }

    fn hex_digest(value: &[u8]) -> String {
        Sha256::digest(value)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }
}
