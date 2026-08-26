#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Approved Arandur queue-operation boundary for canonical queue appends.

use super::decomposer::PlannedTask;
use super::delegation::DelegationReport;
use super::planner::ObjectivePacket;
use super::queue_writer::{append_plan_to_queue_with_gate_metadata, QueueGateMetadata};
use serde::Serialize;
use std::path::Path;

pub const QUEUE_OPERATION_CONTRACT: &str = "arda.arandur.queue_operation.v1";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QueueOperationStatus {
    Appended,
    BlockedReadOnly,
    BlockedMissingApproval,
    BlockedPacketNotSelected,
    BlockedPacketDisallowsMutation,
    BlockedAutonomyReadiness,
    BlockedWriteFailed,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueOperation {
    pub contract: String,
    pub operation_id: String,
    pub source_objective_packet_id: String,
    pub approval_packet_id: Option<String>,
    pub append_target: String,
    pub read_only: bool,
    pub mutation_authorized: bool,
    pub result_status: QueueOperationStatus,
    pub result_path: Option<String>,
    pub blocked_reason_code: Option<String>,
    pub appended_task_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct QueueOperationBlockedSummary {
    pub reason_code: String,
    pub count: usize,
}

impl QueueOperation {
    pub fn blocked(
        operation_id: impl Into<String>,
        packet: &ObjectivePacket,
        append_target: impl AsRef<Path>,
        read_only: bool,
        status: QueueOperationStatus,
        reason_code: impl Into<String>,
    ) -> Self {
        Self {
            contract: QUEUE_OPERATION_CONTRACT.into(),
            operation_id: operation_id.into(),
            source_objective_packet_id: packet.packet_id.clone(),
            approval_packet_id: packet.approval_packet_id.clone(),
            append_target: append_target.as_ref().to_string_lossy().to_string(),
            read_only,
            mutation_authorized: false,
            result_status: status,
            result_path: None,
            blocked_reason_code: Some(reason_code.into()),
            appended_task_ids: Vec::new(),
        }
    }
}

// Keep every authorization input explicit at this security-sensitive queue boundary.
#[allow(clippy::too_many_arguments)]
pub fn append_approved_packet_plan(
    queue_path: impl AsRef<Path>,
    packet: &ObjectivePacket,
    objective_id: &str,
    plan: &[PlannedTask],
    delegation: Option<&DelegationReport>,
    oracle_conditions: &[String],
    autonomy_readiness_decision: &str,
    autonomy_readiness_reasons: &[String],
    read_only: bool,
) -> QueueOperation {
    let queue_path = queue_path.as_ref();
    let operation_id = format!("queue_operation:{}", packet.packet_id);
    if read_only {
        return QueueOperation::blocked(
            operation_id,
            packet,
            queue_path,
            true,
            QueueOperationStatus::BlockedReadOnly,
            "read_only_mode",
        );
    }
    if !packet.selected {
        return QueueOperation::blocked(
            operation_id,
            packet,
            queue_path,
            false,
            QueueOperationStatus::BlockedPacketNotSelected,
            "objective_packet_not_selected",
        );
    }
    if packet.approval_packet_id.is_none() {
        return QueueOperation::blocked(
            operation_id,
            packet,
            queue_path,
            false,
            QueueOperationStatus::BlockedMissingApproval,
            "operator_approval_packet_missing",
        );
    }
    if !packet.canonical_queue_mutation_allowed {
        return QueueOperation::blocked(
            operation_id,
            packet,
            queue_path,
            false,
            QueueOperationStatus::BlockedPacketDisallowsMutation,
            "objective_packet_disallows_canonical_queue_mutation",
        );
    }

    match append_plan_to_queue_with_gate_metadata(
        queue_path,
        objective_id,
        plan,
        delegation,
        QueueGateMetadata {
            oracle_conditions,
            autonomy_readiness_decision,
            autonomy_readiness_reasons,
            source_objective_packet_id: Some(&packet.packet_id),
            approval_packet_id: packet.approval_packet_id.as_deref(),
        },
    ) {
        Ok(appended_task_ids) => QueueOperation {
            contract: QUEUE_OPERATION_CONTRACT.into(),
            operation_id,
            source_objective_packet_id: packet.packet_id.clone(),
            approval_packet_id: packet.approval_packet_id.clone(),
            append_target: queue_path.to_string_lossy().to_string(),
            read_only: false,
            mutation_authorized: true,
            result_status: QueueOperationStatus::Appended,
            result_path: Some(queue_path.to_string_lossy().to_string()),
            blocked_reason_code: None,
            appended_task_ids,
        },
        Err(_) => QueueOperation::blocked(
            operation_id,
            packet,
            queue_path,
            false,
            QueueOperationStatus::BlockedWriteFailed,
            "queue_append_failed",
        ),
    }
}

pub fn blocked_summaries(operations: &[QueueOperation]) -> Vec<QueueOperationBlockedSummary> {
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for operation in operations {
        if let Some(reason_code) = &operation.blocked_reason_code {
            *counts.entry(reason_code.clone()).or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(reason_code, count)| QueueOperationBlockedSummary { reason_code, count })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::Priority;
    use super::super::governance_policy::GovernanceGate;
    use super::super::planner::{ObjectivePacketInput, OBJECTIVE_PACKET_CONTRACT};
    use super::super::source_registry::ArandurSourceType;
    use super::*;

    fn packet(selected: bool, approval_packet_id: Option<String>) -> ObjectivePacket {
        let mut packet = ObjectivePacket::from_report(
            "arda.arandur.recommendations.v1",
            ArandurSourceType::ArandurRecommendation,
            ObjectivePacketInput {
                source_path: "/repo/data/arandur/recommendations.jsonl".into(),
                source_record_id: "reco-1".into(),
                candidate_id: "candidate-1".into(),
                title: "Execute approved gate".into(),
                owner: Some("prometheus".into()),
                priority: Some("high".into()),
                governance_class: "operator_approved".into(),
                review_gate: GovernanceGate::SafeAutonomous,
                acceptance_criteria: vec!["approval present".into()],
                approval_packet_id,
                completion_receipt_path: None,
                blocked_reason_code: None,
                selected,
            },
        );
        packet.canonical_queue_mutation_allowed = true;
        assert_eq!(packet.contract, OBJECTIVE_PACKET_CONTRACT);
        packet
    }

    fn task() -> PlannedTask {
        PlannedTask {
            key: "step".into(),
            title: "Run approved step".into(),
            task_type: "ops".into(),
            depends_on: Vec::new(),
            priority: Priority::High,
            joule_cost: 1.0,
            eta_seconds: 30,
            assigned_agent: Some("prometheus".into()),
        }
    }

    #[test]
    fn read_only_queue_operation_rejects_mutation_without_writing_queue() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let queue_path = dir.path().join("queue.jsonl");
        let operation = append_approved_packet_plan(
            &queue_path,
            &packet(true, Some("approval-1".into())),
            "candidate-1",
            &[task()],
            None,
            &[],
            "allow",
            &[],
            true,
        );

        assert_eq!(operation.contract, QUEUE_OPERATION_CONTRACT);
        assert_eq!(
            operation.result_status,
            QueueOperationStatus::BlockedReadOnly
        );
        assert!(!operation.mutation_authorized);
        assert_eq!(
            operation.blocked_reason_code.as_deref(),
            Some("read_only_mode")
        );
        assert!(!queue_path.exists());
    }

    #[test]
    fn queue_operation_requires_explicit_approval_packet() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let queue_path = dir.path().join("queue.jsonl");
        let operation = append_approved_packet_plan(
            &queue_path,
            &packet(true, None),
            "candidate-1",
            &[task()],
            None,
            &[],
            "allow",
            &[],
            false,
        );

        assert_eq!(
            operation.result_status,
            QueueOperationStatus::BlockedMissingApproval
        );
        assert_eq!(
            operation.blocked_reason_code.as_deref(),
            Some("operator_approval_packet_missing")
        );
        assert!(!queue_path.exists());
    }

    #[test]
    fn queue_operation_appends_only_authorized_selected_packet() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let queue_path = dir.path().join("queue.jsonl");
        let operation = append_approved_packet_plan(
            &queue_path,
            &packet(true, Some("approval-1".into())),
            "candidate-1",
            &[task()],
            None,
            &[],
            "allow",
            &[],
            false,
        );

        assert_eq!(operation.result_status, QueueOperationStatus::Appended);
        assert!(operation.mutation_authorized);
        assert_eq!(operation.appended_task_ids.len(), 1);
        let contents = std::fs::read_to_string(&queue_path)
            .unwrap_or_else(|err| panic!("queue read failed: {err}"));
        assert!(contents.contains("\"objective_id\":\"candidate-1\""));
    }

    #[test]
    fn blocked_queue_operation_summaries_count_reason_codes() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let queue_path = dir.path().join("queue.jsonl");
        let operations = vec![
            append_approved_packet_plan(
                &queue_path,
                &packet(true, None),
                "candidate-1",
                &[task()],
                None,
                &[],
                "allow",
                &[],
                false,
            ),
            append_approved_packet_plan(
                &queue_path,
                &packet(true, None),
                "candidate-2",
                &[task()],
                None,
                &[],
                "allow",
                &[],
                false,
            ),
            append_approved_packet_plan(
                &queue_path,
                &packet(true, Some("approval-1".into())),
                "candidate-3",
                &[task()],
                None,
                &[],
                "allow",
                &[],
                true,
            ),
        ];
        let summaries = blocked_summaries(&operations);

        assert!(summaries.iter().any(|summary| {
            summary.reason_code == "operator_approval_packet_missing" && summary.count == 2
        }));
        assert!(summaries
            .iter()
            .any(|summary| { summary.reason_code == "read_only_mode" && summary.count == 1 }));
    }
}
