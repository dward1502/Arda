#![cfg(feature = "full-cli")]

use arda_aule::prometheus::autopilot::{
    CouncilMode, ExecutiveCycleError, ExecutiveCycleInput, ExecutiveCycleStore,
    ExecutiveDisposition, ExecutivePhase, ExecutiveResourceBudget, RoleRequest,
    EXECUTIVE_CYCLE_CONTRACT,
};
use chrono::{TimeZone, Utc};
use serde_json::json;
use std::fs;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

fn role(role: &str, capabilities: &[&str]) -> RoleRequest {
    RoleRequest {
        role: role.into(),
        capabilities: capabilities.iter().map(|value| (*value).into()).collect(),
    }
}

fn input(cycle_id: &str, phase: ExecutivePhase) -> ExecutiveCycleInput {
    ExecutiveCycleInput {
        cycle_id: cycle_id.into(),
        phase,
        objective_id: "objective-stage6-proof".into(),
        objective_source_ref: "core/projects/tasks/queue.jsonl#objective-stage6-proof".into(),
        context_receipt_ref: "data/mnemosyne/context/stage6.json#context-receipt".into(),
        recommendation_id: "arandur-stage6-recommendation".into(),
        approval_packet_id: Some("operator-approval-stage6".into()),
        proposed_action: "observe worker and collect completion evidence".into(),
        requested_roles: vec![role("observer", &["process_health", "receipt_read"])],
        governance_receipt_ref: Some("data/governance/stage6.json#allow".into()),
        placement_receipt_refs: vec!["data/manwe/placements.jsonl#placement-1".into()],
        queue_handoff_receipt_refs: Vec::new(),
        execution_receipt_refs: Vec::new(),
        failure_receipt_ref: None,
        revised_action: None,
        revised_requested_roles: Vec::new(),
        acceptance_receipt_refs: Vec::new(),
        council_mode: CouncilMode::Disabled,
        full_council_approval_ref: None,
        resource_budget: ExecutiveResourceBudget {
            max_roles: 3,
            max_dispatches: 2,
            max_joules: 20.0,
            requested_joules: 4.0,
            max_council_opinions: 1,
            requested_council_opinions: 0,
        },
        operator_stop_requested: false,
        read_only: false,
        parent_receipt_id: None,
    }
}

#[test]
fn read_only_and_operator_stop_are_non_mutating_and_stoppable() {
    let dir = tempfile::tempdir().unwrap();
    let store = ExecutiveCycleStore::from_root(dir.path());
    let mut read_only = input("stage6-read-only", ExecutivePhase::Review);
    read_only.read_only = true;
    read_only.approval_packet_id = None;
    read_only.governance_receipt_ref = None;
    read_only.placement_receipt_refs.clear();
    let projected = store
        .evaluate(
            read_only,
            Utc.with_ymd_and_hms(2026, 8, 22, 12, 0, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(
        projected.receipt.disposition,
        ExecutiveDisposition::ObservedReadOnly
    );
    assert!(!projected.ledger_appended);
    assert!(!store.ledger_path().exists());

    let mut stopped = input("stage6-stopped", ExecutivePhase::Execute);
    stopped.operator_stop_requested = true;
    stopped.queue_handoff_receipt_refs = vec!["queue#handoff".into()];
    let result = store
        .evaluate(
            stopped,
            Utc.with_ymd_and_hms(2026, 8, 22, 12, 1, 0).unwrap(),
        )
        .unwrap();
    assert_eq!(result.receipt.disposition, ExecutiveDisposition::Stopped);
    assert!(!result.receipt.queue_handoff_allowed);
    assert!(result.receipt.operator_can_stop);
    assert!(!result.receipt.queue_mutation_performed_by_arandur);
    assert!(!result.receipt.execution_performed_by_arandur);
}

#[test]
fn council_and_resource_policy_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let store = ExecutiveCycleStore::from_root(dir.path());
    let mut full_council = input("stage6-council", ExecutivePhase::Plan);
    full_council.council_mode = CouncilMode::FullDeliberation;
    full_council.resource_budget.requested_council_opinions = 3;
    let held = store.evaluate(full_council, Utc::now()).unwrap();
    assert_eq!(held.receipt.disposition, ExecutiveDisposition::Held);
    assert!(held.receipt.reason.contains("full council"));

    let mut over_budget = input("stage6-budget", ExecutivePhase::Plan);
    over_budget
        .requested_roles
        .push(role("critic", &["risk_review"]));
    over_budget.resource_budget.max_roles = 1;
    let held = store.evaluate(over_budget, Utc::now()).unwrap();
    assert_eq!(held.receipt.disposition, ExecutiveDisposition::Held);
    assert!(held.receipt.reason.contains("roles"));

    let mut invalid_energy = input("stage6-nan", ExecutivePhase::Plan);
    invalid_energy.resource_budget.requested_joules = f64::NAN;
    assert!(matches!(
        store.evaluate(invalid_energy, Utc::now()),
        Err(ExecutiveCycleError::Invalid(_))
    ));

    let mut unsupported_acceptance = input("stage6-acceptance", ExecutivePhase::Plan);
    unsupported_acceptance.acceptance_receipt_refs = vec!["acceptance#unsupported".into()];
    let held = store.evaluate(unsupported_acceptance, Utc::now()).unwrap();
    assert_eq!(held.receipt.disposition, ExecutiveDisposition::Held);
    assert!(held.receipt.reason.contains("execution receipts"));
}

#[test]
fn reviewed_cycle_records_one_decision_per_phase_and_rejects_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let store = ExecutiveCycleStore::from_root(dir.path());
    let first_input = input("stage6-reviewed", ExecutivePhase::Plan);
    let first = store.evaluate(first_input.clone(), Utc::now()).unwrap();
    assert_eq!(first.receipt.disposition, ExecutiveDisposition::Planned);
    assert!(first.ledger_appended);
    let replay = ExecutiveCycleStore::from_root(dir.path())
        .evaluate(first_input, Utc::now())
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.receipt.receipt_id, first.receipt.receipt_id);
    assert_eq!(
        fs::read_to_string(store.ledger_path())
            .unwrap()
            .lines()
            .count(),
        1
    );

    let mut conflict = input("stage6-reviewed", ExecutivePhase::Plan);
    conflict.proposed_action = "different action under the same phase key".into();
    assert!(matches!(
        store.evaluate(conflict, Utc::now()),
        Err(ExecutiveCycleError::ConflictingReplay { .. })
    ));
}

#[test]
fn real_worker_failure_is_cited_replanned_and_restart_safe() {
    let dir = tempfile::tempdir().unwrap();
    let evidence_dir = dir.path().join("evidence");
    fs::create_dir_all(&evidence_dir).unwrap();
    let mut worker = Command::new("/bin/sh")
        .args(["-c", "exec sleep 60"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let worker_pid = worker.id();
    thread::sleep(Duration::from_millis(50));
    assert!(worker.try_wait().unwrap().is_none());
    worker.kill().unwrap();
    let status = worker.wait().unwrap();
    assert!(!status.success());

    let failure_path = evidence_dir.join("worker-failure.json");
    fs::write(
        &failure_path,
        serde_json::to_vec_pretty(&json!({
            "contract": "arda.stage6.real_failure_receipt.v1",
            "worker_pid": worker_pid,
            "observed_state": "process_terminated",
            "success": false
        }))
        .unwrap(),
    )
    .unwrap();
    let placement_path = evidence_dir.join("placement-receipt.json");
    fs::write(
        &placement_path,
        serde_json::to_vec_pretty(&json!({
            "contract": "arda.manwe.placement_decision.v1",
            "requested_role": "recovery_observer",
            "capabilities": ["process_health", "receipt_read", "recovery_assessment"],
            "decision": "fixture_node_selected_by_placement_authority"
        }))
        .unwrap(),
    )
    .unwrap();

    let store = ExecutiveCycleStore::from_root(dir.path());
    let mut assess = input("stage6-real-failure", ExecutivePhase::Assess);
    assess.queue_handoff_receipt_refs = vec!["core/projects/tasks/queue.jsonl#handoff".into()];
    assess.execution_receipt_refs = vec!["data/runs/stage6/execution.json#attempt-1".into()];
    assess.failure_receipt_ref = Some(failure_path.display().to_string());
    assess.revised_action =
        Some("request recovery assessment before retrying the interrupted observation".into());
    assess.revised_requested_roles = vec![role(
        "recovery_observer",
        &["process_health", "receipt_read", "recovery_assessment"],
    )];
    assess.placement_receipt_refs = vec![placement_path.display().to_string()];
    let first = store.evaluate(assess.clone(), Utc::now()).unwrap();
    assert_eq!(first.receipt.contract, EXECUTIVE_CYCLE_CONTRACT);
    assert_eq!(first.receipt.disposition, ExecutiveDisposition::Replanned);
    assert!(first.receipt.learning_candidate.is_some());
    assert_eq!(
        first.receipt.failure_receipt_ref,
        assess.failure_receipt_ref
    );
    assert!(first.receipt.queue_handoff_allowed);
    assert!(!first.receipt.placement_performed_by_arandur);
    assert!(!first.receipt.execution_performed_by_arandur);

    let before = fs::read(store.ledger_path()).unwrap();
    let replay = ExecutiveCycleStore::from_root(dir.path())
        .evaluate(assess, Utc::now())
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.receipt.receipt_id, first.receipt.receipt_id);
    assert_eq!(fs::read(store.ledger_path()).unwrap(), before);

    if let Ok(path) = std::env::var("STAGE6_EVIDENCE_PATH") {
        let failure_observation: serde_json::Value =
            serde_json::from_slice(&fs::read(&failure_path).unwrap()).unwrap();
        let placement_observation: serde_json::Value =
            serde_json::from_slice(&fs::read(&placement_path).unwrap()).unwrap();
        let durable_ledger_row: serde_json::Value =
            serde_json::from_slice(before.strip_suffix(b"\n").unwrap_or(&before)).unwrap();
        let evidence = json!({
            "contract": "arda.digital-organism.stage6-proof.v1",
            "cycle_receipt": first.receipt,
            "failure_observation": failure_observation,
            "placement_observation": placement_observation,
            "durable_ledger_row": durable_ledger_row,
            "worker_pid": worker_pid,
            "worker_exit_success": status.success(),
            "ledger_rows": fs::read_to_string(store.ledger_path()).unwrap().lines().count(),
            "restart_replay_suppressed": replay.replayed,
            "claim_limit": "Real local process termination and durable replay prove this bounded executive-cycle contract; they do not prove deployed timer productivity or operator acceptance."
        });
        let path = std::path::PathBuf::from(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
