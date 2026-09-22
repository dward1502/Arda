use super::*;
use arda_engine::objectives::{
    ControlAction, NewLeaf, NewObjective, ObjectiveStore, ProjectAuthority, RetainedSnapshot,
    SnapshotAdmission,
};
use std::{path::Path, sync::Arc};

struct ReceiptKeeper;
impl SnapshotAdmission for ReceiptKeeper {
    fn prepare(&self, _: &str, _: &Path, _: &str) -> anyhow::Result<RetainedSnapshot> {
        Ok(RetainedSnapshot {
            endpoint: "/nonexistent/receipt-fixture.sock".into(),
            capability: "fixture".into(),
            manifest_digest: "a".repeat(64),
        })
    }
    fn commit(&self, _: &RetainedSnapshot, _: &str, _: i64, _: &str, _: i64) -> anyhow::Result<()> {
        Ok(())
    }
    fn release(&self, _: &RetainedSnapshot, _: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn stored_receipt_replays_after_retained_lease_expiry_and_release() {
    for phase in ["expired", "released"] {
        let root = TempDir::new().unwrap();
        write_file_only_hermes_config(&root);
        let (bound, shutdown, handle) = start_harness(&root).await;
        let client = reqwest::Client::new();
        attach(&client, bound).await;
        let database = root.path().join("data/arda/objectives.sqlite3");
        let objectives = ObjectiveStore::open(&database)
            .unwrap()
            .with_snapshot_admission(Arc::new(ReceiptKeeper));
        objectives
            .create_authenticated_objective(
                NewObjective {
                    id: "receipt-replay".into(),
                    source_id: "receipt-replay".into(),
                    idempotency_key: "receipt-replay".into(),
                    operator_id: "operator".into(),
                    text: "Replay durable receipt".into(),
                    priority: 1,
                    projects: vec![ProjectAuthority {
                        project_id: PROJECT_ID.into(),
                        contract_digest: "sha256:fixture".into(),
                    }],
                    leaves: vec![NewLeaf {
                        id: "leaf".into(),
                        project_id: Some(PROJECT_ID.into()),
                        workspace_root: root.path().to_str().unwrap().into(),
                        authority: "read_only".into(),
                        dependencies: vec![],
                        execution: None,
                    }],
                },
                1,
            )
            .unwrap();
        objectives
            .apply_control(
                "receipt-replay",
                ControlAction::Approve { revision: 1 },
                "approve",
                "operator",
                2,
            )
            .unwrap();
        let claim = objectives
            .claim_runnable("worker", 10, 100, 1)
            .unwrap()
            .remove(0);
        let run_id = claim.execution_run_id.unwrap();
        let mut graph = provider_review_graph(&run_id);
        graph["nodes"][0]["state"] = json!("ready");
        graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
        client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID, "graph": graph, "envelope": envelope("retained-replay-plan")
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
        let planned: Value = client
            .get(format!("http://{bound}/v1/runs/{run_id}"))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap()
            .json()
            .await
            .unwrap();
        let mut receipt = stored_review_receipt(
            &run_id,
            planned["graph"]["provenance"]["project_contract_digest"]
                .as_str()
                .unwrap(),
            vec!["receipt:verify".into()],
            "Replay durable receipt",
        );
        if std::env::var_os("ARDA_TEST_RETAINED_REPLAY_OVERRIDE").is_some() {
            receipt.adapter_version = "retained-test".into();
            receipt.receipt_digest = receipt.computed_digest().unwrap();
            if phase == "released" {
                fs::remove_file(root.path().join("config/adapters/hermes-workbench.toml")).unwrap();
            }
        }
        write_stored_review_receipt(&root, &receipt);
        drop(objectives);
        let objectives = ObjectiveStore::open(&database)
            .unwrap()
            .with_snapshot_admission(Arc::new(ReceiptKeeper));
        if phase == "released" {
            objectives
                .apply_control(
                    "receipt-replay",
                    ControlAction::Cancel,
                    "cancel",
                    "operator",
                    200,
                )
                .unwrap();
            objectives.reconcile_snapshot_commits().unwrap();
        }
        assert!(objectives.retained_execution(&run_id, 300).is_err());
        let receipt_path = root
            .path()
            .join(format!("data/runs/{run_id}/execution-receipts/review.json"));
        let receipt_bytes = fs::read(&receipt_path).unwrap();
        if std::env::var_os("ARDA_TEST_RETAINED_REPLAY_OVERRIDE").is_some() {
            let config = std::path::PathBuf::from(
                std::env::var_os("ARDA_HERMES_RETAINED_ADAPTER_CONFIG").unwrap(),
            );
            let original = fs::read_to_string(&config).unwrap();
            for invalid in [
                None,
                Some(original.replace(
                    "adapter_version = \"retained-test\"",
                    "adapter_version = \"wrong-retained-version\"",
                )),
            ] {
                let expected = if invalid.is_none() {
                    "read receipt validation config"
                } else {
                    "stored provider receipt does not match the configured adapter route"
                };
                match invalid {
                    None => fs::remove_file(&config).unwrap(),
                    Some(text) => fs::write(&config, text).unwrap(),
                }
                let rejected = client.post(format!("http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"))
                    .json(&json!({ "envelope": envelope(&format!("invalid-replay-{phase}")), "objective": "Replay durable receipt" }))
                    .send().await.unwrap();
                assert!(
                    !rejected.status().is_success(),
                    "missing/mismatched retained config fell back to ordinary"
                );
                assert_eq!(fs::read(&receipt_path).unwrap(), receipt_bytes);
                let error: Value = rejected.json().await.unwrap();
                assert_eq!(error["code"], "conflict");
                let message = error["message"].as_str().unwrap();
                assert!(
                    message.contains("stored provider receipt failed current authority binding")
                        && message.contains(expected),
                    "{message}"
                );
            }
            fs::write(config, original).unwrap();
        }
        let response = client.post(format!("http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"))
            .json(&json!({ "envelope": envelope(&format!("replay-{phase}")), "objective": "Replay durable receipt" }))
            .send().await.unwrap();
        let status = response.status();
        let body = response.text().await.unwrap();
        assert!(status.is_success(), "{phase}: {status}: {body}");
        assert_eq!(fs::read(&receipt_path).unwrap(), receipt_bytes);
        shutdown.notify_waiters();
        handle.await.unwrap();
    }
    println!("retained-replay-child-completed");
}

#[test]
fn retained_replay_uses_override_after_expiry_and_release() {
    let root = TempDir::new().unwrap();
    write_file_only_hermes_config(&root);
    let config = root.path().join("config/adapters/hermes-workbench.toml");
    let raw = fs::read_to_string(&config).unwrap().replace(
        "adapter_version = \"1\"",
        "adapter_version = \"retained-test\"",
    );
    fs::write(&config, raw).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "retained_replay::stored_receipt_replays_after_retained_lease_expiry_and_release",
            "--nocapture",
        ])
        .env("ARDA_TEST_RETAINED_REPLAY_OVERRIDE", "1")
        .env_remove("ARDA_HERMES_ADAPTER_CONFIG")
        .env("ARDA_HERMES_RETAINED_ADAPTER_CONFIG", &config)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("retained-replay-child-completed"));
}
