use arda_core::operator_projection::{
    DependencyHealth, MeasurementSource, ObjectiveStatus, RunStatus,
};
use arda_engine::operator_projection::publish_operator_projection;
#[path = "fixtures/objective_agenda.rs"]
mod objective_agenda;
use chrono::{TimeZone, Utc};
use std::fs;

fn write_run(root: &std::path::Path, run_id: &str, state: &str) {
    objective_agenda::seed(root, "objective-live", "operator:test", "running", 70);
    rusqlite::Connection::open(root.join("data/arda/objectives.sqlite3"))
        .unwrap()
        .execute(
            "UPDATE leaves SET execution_run_id = ?1 WHERE id = 'leaf-objective-live'",
            [run_id],
        )
        .unwrap();
    let directory = root.join("data/runs").join(run_id);
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("checkpoint.json"),
        format!(
            r#"{{
  "schema_version": "arda.run-graph.v1",
  "run_id": "{run_id}",
  "objective_id": "objective-live",
  "nodes": [{{
    "id": "plan",
    "kind": "plan",
    "state": "{state}",
    "authority": "read_only",
    "budget": {{ "max_joules": 40.0, "max_cost_usd": 0.0 }},
    "retry": {{ "max_attempts": 1 }},
    "timeout_ms": 60000,
    "idempotency_key": "{run_id}-plan",
    "input_digest": "objective:objective-live",
    "output_digest": null,
    "parent_receipts": [],
    "checkpoint": {{ "sequence": 0, "recovery_token": null, "checkpoint_digest": null }}
  }}],
  "edges": [],
  "provenance": {{
    "project_contract_digest": "project:live",
    "created_by": "publisher-test",
    "parent_receipts": []
  }}
}}"#
        ),
    )
    .unwrap();
    let registry_path = root.join("data/workbench/current-runs.json");
    fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    let mut run_ids = fs::read_to_string(&registry_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
        .and_then(|value| value["run_ids"].as_array().cloned())
        .unwrap_or_default();
    run_ids.push(serde_json::Value::String(run_id.to_owned()));
    fs::write(
        registry_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": "arda.workbench.current-runs.v1",
            "run_ids": run_ids,
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn publishes_valid_projection_from_canonical_run_and_resource_stores() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    fs::create_dir_all(root.path().join("data/resource-ledger")).unwrap();
    fs::write(
        root.path().join("data/resource-ledger/events.jsonl"),
        r#"{"schema_version":"arda.resource-ledger-entry.v1","sequence":1,"run_id":"run-live","idempotency_key":"usage-1","source":"observed","provider_id":null,"local_joulework":12.5,"hosted_cost_usd":0.0,"hosted_requests":0,"recorded_after_run_completion":false,"recorded_at_unix_ms":1}
"#,
    )
    .unwrap();

    let generated_at = Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap();
    let projection = publish_operator_projection(root.path(), generated_at).unwrap();

    assert_eq!(projection.objectives.len(), 1);
    assert_eq!(projection.objectives[0].objective_id, "objective-live");
    assert_eq!(projection.objectives[0].status, ObjectiveStatus::Active);
    assert_eq!(projection.runs[0].run_id, "run-live");
    assert_eq!(projection.runs[0].status, RunStatus::Running);
    assert_eq!(projection.runs[0].nodes[0].node_id, "plan");
    assert_eq!(projection.joulework.budget_joules, 40.0);
    assert_eq!(projection.joulework.consumed_joules, 12.5);
    assert_eq!(projection.joulework.remaining_joules, 27.5);
    assert_eq!(projection.joulework.source, MeasurementSource::Observed);
    projection.validate().unwrap();

    let persisted =
        fs::read_to_string(root.path().join("core/state/operator_projection.json")).unwrap();
    assert_eq!(
        arda_core::operator_projection::OperatorProjection::from_json_str(&persisted).unwrap(),
        projection
    );
}

#[test]
fn corrupt_runtime_input_does_not_replace_last_valid_projection() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "pending");
    let generated_at = Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap();
    publish_operator_projection(root.path(), generated_at).unwrap();
    let output_path = root.path().join("core/state/operator_projection.json");
    let before = fs::read(&output_path).unwrap();

    fs::write(
        root.path().join("data/runs/run-live/checkpoint.json"),
        "{not-json",
    )
    .unwrap();
    assert!(publish_operator_projection(root.path(), generated_at).is_err());
    assert_eq!(fs::read(output_path).unwrap(), before);
}

#[test]
fn corrupt_objective_store_does_not_replace_last_valid_projection() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "pending");
    let now = Utc::now();
    publish_operator_projection(root.path(), now).unwrap();
    let output = root.path().join("core/state/operator_projection.json");
    let before = fs::read(&output).unwrap();
    fs::write(
        root.path().join("data/arda/objectives.sqlite3"),
        "invalid SQLite",
    )
    .unwrap();
    assert!(publish_operator_projection(root.path(), now).is_err());
    assert_eq!(fs::read(output).unwrap(), before);
}

#[test]
fn missing_queue_and_schedule_inputs_remain_honest_absent_fields() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let objective = &projection.objectives[0];

    assert_eq!(objective.current_run_id.as_deref(), Some("run-live"));
    assert_eq!(
        objective.current_task_id.as_deref(),
        Some("leaf-objective-live")
    );
    assert!(objective.next_continuation.is_none());
    assert!(objective.next_wake_at.is_none());
    assert!(objective.blocker.is_none());
}

#[test]
fn approval_without_canonical_expiry_is_visible_as_an_unavailable_dependency() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "ready");
    let checkpoint = root.path().join("data/runs/run-live/checkpoint.json");
    let approval = fs::read_to_string(&checkpoint)
        .unwrap()
        .replace("\"kind\": \"plan\"", "\"kind\": \"approval\"");
    fs::write(checkpoint, approval).unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();

    assert_eq!(projection.runs[0].status, RunStatus::AwaitingApproval);
    assert!(projection.pending_approvals.is_empty());
    let dependency = projection
        .dependencies
        .iter()
        .find(|dependency| dependency.dependency_id == "approval_expiry_store")
        .expect("missing-expiry state must be explicit");
    assert_eq!(dependency.health, DependencyHealth::NotConfigured);
    assert!(dependency.detail.contains("expiry"));
}

#[test]
fn capability_projection_preserves_versions_and_derives_optional_only_from_receipt_reason() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    fs::write(
        root.path()
            .join("data/runs/run-live/capability-composition.json"),
        r#"{
  "schema_version":"arda.capability-composition-receipt.v1",
  "run_id":"run-live",
  "composition_digest":"sha256:composition",
  "registry_constraint_digest":"sha256:registry",
  "trigger":"initial",
  "prior_receipt_digest":null,
  "model_recommendations":[],
  "selected_capabilities":[{"id":"search","version":"2","owner":"arda"}],
  "decisions":[
    {"capability":{"id":"search","version":"2","owner":"arda"},"selected":true,"reasons":["signed_request"]},
    {"capability":{"id":"search","version":"1","owner":"arda"},"selected":false,"reasons":["alternate_version_not_selected"]},
    {"capability":{"id":"voice","version":"1","owner":"arda"},"selected":false,"reasons":["not_required_by_signed_contract_or_role"]}
  ]
}"#,
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();

    assert_eq!(projection.capabilities.len(), 2);
    let search = projection
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == "search")
        .unwrap();
    assert_eq!(search.version, "2");
    assert!(search.selected);
    assert!(!search.optional);
    let voice = projection
        .capabilities
        .iter()
        .find(|capability| capability.capability_id == "voice")
        .unwrap();
    assert!(voice.optional);
}

#[test]
fn historical_terminal_runs_remain_stored_but_are_excluded_from_current_projection() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "runtime-proof-20260813-v2", "succeeded");
    write_run(root.path(), "run-current", "running");

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 20, 18, 0, 0).unwrap(),
    )
    .unwrap();

    assert_eq!(projection.runs.len(), 1);
    assert_eq!(projection.runs[0].run_id, "run-current");
    assert!(root
        .path()
        .join("data/runs/runtime-proof-20260813-v2/checkpoint.json")
        .is_file());
    let run_store = projection
        .dependencies
        .iter()
        .find(|dependency| dependency.dependency_id == "run_store")
        .unwrap();
    assert!(run_store.detail.contains("1 historical checkpoint"));
}

// Queue control/alias/schedule tests were retired with the live queue reader.
// Current control and scheduling authority is exclusively ObjectiveStore.
#[test]
fn canonical_objective_without_a_run_is_visible_and_pause_suppresses_wake() {
    let root = tempfile::tempdir().unwrap();
    let store = objective_agenda::seed(root.path(), "current", "operator:test", "approved", 70);
    store
        .put_schedule(
            arda_engine::objectives::ScheduleSpec {
                id: "wake".into(),
                objective_id: "current".into(),
                next_wake_ms: 5000,
                recurrence: None,
                idempotency_key: "wake".into(),
            },
            1,
        )
        .unwrap();
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert_eq!(projection.objectives.len(), 1);
    let objective = &projection.objectives[0];
    assert_eq!(objective.title, "Current objective current");
    assert_eq!(objective.status, ObjectiveStatus::Pending);
    assert_eq!(objective.next_wake_at.unwrap().timestamp_millis(), 5000);
    assert!(objective.current_run_id.is_none());
    store
        .apply_control(
            "current",
            arda_engine::objectives::ControlAction::Pause,
            "pause",
            "operator:test",
            2,
        )
        .unwrap();
    let paused = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert_eq!(paused.objectives[0].status, ObjectiveStatus::Blocked);
    assert!(paused.objectives[0].next_wake_at.is_none());
    assert_eq!(
        paused.objectives[0].blocker.as_deref(),
        Some("paused by operator")
    );
}

#[test]
fn terminal_objectives_and_legacy_only_objectives_do_not_reenter_the_agenda() {
    let root = tempfile::tempdir().unwrap();
    for state in ["completed", "cancelled", "failed"] {
        objective_agenda::seed(root.path(), state, "operator:test", state, 100);
    }
    let legacy = root.path().join("core/projects/tasks");
    fs::create_dir_all(&legacy).unwrap();
    fs::write(
        legacy.join("queue.jsonl"),
        "{\"id\":\"ghost\",\"status\":\"in_progress\",\"meta\":{\"objective_id\":\"ghost\"}}\n",
    )
    .unwrap();
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert!(projection.objectives.is_empty());
}

#[test]
fn run_evidence_digests_are_deduplicated() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let checkpoint = root.path().join("data/runs/run-live/checkpoint.json");
    let mut graph: serde_json::Value =
        serde_json::from_slice(&fs::read(&checkpoint).unwrap()).unwrap();
    let first = &mut graph["nodes"][0];
    first["state"] = serde_json::json!("succeeded");
    first["output_digest"] = serde_json::json!("sha256:shared-evidence");
    let mut second = first.clone();
    second["id"] = serde_json::json!("verify");
    second["idempotency_key"] = serde_json::json!("run-live-verify");
    let mut current = first.clone();
    current["id"] = serde_json::json!("review");
    current["state"] = serde_json::json!("running");
    current["idempotency_key"] = serde_json::json!("run-live-review");
    current["output_digest"] = serde_json::Value::Null;
    graph["nodes"]
        .as_array_mut()
        .unwrap()
        .extend([second, current]);
    fs::write(checkpoint, serde_json::to_vec_pretty(&graph).unwrap()).unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();

    assert_eq!(
        projection.objectives[0].evidence,
        vec!["sha256:shared-evidence"]
    );
}

#[test]
fn current_run_comes_from_exact_resident_leaf_binding_not_legacy_or_registry_order() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "a-unbound", "running");
    write_run(root.path(), "z-bound", "running");
    fs::remove_file(root.path().join("data/workbench/current-runs.json")).unwrap();
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert_eq!(projection.runs.len(), 1);
    assert_eq!(
        projection.objectives[0].current_run_id.as_deref(),
        Some("z-bound")
    );
    assert_eq!(
        projection.objectives[0].current_task_id.as_deref(),
        Some("leaf-objective-live")
    );
}

#[test]
fn consumed_and_quarantined_schedules_are_not_projected_as_next_wakes() {
    let root = tempfile::tempdir().unwrap();
    let store = objective_agenda::seed(root.path(), "current", "operator:test", "approved", 70);
    for (id, wake) in [("consumed", 100), ("quarantined", 200), ("next", 300)] {
        store
            .put_schedule(
                arda_engine::objectives::ScheduleSpec {
                    id: id.into(),
                    objective_id: "current".into(),
                    next_wake_ms: wake,
                    recurrence: None,
                    idempotency_key: id.into(),
                },
                1,
            )
            .unwrap();
    }
    let db = rusqlite::Connection::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    db.execute("INSERT INTO schedule_wakes VALUES ('consumed', 100)", [])
        .unwrap();
    db.execute(
        "INSERT INTO schedule_errors VALUES ('quarantined', 'invalid recurrence', 1)",
        [],
    )
    .unwrap();
    let projection = publish_operator_projection(root.path(), Utc::now()).unwrap();
    assert_eq!(
        projection.objectives[0]
            .next_wake_at
            .unwrap()
            .timestamp_millis(),
        300
    );
}

#[test]
fn projection_reads_do_not_migrate_pre_cutover_state_or_load_execution_payloads() {
    let root = tempfile::tempdir().unwrap();
    objective_agenda::seed(root.path(), "current", "operator:test", "approved", 70);
    let path = root.path().join("data/arda/objectives.sqlite3");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch(
        "DROP TABLE schedule_wakes; DROP TABLE schedule_errors;
        UPDATE leaves SET execution_json = 'invalid payload which summaries must not deserialize';",
    )
    .unwrap();
    let schema_before: i64 = db
        .query_row("PRAGMA schema_version", [], |row| row.get(0))
        .unwrap();
    publish_operator_projection(root.path(), Utc::now()).unwrap();
    let schema_after: i64 = db
        .query_row("PRAGMA schema_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(schema_before, schema_after);
}
