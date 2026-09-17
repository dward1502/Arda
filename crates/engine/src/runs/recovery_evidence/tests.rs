use super::*;
use crate::runs::{RecoveryGrant, RunEventDraft, RECOVERY_WINDOW_MS};
use serde_json::json;
#[path = "adversarial_tests.rs"]
mod adversarial_tests;

pub(crate) fn fixture() -> (
    tempfile::TempDir,
    RunStore,
    RunGraph,
    RecoveryBindings,
    HermesExecutionReceipt,
) {
    let temp = tempfile::tempdir().unwrap();
    let digest = format!("sha256:{}", "a".repeat(64));
    let mut receipt: HermesExecutionReceipt = serde_json::from_value(json!({
        "schema_version":"arda.hermes.execution-receipt.v1", "receipt_digest":"",
        "authority_binding_digest":digest, "run_id":"recovery-evidence-fixture", "node_id":"execute",
        "idempotency_key":"execute-key", "status":"succeeded", "summary":"synthetic test receipt",
        "tool_evidence":[],"test_evidence":[],"artifacts":[],
        "usage":{"provider":"fixture","model":"fixture","api_calls":1,"input_tokens":1,
            "output_tokens":1,"total_tokens":2,"estimated_cost_usd":0.0,"completed":true,"failed":false},
        "adapter":"fixture","adapter_version":"fixture","project_contract_digest":digest,
        "parent_receipts":[digest],"recorded_at_unix_ms":100
    })).unwrap();
    receipt.receipt_digest = receipt.computed_digest().unwrap();
    let nodes: Vec<_> = [
        ("approval","approval","succeeded","human_approval"),
        ("execute","inspect","succeeded","read_only"),
        ("verify","verify","failed","verify"),
        ("review","review","pending","read_only"),
        ("close","close","pending","read_only"),
    ].into_iter().map(|(id,kind,state,authority)| json!({
        "id":id,"kind":kind,"state":state,"authority":authority,
        "budget":{"max_joules":10.0,"max_cost_usd":1.0},"retry":{"max_attempts":2},
        "timeout_ms":1000,"idempotency_key":format!("{id}-key"),
        "input_digest":null,
        "output_digest": match id {"approval"=>Some(digest.clone()),"execute"=>Some(receipt.receipt_digest.clone()),_=>None},
        "parent_receipts": if id=="execute" {vec![digest.clone()]} else {vec![]}
    })).collect();
    let graph: RunGraph = serde_json::from_value(json!({
        "schema_version":"arda.run-graph.v1","run_id":"recovery-evidence-fixture",
        "objective_id":"fixture-objective","nodes":nodes,"edges":[],
        "provenance":{"project_contract_digest":digest,"created_by":"fixture","parent_receipts":[]}
    }))
    .unwrap();
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    for (id, state, key, output) in [
        (
            "execute",
            NodeState::Running,
            "execute-key:provider-running:1",
            None,
        ),
        (
            "execute",
            NodeState::Succeeded,
            "execute-key",
            Some(receipt.receipt_digest.clone()),
        ),
        (
            "verify",
            NodeState::Running,
            "verify-key:provider-running:1",
            None,
        ),
        (
            "verify",
            NodeState::Failed,
            "verify-key:provider-error:1",
            None,
        ),
        (
            "verify",
            NodeState::Running,
            "verify-key:provider-running:2",
            None,
        ),
        (
            "verify",
            NodeState::Failed,
            "verify-key:provider-error:2",
            None,
        ),
    ] {
        store
            .append(RunEventDraft {
                node_id: NodeId::new(id).unwrap(),
                idempotency_key: key.into(),
                kind: RunEventKind::NodeTransition { state },
                receipt_digest: output,
            })
            .unwrap();
    }
    store.write_checkpoint(&graph).unwrap();
    store
        .write_execution_receipt(
            &NodeId::new("execute").unwrap(),
            &serde_json::to_value(&receipt).unwrap(),
        )
        .unwrap();
    let failure = store.recover().unwrap().events.last().unwrap().clone();
    let bindings: RecoveryBindings = serde_json::from_value(json!({
        "objective_id":"fixture-objective","objective_revision":1,"stop_generation":1,
        "leaf_id":"fixture-leaf","run_id":graph.run_id,"failed_event_sequence":failure.sequence,
        "failed_event_digest":format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(&failure).unwrap())),
        "execute_receipt_digest":receipt.receipt_digest,"project_contract_digest":digest,
        "context_capsule_digest":digest,"context_use_receipt_digest":digest,"retained_authority_digest":digest,
        "verify_node_id":"verify","review_node_id":"review","close_node_id":"close",
        "prior_verify_starts":2,"provider_start_ceilings":{"verify":3,"review":2}
    })).unwrap();
    (temp, store, graph, bindings, receipt)
}

#[test]
fn canonical_files_and_historical_failure_remain_valid_after_forward_progress() {
    let (_temp, store, mut graph, bindings, _) = fixture();
    store.validate_recovery_run_evidence(&bindings).unwrap();
    let now = 1_000;
    let grant = RecoveryGrant {
        authenticated_event_id: "fixture-event".into(),
        authenticated_payload_digest: format!("sha256:{}", "b".repeat(64)),
        bindings: bindings.clone(),
        activated_at_unix_ms: now,
        expires_at_unix_ms: now + RECOVERY_WINDOW_MS,
    };
    store.activate_recovery(grant).unwrap();
    let mut events = store.recover().unwrap().events;
    let mut start = events
        .iter()
        .find(|e| e.idempotency_key == "verify-key:provider-running:2")
        .unwrap()
        .clone();
    start.sequence = events.last().unwrap().sequence + 1;
    start.idempotency_key = "recovery-third-start".into();
    events.push(start);
    graph
        .nodes
        .iter_mut()
        .find(|n| n.id.as_str() == "verify")
        .unwrap()
        .state = NodeState::Succeeded;
    let receipt = serde_json::from_value(
        store
            .read_execution_receipt(&NodeId::new("execute").unwrap())
            .unwrap()
            .unwrap(),
    )
    .unwrap();
    validate_history(&graph, &events, &receipt, &bindings).unwrap();
    events.retain(|e| !matches!(e.kind, RunEventKind::RecoveryActivated { .. }));
    assert!(validate_history(&graph, &events, &receipt, &bindings).is_err());
}

#[test]
fn canonical_tampering_and_replayed_execution_fail_closed() {
    let (_temp, store, graph, bindings, receipt) = fixture();
    let events = store.recover().unwrap().events;
    for case in [
        "failure",
        "ceiling",
        "authority",
        "receipt",
        "replay",
        "cancel",
    ] {
        let mut b = bindings.clone();
        let mut g = graph.clone();
        let mut r = receipt.clone();
        let mut e = events.clone();
        match case {
            "failure" => b.failed_event_digest = format!("sha256:{}", "c".repeat(64)),
            "ceiling" => b
                .provider_start_ceilings
                .insert("review".into(), 3)
                .map(|_| ())
                .unwrap(),
            "authority" => g.nodes[1].authority = AuthorityClass::ExecuteWithApproval,
            "receipt" => r.summary.push_str(" changed"),
            "replay" => {
                let mut v = e[0].clone();
                v.sequence = 100;
                e.push(v);
            }
            "cancel" => {
                let mut v = e[0].clone();
                v.sequence = 100;
                v.kind = RunEventKind::NodeTransition {
                    state: NodeState::Cancelled,
                };
                e.push(v);
            }
            _ => unreachable!(),
        }
        assert!(validate_history(&g, &e, &r, &b).is_err(), "accepted {case}");
    }
    let mut changed = receipt;
    changed.summary.push_str(" changed");
    store
        .write_execution_receipt(
            &NodeId::new("execute").unwrap(),
            &serde_json::to_value(changed).unwrap(),
        )
        .unwrap();
    assert!(store.validate_recovery_run_evidence(&bindings).is_err());
}
