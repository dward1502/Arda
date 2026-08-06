//! Append-only Company Operations ledger and deterministic projections.

mod projection;
mod store;

pub use projection::{
    build_projection, score_opportunity, CompanyOpsProjection, ScoredOpportunity, ValueScore,
};
pub use store::{AppendOutcome, CompanyOpsStore, CompanyOpsStoreError};

use arda_core::company_ops::{
    ClientEngagement, Commitment, Opportunity, OutcomeReceipt, ProposalDraft, RevenueExperiment,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const COMPANY_OPS_EVENT_SCHEMA_VERSION: &str = "arda.company-ops.event.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompanyOpsEvent {
    pub schema_version: String,
    pub event_id: Uuid,
    pub occurred_at: DateTime<Utc>,
    pub idempotency_key: String,
    #[serde(flatten)]
    pub kind: CompanyOpsEventKind,
}

impl CompanyOpsEvent {
    pub fn new(
        idempotency_key: impl Into<String>,
        occurred_at: DateTime<Utc>,
        kind: CompanyOpsEventKind,
    ) -> Self {
        Self {
            schema_version: COMPANY_OPS_EVENT_SCHEMA_VERSION.into(),
            event_id: Uuid::new_v4(),
            occurred_at,
            idempotency_key: idempotency_key.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event_type", content = "record", rename_all = "snake_case")]
pub enum CompanyOpsEventKind {
    EngagementObserved(ClientEngagement),
    OpportunityObserved(Opportunity),
    ProposalDrafted(ProposalDraft),
    CommitmentApproved(Commitment),
    ExperimentProposed(RevenueExperiment),
    OutcomeRecorded(OutcomeReceipt),
}
