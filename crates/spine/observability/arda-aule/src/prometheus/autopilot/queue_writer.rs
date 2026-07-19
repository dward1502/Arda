#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Persist PlannedTasks back into `core/projects/tasks/queue.jsonl` as
//! pending records other agents/services will pick up.

use super::apollo_bridge::Dispatch;
use super::decomposer::PlannedTask;
use super::delegation::DelegationReport;
use chrono::Utc;
use serde_json::json;
use std::io::Write;
use std::path::Path;

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
        oracle_conditions,
        "not_evaluated",
        &[],
    )
}

pub fn append_plan_to_queue_with_gate_metadata(
    queue_path: impl AsRef<Path>,
    objective_id: &str,
    plan: &[PlannedTask],
    delegation: Option<&DelegationReport>,
    oracle_conditions: &[String],
    autonomy_readiness_decision: &str,
    autonomy_readiness_reasons: &[String],
) -> std::io::Result<Vec<String>> {
    let path = queue_path.as_ref();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let now = Utc::now();
    let mut written = Vec::new();
    for t in plan {
        let owner = delegation
            .and_then(|d| d.delegations.iter().find(|x| x.task_key == t.key))
            .map(|d| d.assigned_agent.clone())
            .or_else(|| t.assigned_agent.clone())
            .unwrap_or_else(|| "ceo".into());
        let id = task_id_for(objective_id, &t.key, now);
        let record = json!({
            "id": id,
            "title": t.title,
            "owner": owner,
            "priority": format!("{:?}", t.priority).to_lowercase(),
            "status": "pending",
            "task_type": t.task_type,
            "depends_on": t.depends_on,
            "joule_cost_estimate": t.joule_cost,
            "eta_seconds": t.eta_seconds,
            "queued_at_utc": now.to_rfc3339(),
            "meta": {
                "origin": "ceo_autopilot",
                "objective_id": objective_id,
                "plan_key": t.key,
                "oracle_conditions": oracle_conditions,
                "autonomy_readiness_decision": autonomy_readiness_decision,
                "autonomy_readiness_reasons": autonomy_readiness_reasons,
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
        ExecutionStatus::Pending | ExecutionStatus::Running => ("in_progress", "running"),
    };

    let path = queue_path.as_ref();
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let now = Utc::now();
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
        "started_at_utc": now.to_rfc3339(),
        "completed_at_utc": now.to_rfc3339(),
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
    fn appends_plan_records() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        let ids = append_plan_to_queue(&q, "obj1", &[task("a"), task("b")], None).unwrap();
        assert_eq!(ids.len(), 2);
        let contents = std::fs::read_to_string(&q).unwrap();
        assert_eq!(contents.lines().count(), 2);
        assert!(contents.contains("\"objective_id\":\"obj1\""));
        assert!(contents.contains("\"status\":\"pending\""));
    }

    #[test]
    fn appends_oracle_conditions_to_plan_records() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        let conditions = vec!["require operator-visible rollback path".to_string()];
        let ids = append_plan_to_queue_with_conditions(&q, "obj1", &[task("a")], None, &conditions)
            .unwrap();
        assert_eq!(ids.len(), 1);
        let contents = std::fs::read_to_string(&q).unwrap();
        assert!(
            contents.contains("\"oracle_conditions\":[\"require operator-visible rollback path\"]")
        );
    }

    #[test]
    fn appends_apollo_completion_record() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        let task = task("apollo");
        let dispatch = Dispatch::Submitted {
            task_id: "tsk_apollo".into(),
            status: ExecutionStatus::Completed,
            joules: 2.5,
            transport: "in_process",
        };
        assert!(append_apollo_dispatch_to_queue(&q, "obj1", &task, &dispatch).unwrap());
        let contents = std::fs::read_to_string(&q).unwrap();
        assert!(contents.contains("\"id\":\"tsk_apollo\""));
        assert!(contents.contains("\"status\":\"completed\""));
        assert!(contents.contains("\"apollo_transport\":\"in_process\""));
    }

    #[test]
    fn appends_apollo_retry_attempt_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        let task = task("apollo");
        let dispatch = Dispatch::Submitted {
            task_id: "tsk_apollo".into(),
            status: ExecutionStatus::Timeout,
            joules: 1.0,
            transport: "daemon",
        };
        assert!(
            append_apollo_dispatch_attempt_to_queue(&q, "obj1", &task, &dispatch, 2, 3).unwrap()
        );
        let contents = std::fs::read_to_string(&q).unwrap();
        assert!(contents.contains("\"status\":\"failed\""));
        assert!(contents.contains("\"result\":\"timeout\""));
        assert!(contents.contains("\"retry_attempt\":2"));
        assert!(contents.contains("\"retry_max_attempts\":3"));
    }
}
