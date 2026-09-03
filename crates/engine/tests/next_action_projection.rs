use arda_core::next_action::{NextActionAuthorityState, NextActionSourceKind, NextActionStatus};
use arda_core::personal_ops::{
    CaptureContent, CaptureRecordedEvent, CaptureSource, EvidenceClass, InboxCapture,
    ItemClassifiedEvent, PersonalItemKind, PersonalOpsEnvelope, PersonalOpsRecord,
};
use arda_engine::next_action::publish_next_action_projection;
use arda_engine::personal_ops::PersonalOpsLogStore;
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 20, 18, 0, 0).unwrap()
}

fn write_queue(root: &std::path::Path, rows: &[serde_json::Value]) {
    let path = root.join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let content = rows
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    fs::write(path, content).unwrap();
}

fn write_current_run(root: &std::path::Path, state: &str) {
    let run_id = "run-current";
    let run_dir = root.join("data/runs").join(run_id);
    fs::create_dir_all(&run_dir).unwrap();
    fs::write(
        run_dir.join("checkpoint.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": "arda.run-graph.v1",
            "run_id": run_id,
            "objective_id": "objective-current",
            "nodes": [{
                "id": "approval",
                "kind": "approval",
                "state": state,
                "authority": "human_approval",
                "budget": {"max_joules": 10.0, "max_cost_usd": 0.0},
                "retry": {"max_attempts": 1},
                "timeout_ms": 60000,
                "idempotency_key": "approval-current",
                "input_digest": null,
                "output_digest": null,
                "parent_receipts": [],
                "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
            }],
            "edges": [],
            "provenance": {
                "project_contract_digest": "sha256:project",
                "created_by": "operator-test",
                "parent_receipts": []
            }
        }))
        .unwrap(),
    )
    .unwrap();
    fs::create_dir_all(root.join("data/workbench")).unwrap();
    fs::write(
        root.join("data/workbench/current-runs.json"),
        serde_json::to_vec_pretty(&json!({
            "schema_version": "arda.workbench.current-runs.v1",
            "run_ids": [run_id]
        }))
        .unwrap(),
    )
    .unwrap();
}

fn write_personal_item(root: &std::path::Path, evidence_class: EvidenceClass) -> Uuid {
    let item_id = Uuid::new_v4();
    let store = PersonalOpsLogStore::new(root);
    store
        .append(&PersonalOpsEnvelope::new(
            PersonalOpsRecord::CaptureRecorded(CaptureRecordedEvent {
                event_id: Uuid::new_v4(),
                occurred_at: now(),
                operator_id: "operator:mythos".to_string(),
                capture: InboxCapture {
                    capture_id: item_id,
                    captured_at: now(),
                    source: CaptureSource::Text,
                    content: CaptureContent {
                        text: Some("Prepare the transplant-safe grocery list".to_string()),
                        audio_reference: None,
                    },
                    attachments: Vec::new(),
                    project_id: None,
                    priority: None,
                    due_at: None,
                },
            }),
        ))
        .unwrap();
    store
        .append(&PersonalOpsEnvelope::new(
            PersonalOpsRecord::ItemClassified(ItemClassifiedEvent {
                event_id: Uuid::new_v4(),
                occurred_at: now(),
                operator_id: "operator:mythos".to_string(),
                item_id,
                kind: PersonalItemKind::Task,
                evidence_class,
                confidence: None,
                rationale: Some("test classification".to_string()),
            }),
        ))
        .unwrap();
    item_id
}

#[test]
fn source_projection_selects_current_operator_queue_and_excludes_future_gate() {
    let root = tempfile::tempdir().unwrap();
    write_queue(
        root.path(),
        &[
            json!({
                "id": "current-critical",
                "title": "Review Arda against the operator vision",
                "status": "pending",
                "priority": "critical",
                "owner": "operator:mythos",
                "origin": "operator-authored-session-objective",
                "meta": {"mutation_risk": "review_required", "execution_authority": "none_until_review", "lifecycle_phase": "current"}
            }),
            json!({
                "id": "future-economic",
                "title": "Create a funded agent account",
                "status": "pending",
                "priority": "critical",
                "owner": "operator:mythos",
                "origin": "operator-authored-session-objective",
                "meta": {"mutation_risk": "review_required", "execution_authority": "none_until_review", "lifecycle_phase": "future-gated"}
            }),
            json!({
                "id": "agent-inferred",
                "title": "Agent-inferred task without review",
                "status": "pending",
                "priority": "critical",
                "owner": "agent:planner",
                "origin": "inferred",
                "meta": {"lifecycle_phase": "current"}
            }),
        ],
    );

    let projection = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();

    assert_eq!(projection.status, NextActionStatus::Ready);
    let selected = projection.selected.unwrap();
    assert_eq!(selected.id, "current-critical");
    assert_eq!(selected.source_kind, NextActionSourceKind::Queue);
    assert_eq!(
        selected.authority_state,
        NextActionAuthorityState::ReviewRequired
    );
    assert_eq!(projection.excluded.future_gated, 1);
    assert_eq!(projection.excluded.inferred_without_review, 1);
}

#[test]
fn awaiting_workbench_approval_preempts_queue_and_survives_reopen() {
    let root = tempfile::tempdir().unwrap();
    write_queue(
        root.path(),
        &[json!({
            "id": "queue-high",
            "title": "Continue core repair",
            "status": "pending",
            "priority": "high",
            "owner": "operator:mythos",
            "origin": "operator-authored-session-objective",
            "meta": {"lifecycle_phase": "current"}
        })],
    );
    write_current_run(root.path(), "pending");

    let before = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();
    let after = publish_next_action_projection(
        root.path(),
        "operator:mythos",
        now() + chrono::Duration::minutes(1),
    )
    .unwrap();

    assert_eq!(before.selected.as_ref().unwrap().id, "run-current");
    assert_eq!(
        before.selected.as_ref().unwrap().source_kind,
        NextActionSourceKind::Workbench
    );
    assert_eq!(
        before.selected.as_ref().unwrap().authority_state,
        NextActionAuthorityState::ReviewRequired
    );
    assert_eq!(after.selected.as_ref().unwrap().id, "run-current");
    assert!(root.path().join("core/state/next_action.json").is_file());
}

#[test]
fn operator_authored_personal_item_preempts_lower_priority_queue_work() {
    let root = tempfile::tempdir().unwrap();
    write_queue(
        root.path(),
        &[json!({
            "id": "queue-low",
            "title": "Low priority queue work",
            "status": "pending",
            "priority": "low",
            "owner": "operator:mythos",
            "origin": "operator-authored-session-objective",
            "meta": {"lifecycle_phase": "current"}
        })],
    );
    let item_id = write_personal_item(root.path(), EvidenceClass::OperatorAuthored);

    let projection = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();

    assert_eq!(
        projection.selected.as_ref().unwrap().id,
        item_id.to_string()
    );
    assert_eq!(
        projection.selected.as_ref().unwrap().source_kind,
        NextActionSourceKind::PersonalOperations
    );
}

#[test]
fn research_projection_excludes_expired_question_and_selects_current_hold() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("data/workbench/research/questions.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        serde_json::to_vec_pretty(&json!({
            "schema_version": "arda.workbench.research-questions.v1",
            "records": [
                {"question_id": "expired", "owner": "operator:mythos", "question": "Old question", "state": "enabled", "expires_at_utc": "2026-08-19T00:00:00Z"},
                {"question_id": "current", "owner": "operator:mythos", "question": "Current bounded question", "state": "enabled", "expires_at_utc": "2026-08-21T00:00:00Z"}
            ]
        }))
        .unwrap(),
    )
    .unwrap();

    let projection = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();

    assert_eq!(projection.selected.as_ref().unwrap().id, "current");
    assert_eq!(
        projection.selected.as_ref().unwrap().source_kind,
        NextActionSourceKind::Research
    );
    assert_eq!(projection.excluded.stale, 1);
}
