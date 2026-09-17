use super::*;
use serde_json::json;

#[test]
fn recovery_close_resumes_each_durable_transition() {
    let digest = format!("sha256:{}", "b".repeat(64));
    let request = json!({"receipt_digest":digest,"envelope":{
        "idempotency_key":"close-key", "approval":{
            "schema_version":"arda.orome.task_approval.v1","proposal_id":"fixture",
            "approval_id":"fixture","ledger_writes":["fixture.jsonl"],
            "decision":"policy_safe","created_at_utc":"2026-07-31T00:00:00Z"}}});
    for durable in 0..=3 {
        for checkpoint_survived in [false, true] {
            let temp = tempfile::tempdir().unwrap();
            let mut graph: RunGraph = serde_json::from_value(json!({
                "schema_version":"arda.run-graph.v1","run_id":"close-crash","objective_id":"fixture",
                "nodes":[{"id":"close","kind":"close","state":"pending","authority":"read_only",
                    "budget":{"max_joules":1.0,"max_cost_usd":0.0},"retry":{"max_attempts":1},
                    "timeout_ms":1000,"idempotency_key":"close-key","input_digest":null,
                    "output_digest":null,"parent_receipts":[]}],"edges":[],
                "provenance":{"project_contract_digest":digest,"created_by":"fixture","parent_receipts":[]}
            })).unwrap();
            let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
            let node = NodeId::new("close").unwrap();
            let old = graph.clone();
            for (state, key) in [
                (NodeState::Ready, "close-key:ready"),
                (NodeState::Running, "close-key:running"),
                (NodeState::Succeeded, "close-key"),
            ]
            .iter()
            .take(durable)
            {
                apply_transition_once(
                    &store,
                    &mut graph,
                    &node,
                    *state,
                    *key,
                    Some(digest.clone()),
                )
                .unwrap();
            }
            store
                .write_checkpoint(if checkpoint_survived { &graph } else { &old })
                .unwrap();
            apply_close_projection(&store, serde_json::from_value(request.clone()).unwrap())
                .unwrap();
            let first = std::fs::read(store.events_path()).unwrap();
            apply_close_projection(&store, serde_json::from_value(request.clone()).unwrap())
                .unwrap();
            assert_eq!(first, std::fs::read(store.events_path()).unwrap());
            let complete = store.recover().unwrap().checkpoint.unwrap();
            assert_eq!(complete.nodes[0].state, NodeState::Succeeded);
            assert_eq!(
                complete.nodes[0].output_digest.as_deref(),
                Some(digest.as_str())
            );
            assert_eq!(complete.nodes[0].checkpoint.sequence, 3);
        }
    }
}

#[test]
fn recovery_publication_rebuilds_success_lineage_after_checkpoint_crash() {
    let temp = tempfile::tempdir().unwrap();
    let digest = format!("sha256:{}", "a".repeat(64));
    let mut receipt: HermesExecutionReceipt = serde_json::from_value(json!({
        "schema_version":"arda.execution-receipt.v3", "receipt_digest":"",
        "authority_binding_digest":digest, "run_id":"publication-crash", "node_id":"verify",
        "idempotency_key":"verify-key", "status":"succeeded", "summary":"fixture verified",
        "tool_evidence":[],"test_evidence":[],"artifacts":[],
        "usage":{"provider":"fixture","model":"fixture","api_calls":1,"input_tokens":1,
            "output_tokens":1,"total_tokens":2,"estimated_cost_usd":0.0,"completed":true,"failed":false},
        "adapter":"fixture","adapter_version":"fixture","project_contract_digest":digest,
        "parent_receipts":[],"recorded_at_unix_ms":100
    })).unwrap();
    receipt.receipt_digest = receipt.computed_digest().unwrap();
    let nodes: Vec<_> = [
        ("verify", "verify", "running", "verify"),
        ("review", "review", "pending", "read_only"),
    ]
    .into_iter()
    .map(|(id, kind, state, authority)| {
        json!({
            "id":id,"kind":kind,"state":state,"authority":authority,
            "budget":{"max_joules":10.0,"max_cost_usd":1.0},"retry":{"max_attempts":2},
            "timeout_ms":1000,"idempotency_key":format!("{id}-key"),
            "input_digest":null,"output_digest":null,"parent_receipts":[]
        })
    })
    .collect();
    let graph: RunGraph = serde_json::from_value(json!({
        "schema_version":"arda.run-graph.v1","run_id":"publication-crash",
        "objective_id":"fixture","nodes":nodes,
        "edges":[{"id":"verify-review","from":"verify","to":"review","parent_receipt":null}],
        "provenance":{"project_contract_digest":digest,"created_by":"fixture","parent_receipts":[]}
    }))
    .unwrap();
    let store = RunStore::open(temp.path(), graph.run_id.clone()).unwrap();
    let node = NodeId::new("verify").unwrap();
    for (state, key, output) in [
        (NodeState::Running, "verify-key:provider-running:1", None),
        (
            NodeState::Succeeded,
            "verify-key",
            Some(receipt.receipt_digest.clone()),
        ),
    ] {
        store
            .append(RunEventDraft {
                node_id: node.clone(),
                idempotency_key: key.into(),
                kind: RunEventKind::NodeTransition { state },
                receipt_digest: output,
            })
            .unwrap();
    }
    // The terminal append exists, but only the pre-terminal checkpoint survived.
    store.write_checkpoint(&graph).unwrap();
    let before = std::fs::read(store.events_path()).unwrap();
    let publication = crate::objectives::RecoveryPublication {
        key: "terminal".into(),
        kind: "provider-finalization".into(),
        node_id: node.clone(),
        payload: json!({"start_key":"verify-key:provider-running:1","attempt":1,"receipt":receipt}),
    };
    let mut projected = None;
    for _ in 0..2 {
        apply_provider_publication(&store, &publication).unwrap();
        let restored = store.recover().unwrap().checkpoint.unwrap();
        assert_eq!(
            restored.edges[0].parent_receipt.as_deref(),
            Some(receipt.receipt_digest.as_str())
        );
        assert_eq!(
            restored.nodes[1].parent_receipts,
            vec![receipt.receipt_digest.clone()]
        );
        assert_eq!(restored.nodes[0].checkpoint.sequence, 2);
        assert!(restored.nodes[0].checkpoint.recovery_token.is_some());
        assert!(restored.nodes[0].checkpoint.checkpoint_digest.is_some());
        let bytes = std::fs::read(store.events_path()).unwrap();
        assert!(bytes.starts_with(&before));
        if let Some(previous) = &projected {
            assert_eq!(&bytes, previous);
        }
        projected = Some(bytes);
    }
}
