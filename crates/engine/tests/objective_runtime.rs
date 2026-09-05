use anyhow::Result;
use arda_engine::objectives::{
    ControlAction, LeafExecution, LeafExecutionResult, LeafExecutionSpec, NewLeaf, NewObjective,
    ObjectiveRuntime, ObjectiveState, ObjectiveStore, ProjectAuthority, ReceiptStage, StageReceipt,
};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
struct RecordingExecutor {
    active: Arc<AtomicUsize>,
    maximum: Arc<AtomicUsize>,
}

impl LeafExecution for RecordingExecutor {
    fn execute(
        &self,
        claim: arda_engine::objectives::ClaimedLeaf,
    ) -> Pin<Box<dyn Future<Output = Result<LeafExecutionResult>> + Send>> {
        let active = Arc::clone(&self.active);
        let maximum = Arc::clone(&self.maximum);
        Box::pin(async move {
            assert!(claim.execution.is_some());
            assert!(claim.project_contract_digest.is_some());
            if claim.leaf_id == "join" {
                assert_eq!(claim.dependency_receipts.len(), 2);
                assert!(claim.dependency_receipts.iter().all(|receipt| {
                    receipt.contract == "arda.hermes_execution_receipt.v4"
                        && receipt.stage == ReceiptStage::Close
                        && receipt.verdict == "succeeded"
                }));
            } else {
                assert!(claim.dependency_receipts.is_empty());
            }
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;
            maximum.fetch_max(current, Ordering::SeqCst);
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            active.fetch_sub(1, Ordering::SeqCst);

            let mut predecessor = claim.current_receipt_digest.clone();
            let mut receipts = Vec::new();
            for (index, stage) in [
                ReceiptStage::Execute,
                ReceiptStage::Verify,
                ReceiptStage::Review,
                ReceiptStage::Close,
            ]
            .into_iter()
            .enumerate()
            {
                let seed = format!("{}-{index}", claim.leaf_id);
                let digest = format!("sha256:{:0<64}", seed);
                receipts.push(StageReceipt {
                    contract: "arda.hermes_execution_receipt.v4".into(),
                    stage,
                    digest: digest.clone(),
                    predecessor_digest: predecessor,
                    run_path: format!(
                        "data/runs/{}/execution-receipts/{}.json",
                        claim.leaf_id, index
                    ),
                    provider: "test-provider".into(),
                    model: "test-model".into(),
                    started_at_ms: 200 + index as i64,
                    completed_at_ms: 201 + index as i64,
                    verdict: "succeeded".into(),
                    context_outcome_receipt_id: None,
                    context_outcome_receipt_digest: None,
                    binding_digest: None,
                });
                predecessor = Some(digest);
            }
            Ok(LeafExecutionResult { receipts })
        })
    }
}

fn execution_spec(leaf_id: &str) -> LeafExecutionSpec {
    LeafExecutionSpec {
        objective: format!("Execute {leaf_id}"),
        execution_prompt: format!("Execute only {leaf_id}"),
        verification_prompt: format!("Verify {leaf_id}"),
        review_prompt: format!("Review {leaf_id}"),
        approval_envelope: serde_json::json!({
            "approval": {
                "schema_version": "arda.orome.task_approval.v1",
                "proposal_id": "operator-objective-runtime-1",
                "approval_id": "operator-approval-runtime-1",
                "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                "decision": "policy_safe",
                "created_at_utc": "2026-09-01T00:00:00Z"
            },
            "idempotency_key": format!("objective-runtime-1-{leaf_id}")
        }),
        objective_plan_receipt: format!("sha256:{}", "1".repeat(64)),
    }
}

fn objective(root: &std::path::Path) -> NewObjective {
    let project_a = "b22c0000-e29b-41d4-a716-446655440002";
    let project_b = "c33d0000-e29b-41d4-a716-446655440003";
    NewObjective {
        id: "objective-runtime-1".into(),
        source_id: "operator-objective-runtime-1".into(),
        idempotency_key: "ingress-runtime-1".into(),
        operator_id: "operator-1".into(),
        text: "Inspect two projects and join the evidence".into(),
        priority: 100,
        projects: vec![
            ProjectAuthority {
                project_id: project_a.into(),
                contract_digest: format!("sha256:{}", "a".repeat(64)),
            },
            ProjectAuthority {
                project_id: project_b.into(),
                contract_digest: format!("sha256:{}", "b".repeat(64)),
            },
        ],
        leaves: vec![
            NewLeaf {
                id: "inspect-a".into(),
                project_id: Some(project_a.into()),
                workspace_root: root.join("project-a").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec![],
                execution: Some(execution_spec("inspect-a")),
            },
            NewLeaf {
                id: "inspect-b".into(),
                project_id: Some(project_b.into()),
                workspace_root: root.join("project-b").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec![],
                execution: Some(execution_spec("inspect-b")),
            },
            NewLeaf {
                id: "join".into(),
                project_id: Some(project_a.into()),
                workspace_root: root.join("join").display().to_string(),
                authority: "read_only".into(),
                dependencies: vec!["inspect-a".into(), "inspect-b".into()],
                execution: Some(execution_spec("join")),
            },
        ],
    }
}

#[tokio::test]
async fn resident_runtime_joins_independent_leaves_and_rehydrates_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    for path in ["project-a", "project-b", "join"] {
        std::fs::create_dir_all(dir.path().join(path)).unwrap();
    }
    let database = dir.path().join("data/arda/objectives.sqlite3");
    let store = ObjectiveStore::open(&database).unwrap();
    store
        .create_authenticated_objective(objective(dir.path()), 100)
        .unwrap();
    store
        .apply_control(
            "objective-runtime-1",
            ControlAction::Approve { revision: 1 },
            "approve-runtime-1",
            "operator-1",
            101,
        )
        .unwrap();
    let active = Arc::new(AtomicUsize::new(0));
    let maximum = Arc::new(AtomicUsize::new(0));
    let executor = RecordingExecutor {
        active,
        maximum: Arc::clone(&maximum),
    };

    let mut runtime = ObjectiveRuntime::new(store, executor.clone(), "arda-runtime-1", 4, 60_000);
    let first = runtime.run_round(200).await.unwrap();

    assert_eq!(first.len(), 2);
    assert_eq!(maximum.load(Ordering::SeqCst), 2);
    assert_eq!(
        runtime
            .store()
            .objective("objective-runtime-1")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Running
    );
    drop(runtime);

    let mut restarted = ObjectiveRuntime::new(
        ObjectiveStore::open(&database).unwrap(),
        executor,
        "arda-runtime-2",
        4,
        60_000,
    );
    let second = restarted.run_round(300).await.unwrap();

    assert_eq!(second.len(), 1);
    assert_eq!(second[0].leaf_id, "join");
    assert_eq!(
        restarted
            .store()
            .objective("objective-runtime-1")
            .unwrap()
            .unwrap()
            .state,
        ObjectiveState::Completed
    );
    assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    assert!(!dir
        .path()
        .join("core/projects/tasks/schedules.jsonl")
        .exists());
}
