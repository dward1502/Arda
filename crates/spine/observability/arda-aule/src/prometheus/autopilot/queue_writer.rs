#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Persist PlannedTasks back into `core/projects/tasks/queue.jsonl` as
//! pending records other agents/services will pick up.

use super::core_executor_bridge::{Dispatch, ExecutionStatus};
use super::decomposer::PlannedTask;
use super::delegation::DelegationReport;
use chrono::Utc;
use serde_json::json;
use std::io::Write;
use std::path::Path;

pub(super) struct QueueGateMetadata<'a> {
    pub oracle_conditions: &'a [String],
    pub autonomy_readiness_decision: &'a str,
    pub autonomy_readiness_reasons: &'a [String],
    pub source_objective_packet_id: Option<&'a str>,
    pub approval_packet_id: Option<&'a str>,
    pub governance_authorization_id: Option<&'a str>,
    pub governance_action_class: Option<&'a str>,
    pub governance_gate: Option<&'a str>,
    pub mutation_risk: &'a str,
}

pub fn task_id_for(objective_id: &str, plan_key: &str, ts: chrono::DateTime<Utc>) -> String {
    format!(
        "tsk_{}_{}__{}",
        ts.format("%Y%m%dT%H%M%SZ"),
        objective_id,
        plan_key
    )
}

pub fn append_plan_to_queue(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &[PlannedTask],
    delegation: Option<&DelegationReport>,
) -> std::io::Result<Vec<String>> {
    append_plan_to_queue_with_conditions(queue_path, objective_id, plan, delegation, &[])
}

pub fn append_plan_to_queue_with_conditions(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &[PlannedTask],
    delegation: Option<&DelegationReport>,
    oracle_conditions: &[String],
) -> std::io::Result<Vec<String>> {
    append_plan_to_queue_with_gate_metadata(
        queue_path,
        objective_id,
        plan,
        delegation,
        QueueGateMetadata {
            oracle_conditions,
            autonomy_readiness_decision: "not_evaluated",
            autonomy_readiness_reasons: &[],
            source_objective_packet_id: None,
            approval_packet_id: None,
            governance_authorization_id: None,
            governance_action_class: None,
            governance_gate: None,
            mutation_risk: "unclassified",
        },
    )
}

pub(super) fn append_plan_to_queue_with_gate_metadata(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &[PlannedTask],
    delegation: Option<&DelegationReport>,
    gate: QueueGateMetadata<'_>,
) -> std::io::Result<Vec<String>> {
    super::schedule::require_legacy_schedule_writer()?;
    let path = queue_path.as_ref();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let now = Utc::now();
    // Resolve plan keys to full canonical task IDs so depends_on
    // entries reference the actual queue records, not local template keys.
    let id_for_key: std::collections::HashMap<&str, String> = plan
        .iter()
        .map(|t| (t.key.as_str(), task_id_for(objective_id, &t.key, now)))
        .collect();
    let mut written = Vec::new();
    for t in plan {
        let owner = delegation
            .and_then(|d| d.delegations.iter().find(|x| x.task_key == t.key))
            .map(|d| d.assigned_agent.clone())
            .or_else(|| t.assigned_agent.clone())
            .unwrap_or_else(|| "ceo".into());
        let id = id_for_key.get(t.key.as_str()).cloned().unwrap_or_else(|| task_id_for(objective_id, &t.key, now));
        let depends_on: Vec<String> = t
            .depends_on
            .iter()
            .map(|dep| {
                id_for_key
                    .get(dep.as_str())
                    .cloned()
                    .unwrap_or_else(|| dep.clone())
            })
            .collect();
        let record = json!({
            "id": id,
            "title": t.title,
            "owner": owner,
            "priority": format!("{:?}", t.priority).to_lowercase(),
            "status": "pending",
            "task_type": t.task_type,
            "depends_on": depends_on,
            "joule_cost_estimate": t.joule_cost,
            "eta_seconds": t.eta_seconds,
            "queued_at_utc": now.to_rfc3339(),
            "meta": {
                "origin": "ceo_autopilot",
                "objective_id": objective_id,
                "plan_key": t.key,
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": gate.mutation_risk,
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": gate.source_objective_packet_id,
                "approval_packet_id": gate.approval_packet_id,
                "governance_authorization_id": gate.governance_authorization_id,
                "governance_action_class": gate.governance_action_class,
                "governance_gate": gate.governance_gate,
                "oracle_conditions": gate.oracle_conditions,
                "autonomy_readiness_decision": gate.autonomy_readiness_decision,
                "autonomy_readiness_reasons": gate.autonomy_readiness_reasons,
            },
            "glyphs": ["∇"],
        });
        writeln!(f, "{}", record)?;
        written.push(id);
    }
    Ok(written)
}

pub fn append_apollo_dispatch_to_queue(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &PlannedTask,
    dispatch: &Dispatch,
) -> std::io::Result<bool> {
    append_apollo_dispatch_attempt_to_queue(queue_path, objective_id, plan, dispatch, 1, 1)
}

pub fn append_apollo_dispatch_attempt_to_queue(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &PlannedTask,
    dispatch: &Dispatch,
    attempt: u32,
    max_attempts: u32,
) -> std::io::Result<bool> {
    super::schedule::require_legacy_schedule_writer()?;
    let Dispatch::Submitted {
        task_id,
        status,
        joules,
        transport,
    } = dispatch
    else {
        return Ok(false);
    };

    let (queue_status, result) = match status {
        ExecutionStatus::Completed => ("completed", "completed"),
        ExecutionStatus::Failed => ("failed", "failed"),
        ExecutionStatus::Cancelled => ("failed", "cancelled"),
        ExecutionStatus::Timeout => ("failed", "timeout"),
        ExecutionStatus::Pending => ("pending", "submitted"),
        ExecutionStatus::Running => ("in_progress", "running"),
    };

    super::schedule::require_legacy_schedule_writer()?;
    let path = queue_path.as_ref();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let now = Utc::now();
    let started_at_utc = if matches!(status, ExecutionStatus::Pending) {
        None
    } else {
        Some(now.to_rfc3339())
    };
    let completed_at_utc = if matches!(
        status,
        ExecutionStatus::Completed
            | ExecutionStatus::Failed
            | ExecutionStatus::Cancelled
            | ExecutionStatus::Timeout
    ) {
        Some(now.to_rfc3339())
    } else {
        None
    };
    let owner = plan.assigned_agent.clone().unwrap_or_else(|| "ceo".into());
    let record = json!({
        "id": task_id,
        "title": plan.title,
        "owner": owner,
        "priority": format!("{:?}", plan.priority).to_lowercase(),
        "status": queue_status,
        "result": result,
        "task_type": plan.task_type,
        "depends_on": plan.depends_on,
        "joule_cost_estimate": plan.joule_cost,
        "joule_work": joules,
        "queued_at_utc": now.to_rfc3339(),
        "started_at_utc": started_at_utc,
        "completed_at_utc": completed_at_utc,
        "meta": {
            "origin": "ceo_autopilot",
            "objective_id": objective_id,
            "plan_key": plan.key,
            "apollo": true,
            "apollo_transport": transport,
            "apollo_status": format!("{:?}", status).to_lowercase(),
            "retry_attempt": attempt,
            "retry_max_attempts": max_attempts,
        },
        "glyphs": ["∇", "⚡"],
    });
    writeln!(f, "{}", record)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::super::decomposer::Priority;
    use super::*;
    fn task(k: &str) -> PlannedTask {
        PlannedTask {
            key: k.into(),
            title: format!("T {k}"),
            task_type: "ops".into(),
            depends_on: vec![],
            priority: Priority::Medium,
            joule_cost: 5.0,
            eta_seconds: 30,
            assigned_agent: Some("ceo".into()),
        }
    }
    #[test]
    fn retired_plan_writer_preserves_missing_and_malformed_history() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("missing/queue.jsonl");
        for historical in [false, true] {
            if historical {
                std::fs::create_dir_all(q.parent().unwrap()).unwrap();
                std::fs::write(&q, b"malformed history\n").unwrap();
            }
            let error =
                append_plan_to_queue(&q, "obj1", &[task("a"), task("b")], None).unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert!(error.to_string().contains("retired"));
            if historical {
                assert_eq!(std::fs::read(&q).unwrap(), b"malformed history\n");
            } else {
                assert!(!q.parent().unwrap().exists());
            }
        }
    }

    #[test]
    fn retired_condition_and_dispatch_writers_preserve_all_history() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("missing/queue.jsonl");
        for bytes in [
            None,
            Some(b"malformed history\n".as_slice()),
            Some(b"{\"id\":\"historic\",\"status\":\"completed\"}\n".as_slice()),
        ] {
            if let Some(bytes) = bytes {
                std::fs::create_dir_all(q.parent().unwrap()).unwrap();
                std::fs::write(&q, bytes).unwrap();
            }
            let conditions = vec!["rollback required".into()];
            let error =
                append_plan_to_queue_with_conditions(&q, "obj", &[task("a")], None, &conditions)
                    .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            assert!(error.to_string().contains("retired"));
            let mut dispatches = vec![Dispatch::Skipped {
                reason: "not submitted".into(),
            }];
            for status in [
                ExecutionStatus::Pending,
                ExecutionStatus::Running,
                ExecutionStatus::Completed,
                ExecutionStatus::Failed,
                ExecutionStatus::Cancelled,
                ExecutionStatus::Timeout,
            ] {
                dispatches.push(Dispatch::Submitted {
                    task_id: "historic".into(),
                    status,
                    joules: 1.0,
                    transport: "arda_core_queue",
                });
            }
            for dispatch in dispatches {
                for error in [
                    append_apollo_dispatch_to_queue(&q, "obj", &task("a"), &dispatch).unwrap_err(),
                    append_apollo_dispatch_attempt_to_queue(&q, "obj", &task("a"), &dispatch, 2, 3)
                        .unwrap_err(),
                ] {
                    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
                    assert!(error.to_string().contains("retired"));
                }
            }
            if let Some(bytes) = bytes {
                assert_eq!(std::fs::read(&q).unwrap(), bytes);
            } else {
                assert!(!q.parent().unwrap().exists());
            }
        }
    }
}
