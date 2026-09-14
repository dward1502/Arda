use arda_core::next_action::{NextActionAuthorityState, NextActionSourceKind, NextActionStatus};
use arda_core::personal_ops::{
    CaptureContent, CaptureRecordedEvent, CaptureSource, EvidenceClass, InboxCapture,
    ItemClassifiedEvent, PersonalItemKind, PersonalOpsEnvelope, PersonalOpsRecord,
};
use arda_engine::next_action::publish_next_action_projection;
#[path = "fixtures/objective_agenda.rs"]
mod objective_agenda;
use arda_engine::personal_ops::PersonalOpsLogStore;
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::fs;
use uuid::Uuid;

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 20, 18, 0, 0).unwrap()
}

fn write_queue(root: &std::path::Path, rows: &[serde_json::Value]) {
    for row in rows {
        objective_agenda::seed(
            root,
            row["id"].as_str().unwrap(),
            "operator:mythos",
            "approved",
            if row["priority"] == "low" { 30 } else { 70 },
        );
    }
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
fn source_projection_selects_only_current_objectives_owned_by_the_operator() {
    let root = tempfile::tempdir().unwrap();
    objective_agenda::seed(
        root.path(),
        "current-critical",
        "operator:mythos",
        "pending_approval",
        90,
    );
    objective_agenda::seed(
        root.path(),
        "other-operator",
        "operator:other",
        "approved",
        100,
    );
    objective_agenda::seed(root.path(), "closed", "operator:mythos", "completed", 100);
    let projection = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();
    assert_eq!(projection.status, NextActionStatus::Ready);
    let selected = projection.selected.unwrap();
    assert_eq!(selected.id, "current-critical");
    assert_eq!(selected.source_kind, NextActionSourceKind::Objective);
    assert_eq!(
        selected.authority_state,
        NextActionAuthorityState::ReviewRequired
    );
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

    objective_agenda::seed(
        root.path(),
        "objective-current",
        "operator:mythos",
        "running",
        70,
    );
    rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap()
        .execute("UPDATE leaves SET execution_run_id = 'run-current' WHERE id = 'leaf-objective-current'", []).unwrap();
    // The registry is neither required nor a fallback source of authority.
    fs::remove_file(root.path().join("data/workbench/current-runs.json")).unwrap();

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

#[test]
fn historical_registry_cannot_supply_next_action_without_current_owned_leaf_authority() {
    for state in [
        None,
        Some("completed"),
        Some("cancelled"),
        Some("failed"),
        Some("other_operator"),
        Some("unbound"),
    ] {
        let root = tempfile::tempdir().unwrap();
        write_current_run(root.path(), "pending");
        if let Some(state) = state {
            let owner = if state == "other_operator" {
                "operator:other"
            } else {
                "operator:mythos"
            };
            let objective_state = if matches!(state, "other_operator" | "unbound") {
                "running"
            } else {
                state
            };
            objective_agenda::seed(root.path(), "objective-current", owner, objective_state, 70);
            if state != "unbound" {
                rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3"))
                    .unwrap()
                    .execute("UPDATE leaves SET execution_run_id = 'run-current'", [])
                    .unwrap();
            }
        }
        let projection =
            publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap();
        assert!(
            projection
                .selected
                .as_ref()
                .is_none_or(|candidate| candidate.source_kind != NextActionSourceKind::Workbench),
            "historical run selected: {state:?}"
        );
    }
}

#[test]
fn checkpoint_identity_must_match_the_owned_resident_binding() {
    let root = tempfile::tempdir().unwrap();
    write_current_run(root.path(), "pending");
    objective_agenda::seed(
        root.path(),
        "different-objective",
        "operator:mythos",
        "running",
        70,
    );
    rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3"))
        .unwrap()
        .execute("UPDATE leaves SET execution_run_id = 'run-current'", [])
        .unwrap();
    let error = publish_next_action_projection(root.path(), "operator:mythos", now()).unwrap_err();
    assert!(error.to_string().contains("checkpoint disagrees"));
}
