use arda_vaire::{OrganismContext, OrganismContextError};

fn valid_context() -> String {
    serde_json::json!({
        "schema_version": "arda.organism-context.v1",
        "organism_id": "arda:mythos:primary",
        "generated_at_unix_ms": 1000,
        "expires_at_unix_ms": 2000,
        "consumer": {
            "consumer_id": "hermes:default:worker-1",
            "role": "planner",
            "authority_ceiling": "plan",
            "operator_authorized": false,
            "memory_domains": ["system"],
            "data_classes": ["internal"],
            "permitted_egress": ["local_device"],
            "compute_node_refs": ["node-core-hub"],
            "agent_ref": "hermes:default"
        },
        "lineage": {
            "objective_id": "operator-objective-1",
            "project_id": null,
            "run_id": null,
            "task_id": "operator-objective-1",
            "session_ref": "hermes:session-1",
            "parent_receipts": []
        },
        "objective": {
            "requested_outcome": "Produce a source-backed plan.",
            "acceptance_conditions": ["Every claim has evidence."],
            "required_capabilities": ["research"],
            "forbidden_capabilities": ["payments"]
        },
        "evidence_refs": ["varda:evidence-1"],
        "memory_refs": ["vaire:memory-1"],
        "unresolved_failures": [],
        "return_contract": {
            "schema_version": "arda.organism-outcome.v1",
            "required_receipt_types": ["arda.handoff-receipt.v1"],
            "max_output_bytes": 32768
        }
    })
    .to_string()
}

#[test]
fn valid_context_reuses_core_lineage_and_policy_types() {
    let context = OrganismContext::from_json_str(&valid_context()).expect("valid context");
    assert_eq!(
        context.lineage.objective_id.as_str(),
        "operator-objective-1"
    );
    assert_eq!(context.consumer.compute_node_refs, vec!["node-core-hub"]);
    assert_eq!(context.return_contract.max_output_bytes, 32768);
}

#[test]
fn context_rejects_sensitive_external_egress() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    value["consumer"]["data_classes"] = serde_json::json!(["private"]);
    value["consumer"]["permitted_egress"] = serde_json::json!(["hosted_provider"]);
    assert!(matches!(
        OrganismContext::from_json_str(&value.to_string()),
        Err(OrganismContextError::SensitiveExternalEgress)
    ));
}

#[test]
fn personal_context_requires_operator_authority() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    value["consumer"]["memory_domains"] = serde_json::json!(["personal"]);
    assert!(matches!(
        OrganismContext::from_json_str(&value.to_string()),
        Err(OrganismContextError::PersonalContextRequiresOperator)
    ));
}

#[test]
fn context_rejects_expiry_duplicates_and_unknown_fields() {
    let mut expired: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    expired["expires_at_unix_ms"] = serde_json::json!(999);
    assert!(matches!(
        OrganismContext::from_json_str(&expired.to_string()),
        Err(OrganismContextError::InvalidExpiry)
    ));

    let mut duplicate: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    duplicate["consumer"]["compute_node_refs"] =
        serde_json::json!(["node-core-hub", "node-core-hub"]);
    assert!(matches!(
        OrganismContext::from_json_str(&duplicate.to_string()),
        Err(OrganismContextError::DuplicateReference {
            field: "consumer.compute_node_refs",
            ..
        })
    ));

    let mut unknown: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    unknown["transcript"] = serde_json::json!("must never be copied");
    assert!(matches!(
        OrganismContext::from_json_str(&unknown.to_string()),
        Err(OrganismContextError::InvalidJson(_))
    ));
}

#[test]
fn context_rejects_ambiguous_failure_lineage() {
    let mut value: serde_json::Value = serde_json::from_str(&valid_context()).unwrap();
    value["unresolved_failures"] = serde_json::json!([
        {
            "failure_id": "failure-1",
            "class": "tool",
            "summary": "First failure",
            "receipt_ref": "receipt:failure-1"
        },
        {
            "failure_id": "failure-1",
            "class": "provider",
            "summary": "Conflicting duplicate identity",
            "receipt_ref": null
        }
    ]);

    assert!(matches!(
        OrganismContext::from_json_str(&value.to_string()),
        Err(OrganismContextError::DuplicateReference {
            field: "unresolved_failures.failure_id",
            ..
        })
    ));
}
