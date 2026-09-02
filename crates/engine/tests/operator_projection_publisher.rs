use arda_core::operator_projection::{
    DependencyHealth, MeasurementSource, ObjectiveStatus, RunStatus,
};
use arda_engine::operator_projection::publish_operator_projection;
use chrono::{TimeZone, Utc};
use std::fs;

fn write_run(root: &std::path::Path, run_id: &str, state: &str) {
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
fn corrupt_legacy_queue_input_is_ignored_by_resident_projection() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "pending");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        &queue,
        "{\"id\":\"task-live\",\"status\":\"pending\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
    )
    .unwrap();
    let generated_at = Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap();
    publish_operator_projection(root.path(), generated_at).unwrap();
    let output_path = root.path().join("core/state/operator_projection.json");
    let before = fs::read(&output_path).unwrap();

    fs::write(queue, "{not-json\n").unwrap();
    publish_operator_projection(root.path(), generated_at).unwrap();
    assert_eq!(fs::read(output_path).unwrap(), before);
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
    assert!(objective.current_task_id.is_none());
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
    write_run(root.path(), "run-current", "running");
    write_run(root.path(), "runtime-proof-20260813-v2", "succeeded");

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

#[test]
fn legacy_queue_does_not_override_run_projection_state() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let checkpoint = root.path().join("data/runs/run-live/checkpoint.json");
    let raw = fs::read_to_string(&checkpoint).unwrap().replace(
        "\"output_digest\": null,",
        concat!(
            "\"output_digest\": \"sha256:",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",",
            "\n    \"worker\": {",
            "\"role\":\"planner_proposer\",",
            "\"worker_id\":\"hermes-live\",",
            "\"route_id\":\"hosted:hermes-workbench\",",
            "\"route_class\":\"hosted\",",
            "\"prompt_digest\":\"sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb\",",
            "\"allowed_toolsets\":[],",
            "\"dependencies\":[],",
            "\"deadline_unix_ms\":1,",
            "\"output_contract\":\"worker-report.v1\",",
            "\"evidence_policy\":\"worker_report\"},"
        ),
    );
    fs::write(checkpoint, raw).unwrap();

    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        concat!(
            "{\"id\":\"task-live\",\"title\":\"Repair live objective\",",
            "\"status\":\"in_progress\",\"workbench_run_id\":\"run-live\",",
            "\"continuation_decision\":\"continue_verify\",",
            "\"execution_receipt_digest\":\"sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc\",",
            "\"detail\":\"verification is the next required stage\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n"
        ),
    )
    .unwrap();
    let schedules = root.path().join("core/projects/tasks/schedules.jsonl");
    fs::write(
        schedules,
        concat!(
            "{\"contract\":\"arda.workbench.schedule_record.v1\",",
            "\"task_id\":\"task-live\",\"objective_id\":\"objective-live\",",
            "\"mode\":\"deferred\",\"state\":\"scheduled\",",
            "\"not_before_utc\":\"2026-08-10T19:00:00Z\",",
            "\"recorded_at_utc\":\"2026-08-10T18:00:00Z\"}\n"
        ),
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let value = serde_json::to_value(projection).unwrap();
    let objective = &value["objectives"][0];

    assert_eq!(objective["title"], "objective-live");
    assert!(objective.get("current_task_id").is_none());
    assert_eq!(objective["current_run_id"], "run-live");
    assert_eq!(objective["current_node_id"], "plan");
    assert!(objective.get("next_continuation").is_none());
    assert!(objective.get("next_wake_at").is_none());
    assert_eq!(objective["provider_route"], "hosted:hermes-workbench");
    assert_eq!(objective["budget"]["max_joules"], 40.0);
    assert_eq!(objective["budget"]["max_cost_usd"], 0.0);
    assert_eq!(
        objective["evidence"],
        serde_json::json!([
            "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ])
    );
    assert!(objective.get("blocker").is_none());
}

#[test]
fn objective_projection_rejects_cross_objective_schedule_lineage() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        "{\"id\":\"task-live\",\"status\":\"in_progress\",\"workbench_run_id\":\"run-live\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
    )
    .unwrap();
    fs::write(
        root.path().join("core/projects/tasks/schedules.jsonl"),
        "{\"contract\":\"arda.workbench.schedule_record.v1\",\"task_id\":\"task-live\",\"objective_id\":\"different-objective\",\"mode\":\"deferred\",\"state\":\"scheduled\",\"not_before_utc\":\"2026-08-10T19:00:00Z\",\"recorded_at_utc\":\"2026-08-10T18:00:00Z\"}\n",
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let value = serde_json::to_value(projection).unwrap();

    assert!(value["objectives"][0].get("next_wake_at").is_none());
}

#[test]
fn scheduled_legacy_queue_objective_is_not_projected_without_resident_authority() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("data/runs")).unwrap();
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        concat!(
            "{\"id\":\"task-deferred\",\"title\":\"Resume deferred repair\",",
            "\"status\":\"blocked\",\"continuation_decision\":\"wait_until\",",
            "\"detail\":\"waiting for the declared dependency window\",",
            "\"meta\":{\"objective_id\":\"objective-deferred\",",
            "\"project_id\":\"project-deferred\"}}\n"
        ),
    )
    .unwrap();
    fs::write(
        root.path().join("core/projects/tasks/schedules.jsonl"),
        concat!(
            "{\"contract\":\"arda.workbench.schedule_record.v1\",",
            "\"task_id\":\"task-deferred\",\"objective_id\":\"objective-deferred\",",
            "\"mode\":\"deferred\",\"state\":\"scheduled\",",
            "\"not_before_utc\":\"2026-08-11T08:00:00Z\",",
            "\"recorded_at_utc\":\"2026-08-10T18:00:00Z\"}\n"
        ),
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    assert!(projection.objectives.is_empty());
}

#[test]
fn run_backed_objective_ignores_nonterminal_legacy_queue_leaf() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        concat!(
            "{\"id\":\"task-later\",\"title\":\"Lower-priority queued leaf\",",
            "\"status\":\"pending\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
            "{\"id\":\"task-next\",\"title\":\"Continue objective after current run\",",
            "\"status\":\"in_progress\",\"continuation_decision\":\"wait_until\",",
            "\"detail\":\"waiting for the next dependency\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n"
        ),
    )
    .unwrap();
    fs::write(
        root.path().join("core/projects/tasks/schedules.jsonl"),
        concat!(
            "{\"contract\":\"arda.workbench.schedule_record.v1\",",
            "\"task_id\":\"task-next\",\"objective_id\":\"objective-live\",",
            "\"mode\":\"deferred\",\"state\":\"scheduled\",",
            "\"not_before_utc\":\"2026-08-11T08:00:00Z\",",
            "\"recorded_at_utc\":\"2026-08-10T18:00:00Z\"}\n"
        ),
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let value = serde_json::to_value(projection).unwrap();
    let objective = &value["objectives"][0];

    assert_eq!(objective["current_run_id"], "run-live");
    assert!(objective.get("current_task_id").is_none());
    assert!(objective.get("next_continuation").is_none());
    assert!(objective.get("next_wake_at").is_none());
}

#[test]
fn legacy_queue_cannot_select_a_current_run() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-first", "running");
    write_run(root.path(), "run-selected", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        "{\"id\":\"task-selected\",\"status\":\"in_progress\",\"workbench_run_id\":\"run-selected\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let objective = &projection.objectives[0];

    assert_eq!(objective.current_run_id.as_deref(), Some("run-first"));
    assert!(objective.current_task_id.is_none());
}

#[test]
fn legacy_bound_and_unbound_leaves_do_not_control_current_run() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-first", "running");
    write_run(root.path(), "run-selected", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        concat!(
            "{\"id\":\"task-unbound\",\"status\":\"in_progress\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n",
            "{\"id\":\"task-bound\",\"status\":\"in_progress\",",
            "\"workbench_run_id\":\"run-selected\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n"
        ),
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let objective = &projection.objectives[0];

    assert_eq!(objective.current_run_id.as_deref(), Some("run-first"));
    assert!(objective.current_task_id.is_none());
}

#[test]
fn nonterminal_legacy_queue_status_does_not_override_run_status() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        "{\"id\":\"task-blocked\",\"status\":\"blocked\",\"workbench_run_id\":\"run-live\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let objective = &projection.objectives[0];

    assert_eq!(objective.status, ObjectiveStatus::Active);
    assert!(objective.current_task_id.is_none());
}

#[test]
fn canonical_completed_alias_is_not_projected_as_current_work() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("data/runs")).unwrap();
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        "{\"id\":\"task-done\",\"status\":\"done\",\"meta\":{\"objective_id\":\"objective-done\"}}\n",
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();

    assert!(projection.objectives.is_empty());
}

#[test]
fn stale_legacy_bound_leaf_does_not_supply_current_task() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-live", "running");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        concat!(
            "{\"id\":\"task-stale\",\"status\":\"in_progress\",",
            "\"workbench_run_id\":\"run-missing\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n",
            "{\"id\":\"task-valid\",\"status\":\"pending\",",
            "\"continuation_decision\":\"continue_execute\",",
            "\"meta\":{\"objective_id\":\"objective-live\"}}\n"
        ),
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
    let objective = &projection.objectives[0];

    assert_eq!(objective.current_run_id.as_deref(), Some("run-live"));
    assert!(objective.current_task_id.is_none());
    assert!(objective.next_continuation.is_none());
}

#[test]
fn nonterminal_legacy_continuation_does_not_revive_historical_run() {
    let root = tempfile::tempdir().unwrap();
    write_run(root.path(), "run-history", "succeeded");
    let queue = root.path().join("core/projects/tasks/queue.jsonl");
    fs::create_dir_all(queue.parent().unwrap()).unwrap();
    fs::write(
        queue,
        "{\"id\":\"task-next\",\"status\":\"blocked\",\"meta\":{\"objective_id\":\"objective-live\"}}\n",
    )
    .unwrap();

    let projection = publish_operator_projection(
        root.path(),
        Utc.with_ymd_and_hms(2026, 8, 10, 18, 0, 0).unwrap(),
    )
    .unwrap();
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
