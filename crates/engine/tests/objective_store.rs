use arda_engine::objectives::{
    ControlAction, LeafStage, NewLeaf, NewObjective, ObjectiveState, ObjectiveStore,
    ProjectAuthority, ReceiptStage, ScheduleSpec, StageReceipt,
};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

fn objective(id: &str, idempotency_key: &str) -> NewObjective {
    NewObjective {
        id: id.to_owned(),
        source_id: format!("source-{id}"),
        idempotency_key: idempotency_key.to_owned(),
        operator_id: "operator:primary".to_owned(),
        text: "Inspect both reviewed projects and join the result.".to_owned(),
        priority: 50,
        projects: vec![
            ProjectAuthority {
                project_id: "project-a".to_owned(),
                contract_digest: "sha256:project-a".to_owned(),
            },
            ProjectAuthority {
                project_id: "project-b".to_owned(),
                contract_digest: "sha256:project-b".to_owned(),
            },
        ],
        leaves: vec![
            NewLeaf {
                id: format!("{id}-a"),
                project_id: Some("project-a".to_owned()),
                workspace_root: "/work/a".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![],
                execution: None,
            },
            NewLeaf {
                id: format!("{id}-b"),
                project_id: Some("project-b".to_owned()),
                workspace_root: "/work/b".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![],
                execution: None,
            },
            NewLeaf {
                id: format!("{id}-join"),
                project_id: None,
                workspace_root: "/work/join".to_owned(),
                authority: "read_only".to_owned(),
                dependencies: vec![format!("{id}-a"), format!("{id}-b")],
                execution: None,
            },
        ],
    }
}

fn close_claim(store: &ObjectiveStore, leaf_id: &str, lease_owner: &str, now_ms: i64) {
    let mut predecessor = None;
    for (offset, (stage, stage_name)) in [
        (ReceiptStage::Execute, "execute"),
        (ReceiptStage::Verify, "verify"),
        (ReceiptStage::Review, "review"),
        (ReceiptStage::Close, "close"),
    ]
    .into_iter()
    .enumerate()
    {
        let digest = format!(
            "sha256:{:x}",
            Sha256::digest(format!("{leaf_id}-{stage_name}").as_bytes())
        );
        store
            .record_stage_receipt(
                leaf_id,
                lease_owner,
                StageReceipt {
                    contract: "arda.hermes_execution_receipt.v4".to_owned(),
                    stage,
                    digest: digest.clone(),
                    predecessor_digest: predecessor,
                    run_path: format!("data/runs/{leaf_id}/execution-receipts/{stage_name}.json"),
                    provider: "provider-a".to_owned(),
                    model: "model-a".to_owned(),
                    started_at_ms: now_ms + offset as i64,
                    completed_at_ms: now_ms + offset as i64 + 1,
                    verdict: "succeeded".to_owned(),
                },
                now_ms + offset as i64 + 1,
            )
            .unwrap();
        predecessor = Some(digest);
    }
}

#[test]
fn objective_creation_is_atomic_idempotent_and_restart_durable() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let created = store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    let replayed = store
        .create_authenticated_objective(objective("objective-1", "message-1"), 101)
        .unwrap();

    assert_eq!(created, replayed);
    assert_eq!(created.state, ObjectiveState::PendingApproval);
    assert_eq!(created.project_ids, vec!["project-a", "project-b"]);
    assert_eq!(store.list_leaves("objective-1").unwrap().len(), 3);
    drop(store);

    let reopened = ObjectiveStore::open(&path).unwrap();
    let recovered = reopened.objective("objective-1").unwrap().unwrap();
    assert_eq!(recovered, created);
    assert_eq!(reopened.list_leaves("objective-1").unwrap().len(), 3);
}

#[test]
fn duplicate_ingress_key_with_different_payload_fails_closed() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();

    let error = store
        .create_authenticated_objective(objective("different-id", "message-1"), 101)
        .unwrap_err();
    assert!(error.to_string().contains("idempotency conflict"));
    assert!(store.objective("different-id").unwrap().is_none());
}

#[test]
fn transactional_claims_require_approval_respect_dependencies_and_do_not_duplicate() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    assert!(store
        .claim_runnable("worker-0", 110, 100, 8)
        .unwrap()
        .is_empty());
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            120,
        )
        .unwrap();
    drop(store);

    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for owner in ["worker-1", "worker-2"] {
        let path = path.clone();
        let barrier = barrier.clone();
        handles.push(thread::spawn(move || {
            let store = ObjectiveStore::open(path).unwrap();
            barrier.wait();
            store.claim_runnable(owner, 130, 100, 2).unwrap()
        }));
    }
    barrier.wait();
    let mut claimed = handles
        .into_iter()
        .flat_map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    claimed.sort_by(|left, right| left.leaf_id.cmp(&right.leaf_id));

    assert_eq!(claimed.len(), 2);
    assert_eq!(claimed[0].leaf_id, "objective-1-a");
    assert_eq!(claimed[1].leaf_id, "objective-1-b");
    assert!(claimed
        .iter()
        .all(|claim| claim.stage == LeafStage::Execute));
    assert!(!claimed
        .iter()
        .any(|claim| claim.leaf_id == "objective-1-join"));

    let store = ObjectiveStore::open(path).unwrap();
    for claim in &claimed {
        close_claim(&store, &claim.leaf_id, &claim.lease_owner, 140);
    }
    let join = store
        .claim_runnable("join-worker", 150, 100, 1)
        .unwrap()
        .remove(0);
    assert_eq!(join.leaf_id, "objective-1-join");
    assert_eq!(join.dependency_receipts.len(), 2);
    assert!(join
        .dependency_receipts
        .iter()
        .all(|receipt| receipt.stage == ReceiptStage::Close && receipt.verdict == "succeeded"));
    assert_eq!(
        join.dependency_receipts
            .iter()
            .map(|receipt| receipt.run_path.as_str())
            .collect::<Vec<_>>(),
        vec![
            "data/runs/objective-1-a/execution-receipts/close.json",
            "data/runs/objective-1-b/execution-receipts/close.json",
        ]
    );
}

#[test]
fn expired_lease_is_recovered_after_restart() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    let mut single_leaf = objective("objective-1", "message-1");
    single_leaf.leaves.truncate(1);
    store
        .create_authenticated_objective(single_leaf, 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let first = store.claim_runnable("worker-1", 120, 10, 1).unwrap();
    assert_eq!(first.len(), 1);
    drop(store);

    let reopened = ObjectiveStore::open(&path).unwrap();
    assert!(reopened
        .claim_runnable("worker-2", 129, 10, 1)
        .unwrap()
        .is_empty());
    let recovered = reopened.claim_runnable("worker-2", 131, 10, 1).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].leaf_id, first[0].leaf_id);
    assert_eq!(recovered[0].attempt, 2);
}

#[test]
fn schedules_are_idempotent_restart_durable_and_pause_with_the_objective() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("objectives.sqlite3");
    let store = ObjectiveStore::open(&path).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    let schedule = ScheduleSpec {
        id: "schedule-1".to_owned(),
        objective_id: "objective-1".to_owned(),
        next_wake_ms: 200,
        recurrence: Some("PT5M".to_owned()),
        idempotency_key: "schedule-message-1".to_owned(),
    };
    assert_eq!(store.put_schedule(schedule.clone(), 110).unwrap(), schedule);
    assert_eq!(store.put_schedule(schedule.clone(), 111).unwrap(), schedule);
    assert_eq!(
        store.due_schedules(200, 10).unwrap(),
        vec![schedule.clone()]
    );
    store
        .apply_control(
            "objective-1",
            ControlAction::Pause,
            "pause-1",
            "operator:primary",
            120,
        )
        .unwrap();
    assert!(store.due_schedules(300, 10).unwrap().is_empty());
    drop(store);

    let reopened = ObjectiveStore::open(path).unwrap();
    assert_eq!(reopened.schedule("schedule-1").unwrap(), Some(schedule));
}

#[test]
fn stage_progression_requires_exact_receipt_lineage() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let claim = store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);
    let execute_digest = format!("sha256:{}", "a".repeat(64));
    let verify_digest = format!("sha256:{}", "b".repeat(64));
    let wrong_digest = format!("sha256:{}", "c".repeat(64));

    let execute = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Execute,
        digest: execute_digest.clone(),
        predecessor_digest: None,
        run_path: "data/runs/run-1/execution-receipts/execute.json".to_owned(),
        provider: "provider-a".to_owned(),
        model: "model-a".to_owned(),
        started_at_ms: 121,
        completed_at_ms: 130,
        verdict: "succeeded".to_owned(),
    };
    let mut invalid_contract = execute.clone();
    invalid_contract.contract = "legacy.synthetic_receipt.v1".to_owned();
    assert!(store
        .record_stage_receipt(&claim.leaf_id, "worker-1", invalid_contract, 131)
        .unwrap_err()
        .to_string()
        .contains("arda.hermes_execution_receipt.v4"));
    store
        .record_stage_receipt(&claim.leaf_id, "worker-1", execute, 131)
        .unwrap();

    let wrong = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Verify,
        digest: verify_digest.clone(),
        predecessor_digest: Some(wrong_digest),
        run_path: "data/runs/run-1/execution-receipts/verify.json".to_owned(),
        provider: "provider-b".to_owned(),
        model: "model-b".to_owned(),
        started_at_ms: 132,
        completed_at_ms: 140,
        verdict: "succeeded".to_owned(),
    };
    assert!(store
        .record_stage_receipt(&claim.leaf_id, "worker-1", wrong, 141)
        .unwrap_err()
        .to_string()
        .contains("predecessor"));

    let verify = StageReceipt {
        predecessor_digest: Some(execute_digest),
        ..StageReceipt {
            contract: "arda.hermes_execution_receipt.v4".to_owned(),
            stage: ReceiptStage::Verify,
            digest: verify_digest,
            predecessor_digest: None,
            run_path: "data/runs/run-1/execution-receipts/verify.json".to_owned(),
            provider: "provider-b".to_owned(),
            model: "model-b".to_owned(),
            started_at_ms: 132,
            completed_at_ms: 140,
            verdict: "succeeded".to_owned(),
        }
    };
    store
        .record_stage_receipt(&claim.leaf_id, "worker-1", verify, 141)
        .unwrap();
    assert_eq!(
        store.leaf(&claim.leaf_id).unwrap().unwrap().stage,
        LeafStage::Review
    );
}

#[test]
fn revision_invalidates_approval_and_terminal_root_requires_every_leaf_close() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Revise {
                text: "Revised reviewed objective".to_owned(),
            },
            "revision-1",
            "operator:primary",
            120,
        )
        .unwrap();
    let revised = store.objective("objective-1").unwrap().unwrap();
    assert_eq!(revised.revision, 2);
    assert_eq!(revised.state, ObjectiveState::PendingApproval);
    assert!(store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "stale-approval",
            "operator:primary",
            121,
        )
        .unwrap_err()
        .to_string()
        .contains("revision"));
    assert!(store
        .close_objective("objective-1", "sha256:root", 130)
        .unwrap_err()
        .to_string()
        .contains("not complete"));
}

#[test]
fn objective_creation_rejects_dependency_cycles() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    let mut cyclic = objective("objective-cycle", "message-cycle");
    cyclic.leaves[0].dependencies = vec!["objective-cycle-join".to_owned()];

    let error = store
        .create_authenticated_objective(cyclic, 100)
        .unwrap_err();

    assert!(
        error.to_string().contains("contain a cycle"),
        "unexpected error: {error:#}"
    );
    assert!(store.list_objectives().unwrap().is_empty());
}

#[test]
fn revision_is_rejected_after_leaf_execution_begins() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);

    let error = store
        .apply_control(
            "objective-1",
            ControlAction::Revise {
                text: "Unsafe mid-execution revision".to_owned(),
            },
            "revision-after-execution",
            "operator:primary",
            121,
        )
        .unwrap_err();

    assert!(
        error.to_string().contains("execution started"),
        "unexpected error: {error:#}"
    );
    assert_eq!(store.objective("objective-1").unwrap().unwrap().revision, 1);
}

#[test]
fn stage_receipts_require_canonical_digests_and_safe_relative_paths() {
    let temp = TempDir::new().unwrap();
    let store = ObjectiveStore::open(temp.path().join("objectives.sqlite3")).unwrap();
    store
        .create_authenticated_objective(objective("objective-1", "message-1"), 100)
        .unwrap();
    store
        .apply_control(
            "objective-1",
            ControlAction::Approve { revision: 1 },
            "approval-1",
            "operator:primary",
            110,
        )
        .unwrap();
    let claim = store
        .claim_runnable("worker-1", 120, 100, 1)
        .unwrap()
        .remove(0);
    let receipt = StageReceipt {
        contract: "arda.hermes_execution_receipt.v4".to_owned(),
        stage: ReceiptStage::Execute,
        digest: "sha256:not-a-digest".to_owned(),
        predecessor_digest: None,
        run_path: "data/runs/run-1/execution-receipts/execute.json".to_owned(),
        provider: "provider-a".to_owned(),
        model: "model-a".to_owned(),
        started_at_ms: 121,
        completed_at_ms: 130,
        verdict: "succeeded".to_owned(),
    };
    let digest_error = store
        .record_stage_receipt(&claim.leaf_id, "worker-1", receipt.clone(), 131)
        .unwrap_err();
    assert!(digest_error.to_string().contains("lowercase-hex"));

    let path_error = store
        .record_stage_receipt(
            &claim.leaf_id,
            "worker-1",
            StageReceipt {
                digest: format!("sha256:{}", "d".repeat(64)),
                run_path: "../outside.json".to_owned(),
                ..receipt
            },
            131,
        )
        .unwrap_err();
    assert!(path_error.to_string().contains("repository-relative"));
}
