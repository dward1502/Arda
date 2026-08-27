#![cfg(feature = "full-cli")]
//! Unified, bounded Arandur executive-cycle receipts.
//!
//! This module records how Arandur observes canonical state, recovers context,
//! proposes role capabilities, consumes governance/placement/dispatch receipts,
//! assesses outcomes, and emits a learning candidate. It does not own any of
//! those authorities and never treats a queue handoff as execution completion.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const EXECUTIVE_CYCLE_CONTRACT: &str = "arda.arandur.executive_cycle_receipt.v1";
pub const EXECUTIVE_CYCLE_LEDGER: &str = "data/arandur/executive_cycles.jsonl";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutivePhase {
    Review,
    Record,
    Plan,
    Execute,
    Assess,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilMode {
    Disabled,
    CriticOnly,
    Adjudication,
    FullDeliberation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutiveDisposition {
    ObservedReadOnly,
    AwaitingReview,
    Held,
    Planned,
    HandedOff,
    Assessing,
    Replanned,
    Accepted,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleRequest {
    pub role: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutiveResourceBudget {
    pub max_roles: usize,
    pub max_dispatches: usize,
    pub max_joules: f64,
    pub requested_joules: f64,
    pub max_council_opinions: usize,
    pub requested_council_opinions: usize,
}

impl Default for ExecutiveResourceBudget {
    fn default() -> Self {
        Self {
            max_roles: 4,
            max_dispatches: 8,
            max_joules: 100.0,
            requested_joules: 0.0,
            max_council_opinions: 1,
            requested_council_opinions: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutiveCycleInput {
    pub cycle_id: String,
    pub phase: ExecutivePhase,
    pub objective_id: String,
    pub objective_source_ref: String,
    pub context_receipt_ref: String,
    pub recommendation_id: String,
    pub approval_packet_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_authorization_id: Option<String>,
    pub proposed_action: String,
    pub requested_roles: Vec<RoleRequest>,
    pub governance_receipt_ref: Option<String>,
    pub placement_receipt_refs: Vec<String>,
    pub queue_handoff_receipt_refs: Vec<String>,
    pub execution_receipt_refs: Vec<String>,
    pub failure_receipt_ref: Option<String>,
    pub revised_action: Option<String>,
    pub revised_requested_roles: Vec<RoleRequest>,
    pub acceptance_receipt_refs: Vec<String>,
    pub council_mode: CouncilMode,
    pub full_council_approval_ref: Option<String>,
    pub resource_budget: ExecutiveResourceBudget,
    pub operator_stop_requested: bool,
    pub read_only: bool,
    pub parent_receipt_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutiveCycleReceipt {
    pub contract: String,
    pub receipt_id: String,
    pub cycle_id: String,
    pub phase: ExecutivePhase,
    pub objective_id: String,
    pub objective_source_ref: String,
    pub context_receipt_ref: String,
    pub recommendation_id: String,
    pub approval_packet_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governance_authorization_id: Option<String>,
    pub input_digest: String,
    pub disposition: ExecutiveDisposition,
    pub reason: String,
    pub proposed_action: String,
    pub requested_roles: Vec<RoleRequest>,
    pub revised_action: Option<String>,
    pub revised_requested_roles: Vec<RoleRequest>,
    pub governance_receipt_ref: Option<String>,
    pub placement_receipt_refs: Vec<String>,
    pub queue_handoff_receipt_refs: Vec<String>,
    pub execution_receipt_refs: Vec<String>,
    pub failure_receipt_ref: Option<String>,
    pub acceptance_receipt_refs: Vec<String>,
    pub council_mode: CouncilMode,
    pub full_council_approval_ref: Option<String>,
    pub resource_budget: ExecutiveResourceBudget,
    pub queue_mutation_performed_by_arandur: bool,
    pub placement_performed_by_arandur: bool,
    pub execution_performed_by_arandur: bool,
    pub queue_handoff_allowed: bool,
    pub execution_observed: bool,
    pub operator_can_stop: bool,
    pub operator_update: String,
    pub learning_candidate: Option<String>,
    pub parent_receipt_id: Option<String>,
    pub recorded_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutiveCycleResult {
    pub receipt: ExecutiveCycleReceipt,
    pub replayed: bool,
    pub ledger_appended: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutiveCycleError {
    #[error("invalid executive cycle input: {0}")]
    Invalid(String),
    #[error("conflicting replay for cycle `{cycle_id}` phase `{phase:?}`")]
    ConflictingReplay {
        cycle_id: String,
        phase: ExecutivePhase,
    },
    #[error("executive cycle ledger I/O failed at `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("executive cycle ledger JSON failed at `{path}`: {source}")]
    Json {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

#[derive(Debug, Clone)]
pub struct ExecutiveCycleStore {
    ledger_path: PathBuf,
}

impl ExecutiveCycleStore {
    pub fn from_root(root: impl AsRef<Path>) -> Self {
        Self {
            ledger_path: root.as_ref().join(EXECUTIVE_CYCLE_LEDGER),
        }
    }

    pub fn ledger_path(&self) -> &Path {
        &self.ledger_path
    }

    pub fn evaluate(
        &self,
        input: ExecutiveCycleInput,
        now: DateTime<Utc>,
    ) -> Result<ExecutiveCycleResult, ExecutiveCycleError> {
        validate(&input)?;
        let input_digest = digest(&input)?;
        let receipt = build_receipt(input, input_digest.clone(), now);
        if receipt.disposition == ExecutiveDisposition::ObservedReadOnly {
            return Ok(ExecutiveCycleResult {
                receipt,
                replayed: false,
                ledger_appended: false,
            });
        }

        if let Some(parent) = self.ledger_path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| ExecutiveCycleError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let mut ledger = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.ledger_path)
            .map_err(|source| ExecutiveCycleError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        ledger
            .lock_exclusive()
            .map_err(|source| ExecutiveCycleError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        ledger
            .seek(SeekFrom::Start(0))
            .map_err(|source| ExecutiveCycleError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        for line in BufReader::new(&ledger).lines() {
            let line = line.map_err(|source| ExecutiveCycleError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let existing: ExecutiveCycleReceipt =
                serde_json::from_str(&line).map_err(|source| ExecutiveCycleError::Json {
                    path: self.ledger_path.clone(),
                    source,
                })?;
            if existing.cycle_id == receipt.cycle_id && existing.phase == receipt.phase {
                if existing.input_digest == input_digest {
                    return Ok(ExecutiveCycleResult {
                        receipt: existing,
                        replayed: true,
                        ledger_appended: false,
                    });
                }
                return Err(ExecutiveCycleError::ConflictingReplay {
                    cycle_id: receipt.cycle_id,
                    phase: receipt.phase,
                });
            }
        }
        let encoded =
            serde_json::to_string(&receipt).map_err(|source| ExecutiveCycleError::Json {
                path: self.ledger_path.clone(),
                source,
            })?;
        writeln!(ledger, "{encoded}").map_err(|source| ExecutiveCycleError::Io {
            path: self.ledger_path.clone(),
            source,
        })?;
        ledger
            .sync_data()
            .map_err(|source| ExecutiveCycleError::Io {
                path: self.ledger_path.clone(),
                source,
            })?;
        Ok(ExecutiveCycleResult {
            receipt,
            replayed: false,
            ledger_appended: true,
        })
    }
}

fn validate(input: &ExecutiveCycleInput) -> Result<(), ExecutiveCycleError> {
    for (name, value) in [
        ("cycle_id", input.cycle_id.as_str()),
        ("objective_id", input.objective_id.as_str()),
        ("objective_source_ref", input.objective_source_ref.as_str()),
        ("context_receipt_ref", input.context_receipt_ref.as_str()),
        ("recommendation_id", input.recommendation_id.as_str()),
        ("proposed_action", input.proposed_action.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ExecutiveCycleError::Invalid(format!(
                "{name} cannot be empty"
            )));
        }
    }
    if input.resource_budget.max_roles == 0
        || input.resource_budget.max_dispatches == 0
        || !input.resource_budget.max_joules.is_finite()
        || !input.resource_budget.requested_joules.is_finite()
        || input.resource_budget.max_joules < 0.0
        || input.resource_budget.requested_joules < 0.0
    {
        return Err(ExecutiveCycleError::Invalid(
            "resource limits must be positive and joules non-negative".into(),
        ));
    }
    if input
        .requested_roles
        .iter()
        .chain(input.revised_requested_roles.iter())
        .any(|role| {
            role.role.trim().is_empty()
                || role.capabilities.is_empty()
                || role.capabilities.iter().any(|cap| cap.trim().is_empty())
        })
    {
        return Err(ExecutiveCycleError::Invalid(
            "roles require a name and non-empty capabilities".into(),
        ));
    }
    if input.failure_receipt_ref.is_some()
        && (input.revised_action.as_deref().is_none_or(str::is_empty)
            || input.revised_requested_roles.is_empty())
    {
        return Err(ExecutiveCycleError::Invalid(
            "a failed assumption requires a revised action and role composition".into(),
        ));
    }
    if input.failure_receipt_ref.is_some()
        && composition_digest(&input.requested_roles)
            == composition_digest(&input.revised_requested_roles)
    {
        return Err(ExecutiveCycleError::Invalid(
            "replan cannot blindly repeat the failed role composition".into(),
        ));
    }
    Ok(())
}

fn build_receipt(
    input: ExecutiveCycleInput,
    input_digest: String,
    now: DateTime<Utc>,
) -> ExecutiveCycleReceipt {
    let exceeded = resource_exceeded(&input);
    let full_council_unapproved = input.council_mode == CouncilMode::FullDeliberation
        && input.full_council_approval_ref.is_none();
    let (disposition, reason) = if input.read_only {
        (
            ExecutiveDisposition::ObservedReadOnly,
            "read-only inspection projected the next action without durable mutation".into(),
        )
    } else if input.operator_stop_requested {
        (
            ExecutiveDisposition::Stopped,
            "operator stop request suppressed queue handoff and execution".into(),
        )
    } else if input.approval_packet_id.is_none() && input.governance_authorization_id.is_none() {
        (
            ExecutiveDisposition::AwaitingReview,
            "canonical operator review or binding governance authorization is required before handoff"
                .into(),
        )
    } else if input.governance_receipt_ref.is_none() {
        (
            ExecutiveDisposition::Held,
            "governance receipt is missing".into(),
        )
    } else if full_council_unapproved {
        (
            ExecutiveDisposition::Held,
            "full council deliberation requires explicit approval".into(),
        )
    } else if !exceeded.is_empty() {
        (
            ExecutiveDisposition::Held,
            format!("resource bounds exceeded: {}", exceeded.join(",")),
        )
    } else if input.failure_receipt_ref.is_some() {
        (
            ExecutiveDisposition::Replanned,
            "failure receipt cited; revised action and non-repeating role composition proposed"
                .into(),
        )
    } else if !input.acceptance_receipt_refs.is_empty()
        && input.phase == ExecutivePhase::Assess
        && !input.execution_receipt_refs.is_empty()
    {
        (
            ExecutiveDisposition::Accepted,
            "acceptance receipts satisfy the declared objective assessment".into(),
        )
    } else if !input.acceptance_receipt_refs.is_empty() {
        (
            ExecutiveDisposition::Held,
            "acceptance requires assess phase and execution receipts".into(),
        )
    } else if !input.execution_receipt_refs.is_empty() {
        (
            ExecutiveDisposition::Assessing,
            "execution receipts observed; acceptance assessment remains open".into(),
        )
    } else if !input.queue_handoff_receipt_refs.is_empty() {
        (
            ExecutiveDisposition::HandedOff,
            "canonical queue accepted the approved plan; execution remains external".into(),
        )
    } else {
        (
            ExecutiveDisposition::Planned,
            "approved, governed role composition is ready for canonical queue handoff".into(),
        )
    };
    let queue_handoff_allowed = matches!(
        disposition,
        ExecutiveDisposition::Planned | ExecutiveDisposition::Replanned
    );
    let learning_candidate = input.failure_receipt_ref.as_ref().map(|failure| {
        format!(
            "avoid composition {} after failure {}; evaluate revised composition {}",
            composition_digest(&input.requested_roles),
            failure,
            composition_digest(&input.revised_requested_roles)
        )
    });
    let receipt_id = format!(
        "arandur-cycle-{}",
        &digest_bytes(format!("{}:{:?}:{input_digest}", input.cycle_id, input.phase).as_bytes())
            [..24]
    );
    let operator_update = format!(
        "objective={} phase={:?} disposition={:?}; {}; next={}",
        input.objective_id,
        input.phase,
        disposition,
        reason,
        input
            .revised_action
            .as_deref()
            .unwrap_or(input.proposed_action.as_str())
    );
    ExecutiveCycleReceipt {
        contract: EXECUTIVE_CYCLE_CONTRACT.into(),
        receipt_id,
        cycle_id: input.cycle_id,
        phase: input.phase,
        objective_id: input.objective_id,
        objective_source_ref: input.objective_source_ref,
        context_receipt_ref: input.context_receipt_ref,
        recommendation_id: input.recommendation_id,
        approval_packet_id: input.approval_packet_id,
        governance_authorization_id: input.governance_authorization_id,
        input_digest,
        disposition,
        reason,
        proposed_action: input.proposed_action,
        requested_roles: input.requested_roles,
        revised_action: input.revised_action,
        revised_requested_roles: input.revised_requested_roles,
        governance_receipt_ref: input.governance_receipt_ref,
        placement_receipt_refs: input.placement_receipt_refs,
        queue_handoff_receipt_refs: input.queue_handoff_receipt_refs,
        execution_observed: !input.execution_receipt_refs.is_empty(),
        execution_receipt_refs: input.execution_receipt_refs,
        failure_receipt_ref: input.failure_receipt_ref,
        acceptance_receipt_refs: input.acceptance_receipt_refs,
        council_mode: input.council_mode,
        full_council_approval_ref: input.full_council_approval_ref,
        resource_budget: input.resource_budget,
        queue_mutation_performed_by_arandur: false,
        placement_performed_by_arandur: false,
        execution_performed_by_arandur: false,
        queue_handoff_allowed,
        operator_can_stop: true,
        operator_update,
        learning_candidate,
        parent_receipt_id: input.parent_receipt_id,
        recorded_at_utc: now.to_rfc3339(),
    }
}

fn resource_exceeded(input: &ExecutiveCycleInput) -> Vec<&'static str> {
    let mut exceeded = Vec::new();
    if input.requested_roles.len() > input.resource_budget.max_roles {
        exceeded.push("roles");
    }
    if input.requested_roles.len() > input.resource_budget.max_dispatches
        || input.queue_handoff_receipt_refs.len() > input.resource_budget.max_dispatches
    {
        exceeded.push("dispatches");
    }
    if input.resource_budget.requested_joules > input.resource_budget.max_joules {
        exceeded.push("joules");
    }
    if input.resource_budget.requested_council_opinions > input.resource_budget.max_council_opinions
    {
        exceeded.push("council_opinions");
    }
    exceeded
}

fn composition_digest(roles: &[RoleRequest]) -> String {
    let mut normalized = roles
        .iter()
        .map(|role| {
            let mut caps = role.capabilities.clone();
            caps.sort();
            format!("{}:{}", role.role, caps.join("+"))
        })
        .collect::<Vec<_>>();
    normalized.sort();
    digest_bytes(normalized.join("|").as_bytes())
}

fn digest<T: Serialize>(value: &T) -> Result<String, ExecutiveCycleError> {
    let bytes = serde_json::to_vec(value).map_err(|source| ExecutiveCycleError::Json {
        path: PathBuf::from("<executive-cycle-input>"),
        source,
    })?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}
