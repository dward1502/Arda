//! Approval-gated commercial drafts and delivery truth.

use crate::provider::{DispatchReceipt, FleetScope, TransportRequest};
use arda_core::capability_composition::CompositionScope;
use arda_core::company_ops::{
    ApprovalReceipt, CommercialEgress, CommercialLifecycleRecord, CommercialLifecycleState,
    CommercialLineage, PrivacyClass, ProposalDraft, COMMERCIAL_LIFECYCLE_SCHEMA_VERSION,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommercialDraft {
    pub lineage: CommercialLineage,
    pub business_scope: CompositionScope,
    pub privacy: PrivacyClass,
    pub egress: CommercialEgress,
    pub proposal: ProposalDraft,
    pub source_context: Vec<String>,
    pub commitments: Vec<String>,
    pub approval_required: bool,
}

impl CommercialDraft {
    pub fn prepare_external_request(
        &self,
        approval: &ApprovalReceipt,
        now: DateTime<Utc>,
    ) -> Result<TransportRequest, CommercialDeliveryError> {
        if !self.approval_required
            || approval.proposal_id != self.proposal.proposal_id
            || approval.expires_at < now
            || self.proposal.expires_at < now
            || approval.approved_scope != self.proposal.scope
            || approval.approved_price
                != format!(
                    "{} {}",
                    self.proposal.price.currency, self.proposal.price.range.expected
                )
            || approval.approved_due_at != self.proposal.proposed_due_at
        {
            return Err(CommercialDeliveryError::ApprovalRequired);
        }
        let lifecycle = CommercialLifecycleRecord {
            schema_version: COMMERCIAL_LIFECYCLE_SCHEMA_VERSION.into(),
            record_id: self.proposal.proposal_id,
            engagement_id: self.proposal.engagement_id,
            subject_id: self.proposal.proposal_id.to_string(),
            state: CommercialLifecycleState::Quote,
            lineage: self.lineage.clone(),
            business_scope: self.business_scope,
            privacy: self.privacy,
            evidence_receipt_ids: self.source_context.iter().cloned().collect::<BTreeSet<_>>(),
            artifact_receipt_ids: BTreeSet::new(),
            approval_receipt_id: Some(approval.receipt_id),
            egress: Some(self.egress.clone()),
            recorded_at: now,
        };
        lifecycle
            .validate()
            .map_err(|_| CommercialDeliveryError::InvalidLifecycle)?;
        let payload = serde_json::to_string(&(self, lifecycle))?;
        Ok(
            TransportRequest::new(self.proposal.proposal_id.to_string(), payload)
                .for_scope(FleetScope::External)
                .approved(true),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommercialDeliveryState {
    Attempted,
    Accepted,
    Delivered,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommercialDeliveryReceipt {
    pub proposal_id: String,
    pub provider_id: String,
    pub provider_message_id: Option<String>,
    pub state: CommercialDeliveryState,
    pub attempts: usize,
    pub error: Option<String>,
}

impl CommercialDeliveryReceipt {
    pub fn from_dispatch(proposal_id: impl Into<String>, receipt: DispatchReceipt) -> Self {
        let state = if receipt.delivery_proven() {
            CommercialDeliveryState::Delivered
        } else if receipt.dispatched {
            CommercialDeliveryState::Accepted
        } else if receipt.error.is_some() || receipt.timed_out {
            CommercialDeliveryState::Failed
        } else {
            CommercialDeliveryState::Attempted
        };
        Self {
            proposal_id: proposal_id.into(),
            provider_id: receipt.provider_id,
            provider_message_id: receipt.provider_message_id,
            state,
            attempts: receipt.attempts,
            error: receipt.error,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CommercialDeliveryError {
    #[error("external commercial send requires matching, unexpired operator approval")]
    ApprovalRequired,
    #[error("commercial draft serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("commercial delivery has an invalid lifecycle record")]
    InvalidLifecycle,
}
