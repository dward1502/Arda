use crate::run_graph::{AuthorityClass, RunGraph, WorkerRole, WorkerRouteClass};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilRoleKind {
    Proposer,
    SecurityCritic,
    ImplementationCritic,
    Adjudicator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilState {
    CollectingOpinions,
    PendingOperator,
    RevisionRequired,
    Concluded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CouncilAuthority {
    Advisory,
    HumanDecisionRequired,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilParticipant {
    pub role: CouncilRoleKind,
    pub node_id: String,
    pub worker_id: String,
    pub route_id: String,
    pub route_class: WorkerRouteClass,
    pub provider_id: String,
    pub model_id: String,
    pub opinion_digest: String,
    pub confidence: f64,
    pub uncertainty: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MaterialTension {
    pub tension_id: String,
    pub participant_roles: Vec<CouncilRoleKind>,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub resolved: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperatorDisposition {
    pub operator_id: String,
    pub decision: String,
    pub reason: String,
    pub receipt_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CouncilRun {
    pub schema_version: String,
    pub council_id: String,
    pub canonical_task_ref: String,
    pub run_id: String,
    pub question: String,
    pub evidence_boundary: Vec<String>,
    pub participants: Vec<CouncilParticipant>,
    pub agreements: Vec<String>,
    pub material_tensions: Vec<MaterialTension>,
    pub synthesis: String,
    pub escalation_recommendation: String,
    pub authority: CouncilAuthority,
    pub non_approval: bool,
    pub operator_disposition: Option<OperatorDisposition>,
    pub state: CouncilState,
}

impl CouncilRun {
    pub const SCHEMA_VERSION: &'static str = "arda.council-run.v1";

    pub fn validate(&self, graph: &RunGraph) -> Result<(), CouncilRunError> {
        if self.schema_version != Self::SCHEMA_VERSION {
            return Err(CouncilRunError::UnsupportedSchemaVersion(
                self.schema_version.clone(),
            ));
        }
        graph
            .validate()
            .map_err(|error| CouncilRunError::RunGraph(error.to_string()))?;
        if self.run_id != graph.run_id.as_str() {
            return Err(CouncilRunError::RunMismatch);
        }
        if !self.canonical_task_ref.starts_with("task:") {
            return Err(CouncilRunError::InvalidTaskReference);
        }
        require_text("council_id", &self.council_id)?;
        require_text("question", &self.question)?;
        if self.evidence_boundary.is_empty() {
            return Err(CouncilRunError::EmptyEvidenceBoundary);
        }
        if !self.non_approval {
            return Err(CouncilRunError::ApprovalClaim);
        }
        if !confidence_values_valid(&self.participants) {
            return Err(CouncilRunError::InvalidConfidence);
        }

        let required_roles = [
            CouncilRoleKind::Proposer,
            CouncilRoleKind::SecurityCritic,
            CouncilRoleKind::ImplementationCritic,
            CouncilRoleKind::Adjudicator,
        ];
        for required in required_roles {
            let count = self
                .participants
                .iter()
                .filter(|participant| participant.role == required)
                .count();
            if count != 1 {
                return Err(CouncilRunError::RoleCardinality(required));
            }
        }

        let mut worker_ids = BTreeSet::new();
        let mut opinion_digests = BTreeSet::new();
        let mut node_by_role = BTreeMap::new();
        for participant in &self.participants {
            require_text("worker_id", &participant.worker_id)?;
            require_text("route_id", &participant.route_id)?;
            require_text("provider_id", &participant.provider_id)?;
            require_text("model_id", &participant.model_id)?;
            require_text("uncertainty", &participant.uncertainty)?;
            if participant.evidence_refs.is_empty() {
                return Err(CouncilRunError::MissingOpinionEvidence(participant.role));
            }
            if !worker_ids.insert(participant.worker_id.as_str()) {
                return Err(CouncilRunError::DuplicateWorkerIdentity);
            }
            if !is_sha256_digest(&participant.opinion_digest)
                || !opinion_digests.insert(participant.opinion_digest.as_str())
            {
                return Err(CouncilRunError::InvalidOpinionDigest);
            }
            let node = graph
                .nodes
                .iter()
                .find(|node| node.id.as_str() == participant.node_id)
                .ok_or_else(|| CouncilRunError::MissingRunNode(participant.node_id.clone()))?;
            let worker = node.worker.as_ref().ok_or_else(|| {
                CouncilRunError::MissingWorkerContract(participant.node_id.clone())
            })?;
            if worker.worker_id != participant.worker_id
                || worker.route_id != participant.route_id
                || worker.route_class != participant.route_class
                || worker.role != expected_worker_role(participant.role)
                || node.authority != expected_authority(participant.role)
            {
                return Err(CouncilRunError::ParticipantProvenanceMismatch(
                    participant.node_id.clone(),
                ));
            }
            node_by_role.insert(participant.role as u8, node);
        }

        let adjudicator = node_by_role
            .get(&(CouncilRoleKind::Adjudicator as u8))
            .expect("required role validated");
        let adjudicator_worker = adjudicator.worker.as_ref().expect("worker validated");
        for role in [
            CouncilRoleKind::Proposer,
            CouncilRoleKind::SecurityCritic,
            CouncilRoleKind::ImplementationCritic,
        ] {
            let opinion_node = node_by_role
                .get(&(role as u8))
                .expect("required role validated");
            if !adjudicator_worker.dependencies.contains(&opinion_node.id) {
                return Err(CouncilRunError::AdjudicatorMissingDependency(role));
            }
        }

        for tension in &self.material_tensions {
            require_text("tension_id", &tension.tension_id)?;
            require_text("tension_summary", &tension.summary)?;
            if tension.participant_roles.len() < 2 || tension.evidence_refs.is_empty() {
                return Err(CouncilRunError::InvalidMaterialTension);
            }
        }
        require_text("synthesis", &self.synthesis)?;
        require_text("escalation_recommendation", &self.escalation_recommendation)?;

        match (self.authority, self.state, &self.operator_disposition) {
            (CouncilAuthority::HumanDecisionRequired, CouncilState::Concluded, None) => {
                return Err(CouncilRunError::MissingOperatorDisposition)
            }
            (CouncilAuthority::Advisory, _, Some(_)) => {
                return Err(CouncilRunError::UnexpectedOperatorDisposition)
            }
            _ => {}
        }
        if let Some(disposition) = &self.operator_disposition {
            require_text("operator_id", &disposition.operator_id)?;
            require_text("operator_decision", &disposition.decision)?;
            require_text("operator_reason", &disposition.reason)?;
            if !disposition.receipt_ref.starts_with("receipt:") {
                return Err(CouncilRunError::InvalidDispositionReceipt);
            }
        }
        Ok(())
    }

    pub fn stable_digest(&self) -> Result<String, CouncilRunError> {
        let encoded = serde_json::to_vec(self)
            .map_err(|error| CouncilRunError::Serialization(error.to_string()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(encoded)))
    }
}

fn expected_worker_role(role: CouncilRoleKind) -> WorkerRole {
    match role {
        CouncilRoleKind::Proposer => WorkerRole::PlannerProposer,
        CouncilRoleKind::SecurityCritic => WorkerRole::SecurityPrivacyCritic,
        CouncilRoleKind::ImplementationCritic => WorkerRole::ImplementationRiskCritic,
        CouncilRoleKind::Adjudicator => WorkerRole::Adjudicator,
    }
}

fn expected_authority(role: CouncilRoleKind) -> AuthorityClass {
    match role {
        CouncilRoleKind::Adjudicator => AuthorityClass::Verify,
        _ => AuthorityClass::ReadOnly,
    }
}

fn confidence_values_valid(participants: &[CouncilParticipant]) -> bool {
    participants.iter().all(|participant| {
        participant.confidence.is_finite() && (0.0..=1.0).contains(&participant.confidence)
    })
}

fn is_sha256_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn require_text(field: &'static str, value: &str) -> Result<(), CouncilRunError> {
    if value.trim().is_empty() {
        Err(CouncilRunError::EmptyField(field))
    } else {
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CouncilRunError {
    #[error("unsupported council schema version `{0}`")]
    UnsupportedSchemaVersion(String),
    #[error("invalid run graph: {0}")]
    RunGraph(String),
    #[error("council run does not reference the supplied run graph")]
    RunMismatch,
    #[error("canonical task reference must start with `task:`")]
    InvalidTaskReference,
    #[error("required field `{0}` is empty")]
    EmptyField(&'static str),
    #[error("council evidence boundary cannot be empty")]
    EmptyEvidenceBoundary,
    #[error("council output must be explicitly marked non-approval")]
    ApprovalClaim,
    #[error("participant confidence must be finite and between zero and one")]
    InvalidConfidence,
    #[error("council role `{0:?}` must appear exactly once")]
    RoleCardinality(CouncilRoleKind),
    #[error("council worker identities must be independent")]
    DuplicateWorkerIdentity,
    #[error("opinion digests must be unique sha256 digests")]
    InvalidOpinionDigest,
    #[error("participant `{0:?}` has no opinion evidence")]
    MissingOpinionEvidence(CouncilRoleKind),
    #[error("council participant references missing node `{0}`")]
    MissingRunNode(String),
    #[error("council participant node `{0}` has no worker contract")]
    MissingWorkerContract(String),
    #[error("participant provenance does not match run node `{0}`")]
    ParticipantProvenanceMismatch(String),
    #[error("adjudicator is not joined to `{0:?}`")]
    AdjudicatorMissingDependency(CouncilRoleKind),
    #[error("material tension must cite two roles and evidence")]
    InvalidMaterialTension,
    #[error("concluded human decision requires operator disposition")]
    MissingOperatorDisposition,
    #[error("advisory council cannot carry operator disposition")]
    UnexpectedOperatorDisposition,
    #[error("operator disposition must reference a receipt")]
    InvalidDispositionReceipt,
    #[error("unable to serialize council run: {0}")]
    Serialization(String),
}
