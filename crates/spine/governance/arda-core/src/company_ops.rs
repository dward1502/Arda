//! Governed commercial-operations contracts.
//!
//! These records separate evidence, forecasts, proposals, approvals, execution,
//! and realized outcomes. They never grant Company Operations authority to send,
//! spend, publish, deploy, or make a client commitment on its own.

use crate::capability_composition::{CapabilityComposition, CompositionScope, EgressTarget};
use crate::run_graph::{ObjectiveId, RunGraph, RunId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use uuid::Uuid;

pub const COMPANY_OPS_SCHEMA_VERSION: &str = "arda.company-ops.v1";
pub const COMPANY_OPS_CONFIG_SCHEMA_VERSION: &str = "arda.company-operations.config.v1";
pub const COMMERCIAL_LIFECYCLE_SCHEMA_VERSION: &str = "arda.commercial-lifecycle.v1";
pub const COMPANY_OPERATIONS_CAPABILITY_ID: &str = "company_operations";

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum CompanyOpsError {
    #[error("company operations config I/O failed: {0}")]
    ConfigIo(String),
    #[error("company operations config parse failed: {0}")]
    ConfigParse(String),
    #[error("unsupported company operations config schema or adapter protocol")]
    UnsupportedConfig,
    #[error("confidence bounds and confidence must be finite and within their valid ranges")]
    InvalidConfidenceRange,
    #[error("monetary range minimum cannot exceed maximum")]
    InvalidValueRange,
    #[error("operator time budget must be positive and bounded")]
    InvalidTimeBudget,
    #[error("an approval receipt is required to create a commitment")]
    MissingApprovalReceipt,
    #[error("approval receipt does not authorize this proposal")]
    ApprovalMismatch,
    #[error("approval receipt has expired")]
    ApprovalExpired,
    #[error("realized value requires an outcome receipt")]
    MissingOutcomeReceipt,
    #[error("external action is outside the approved authority")]
    AuthorityDenied,
    #[error("experiment execution requires a matching approval receipt")]
    ExperimentApprovalRequired,
    #[error("client delivery bundle is missing acceptance evidence or a scope boundary")]
    InvalidDeliveryBundle,
    #[error("company operations must be explicitly selected for a business-scoped composition")]
    CompanyOperationsNotSelected,
    #[error("commercial project, objective, and run lineage does not match")]
    CommercialLineageMismatch,
    #[error("commercial lifecycle record is invalid")]
    InvalidCommercialRecord,
    #[error("external commercial state requires an explicit egress destination")]
    MissingCommercialEgress,
    #[error("commercial egress must name an external destination")]
    InvalidCommercialEgress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    General,
    CommercialConfidential,
    ContactRestricted,
    PersonalRestricted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommercialLineage {
    pub project_id: Uuid,
    pub project_contract_digest: String,
    pub objective_id: ObjectiveId,
    pub run_id: RunId,
}

impl CommercialLineage {
    pub fn from_composition(
        composition: &CapabilityComposition,
        run: &RunGraph,
    ) -> Result<Self, CompanyOpsError> {
        if composition.validate().is_err() || run.validate().is_err() {
            return Err(CompanyOpsError::InvalidCommercialRecord);
        }
        if composition.scope != CompositionScope::Business
            || !composition
                .capabilities
                .required
                .contains(COMPANY_OPERATIONS_CAPABILITY_ID)
        {
            return Err(CompanyOpsError::CompanyOperationsNotSelected);
        }
        if !run.matches_composition_lineage(composition) {
            return Err(CompanyOpsError::CommercialLineageMismatch);
        }
        Ok(Self {
            project_id: composition.lineage.project_id,
            project_contract_digest: composition.lineage.project_contract_digest.clone(),
            objective_id: ObjectiveId::new(composition.lineage.objective_id.clone())
                .map_err(|_| CompanyOpsError::InvalidCommercialRecord)?,
            run_id: RunId::new(composition.lineage.run_id.clone())
                .map_err(|_| CompanyOpsError::InvalidCommercialRecord)?,
        })
    }

    fn validate(&self) -> Result<(), CompanyOpsError> {
        if self.project_contract_digest.trim().is_empty()
            || self.objective_id.as_str().trim().is_empty()
            || self.run_id.as_str().trim().is_empty()
        {
            return Err(CompanyOpsError::InvalidCommercialRecord);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommercialLifecycleState {
    Opportunity,
    Quote,
    Deliverable,
    Acceptance,
    Invoice,
    Settlement,
    AccountingExport,
}

impl CommercialLifecycleState {
    pub fn requires_external_approval(self) -> bool {
        matches!(
            self,
            Self::Quote
                | Self::Deliverable
                | Self::Invoice
                | Self::Settlement
                | Self::AccountingExport
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommercialEgress {
    pub target: EgressTarget,
    pub destination: String,
}

impl CommercialEgress {
    fn validate(&self) -> Result<(), CompanyOpsError> {
        if self.destination.trim().is_empty()
            || !matches!(
                self.target,
                EgressTarget::HostedProvider
                    | EgressTarget::ExternalAdapter
                    | EgressTarget::PublicNetwork
            )
        {
            return Err(CompanyOpsError::InvalidCommercialEgress);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommercialLifecycleRecord {
    pub schema_version: String,
    pub record_id: Uuid,
    pub engagement_id: Uuid,
    pub subject_id: String,
    pub state: CommercialLifecycleState,
    pub lineage: CommercialLineage,
    pub business_scope: CompositionScope,
    pub privacy: PrivacyClass,
    #[serde(default)]
    pub evidence_receipt_ids: BTreeSet<String>,
    #[serde(default)]
    pub artifact_receipt_ids: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_receipt_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub egress: Option<CommercialEgress>,
    pub recorded_at: DateTime<Utc>,
}

impl CommercialLifecycleRecord {
    pub fn validate(&self) -> Result<(), CompanyOpsError> {
        if self.schema_version != COMMERCIAL_LIFECYCLE_SCHEMA_VERSION
            || self.subject_id.trim().is_empty()
            || self.business_scope != CompositionScope::Business
            || !matches!(
                self.privacy,
                PrivacyClass::CommercialConfidential | PrivacyClass::ContactRestricted
            )
            || self
                .evidence_receipt_ids
                .iter()
                .chain(&self.artifact_receipt_ids)
                .any(|receipt| receipt.trim().is_empty())
        {
            return Err(CompanyOpsError::InvalidCommercialRecord);
        }
        self.lineage.validate()?;
        if self.state.requires_external_approval() {
            if self.approval_receipt_id.is_none() {
                return Err(CompanyOpsError::MissingApprovalReceipt);
            }
            self.egress
                .as_ref()
                .ok_or(CompanyOpsError::MissingCommercialEgress)?
                .validate()?;
        } else if let Some(egress) = &self.egress {
            egress.validate()?;
        }
        if self.state == CommercialLifecycleState::Deliverable
            && self.artifact_receipt_ids.is_empty()
        {
            return Err(CompanyOpsError::InvalidCommercialRecord);
        }
        if matches!(
            self.state,
            CommercialLifecycleState::Acceptance
                | CommercialLifecycleState::Settlement
                | CommercialLifecycleState::AccountingExport
        ) && self.evidence_receipt_ids.is_empty()
        {
            return Err(CompanyOpsError::InvalidCommercialRecord);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommercialAuthority {
    ReadOnly,
    BoundedObservation,
    ProposalOnly,
    ReviewRequired,
    ExplicitOperatorApproval,
    Prohibited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyOpsConfig {
    pub schema_version: String,
    pub mode: String,
    pub authority: CompanyOpsAuthorityConfig,
    pub storage: CompanyOpsStorageConfig,
    pub adapters: CompanyOpsAdapterConfig,
    pub scoring: CompanyOpsScoringConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyOpsAuthorityConfig {
    pub external_messages: CommercialAuthority,
    pub scope_date_price: CommercialAuthority,
    pub spend: CommercialAuthority,
    pub deploy_publish: CommercialAuthority,
    pub legal_agreements: CommercialAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyOpsStorageConfig {
    pub event_log: String,
    pub opportunities_projection: String,
    pub drafts_projection: String,
    pub commitments_projection: String,
    pub experiments_projection: String,
    pub outcomes_projection: String,
    pub summary_projection: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyOpsAdapterConfig {
    pub protocol: String,
    pub secrets: String,
    pub crm_mode: String,
    pub communications_owner: String,
    pub work_execution_owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompanyOpsScoringConfig {
    pub show_components: bool,
    pub show_uncertainty: bool,
    pub include_operator_time: bool,
    pub include_strategic_fit: bool,
    pub include_family_time_constraints: bool,
}

impl CompanyOpsConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, CompanyOpsError> {
        let content = std::fs::read_to_string(path)
            .map_err(|error| CompanyOpsError::ConfigIo(error.to_string()))?;
        let config: Self = toml::from_str(&content)
            .map_err(|error| CompanyOpsError::ConfigParse(error.to_string()))?;
        if config.schema_version != COMPANY_OPS_CONFIG_SCHEMA_VERSION
            || config.adapters.protocol != "arda.company-adapter.v1"
        {
            return Err(CompanyOpsError::UnsupportedConfig);
        }
        Ok(config)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReference {
    pub source_id: String,
    pub source_kind: String,
    pub captured_at: DateTime<Utc>,
    pub digest: String,
    pub citation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterProvenance {
    pub adapter_id: String,
    pub adapter_version: String,
    pub external_id: String,
    pub observed_at: DateTime<Utc>,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceRange {
    pub low: f64,
    pub expected: f64,
    pub high: f64,
    pub confidence: f64,
}

impl ConfidenceRange {
    pub fn validate(&self) -> Result<(), CompanyOpsError> {
        if !self.low.is_finite()
            || !self.expected.is_finite()
            || !self.high.is_finite()
            || !self.confidence.is_finite()
            || self.low > self.expected
            || self.expected > self.high
            || !(0.0..=1.0).contains(&self.confidence)
        {
            return Err(CompanyOpsError::InvalidConfidenceRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValueEstimate {
    pub currency: String,
    pub range: ConfidenceRange,
    pub basis: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
}

impl ValueEstimate {
    pub fn validate(&self) -> Result<(), CompanyOpsError> {
        self.range.validate()?;
        if self.range.low < 0.0 {
            return Err(CompanyOpsError::InvalidValueRange);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RealizedValue {
    pub currency: String,
    pub amount: f64,
    pub outcome_receipt_id: Uuid,
    pub realized_at: DateTime<Utc>,
}

impl RealizedValue {
    pub fn from_outcome(
        currency: impl Into<String>,
        amount: f64,
        outcome: &OutcomeReceipt,
    ) -> Result<Self, CompanyOpsError> {
        if !amount.is_finite() || amount < 0.0 {
            return Err(CompanyOpsError::InvalidValueRange);
        }
        Ok(Self {
            currency: currency.into(),
            amount,
            outcome_receipt_id: outcome.receipt_id,
            realized_at: outcome.recorded_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperatorTimeBudget {
    pub expected_hours: ConfidenceRange,
    pub maximum_hours: f64,
    pub expires_at: DateTime<Utc>,
}

impl OperatorTimeBudget {
    pub fn validate(&self) -> Result<(), CompanyOpsError> {
        self.expected_hours.validate()?;
        if self.expected_hours.low < 0.0
            || !self.maximum_hours.is_finite()
            || self.maximum_hours <= 0.0
            || self.expected_hours.high > self.maximum_hours
        {
            return Err(CompanyOpsError::InvalidTimeBudget);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Organization {
    pub organization_id: Uuid,
    pub display_name: String,
    pub privacy: PrivacyClass,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_provenance: Option<AdapterProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContactReference {
    pub contact_id: Uuid,
    pub organization_id: Uuid,
    pub display_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub private_locator: Option<String>,
    pub privacy: PrivacyClass,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_provenance: Option<AdapterProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TelemetryContactReference {
    pub contact_id: Uuid,
    pub organization_id: Uuid,
    pub privacy: PrivacyClass,
    pub redacted: bool,
}

impl ContactReference {
    /// Return the only contact representation allowed in general telemetry.
    pub fn for_general_telemetry(&self) -> TelemetryContactReference {
        TelemetryContactReference {
            contact_id: self.contact_id,
            organization_id: self.organization_id,
            privacy: self.privacy,
            redacted: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EngagementState {
    Lead,
    Qualified,
    Proposed,
    Won,
    Lost,
    Delivered,
    Invoiced,
    Paid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientEngagement {
    pub engagement_id: Uuid,
    pub organization_id: Uuid,
    pub title: String,
    pub state: EngagementState,
    pub expected_value: ValueEstimate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realized_value: Option<RealizedValue>,
    pub authority: CommercialAuthority,
    pub privacy: PrivacyClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Opportunity {
    pub opportunity_id: Uuid,
    pub organization_id: Uuid,
    pub title: String,
    pub stage: EngagementState,
    pub expected_value: ValueEstimate,
    pub operator_time: OperatorTimeBudget,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductHypothesis {
    pub hypothesis_id: Uuid,
    pub customer_problem: String,
    pub proposed_offer: String,
    pub target_audience: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
    pub expected_value: ValueEstimate,
    pub build_time: OperatorTimeBudget,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExperimentDecision {
    Continue,
    Pivot,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevenueExperiment {
    pub experiment_id: Uuid,
    pub hypothesis: ProductHypothesis,
    pub success_threshold: String,
    pub stop_condition: String,
    pub maximum_spend: ValueEstimate,
    pub maximum_operator_time: OperatorTimeBudget,
    pub authority: CommercialAuthority,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_receipt_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<ExperimentDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalReceipt {
    pub receipt_id: Uuid,
    pub proposal_id: Uuid,
    pub approved_by: String,
    pub approved_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub approved_scope: String,
    pub approved_price: String,
    pub approved_due_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposalDraft {
    pub proposal_id: Uuid,
    pub engagement_id: Uuid,
    pub title: String,
    pub scope: String,
    pub price: ValueEstimate,
    pub proposed_due_at: DateTime<Utc>,
    pub audience: String,
    pub risk: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
    pub authority: CommercialAuthority,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub commitment_id: Uuid,
    pub proposal_id: Uuid,
    pub engagement_id: Uuid,
    pub scope: String,
    pub price: String,
    pub due_at: DateTime<Utc>,
    pub approval_receipt_id: Uuid,
    pub approved_by: String,
    pub created_at: DateTime<Utc>,
}

impl ProposalDraft {
    pub fn into_commitment(
        self,
        approval: ApprovalReceipt,
        now: DateTime<Utc>,
    ) -> Result<Commitment, CompanyOpsError> {
        if approval.proposal_id != self.proposal_id {
            return Err(CompanyOpsError::ApprovalMismatch);
        }
        if now > approval.expires_at || now > self.expires_at {
            return Err(CompanyOpsError::ApprovalExpired);
        }
        Ok(Commitment {
            commitment_id: Uuid::new_v4(),
            proposal_id: self.proposal_id,
            engagement_id: self.engagement_id,
            scope: approval.approved_scope,
            price: approval.approved_price,
            due_at: approval.approved_due_at,
            approval_receipt_id: approval.receipt_id,
            approved_by: approval.approved_by,
            created_at: now,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeKind {
    Reply,
    Meeting,
    Trial,
    Sale,
    Loss,
    Delivered,
    Invoiced,
    Paid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeReceipt {
    pub receipt_id: Uuid,
    pub engagement_id: Uuid,
    pub experiment_id: Option<Uuid>,
    pub kind: OutcomeKind,
    pub recorded_at: DateTime<Utc>,
    pub summary: String,
    pub delivery_cost: Option<ValueEstimate>,
    pub operator_assessment: String,
    #[serde(default)]
    pub evidence: Vec<EvidenceReference>,
    pub reviewed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchObjectiveProposal {
    pub objective_id: Uuid,
    pub experiment_id: Uuid,
    pub project_contract_id: String,
    pub acceptance_criteria: Vec<String>,
    pub scope_boundary: String,
    pub authority: CommercialAuthority,
    pub approval_receipt_id: Uuid,
}

impl RevenueExperiment {
    pub fn into_workbench_objective(
        &self,
        approval: &ApprovalReceipt,
        project_contract_id: impl Into<String>,
        acceptance_criteria: Vec<String>,
        scope_boundary: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<WorkbenchObjectiveProposal, CompanyOpsError> {
        let scope_boundary = scope_boundary.into();
        if self.authority != CommercialAuthority::ExplicitOperatorApproval
            || self.approval_receipt_id != Some(approval.receipt_id)
            || approval.expires_at < now
            || acceptance_criteria.is_empty()
            || scope_boundary.trim().is_empty()
        {
            return Err(CompanyOpsError::ExperimentApprovalRequired);
        }
        Ok(WorkbenchObjectiveProposal {
            objective_id: Uuid::new_v4(),
            experiment_id: self.experiment_id,
            project_contract_id: project_contract_id.into(),
            acceptance_criteria,
            scope_boundary,
            authority: CommercialAuthority::ReviewRequired,
            approval_receipt_id: approval.receipt_id,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDeliveryBundle {
    pub commitment_id: Uuid,
    pub workbench_run_id: String,
    pub deliverables: Vec<String>,
    pub acceptance_evidence: Vec<String>,
    pub change_requests: Vec<String>,
    pub overrun_warning: Option<String>,
    pub handoff_boundary: String,
    pub support_boundary: String,
    pub invoice_export_only: bool,
    pub lifecycle: CommercialLifecycleRecord,
}

impl ClientDeliveryBundle {
    pub fn validate(&self) -> Result<(), CompanyOpsError> {
        self.lifecycle.validate()?;
        if self.deliverables.is_empty()
            || self.acceptance_evidence.is_empty()
            || self.handoff_boundary.trim().is_empty()
            || self.support_boundary.trim().is_empty()
            || !self.invoice_export_only
            || self.lifecycle.state != CommercialLifecycleState::Deliverable
            || self.lifecycle.subject_id != self.commitment_id.to_string()
            || self.lifecycle.lineage.run_id.as_str() != self.workbench_run_id
        {
            return Err(CompanyOpsError::InvalidDeliveryBundle);
        }
        Ok(())
    }
}
