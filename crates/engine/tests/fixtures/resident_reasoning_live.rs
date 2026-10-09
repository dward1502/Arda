//! Opt-in real-provider source inspection through the production Harness route.
//! Isolated runtime state; not an installed-daemon or keeper-containment claim.
use super::*;

#[tokio::test]
#[ignore = "uses installed Hermes and its configured real inference provider"]
async fn live_resident_reasoning_source_review() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Workspace must not overlap Hermes owner state (TMPDIR may be inside it).
    let root = tempfile::tempdir_in(repo.join("target/qualification")).unwrap();
    let config = root.path().join("config/adapters/hermes-workbench.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::copy(repo.join("config/adapters/hermes-workbench.toml"), &config).unwrap();
    let source = fs::read(repo.join("crates/engine/src/harness/runs/reasoning.rs")).unwrap();
    fs::write(root.path().join("reasoning.rs"), &source).unwrap();
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(330))
        .build()
        .unwrap();
    attach(&client, bound).await;
    let mut graph = provider_review_graph("run-live-reasoning");
    graph["nodes"][0]["id"] = json!("inspect");
    graph["nodes"][0]["kind"] = json!("inspect");
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:bounded-source-review"]);
    graph["nodes"][0]["timeout_ms"] = json!(300_000);
    client.post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id": PROJECT_ID, "graph": graph, "envelope": envelope("plan-live-reasoning")}))
        .send().await.unwrap().error_for_status().unwrap();
    let response = client.post(format!("http://{bound}/v1/runs/run-live-reasoning/nodes/inspect/execute-provider"))
        .json(&json!({
            "envelope": envelope("execute-live-reasoning"),
            "objective": "Read reasoning.rs, the supplied current Arda integration source. Produce a bounded source-grounded assessment, not implementation: (A) identify how the caller should continue when Mandos reports missing evidence; (B) determine whether a Mandos pass grants permission to publish private evidence. Do not publish, modify files, run commands, read other directories, or request new permission. Use only read_file/search_files. Cite actual source symbols for both answers, correlate the supplied Mandos query_id, and explain the appropriate proceed/revise/evidence_unavailable disposition. Findings are the requested deliverable, not grounds to fail this read-only assessment. Do not claim deployed-runtime acceptance."
        }))
        .send().await.unwrap();
    let status = response.status();
    let body: Value = response.json().await.unwrap();
    // Retain actual output, including failure, without inventing provider data.
    let evidence = repo.join("target/qualification/r08-live-resident-reasoning.json");
    fs::write(&evidence, serde_json::to_vec_pretty(&body).unwrap()).unwrap();
    assert!(status.is_success(), "{status}: {body}");
    assert_eq!(body["receipt"]["status"], "succeeded", "{body}");
    let advice: Value = serde_json::from_slice(
        &fs::read(
            root.path()
                .join("data/runs/run-live-reasoning/reasoning/inspect.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let summary = body["receipt"]["summary"].as_str().unwrap();
    assert!(
        summary.contains(advice["query_id"].as_str().unwrap()),
        "uncorrelated disposition: {summary}"
    );
    assert!(
        summary.contains("inspect") || summary.contains("read"),
        "missing continuation: {summary}"
    );
    assert!(
        summary.contains("permission") || summary.contains("authority"),
        "missing authority distinction: {summary}"
    );
    assert!(
        body["receipt"]["tool_evidence"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| { entry["tool"] == "read_file" && entry["exit_code"] == 0 }),
        "no successful source read: {body}"
    );
    assert_eq!(fs::read(root.path().join("reasoning.rs")).unwrap(), source);
    shutdown.notify_waiters();
    handle.await.unwrap();
}
