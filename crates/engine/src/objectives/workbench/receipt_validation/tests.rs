use super::*;
use crate::objectives::{
    ControlAction, LeafExecutionSpec, NewLeaf, NewObjective, ObjectiveStore, ProjectAuthority,
};
use crate::runs::{RunEventDraft, RunEventKind, RunStore};
use arda_core::run_graph::{NodeId, NodeState, RunGraph, RunId};

struct Fixture {
    root: tempfile::TempDir,
    store: ObjectiveStore,
    run: RunStore,
    item: ExplicitWorkbenchWorkItem,
    graph: RunGraph,
    receipts: Vec<HermesExecutionReceipt>,
}

fn fixture() -> Fixture {
    fixture_with_deadline(None)
}

fn fixture_with_deadline(deadline: Option<u64>) -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("project");
    std::fs::create_dir(&workspace).unwrap();
    let digest = format!("sha256:{}", "a".repeat(64));
    let store = ObjectiveStore::open(root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let execution = LeafExecutionSpec {
        execution_deadline_unix_ms: deadline,
        objective: "correct artifact".into(),
        execution_prompt: "implement".into(),
        verification_prompt: "verify".into(),
        review_prompt: "review".into(),
        approval_envelope: json!({"approval":{"approval_id":digest,"ledger_writes":["data/arda/objectives.sqlite3","data/runs"]}}),
        objective_plan_receipt: digest.clone(),
    };
    store
        .create_authenticated_objective(
            NewObjective {
                id: "objective".into(),
                source_id: "source".into(),
                idempotency_key: "ingress".into(),
                operator_id: "operator".into(),
                text: "correct artifact".into(),
                priority: 1,
                projects: vec![ProjectAuthority {
                    project_id: "project".into(),
                    contract_digest: digest.clone(),
                    authority: "execute_with_approval".into(),
                    checks: vec!["compile".into()],
                }],
                leaves: vec![NewLeaf {
                    id: "leaf".into(),
                    project_id: Some("project".into()),
                    workspace_root: workspace.to_string_lossy().into(),
                    authority: "execute_with_approval".into(),
                    dependencies: vec![],
                    execution: Some(execution.clone()),
                }],
            },
            1,
        )
        .unwrap();
    store
        .apply_control(
            "objective",
            ControlAction::Approve { revision: 1 },
            "approve",
            "operator",
            2,
        )
        .unwrap();
    let claim = store
        .claim_runnable("worker", 3, 3_600_000, 1)
        .unwrap()
        .remove(0);
    let run_id = claim.execution_run_id.as_ref().unwrap();
    let (_, context) =
        assemble_resident_context(root.path(), &claim, run_id, &execution, "project").unwrap();
    let request = crate::objectives::request_binding::ResidentRequestBinding {
        objective_id: "objective",
        leaf_id: "leaf",
        project_id: "project",
        project_contract_digest: &digest,
        workspace_root: &claim.workspace_root,
        authority: &claim.authority,
        execution: &execution,
        dependencies: &[],
    }
    .digest()
    .unwrap();
    store
        .bind_resident_context(&claim, &request, &context)
        .unwrap();
    let item = work_item(&claim, &context).unwrap();
    let run = RunStore::open(root.path(), RunId::new(run_id).unwrap()).unwrap();
    let mut parent = digest.clone();
    let mut receipts = Vec::new();
    let mut nodes = Vec::new();
    for stage in ["execute", "verify", "review", "close"] {
        let status = if stage == "review" {
            "failed"
        } else {
            "succeeded"
        };
        let mut output = None;
        if stage != "close" {
            let context = item
                .stage_context(root.path(), stage, vec![parent.clone()])
                .unwrap()
                .unwrap();
            MnemosyneService::new(root.path().join("data/vaire"))
                .unwrap()
                .with_contract_memory_root(root.path().join("core/state/memory"))
                .bind_run_stage_context(
                    item.context_assembly.as_ref().unwrap(),
                    stage,
                    &context.use_receipt.purpose,
                    vec![parent.clone()],
                    Utc::now().timestamp_millis() as u128,
                )
                .unwrap();
            let mut receipt: HermesExecutionReceipt = serde_json::from_value(json!({
                "schema_version":"arda.execution-receipt.v3","receipt_digest":"","authority_binding_digest":digest,
                "run_id":run_id,"node_id":stage,"idempotency_key":format!("{stage}-done"),"status":status,
                "summary":if stage=="review" {"VERDICT: BLOCK\nresult.txt contains the wrong answer; expected 42."} else {"fixture stage"},
                "tool_evidence":[{"tool":"file","action":"read","exit_code":0,"output_digest":digest}],
                "test_evidence":[],"artifacts":[{"path":"result.txt","digest":digest}],
                "usage":{"provider":"fixture","model":"fixture","api_calls":1,"input_tokens":1,"output_tokens":1,"total_tokens":2,"estimated_cost_usd":0.0,"completed":true,"failed":stage=="review"},
                "adapter":"hermes-workbench","adapter_version":"fixture","project_contract_digest":digest,
                "parent_receipts":[parent],"context_capsule_id":context.capsule.capsule_id,
                "context_capsule_digest":context.capsule.capsule_digest,"context_use_receipt_ref":context.use_receipt.receipt_ref(),"recorded_at_unix_ms":100,
            })).unwrap();
            receipt.receipt_digest = receipt.computed_digest().unwrap();
            run.append(RunEventDraft {
                node_id: NodeId::new(stage).unwrap(),
                idempotency_key: format!("{stage}-start"),
                kind: RunEventKind::NodeTransition {
                    state: NodeState::Running,
                },
                receipt_digest: None,
            })
            .unwrap();
            run.append(receipt.run_event_draft().unwrap()).unwrap();
            run.write_execution_receipt(
                &NodeId::new(stage).unwrap(),
                &serde_json::to_value(&receipt).unwrap(),
            )
            .unwrap();
            output = (stage != "review").then(|| receipt.receipt_digest.clone());
            parent = receipt.receipt_digest.clone();
            receipts.push(receipt);
        }
        nodes.push(json!({"id":stage,"kind":stage,"state":if stage=="close" {"pending"} else {status},
            "authority":if stage=="execute" {"execute_with_approval"} else if stage=="verify" {"verify"} else {"read_only"},
            "budget":{"max_joules":5000.0,"max_cost_usd":2.0},"retry":{"max_attempts":2},"timeout_ms":900000,
            "idempotency_key":format!("{stage}-key"),"output_digest":output,"parent_receipts":[],
            "worker":if stage=="close" {serde_json::Value::Null} else {json!({"role":if stage=="execute" {"implementer"} else if stage=="verify" {"independent_verifier"} else {"security_privacy_critic"},"worker_id":format!("worker-{stage}"),"route_id":"fixture","route_class":"hosted","prompt_digest":digest,"allowed_toolsets":["file"],"dependencies":[if stage=="execute" {"approval"} else if stage=="verify" {"execute"} else {"verify"}],"deadline_unix_ms":deadline.unwrap_or(9999999999999u64),"output_contract":"arda.hermes-job-result.v1","evidence_policy":if stage=="verify" {"project_native_checks"} else {"worker_report"}})}
        }));
    }
    nodes.push(json!({"id":"approval","kind":"approval","state":"succeeded","authority":"human_approval","budget":{"max_joules":5000.0,"max_cost_usd":2.0},"retry":{"max_attempts":2},"timeout_ms":900000,"idempotency_key":"approval-key","output_digest":digest,"parent_receipts":[digest]}));
    let edges:Vec<_> = [("approval","execute"),("execute","verify"),("verify","review"),("review","close")].into_iter().map(|(from,to)|json!({"id":format!("{from}-{to}"),"from":from,"to":to,"parent_receipt":digest})).collect();
    let graph: RunGraph = serde_json::from_value(json!({"schema_version":"arda.run-graph.v1","run_id":run_id,"objective_id":"leaf","nodes":nodes,"edges":edges,"provenance":{"project_contract_digest":digest,"created_by":"fixture","parent_receipts":[]}})).unwrap();
    run.write_checkpoint(&graph).unwrap();
    store
        .apply_control("objective", ControlAction::Pause, "pause", "operator", 4)
        .unwrap();
    store.bind_gateway_event("correction", "payload").unwrap();
    Fixture {
        root,
        store,
        run,
        item,
        graph,
        receipts,
    }
}

fn admit(f: &Fixture) -> Result<String> {
    f.store.admit_critic_correction(
        f.root.path(),
        "operator",
        "correction",
        "payload",
        "objective",
        "leaf",
        &f.receipts[2].receipt_digest,
        5,
    )
}

#[test]
fn critic_correction_requires_new_approval_preserves_rejection_and_replays_once() {
    let f = fixture();
    let before = std::fs::read(f.run.checkpoint_path()).unwrap();
    let child = admit(&f).unwrap();
    // Use the actual ingress objective shape, not the short fixture ID: the
    // successor must fit the explicit Workbench API's run identity bound.
    assert!(
        format!("objective-operator-objective-b7a986a5d1ee0830-leaf-{child}-attempt-1").len()
            <= 128
    );
    assert!(f.store.has_critic_correction("correction").unwrap());
    assert!(!f.store.has_critic_correction("unseen").unwrap());
    assert_eq!(admit(&f).unwrap(), child);
    assert_eq!(std::fs::read(f.run.checkpoint_path()).unwrap(), before);
    assert!(f
        .store
        .claim_runnable("worker", 6, 1000, 1)
        .unwrap()
        .is_empty());
    let original = f.store.leaf("leaf").unwrap().unwrap();
    assert_eq!(original.stage, crate::objectives::LeafStage::Failed);
    assert_eq!(original.attempt, 1);
    assert!(f
        .store
        .close_objective("objective", "forged-close", 6)
        .is_err());
    f.store
        .apply_control(
            "objective",
            ControlAction::Approve { revision: 2 },
            "approve-correction",
            "operator",
            7,
        )
        .unwrap();
    let claims = f.store.claim_runnable("worker", 8, 1000, 1).unwrap();
    assert_eq!(claims.len(), 1);
    assert_eq!(claims[0].leaf_id, child);
    assert_ne!(
        claims[0].execution_run_id.as_deref(),
        Some(f.item.run_id.as_str())
    );
    assert!(claims[0]
        .execution
        .as_ref()
        .unwrap()
        .execution_prompt
        .contains(&f.receipts[2].receipt_digest));
    let db =
        rusqlite::Connection::open(f.root.path().join("data/arda/objectives.sqlite3")).unwrap();
    let spent: i64 = db
        .query_row(
            "SELECT attempt+attempt_offset FROM leaves WHERE id=?1",
            [child],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(spent, 2);
    assert!(db.execute("DELETE FROM critic_corrections", []).is_err());
    for (generation, now) in [(2, 1009), (3, 2010), (4, 3011)] {
        let claims = f.store.claim_runnable("retry", now, 1000, 1).unwrap();
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].attempt, generation);
    }
    assert!(f
        .store
        .claim_runnable("exhausted", 4012, 1000, 1)
        .unwrap()
        .is_empty());
    assert_eq!(
        f.store.objective("objective").unwrap().unwrap().state,
        crate::objectives::ObjectiveState::Failed
    );
}

#[test]
fn critic_correction_preserves_absolute_deadline_after_restart() {
    for deadline in [1, 9_999_999_999_999] {
        let f = fixture_with_deadline(Some(deadline));
        let child = admit(&f).unwrap();
        let root = f.root.path().to_path_buf();
        drop(f.store);
        let store =
            ObjectiveStore::open_existing(root.join("data/arda/objectives.sqlite3")).unwrap();
        store
            .apply_control(
                "objective",
                ControlAction::Approve { revision: 2 },
                "approve-correction",
                "operator",
                7,
            )
            .unwrap();
        let claim = store
            .claim_runnable("worker", 8, 1000, 1)
            .unwrap()
            .remove(0);
        assert_eq!(claim.leaf_id, child);
        let execution = claim.execution.as_ref().unwrap();
        assert_eq!(execution.execution_deadline_unix_ms, Some(deadline));
        let (_, context) = assemble_resident_context(
            &root,
            &claim,
            claim.execution_run_id.as_ref().unwrap(),
            execution,
            "project",
        )
        .unwrap();
        assert_eq!(
            work_item(&claim, &context)
                .unwrap()
                .execution_deadline_unix_ms,
            Some(deadline)
        );
    }
}

#[test]
fn critic_correction_rejects_invalid_evidence() {
    for kind in [
        "workerless",
        "same-worker",
        "synthetic",
        "stale",
        "provenance-free",
        "approving",
        "wrong-parent",
        "wrong-operator",
        "unbound-event",
        "exhausted",
    ] {
        let mut f = fixture();
        match kind {
            "workerless" => f.graph.nodes[2].worker = None,
            "same-worker" => f.graph.nodes[2].worker = f.graph.nodes[0].worker.clone(),
            "synthetic" => {
                std::fs::write(
                    f.run
                        .checkpoint_path()
                        .parent()
                        .unwrap()
                        .join("events.jsonl"),
                    b"",
                )
                .unwrap();
            }
            "stale" => f.receipts[2].run_id = "other-run".into(),
            "provenance-free" => f.receipts[2].usage.provider = None,
            "approving" => f.receipts[2].summary = "VERDICT: APPROVE".into(),
            "wrong-parent" => {
                f.receipts[2].parent_receipts = vec![f.receipts[0].receipt_digest.clone()]
            }
            "exhausted" => {
                let db =
                    rusqlite::Connection::open(f.root.path().join("data/arda/objectives.sqlite3"))
                        .unwrap();
                db.execute("UPDATE leaves SET attempt_offset=4", [])
                    .unwrap();
            }
            _ => {}
        }
        if matches!(kind, "workerless" | "same-worker") {
            std::fs::write(
                f.run.checkpoint_path(),
                serde_json::to_vec(&f.graph).unwrap(),
            )
            .unwrap();
        }
        if matches!(
            kind,
            "stale" | "provenance-free" | "approving" | "wrong-parent"
        ) {
            f.receipts[2].receipt_digest = f.receipts[2].computed_digest().unwrap();
            // Keep canonical digest links internally consistent so each refusal
            // exercises the altered semantic binding rather than a stale hash.
            f.graph.nodes[2].output_digest = None;
            std::fs::write(
                f.run.checkpoint_path(),
                serde_json::to_vec(&f.graph).unwrap(),
            )
            .unwrap();
            let events_path = f
                .run
                .checkpoint_path()
                .parent()
                .unwrap()
                .join("events.jsonl");
            let mut events: Vec<crate::runs::RunEvent> = std::fs::read_to_string(&events_path)
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
            events.last_mut().unwrap().receipt_digest = Some(f.receipts[2].receipt_digest.clone());
            std::fs::write(
                events_path,
                events
                    .iter()
                    .map(|e| serde_json::to_string(e).unwrap() + "\n")
                    .collect::<String>(),
            )
            .unwrap();

            std::fs::write(
                f.root
                    .path()
                    .join("data/runs")
                    .join(&f.item.run_id)
                    .join("execution-receipts/review.json"),
                serde_json::to_vec(&f.receipts[2]).unwrap(),
            )
            .unwrap();
        }
        let result = if kind == "wrong-operator" {
            f.store.admit_critic_correction(
                f.root.path(),
                "other",
                "correction",
                "payload",
                "objective",
                "leaf",
                &f.receipts[2].receipt_digest,
                5,
            )
        } else if kind == "unbound-event" {
            f.store.admit_critic_correction(
                f.root.path(),
                "operator",
                "different-event",
                "payload",
                "objective",
                "leaf",
                &f.receipts[2].receipt_digest,
                5,
            )
        } else {
            admit(&f)
        };
        let error = format!("{:#}", result.expect_err(&format!("accepted {kind}")));
        let expected = match kind {
            "workerless" => "workerless",
            "same-worker" => "not independent",
            "synthetic" => "never started",
            "stale" => "canonical validation",
            "provenance-free" => "omitted provider",
            "approving" => "blocking defect",
            "wrong-parent" => "canonical predecessor",
            "wrong-operator" => "operator mismatch",
            "unbound-event" => "authenticated event",
            "exhausted" => "budget exhausted",
            _ => unreachable!(),
        };
        assert!(
            error.contains(expected),
            "wrong refusal for {kind}: {error}"
        );
        assert_eq!(f.store.objective("objective").unwrap().unwrap().revision, 1);
        assert_eq!(f.store.leaf("leaf").unwrap().unwrap().attempt, 1);
    }
}
