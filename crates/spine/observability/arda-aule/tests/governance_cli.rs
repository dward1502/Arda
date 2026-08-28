#![cfg(feature = "full-cli")]

use serde_json::Value;
use std::process::Command;

fn arda_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_arda-cli"))
}

#[test]
fn governance_metrics_command_emits_machine_readable_contract() {
    let output = arda_cli()
        .args(["governance-metrics", "--json"])
        .output()
        .expect("run governance metrics command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot: Value = serde_json::from_slice(&output.stdout).expect("metrics JSON");
    assert!(snapshot.get("collection_mode").is_some());
    assert_eq!(snapshot["owns_http_server"], false);
    assert!(snapshot["counters"].is_array());
    assert!(snapshot["histograms"].is_array());
}

#[test]
fn governance_status_command_reports_conservative_readiness() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ledger = temp.path().join("bacon_lite.jsonl");
    std::fs::write(&ledger, "").expect("empty ledger");

    let output = arda_cli()
        .arg("governance-status")
        .arg("--path")
        .arg(&ledger)
        .arg("--json")
        .output()
        .expect("run governance status command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("status JSON");
    assert!(report.get("readiness").is_some());
    assert!(report.get("recent_ledger").is_some());
    assert!(report.get("metrics").is_some());
}

#[test]
fn autopilot_cli_pauses_and_resumes_canonical_schedule() {
    let temp = tempfile::tempdir().expect("tempdir");
    let schedule_path = temp.path().join("core/projects/tasks/schedules.jsonl");
    std::fs::create_dir_all(schedule_path.parent().unwrap()).expect("schedule directory");
    std::fs::write(
        &schedule_path,
        format!(
            "{}\n",
            serde_json::json!({
                "contract": "arda.workbench.schedule_record.v1",
                "task_id": "task-1",
                "objective_id": "objective-1",
                "mode": "deferred",
                "state": "scheduled",
                "not_before_utc": "2030-01-01T00:00:00Z",
                "recorded_at_utc": "2026-08-28T00:00:00Z"
            })
        ),
    )
    .expect("initial schedule");

    let pause = arda_cli()
        .args([
            "prometheus",
            "autopilot",
            "pause-schedule",
            "task-1",
            "--objective-id",
            "objective-1",
            "--reason",
            "operator maintenance",
            "--root",
        ])
        .arg(temp.path())
        .output()
        .expect("run pause schedule command");
    assert!(
        pause.status.success(),
        "{}",
        String::from_utf8_lossy(&pause.stderr)
    );
    let paused: Value = serde_json::from_slice(&pause.stdout).expect("pause JSON");
    assert_eq!(paused["task_id"], "task-1");
    assert_eq!(paused["objective_id"], "objective-1");
    assert_eq!(paused["state"], "paused");
    assert_eq!(paused["reason"], "operator maintenance");

    let resume = arda_cli()
        .args([
            "prometheus",
            "autopilot",
            "resume-schedule",
            "task-1",
            "--objective-id",
            "objective-1",
            "--reason",
            "operator resumed",
            "--root",
        ])
        .arg(temp.path())
        .output()
        .expect("run resume schedule command");
    assert!(
        resume.status.success(),
        "{}",
        String::from_utf8_lossy(&resume.stderr)
    );
    let resumed: Value = serde_json::from_slice(&resume.stdout).expect("resume JSON");
    assert_eq!(resumed["state"], "scheduled");
    assert_eq!(resumed["reason"], "operator resumed");
    assert_eq!(resumed["not_before_utc"], "2030-01-01T00:00:00Z");

    let raw = std::fs::read_to_string(schedule_path).expect("schedule ledger");
    assert_eq!(raw.lines().count(), 3);
}

#[test]
fn autopilot_cli_reprioritizes_canonical_task() {
    let temp = tempfile::tempdir().expect("tempdir");
    let queue_path = temp.path().join("core/projects/tasks/queue.jsonl");
    std::fs::create_dir_all(queue_path.parent().unwrap()).expect("queue directory");
    std::fs::write(
        &queue_path,
        format!(
            "{}\n",
            serde_json::json!({
                "id": "task-1",
                "title": "Operator-controlled task",
                "owner": "prometheus",
                "priority": "medium",
                "status": "queued",
                "meta": {
                    "action_class": "approved_autopilot_plan_step",
                    "mutation_risk": "operator-approved",
                    "execution_authority": "arda_workbench",
                    "source_objective_packet_id": "objective-1",
                    "approval_packet_id": "approval-1"
                }
            })
        ),
    )
    .expect("initial queue");

    let output = arda_cli()
        .args([
            "prometheus",
            "autopilot",
            "reprioritize-task",
            "task-1",
            "--objective-id",
            "objective-1",
            "--priority",
            "critical",
            "--reason",
            "operator escalation",
            "--root",
        ])
        .arg(temp.path())
        .output()
        .expect("run reprioritize command");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let record: Value = serde_json::from_slice(&output.stdout).expect("reprioritize JSON");
    assert_eq!(record["id"], "task-1");
    assert_eq!(record["title"], "Operator-controlled task");
    assert_eq!(record["priority"], "critical");
    assert_eq!(
        record["contract"],
        "arda.workbench.queue_reprioritization.v1"
    );
    assert_eq!(record["operator_reason"], "operator escalation");
    assert_eq!(
        std::fs::read_to_string(queue_path)
            .expect("queue ledger")
            .lines()
            .count(),
        2
    );
}
