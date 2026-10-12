use super::*;

fn bounded_item(deadline: u64) -> ExplicitWorkbenchWorkItem {
    serde_json::from_value(json!({
        "objective_id":"objective", "leaf_id":"leaf", "run_id":"run",
        "objective":"bounded repair", "execution_prompt":"repair",
        "verification_prompt":"verify", "review_prompt":"review",
        "project_id":"project", "project_contract_digest":"digest",
        "workspace_root":"/fixture", "approval_envelope":{"approval":{"approval_id":"approval"}},
        "objective_plan_receipt":"receipt",
        "execution_deadline_unix_ms":deadline
    }))
    .expect("approved absolute deadline must be retained")
}

#[test]
fn approved_deadline_caps_every_provider_stage() {
    let item = bounded_item(1);
    for _ in 0..2 {
        let graph = explicit_run_graph(&item, "approval");
        for id in ["execute", "verify", "review"] {
            let node = graph["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|n| n["id"] == id)
                .unwrap();
            assert_eq!(node["worker"]["deadline_unix_ms"], 1, "{id}");
        }
    }
}

#[test]
fn deadline_boundary_matrix_never_extends_default_worker_window() {
    let now = Utc::now().timestamp_millis() as u64;
    for deadline in [0, now + 60_000, u64::MAX] {
        let graph = explicit_run_graph(&bounded_item(deadline), "approval");
        for node in graph["nodes"].as_array().unwrap() {
            if let Some(actual) = node["worker"]["deadline_unix_ms"].as_u64() {
                assert!(actual <= deadline);
                if deadline != u64::MAX {
                    assert_eq!(actual, deadline);
                } else {
                    assert!(actual > now);
                    assert!(actual < u64::MAX);
                }
            }
        }
    }
}

#[test]
fn expired_approved_deadline_refuses_new_provider_body() {
    let item = bounded_item(1);
    let error = workspace_authority::provider_body(None, &item, json!({})).unwrap_err();
    assert!(error.to_string().contains("approved execution deadline"));
    // Receipt-only workspace reconciliation must remain available after expiry.
    workspace_authority::bound_workspace(
        Path::new("/fixture"),
        &item,
        workspace_authority::Validation::Reconcile,
    )
    .unwrap();
}

#[test]
fn retained_graph_cannot_extend_or_drop_approved_deadline() {
    let item = bounded_item(1);
    let graph = explicit_run_graph(&item, "approval");
    workspace_authority::run(&item, &json!({"graph":graph.clone()})).unwrap();
    for id in ["execute", "verify", "review"] {
        for value in [json!(2), Value::Null] {
            let mut changed = graph.clone();
            let node = changed["nodes"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|n| n["id"] == id)
                .unwrap();
            node["worker"]["deadline_unix_ms"] = value;
            let error = workspace_authority::run(&item, &json!({"graph":changed})).unwrap_err();
            assert!(error.to_string().contains("approved execution deadline"));
        }
    }
}

#[test]
fn optional_deadline_does_not_change_legacy_work_item_encoding() {
    let mut encoded = serde_json::to_value(bounded_item(1)).unwrap();
    encoded
        .as_object_mut()
        .unwrap()
        .remove("execution_deadline_unix_ms");
    let item: ExplicitWorkbenchWorkItem = serde_json::from_value(encoded).unwrap();
    assert!(serde_json::to_value(&item)
        .unwrap()
        .get("execution_deadline_unix_ms")
        .is_none());
    assert!(workspace_authority::provider_body(None, &item, json!({})).is_ok());
}
