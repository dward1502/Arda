use super::super::tests::{bind_test_run, scripted_harness};
use super::*;
mod recovery;

#[test]
fn explicit_external_workspace_requires_matching_registered_contract() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let mut work = item(external.path());
    work.workspace_root = external.path().to_path_buf();
    let mut contract: Value = serde_json::from_str(include_str!(
        "../../../../../../../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    work.project_id = contract["identity"]["project_id"].as_str().unwrap().into();
    contract["workspace"]["root"] = external.path().to_str().unwrap().into();
    let parsed: arda_core::project_contract::ProjectContract =
        serde_json::from_value(contract).unwrap();
    work.project_contract_digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&parsed).unwrap())
    );
    assert!(validate_explicit_work_item(root.path(), &work).is_err());
    std::fs::create_dir_all(root.path().join("data/workbench")).unwrap();
    std::fs::write(root.path().join("data/workbench/projects.json"), serde_json::to_vec(&json!({
        "schema_version": "arda.workbench.project-registry.v1",
        "projects": [{"contract": parsed, "approval_id":"approved", "proposal_id":"proposal", "idempotency_key":"external"}]
    })).unwrap()).unwrap();
    validate_explicit_work_item(root.path(), &work).unwrap();
    work.project_contract_digest = format!("sha256:{}", "0".repeat(64));
    assert!(validate_explicit_work_item(root.path(), &work).is_err());
}

#[test]
fn explicit_read_only_graph_has_no_terminal_and_rejects_expanded_replay() {
    let root = tempfile::tempdir().unwrap();
    let mut work = item(root.path());
    work.read_only = true;
    let graph = explicit_run_graph(&work, "approved");
    let parsed: arda_core::run_graph::RunGraph = serde_json::from_value(graph.clone()).unwrap();
    parsed.validate().unwrap();
    let bound = bind_test_run(&work, json!({"graph": graph}));
    run(&work, &bound).unwrap();
    for id in ["execute", "verify", "review"] {
        let mut expanded = bound.clone();
        let node = expanded["graph"]["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["id"] == id)
            .unwrap();
        assert_eq!(node["worker"]["allowed_toolsets"], json!(["file"]));
        node["worker"]["allowed_toolsets"] = json!(["file", "terminal"]);
        assert!(run(&work, &expanded)
            .unwrap_err()
            .to_string()
            .contains("inspection scope"));
    }
    let execute = bound["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "execute")
        .unwrap();
    assert_eq!(execute["kind"], "inspect");
    assert_eq!(execute["authority"], "read_only");
    work.read_only = false;
    let legacy = explicit_run_graph(&work, "approved");
    assert!(legacy["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["id"] == "execute" && node["kind"] == "execute"));
}

#[test]
fn external_completed_reconciliation_ignores_live_registry() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let mut work = item(external.path());
    work.workspace_root = external.path().to_path_buf();
    drop(external);
    for contents in [None, Some("broken registry"), Some("{\"projects\":[]}")] {
        if let Some(contents) = contents {
            std::fs::create_dir_all(root.path().join("data/workbench")).unwrap();
            std::fs::write(root.path().join("data/workbench/projects.json"), contents).unwrap();
        }
        bound_workspace(root.path(), &work, Validation::Reconcile).unwrap();
        assert!(bound_workspace(root.path(), &work, Validation::Fresh).is_err());
    }
}

#[tokio::test]
async fn explicit_read_only_dispatch_posts_inspection_graph() {
    let root = tempfile::tempdir().unwrap();
    let mut work = item(root.path());
    work.read_only = true;
    let bound = bind_test_run(
        &work,
        json!({"graph": explicit_run_graph(&work, "approved")}),
    );
    let (url, requests) = scripted_harness(vec![
        Some((404, "{}".into())),
        Some((201, bound.to_string())),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let guard = FencedAfter {
        calls: std::sync::atomic::AtomicUsize::new(0),
        allowed: 2,
    };
    let error = adapter.execute_authorized(&work, &guard).await.unwrap_err();
    assert!(error.to_string().contains("fixture authority fenced"));
    let requests = requests.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("POST /v1/runs/plan"));
    let body: Value = serde_json::from_str(requests[1].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    let nodes = body["graph"]["nodes"].as_array().unwrap();
    let execute = nodes.iter().find(|node| node["id"] == "execute").unwrap();
    assert_eq!(execute["kind"], "inspect");
    assert_eq!(execute["authority"], "read_only");
    for id in ["execute", "verify", "review"] {
        assert_eq!(
            nodes.iter().find(|node| node["id"] == id).unwrap()["worker"]["allowed_toolsets"],
            json!(["file"])
        );
    }
}

struct FencedAfter {
    calls: std::sync::atomic::AtomicUsize,
    allowed: usize,
}
impl ExplicitWorkspaceAuthorization for FencedAfter {
    fn authorize(&self, _: &ExplicitWorkbenchWorkItem) -> Result<()> {
        if self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= self.allowed {
            anyhow::bail!("fixture authority fenced");
        }
        Ok(())
    }
    fn provider_lease(&self, item: &ExplicitWorkbenchWorkItem) -> Result<Value> {
        Ok(json!({"run_id":item.run_id,"generation":1,"owner":"fixture","expires_ms":i64::MAX}))
    }
}

#[tokio::test]
async fn fencing_after_get_prevents_approval_post() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    let (url, requests) = scripted_harness(vec![Some((200, pending_run(&work).to_string()))]).await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let guard = FencedAfter {
        calls: std::sync::atomic::AtomicUsize::new(0),
        allowed: 1,
    };
    assert!(adapter
        .execute_authorized(&work, &guard)
        .await
        .unwrap_err()
        .to_string()
        .contains("fixture authority fenced"));
    let requests = requests.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET "));
    assert!(!root.path().join("data").exists());
}

#[tokio::test]
async fn fencing_after_execute_prevents_verify_and_carries_original_lease() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    let mut initial = pending_run(&work);
    initial["graph"]["nodes"][0]["state"] = json!("succeeded");
    let mut executed = initial.clone();
    executed["graph"]["nodes"][1]["state"] = json!("succeeded");
    let (url, requests) = scripted_harness(vec![
        Some((200, initial.to_string())),
        Some((
            200,
            json!({"receipt":{"status":"succeeded"},"run":executed}).to_string(),
        )),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let guard = FencedAfter {
        calls: std::sync::atomic::AtomicUsize::new(0),
        allowed: 3,
    };
    assert!(adapter
        .execute_authorized(&work, &guard)
        .await
        .unwrap_err()
        .to_string()
        .contains("fixture authority fenced"));
    let requests = requests.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[1].starts_with("POST /v1/runs/retained-run/nodes/execute/execute-provider"));
    let body: Value = serde_json::from_str(requests[1].split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body["expected_retained_lease"]["generation"], 1);
    assert_eq!(body["expected_retained_lease"]["owner"], "fixture");
    assert_eq!(body["expected_retained_lease"]["run_id"], work.run_id);
}

fn item(root: &Path) -> ExplicitWorkbenchWorkItem {
    ExplicitWorkbenchWorkItem {
        read_only: false,
        objective_id: "objective".into(),
        leaf_id: "leaf".into(),
        run_id: "retained-run".into(),
        objective: "Inspect".into(),
        execution_prompt: "Inspect".into(),
        verification_prompt: "Verify".into(),
        review_prompt: "Review".into(),
        project_id: "project".into(),
        project_contract_digest: format!("sha256:{}", "a".repeat(64)),
        workspace_root: root.join("missing"),
        approval_envelope: json!({"approval":{"schema_version":"arda.orome.task_approval.v1","approval_id":"approved","ledger_writes":["data/arda/objectives.sqlite3","data/runs"]}}),
        objective_plan_receipt: format!("sha256:{}", "b".repeat(64)),
        dependency_receipts: vec![],
        context_assembly: None,
    }
}
fn pending_run(item: &ExplicitWorkbenchWorkItem) -> Value {
    bind_test_run(
        item,
        json!({"graph":{"nodes":[
            {"id":"approval","state":"pending"}, {"id":"execute","state":"pending"}, {"id":"verify","state":"pending"}, {"id":"review","state":"pending"}, {"id":"close","state":"pending"}
        ]}}),
    )
}

#[tokio::test]
async fn missing_execution_needs_process_local_authorization() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    let (url, server) = scripted_harness(vec![]).await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    assert!(adapter.execute(&work).await.is_err());
    assert!(server.await.unwrap().is_empty());
    let (url, server) = scripted_harness(vec![]).await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let deny = |_: &ExplicitWorkbenchWorkItem| -> Result<()> { bail!("fixture authority denied") };
    assert!(adapter
        .execute_authorized(&work, &deny)
        .await
        .unwrap_err()
        .to_string()
        .contains("fixture authority denied"));
    assert!(server.await.unwrap().is_empty());
    let (url, server) = scripted_harness(vec![
        Some((404, "{}".into())),
        Some((500, "fixture plan stop".into())),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let allow = |_: &ExplicitWorkbenchWorkItem| -> Result<()> { Ok(()) };
    assert!(adapter
        .execute_authorized(&work, &allow)
        .await
        .unwrap_err()
        .to_string()
        .contains("returned 500"));
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests[0].starts_with("GET "));
    assert!(requests[1].starts_with("POST /v1/runs/plan "));
    let mut serialized = serde_json::to_value(&work).unwrap();
    serialized["allow_missing_workspace"] = json!(true);
    assert!(serde_json::from_value::<ExplicitWorkbenchWorkItem>(serialized).is_err());
}

#[tokio::test]
async fn mismatched_run_scope_is_rejected_before_any_post() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    for field in ["run", "leaf", "contract", "provenance", "duplicate", "kind"] {
        let mut value = pending_run(&work);
        match field {
            "run" => value["graph"]["run_id"] = json!("another-run"),
            "leaf" => value["graph"]["objective_id"] = json!("another-leaf"),
            "contract" => {
                value["graph"]["provenance"]["project_contract_digest"] = json!("another-contract")
            }
            "provenance" => value["graph"]["provenance"]["parent_receipts"] = json!([]),
            "duplicate" => {
                let duplicate = value["graph"]["nodes"][0].clone();
                value["graph"]["nodes"]
                    .as_array_mut()
                    .unwrap()
                    .push(duplicate);
            }
            _ => {
                value["graph"]["nodes"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("kind");
            }
        }
        let (url, server) = scripted_harness(vec![Some((200, value.to_string()))]).await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
        let allow = |_: &ExplicitWorkbenchWorkItem| -> Result<()> { Ok(()) };
        let error = adapter
            .execute_authorized(&work, &allow)
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("identity")
                || error.contains("provenance")
                || error.contains("stages")
                || error.contains("kind"),
            "{error}"
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET "));
        assert!(!root.path().join("data").exists());
    }
}

#[tokio::test]
async fn missing_workspace_receipt_reconciliation_is_get_only() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    let digest = format!("sha256:{}", "c".repeat(64));
    let close = canonical_explicit_close_receipt(&work, &digest).unwrap();
    persist_explicit_close_receipt(root.path(), &work, &close).unwrap();
    let mut value = pending_run(&work);
    for node in value["graph"]["nodes"].as_array_mut().unwrap() {
        node["state"] = json!("succeeded");
        node["output_digest"] = json!(if node["id"] == "close" {
            &close.receipt_digest
        } else {
            &digest
        });
    }
    let (url, server) = scripted_harness(vec![Some((200, value.to_string()))]).await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    assert!(adapter.reconcile(&work).await.unwrap().is_some());
    let requests = server.await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(requests[0].starts_with("GET "));
    assert!(!work.workspace_root.exists());
}

#[tokio::test]
async fn running_retry_inspection_is_get_only_and_preserves_receipt_only_deferral() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    let mut value = pending_run(&work);
    value["graph"]["nodes"][1]["state"] = json!("succeeded");
    value["graph"]["nodes"][2]["state"] = json!("running");
    for retry in [false, true] {
        let (url, server) = scripted_harness(vec![Some((200, value.to_string()))]).await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
        if retry {
            assert!(adapter.reconcile_for_retry(&work).await.unwrap().is_none());
        } else {
            assert!(adapter.reconcile(&work).await.is_err());
        }
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET "));
        assert!(!root.path().join("data").exists());
    }
}

#[tokio::test]
async fn running_retry_inspection_rejects_corruption_before_classifying_incomplete() {
    let root = tempfile::tempdir().unwrap();
    let work = item(root.path());
    for corruption in ["state", "missing", "duplicate", "identity", "provenance"] {
        let mut value = pending_run(&work);
        value["graph"]["nodes"][1]["state"] = json!("succeeded");
        value["graph"]["nodes"][2]["state"] = json!("running");
        match corruption {
            "state" => value["graph"]["nodes"][4]["state"] = json!("invalid"),
            "missing" => {
                value["graph"]["nodes"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                let node = value["graph"]["nodes"][4].clone();
                value["graph"]["nodes"].as_array_mut().unwrap().push(node);
            }
            "identity" => value["graph"]["run_id"] = json!("other"),
            _ => value["graph"]["provenance"]["parent_receipts"] = json!([]),
        }
        let (url, server) = scripted_harness(vec![Some((200, value.to_string()))]).await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
        assert!(
            adapter.reconcile_for_retry(&work).await.is_err(),
            "{corruption}"
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET "));
        assert!(!root.path().join("data").exists());
    }
}

#[test]
fn lexical_and_symlink_escapes_do_not_gain_retained_access() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for id in ["../escape", ".", "..", "a/b", "a\\b", "a?b", "a#b", "a%2fb"] {
        assert!(!safe_run_id(id));
    }
    assert!(workspace(
        root.path(),
        &outside.path().join("missing"),
        Validation::Retained
    )
    .is_err());
    assert!(workspace(
        root.path(),
        &root.path().join("../missing"),
        Validation::Retained
    )
    .is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(outside.path(), root.path().join("escape")).unwrap();
        std::os::unix::fs::symlink(root.path().join("absent"), root.path().join("dangling"))
            .unwrap();
        for path in [
            root.path().join("escape/child"),
            root.path().join("dangling/child"),
        ] {
            assert!(workspace(root.path(), &path, Validation::Fresh).is_err());
            assert!(workspace(root.path(), &path, Validation::Retained).is_err());
        }
    }
}
