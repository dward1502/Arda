//! Scripted HTTP fixtures: these are not live recovery acceptance evidence.
use super::*;

struct RecoveryAuthority(ExplicitRecoveryWindow);
impl ExplicitWorkspaceAuthorization for RecoveryAuthority {
    fn authorize(&self, _: &ExplicitWorkbenchWorkItem) -> Result<()> {
        Ok(())
    }
    fn provider_lease(&self, _: &ExplicitWorkbenchWorkItem) -> Result<Value> {
        Ok(
            json!({"generation": 3, "owner": "fixture", "run_id": "fixture", "expires_ms": self.0.expires_at_unix_ms}),
        )
    }
    fn recovery_window(
        &self,
        _: &ExplicitWorkbenchWorkItem,
    ) -> Result<Option<ExplicitRecoveryWindow>> {
        Ok(Some(self.0.clone()))
    }
}

fn recovery_fixture(root: &Path) -> (ExplicitWorkbenchWorkItem, RecoveryAuthority) {
    use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
    let mut work = item(root);
    work.read_only = true;
    let now = Utc::now().timestamp_millis() as u64;
    let generated = now - 120_000;
    let memory = MnemosyneService::new(root.join("data/vaire"))
        .unwrap()
        .with_contract_memory_root(root.join("core/state/memory"));
    let context = serde_json::from_value(json!({
        "schema_version": arda_vaire::OrganismContext::SCHEMA_VERSION,
        "organism_id": "arda:fixture", "generated_at_unix_ms": generated,
        "expires_at_unix_ms": generated + 60_000,
        "consumer": {"consumer_id": "fixture", "role": "worker", "authority_ceiling": "read_only",
            "operator_authorized": false, "memory_domains": ["system"], "data_classes": ["internal"],
            "permitted_egress": ["local_device"], "compute_node_refs": ["local"], "agent_ref": null},
        "lineage": {"objective_id": work.objective_id, "project_id": null,
            "run_id": work.run_id, "task_id": work.leaf_id, "session_ref": null,
            "parent_receipts": [work.objective_plan_receipt]},
        "objective": {"requested_outcome": work.execution_prompt, "acceptance_conditions": ["cited inspection"],
            "required_capabilities": ["file"], "forbidden_capabilities": ["terminal"]},
        "evidence_refs": [], "memory_refs": [], "unresolved_failures": [],
        "return_contract": {"schema_version": "arda.organism-outcome.v1",
            "required_receipt_types": ["arda.context-use-receipt.v1"], "max_output_bytes": 4096}
    })).unwrap();
    let mut consumer = ConsumerContext::new("fixture", vec![MemoryDomain::System]);
    consumer.purpose = Some(work.execution_prompt.clone());
    work.context_assembly = Some(
        memory
            .assemble_organism_context(context, &consumer, generated.into())
            .unwrap(),
    );
    (
        work,
        RecoveryAuthority(ExplicitRecoveryWindow {
            event_id: "synthetic-saved-admission".into(),
            activated_at_unix_ms: now,
            expires_at_unix_ms: now + 30 * 60_000,
        }),
    )
}

fn recovery_run(work: &ExplicitWorkbenchWorkItem, completed: &[&str]) -> Value {
    let mut run = bind_test_run(work, json!({"graph": explicit_run_graph(work, "approved")}));
    for node in run["graph"]["nodes"].as_array_mut().unwrap() {
        let id = node["id"].as_str().unwrap().to_owned();
        if completed.contains(&id.as_str()) {
            node["state"] = json!("succeeded");
            node["output_digest"] = json!(format!(
                "sha256:{}",
                id.chars().next().unwrap().to_string().repeat(64)
            ));
        }
    }
    run
}

#[tokio::test]
async fn recovery_never_plans_approves_or_reexecutes() {
    for case in ["missing", "approval", "execute"] {
        let root = tempfile::tempdir().unwrap();
        let (work, guard) = recovery_fixture(root.path());
        let (status, body, expected) = match case {
            "missing" => (404, "{}".into(), "cannot plan"),
            "approval" => (200, recovery_run(&work, &[]).to_string(), "cannot approve"),
            _ => (
                200,
                recovery_run(&work, &["approval"]).to_string(),
                "preserved successful execution",
            ),
        };
        let (url, requests) = scripted_harness(vec![Some((status, body))]).await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
        let error = adapter
            .execute_authorized(&work, &guard, &runtime_admission::FixtureAdmission)
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{case}: {error:#}");
        let requests = requests.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET "));
    }
}

#[tokio::test]
async fn recovery_rejects_missing_context_writes_and_invalid_window_before_network() {
    for case in ["context", "write", "expired", "extended", "future", "event"] {
        let root = tempfile::tempdir().unwrap();
        let (mut work, mut guard) = recovery_fixture(root.path());
        let expected = match case {
            "context" => {
                work.context_assembly = None;
                "requires retained context"
            }
            "write" => {
                work.read_only = false;
                "requires read-only scope"
            }
            "expired" => {
                guard.0.activated_at_unix_ms -= 30 * 60_000;
                guard.0.expires_at_unix_ms -= 30 * 60_000;
                "window is invalid or expired"
            }
            "extended" => {
                guard.0.expires_at_unix_ms += 1;
                "window is invalid or expired"
            }
            "future" => {
                guard.0.activated_at_unix_ms += 60_000;
                guard.0.expires_at_unix_ms += 60_000;
                "window is invalid or expired"
            }
            _ => {
                guard.0.event_id.clear();
                "window is invalid or expired"
            }
        };
        let adapter =
            WorkbenchExecutionAdapter::with_harness_url(root.path(), "http://127.0.0.1:9").unwrap();
        let error = adapter
            .execute_authorized(&work, &guard, &runtime_admission::FixtureAdmission)
            .await
            .unwrap_err();
        assert!(error.to_string().contains(expected), "{case}: {error:#}");
    }
}

#[tokio::test]
async fn recovery_resumes_verify_review_and_provider_free_close_only() {
    let root = tempfile::tempdir().unwrap();
    let (work, guard) = recovery_fixture(root.path());
    let before = work.context_assembly.clone().unwrap();
    let stages = ["approval", "execute", "verify", "review", "close"];
    let provider = |n: usize, stage: &str| {
        json!({
            "receipt": {"status": "succeeded", "receipt_digest": format!("sha256:{}", stage.chars().next().unwrap().to_string().repeat(64)), "summary": "synthetic fixture"},
            "run": recovery_run(&work, &stages[..n])
        })
    };
    let mut closed = recovery_run(&work, &stages);
    closed["graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "close")
        .unwrap()["output_digest"] = json!("__REQUEST_RECEIPT_DIGEST__");
    let (url, requests) = scripted_harness(vec![
        Some((200, recovery_run(&work, &stages[..2]).to_string())),
        Some((200, provider(3, "verify").to_string())),
        Some((200, provider(4, "review").to_string())),
        Some((200, closed.to_string())),
    ])
    .await;
    let adapter = WorkbenchExecutionAdapter::with_harness_url(root.path(), url).unwrap();
    let outcome = adapter
        .execute_authorized(&work, &guard, &runtime_admission::FixtureAdmission)
        .await
        .unwrap();
    assert_eq!(outcome.status, "succeeded");
    let requests = requests.await.unwrap();
    assert_eq!(requests.len(), 4);
    for (request, endpoint) in requests[1..].iter().zip([
        "/nodes/verify/execute-provider",
        "/nodes/review/execute-provider",
        "/nodes/close/complete",
    ]) {
        assert!(request.contains(endpoint));
        let body: Value = serde_json::from_str(request.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(body["recovery_event_id"], guard.0.event_id);
        assert_eq!(body["expected_retained_lease"]["generation"], 3);
    }
    assert_eq!(work.context_assembly.unwrap(), before);
    assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
}
