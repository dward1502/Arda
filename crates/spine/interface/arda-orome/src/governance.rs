// sigil: REPAIR
//! Central governance hooks for typed approval and interruption ledger records.

use crate::types::{
    InterruptionEnvelope, InterruptionLedgerDecision, InterruptionMessage, TaskApprovalEnvelope,
};
use arda_core::governance_gates::{GovernanceGates, GovernancePolicyMode};
use arda_core::ledger::Ledger;
use chrono::Utc;
use std::path::Path;

pub struct GovernanceHooks {
    gates: GovernanceGates,
    ledger: Ledger,
}

impl GovernanceHooks {
    pub fn new(gates: GovernanceGates, ledger_dir: impl AsRef<Path>) -> anyhow::Result<Self> {
        Ok(Self {
            gates,
            ledger: Ledger::new(ledger_dir)?,
        })
    }

    pub fn record_task_approval(
        &self,
        proposal_id: impl Into<String>,
        approval_id: impl Into<String>,
    ) -> anyhow::Result<TaskApprovalEnvelope> {
        let ledger_path = self.ledger.path().display().to_string();
        let envelope = TaskApprovalEnvelope {
            schema_version: "arda.orome.task_approval.v1".to_string(),
            proposal_id: proposal_id.into(),
            approval_id: approval_id.into(),
            ledger_writes: vec![ledger_path],
            decision: self.decision_for("task_approval"),
            created_at_utc: Utc::now().to_rfc3339(),
        };
        self.ledger.append(&envelope)?;
        Ok(envelope)
    }

    pub fn record_interruption(
        &self,
        event_id: impl Into<String>,
        message: InterruptionMessage,
        action_class: &str,
    ) -> anyhow::Result<InterruptionEnvelope> {
        let ledger_path = self.ledger.path().display().to_string();
        let envelope = InterruptionEnvelope {
            schema_version: "arda.orome.interruption.v1".to_string(),
            event_id: event_id.into(),
            message,
            ledger_writes: vec![ledger_path],
            decision: self.decision_for(action_class),
            created_at_utc: Utc::now().to_rfc3339(),
        };
        self.ledger.append(&envelope)?;
        Ok(envelope)
    }

    fn decision_for(&self, action_class: &str) -> InterruptionLedgerDecision {
        match self
            .gates
            .policy_for_action_class(action_class)
            .policy_mode()
        {
            GovernancePolicyMode::ObserveOnly | GovernancePolicyMode::RecordAndProceed => {
                InterruptionLedgerDecision::PolicySafe
            }
            GovernancePolicyMode::EscalateToHuman
            | GovernancePolicyMode::RequireIndependentReceipts => {
                InterruptionLedgerDecision::RequiresOperatorReview
            }
            GovernancePolicyMode::BlockOnFail => InterruptionLedgerDecision::PolicyBlocked,
        }
    }
}
