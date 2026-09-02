use arda_engine::adapters::{
    CostMeasurement, HermesExecutionReceipt, HermesNodeTask, HermesReceiptStatus,
    HermesToolEvidence, NormalizedHermesUsage,
};
use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use serde_json::{json, Value};
use std::fs;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

async fn start_harness(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        child_pids: Arc::new(RwLock::new(Vec::new())),
        service_names: Arc::new(Vec::new()),
        service_statuses: Arc::new(RwLock::new(Vec::new())),
        manwe_url: "http://127.0.0.1:1".into(),
        client: reqwest::Client::new(),
        manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
        manwe_proxy_bearer: None,
        warden_scout_url: None,
        warden_scout_timeout: DEFAULT_WARDEN_SCOUT_TIMEOUT,
        presence_inputs: HarnessPresenceState::default(),
        workbench_root: root.path().to_path_buf(),
        operator_id: "operator-0".to_string(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().expect("loopback address")),
        state,
        shutdown.clone(),
    )
    .await
    .expect("start harness");
    (bound, shutdown, handle)
}

fn envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-runs-1",
            "approval_id": "approval-runs-1",
            "ledger_writes": ["test-ledger.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-07-31T00:00:00Z"
        },
        "idempotency_key": key
    })
}

fn receipt_digest(node_id: &str) -> String {
    let marker = match node_id {
        "execute" => 'e',
        "verify" => 'f',
        "verify-recovered" => 'b',
        "review" => 'a',
        "close" => 'c',
        "different-close" => 'd',
        _ => panic!("unexpected receipt fixture {node_id}"),
    };
    format!("sha256:{}", marker.to_string().repeat(64))
}

fn contract() -> Value {
    serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .expect("project fixture")
}

fn graph(run_id: &str, node_id: &str, kind: &str) -> Value {
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [{
            "id": node_id,
            "kind": kind,
            "state": "pending",
            "authority": if kind == "approval" { "human_approval" } else { "read_only" },
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": [],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        }],
        "edges": [],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "integration-test",
            "parent_receipts": []
        }
    })
}

fn provider_review_graph(run_id: &str) -> Value {
    let mut graph = graph(run_id, "review", "review");
    graph["nodes"][0]["worker"] = json!({
        "role": "security_privacy_critic",
        "worker_id": "critic-review-0",
        "route_id": "hosted:hermes-workbench",
        "route_class": "hosted",
        "prompt_digest": format!("sha256:{}", "1".repeat(64)),
        "allowed_toolsets": ["file"],
        "dependencies": [],
        "deadline_unix_ms": 4_000_000_000_000_u64,
        "output_contract": "arda.hermes-job-result.v1",
        "evidence_policy": "worker_report"
    });
    graph
}

fn stored_review_receipt(
    run_id: &str,
    project_contract_digest: &str,
    parent_receipts: Vec<String>,
    objective: &str,
) -> HermesExecutionReceipt {
    let mut node: arda_core::run_graph::RunNode =
        serde_json::from_value(provider_review_graph(run_id)["nodes"][0].clone())
            .expect("provider review node");
    node.parent_receipts = parent_receipts.clone();
    let task = HermesNodeTask {
        run_id: arda_core::run_graph::RunId::new(run_id).expect("run id"),
        node,
        objective: objective.into(),
        instructions: "Work only inside the attached project root. Do not commit or modify project files. Independently inspect the implementation and durable verification evidence without rerunning the declared checks, and report named defects. For an intermediate run-graph node, judge only this node's objective and evidence; do not require downstream whole-objective deliverables such as synthesis, repair backlogs, operator outcomes, or joined closure. Fail rather than approve unsupported completion. For read-only source evidence, exported tool output digests authenticate the actual calls and must not equal source content digests because they hash different envelopes. Treat absence of mutating tool calls under read-only authority as the no-modification evidence. Require a context_use_receipt only when supplied by the governed capsule. Declared checks already covered by the verification receipt: test: cargo test -p arda-core".into(),
        checks: Vec::new(),
        check_commands: Default::default(),
        project_contract_digest: project_contract_digest.into(),
        context_assembly: None,
    };
    let mut receipt = HermesExecutionReceipt {
        schema_version: "arda.execution-receipt.v3".into(),
        receipt_digest: String::new(),
        authority_binding_digest: task
            .authority_binding_digest()
            .expect("authority binding digest"),
        run_id: run_id.into(),
        node_id: "review".into(),
        idempotency_key: format!("node-{run_id}"),
        status: HermesReceiptStatus::Succeeded,
        summary: "Stored independent review completed.".into(),
        tool_evidence: vec![HermesToolEvidence {
            tool: "read_file".into(),
            action: "inspect".into(),
            exit_code: Some(0),
            output_digest: format!("sha256:{}", "7".repeat(64)),
        }],
        test_evidence: Vec::new(),
        artifacts: Vec::new(),
        usage: NormalizedHermesUsage {
            provider: Some("nous".into()),
            model: Some("fixture-model".into()),
            api_calls: 1,
            input_tokens: 10,
            output_tokens: 10,
            total_tokens: 20,
            estimated_cost_usd: 0.0,
            cost_measurement: CostMeasurement::Observed,
            completed: true,
            failed: false,
        },
        adapter: "hermes-workbench".into(),
        adapter_version: "1".into(),
        project_contract_digest: project_contract_digest.into(),
        parent_receipts,
        context_capsule_id: None,
        context_capsule_digest: None,
        context_use_receipt_ref: None,
        context_handoff: None,
        recorded_at_unix_ms: 1,
    };
    receipt.receipt_digest = receipt.computed_digest().expect("receipt digest");
    receipt
}

fn write_stored_review_receipt(root: &TempDir, receipt: &HermesExecutionReceipt) {
    let receipt_path = root.path().join(format!(
        "data/runs/{}/execution-receipts/review.json",
        receipt.run_id
    ));
    fs::create_dir_all(receipt_path.parent().expect("receipt directory"))
        .expect("create receipt directory");
    fs::write(
        receipt_path,
        serde_json::to_vec_pretty(receipt).expect("receipt json"),
    )
    .expect("write stored receipt");
}

fn write_file_only_hermes_config(root: &TempDir) {
    let config_dir = root.path().join("config/adapters");
    fs::create_dir_all(&config_dir).expect("adapter config directory");
    fs::write(
        config_dir.join("hermes-workbench.toml"),
        r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "1"
executable = "/bin/true"
max_timeout_ms = 1000
cancellation_grace_ms = 100
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 65536
inherit_environment = ["PATH"]

[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
    )
    .expect("adapter config");
}

fn completion_graph(run_id: &str) -> Value {
    let node = |id: &str, kind: &str, authority: &str, receipts: Vec<&str>| {
        json!({
            "id": id,
            "kind": kind,
            "state": "pending",
            "authority": authority,
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 1000,
            "idempotency_key": format!("node-{run_id}-{id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": receipts,
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
        })
    };
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": format!("objective-{run_id}"),
        "nodes": [
            node("plan", "plan", "read_only", vec![]),
            node("approval", "approval", "human_approval", vec!["receipt:plan"]),
            node("execute", "execute", "execute_with_approval", vec!["receipt:approval"]),
            node("verify", "verify", "verify", vec!["receipt:execute"]),
            node("review", "review", "verify", vec!["receipt:verify"]),
            node("close", "close", "read_only", vec!["receipt:review"])
        ],
        "edges": [
            {"id": "plan-approval", "from": "plan", "to": "approval", "parent_receipt": "receipt:plan"},
            {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": "receipt:approval"},
            {"id": "execute-verify", "from": "execute", "to": "verify", "parent_receipt": "receipt:execute"},
            {"id": "verify-review", "from": "verify", "to": "review", "parent_receipt": "receipt:verify"},
            {"id": "review-close", "from": "review", "to": "close", "parent_receipt": "receipt:review"}
        ],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "integration-test",
            "parent_receipts": []
        }
    })
}

async fn attach(client: &reqwest::Client, bound: std::net::SocketAddr) {
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope("attach-for-runs")}))
        .send()
        .await
        .expect("attach request")
        .error_for_status()
        .expect("attach status");
}

async fn plan(
    client: &reqwest::Client,
    bound: std::net::SocketAddr,
    run_id: &str,
    node_id: &str,
    kind: &str,
) -> reqwest::Response {
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph(run_id, node_id, kind),
            "envelope": envelope(&format!("plan-{run_id}"))
        }))
        .send()
        .await
        .expect("plan request")
}

#[tokio::test]
async fn plan_rejects_a_stale_expected_project_contract_digest() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let response = client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "expected_project_contract_digest": format!("sha256:{}", "0".repeat(64)),
            "graph": graph("run-stale-project-contract", "plan", "plan"),
            "envelope": envelope("plan-stale-project-contract")
        }))
        .send()
        .await
        .expect("plan request");

    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn provider_contract_replacement_is_rejected_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let planned = client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-provider-replacement"),
            "envelope": envelope("provider-replacement-plan")
        }))
        .send()
        .await
        .expect("plan request");
    assert_eq!(planned.status(), reqwest::StatusCode::CREATED);

    let approved = client
        .post(format!(
            "http://{bound}/v1/runs/run-provider-replacement/approve"
        ))
        .json(&json!({
            "node_id": "approval",
            "envelope": envelope("provider-replacement-approval")
        }))
        .send()
        .await
        .expect("approval request");
    assert_eq!(approved.status(), reqwest::StatusCode::OK);

    let journal_path = root
        .path()
        .join("data/runs/run-provider-replacement/events.jsonl");
    let before = fs::read(&journal_path).expect("journal before replacement");
    let registry_path = root.path().join("data/workbench/projects.json");
    let mut registry: Value = serde_json::from_slice(
        &fs::read(&registry_path).expect("project registry before replacement"),
    )
    .expect("registry json");
    registry["projects"][0]["contract"]["workspace"]["root"] =
        Value::String("other-workspace".into());
    fs::create_dir(root.path().join("other-workspace")).expect("replacement workspace");
    fs::write(
        &registry_path,
        serde_json::to_vec_pretty(&registry).expect("replacement registry json"),
    )
    .expect("replace project registry");

    let execute = client
        .post(format!(
            "http://{bound}/v1/runs/run-provider-replacement/nodes/execute/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("provider-replacement-execute"),
            "objective": "must not execute after contract replacement"
        }))
        .send()
        .await
        .expect("execute request");
    assert_eq!(execute.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replacement"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_revalidates_contract_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-contract";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-contract")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");

    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "must not replay after contract replacement",
    );
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replacement");
    let registry_path = root.path().join("data/workbench/projects.json");
    let mut registry: Value = serde_json::from_slice(
        &fs::read(&registry_path).expect("project registry before replacement"),
    )
    .expect("registry json");
    registry["projects"][0]["contract"]["workspace"]["root"] =
        Value::String("other-workspace".into());
    fs::create_dir(root.path().join("other-workspace")).expect("replacement workspace");
    fs::write(
        &registry_path,
        serde_json::to_vec_pretty(&registry).expect("replacement registry json"),
    )
    .expect("replace project registry");

    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-contract"),
            "objective": "must not replay after contract replacement"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replacement"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_revalidates_parent_lineage_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    let config_dir = root.path().join("config/adapters");
    fs::create_dir_all(&config_dir).expect("adapter config directory");
    fs::write(
        config_dir.join("hermes-workbench.toml"),
        r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "1"
executable = "/bin/true"
max_timeout_ms = 1000
cancellation_grace_ms = 100
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 65536
inherit_environment = ["PATH"]

[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
    )
    .expect("adapter config");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-parent";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-parent")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:other-verification".into()],
        "must not replay with stale parent lineage",
    );
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replay");
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-parent"),
            "objective": "must not replay with stale parent lineage"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_revalidates_toolsets_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    let config_dir = root.path().join("config/adapters");
    fs::create_dir_all(&config_dir).expect("adapter config directory");
    fs::write(
        config_dir.join("hermes-workbench.toml"),
        r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "1"
executable = "/bin/true"
max_timeout_ms = 1000
cancellation_grace_ms = 100
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 65536
inherit_environment = ["PATH"]

[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
    )
    .expect("adapter config");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-toolset";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["worker"]["allowed_toolsets"] = json!(["file", "terminal"]);
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-toolset")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "must not replay after critic authority broadens",
    );
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replay");
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-toolset"),
            "objective": "must not replay after critic authority broadens"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_rejects_omitted_context_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    write_file_only_hermes_config(&root);
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-context-omission";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-context-omission")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let mut receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "must not replay when required context is omitted",
    );
    receipt.context_capsule_id = Some("capsule:stored-review".into());
    receipt.context_capsule_digest = Some(format!("sha256:{}", "8".repeat(64)));
    receipt.context_use_receipt_ref = Some("context-use:stored-review".into());
    receipt.receipt_digest = receipt.computed_digest().expect("receipt digest");
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replay");
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-context-omission"),
            "objective": "must not replay when required context is omitted"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_rejects_adapter_route_drift_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    write_file_only_hermes_config(&root);
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-adapter-drift";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-adapter-drift")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let mut receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "must not replay through an unadmitted adapter route",
    );
    receipt.adapter = "unadmitted-adapter".into();
    receipt.usage.provider = Some("unadmitted-provider".into());
    receipt.usage.model = Some("unadmitted-model".into());
    receipt.receipt_digest = receipt.computed_digest().expect("receipt digest");
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replay");
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-adapter-drift"),
            "objective": "must not replay through an unadmitted adapter route"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_rejects_cross_run_identity_without_journal_mutation() {
    let root = TempDir::new().expect("temp root");
    write_file_only_hermes_config(&root);
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-cross-run";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-cross-run")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let mut receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "must not replay a receipt issued for another run",
    );
    receipt.run_id = "run-from-another-authority".into();
    receipt.receipt_digest = receipt.computed_digest().expect("receipt digest");
    let receipt_path = root
        .path()
        .join(format!("data/runs/{run_id}/execution-receipts/review.json"));
    fs::create_dir_all(receipt_path.parent().expect("receipt directory"))
        .expect("create receipt directory");
    fs::write(
        receipt_path,
        serde_json::to_vec_pretty(&receipt).expect("receipt json"),
    )
    .expect("write cross-run stored receipt");

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let before = fs::read(&journal_path).expect("journal before replay");
    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-cross-run"),
            "objective": "must not replay a receipt issued for another run"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_rejects_worker_authority_drift_without_journal_mutation() {
    let root = TempDir::new().expect("tempdir");
    write_file_only_hermes_config(&root);
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-worker-drift";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    graph["nodes"][0]["worker"]["worker_id"] = json!("critic-review-replacement");
    graph["nodes"][0]["worker"]["prompt_digest"] = json!(format!("sha256:{}", "9".repeat(64)));
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-worker-drift")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan response");

    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "Resume the independent review under changed worker authority.",
    );
    write_stored_review_receipt(&root, &receipt);
    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let journal_before = fs::read(&journal_path).expect("journal before replay");

    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-worker-drift"),
            "objective": "Resume the independent review under changed worker authority."
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after replay"),
        journal_before
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn stored_provider_receipt_rejects_objective_drift_then_replays_once() {
    let root = TempDir::new().expect("tempdir");
    write_file_only_hermes_config(&root);
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let run_id = "run-stored-receipt-current-authority";
    let mut graph = provider_review_graph(run_id);
    graph["nodes"][0]["state"] = json!("ready");
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);
    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-stored-receipt-current-authority")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan response");

    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let receipt = stored_review_receipt(
        run_id,
        planned["graph"]["provenance"]["project_contract_digest"]
            .as_str()
            .expect("planned project contract digest"),
        vec!["receipt:verify".into()],
        "Resume the independent review with current authority.",
    );
    write_stored_review_receipt(&root, &receipt);

    let journal_path = root.path().join(format!("data/runs/{run_id}/events.jsonl"));
    let journal_before_drift = fs::read(&journal_path).expect("journal before objective drift");
    let drift = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("reject-stored-receipt-objective-drift"),
            "objective": "A substituted objective must not reuse this receipt."
        }))
        .send()
        .await
        .expect("objective drift request");
    assert_eq!(drift.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after objective drift"),
        journal_before_drift
    );

    let response = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-current-authority"),
            "objective": "Resume the independent review with current authority."
        }))
        .send()
        .await
        .expect("execute provider request");
    let status = response.status();
    let response_text = response.text().await.expect("provider response text");
    assert_eq!(status, reqwest::StatusCode::OK, "{response_text}");
    let body: serde_json::Value = serde_json::from_str(&response_text).expect("provider response");
    assert_eq!(body["run"]["graph"]["nodes"][0]["state"], "succeeded");
    assert_eq!(body["receipt"]["receipt_digest"], receipt.receipt_digest);

    let journal_after_completion = fs::read(&journal_path).expect("completed journal");
    let replay = client
        .post(format!(
            "http://{bound}/v1/runs/{run_id}/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("replay-stored-receipt-current-authority"),
            "objective": "Resume the independent review with current authority."
        }))
        .send()
        .await
        .expect("idempotent replay");
    assert_eq!(replay.status(), reqwest::StatusCode::CONFLICT);
    assert_eq!(
        fs::read(&journal_path).expect("journal after duplicate replay"),
        journal_after_completion
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn run_routes_plan_approve_read_and_expose_canonical_events() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    let planned = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(planned.status(), 201);
    let retry = plan(&client, bound, "run-approve", "approval-node", "approval").await;
    assert_eq!(retry.status(), 200);

    let approved: Value = client
        .post(format!("http://{bound}/v1/runs/run-approve/approve"))
        .json(&json!({
            "node_id": "approval-node",
            "envelope": envelope("approve-run-approve")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status")
        .json()
        .await
        .expect("approve body");
    assert_eq!(approved["graph"]["nodes"][0]["state"], "succeeded");

    let run: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve"))
        .send()
        .await
        .expect("get run")
        .error_for_status()
        .expect("get run status")
        .json()
        .await
        .expect("get run body");
    assert_eq!(run["graph"]["run_id"], "run-approve");

    let events: Value = client
        .get(format!("http://{bound}/v1/runs/run-approve/events"))
        .send()
        .await
        .expect("get events")
        .error_for_status()
        .expect("events status")
        .json()
        .await
        .expect("events body");
    assert!(events["events"]
        .as_array()
        .is_some_and(|events| events.len() >= 4));
    assert!(root
        .path()
        .join("data/runs/run-approve/events.jsonl")
        .exists());

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn run_event_stream_is_sse_and_emits_canonical_events() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    plan(&client, bound, "run-stream", "plan-node", "plan")
        .await
        .error_for_status()
        .expect("plan status");

    let projection_started = std::time::Instant::now();
    let mut response = client
        .get(format!("http://{bound}/v1/runs/run-stream/events/stream"))
        .send()
        .await
        .expect("stream request")
        .error_for_status()
        .expect("stream status");
    assert_eq!(
        response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok()),
        Some("text/event-stream")
    );
    let frame = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let mut body = Vec::new();
        while !body.windows(2).any(|window| window == b"\n\n") {
            body.extend_from_slice(
                &response
                    .chunk()
                    .await
                    .expect("stream chunk")
                    .expect("open stream"),
            );
        }
        body
    })
    .await
    .expect("stream event timeout");
    assert!(
        projection_started.elapsed() < std::time::Duration::from_secs(1),
        "event projection exceeded the 1s U3 budget"
    );
    let text = String::from_utf8(frame).expect("utf-8 SSE frame");
    assert!(text.contains("event: run_event"));
    assert!(text.contains("\"schema_version\":\"arda.run-event.v1\""));
    assert!(text.contains("\"run_id\":\"run-stream\""));

    drop(response);
    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn cancel_is_idempotent_and_mutations_require_typed_envelopes() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    assert_eq!(
        plan(&client, bound, "run-cancel", "approval-node", "approval")
            .await
            .status(),
        201
    );

    let missing_envelope = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&json!({"reason": "operator requested"}))
        .send()
        .await
        .expect("missing-envelope request");
    assert_eq!(missing_envelope.status(), 422);

    let cancel_body = json!({
        "reason": "operator requested",
        "envelope": envelope("cancel-run-cancel")
    });
    let cancelled: Value = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel request")
        .error_for_status()
        .expect("cancel status")
        .json()
        .await
        .expect("cancel body");
    assert_eq!(cancelled["graph"]["nodes"][0]["state"], "cancelled");

    let retry = client
        .post(format!("http://{bound}/v1/runs/run-cancel/cancel"))
        .json(&cancel_body)
        .send()
        .await
        .expect("cancel retry");
    assert_eq!(retry.status(), 200);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn operator_rejection_is_durable_and_cannot_authorize_execution() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    assert_eq!(
        plan(&client, bound, "run-rejected", "approval-node", "approval")
            .await
            .status(),
        201
    );

    let rejected: Value = client
        .post(format!("http://{bound}/v1/runs/run-rejected/cancel"))
        .json(&json!({
            "reason": "approval rejected; revise objective before replanning",
            "envelope": envelope("reject-run-rejected")
        }))
        .send()
        .await
        .expect("rejection request")
        .error_for_status()
        .expect("rejection status")
        .json()
        .await
        .expect("rejection body");
    assert!(rejected["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|node| node["state"] == "cancelled"));
    assert!(rejected["events"].as_array().unwrap().iter().any(|event| {
        event["kind"]["type"] == "cancelled"
            && event["kind"]["reason"] == "approval rejected; revise objective before replanning"
    }));

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn operator_receipt_cannot_complete_provider_owned_review() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": provider_review_graph("run-provider-review"),
            "envelope": envelope("plan-provider-review")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");

    let response = client
        .post(format!(
            "http://{bound}/v1/runs/run-provider-review/nodes/review/complete"
        ))
        .json(&json!({
            "envelope": envelope("bypass-provider-review"),
            "receipt_digest": receipt_digest("review")
        }))
        .send()
        .await
        .expect("operator completion request");

    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);
    let body: Value = response.json().await.expect("conflict body");
    assert!(body.to_string().contains("provider execution"));

    let run: Value = client
        .get(format!("http://{bound}/v1/runs/run-provider-review"))
        .send()
        .await
        .expect("get run")
        .error_for_status()
        .expect("get run status")
        .json()
        .await
        .expect("get run body");
    assert_eq!(run["graph"]["nodes"][0]["state"], "pending");

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn provider_review_toolset_escalation_is_rejected_without_state_mutation() {
    let root = TempDir::new().expect("temp root");
    let config_dir = root.path().join("config/adapters");
    fs::create_dir_all(&config_dir).expect("adapter config directory");
    fs::write(
        config_dir.join("hermes-workbench.toml"),
        r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "1"
executable = "/bin/true"
max_timeout_ms = 1000
cancellation_grace_ms = 100
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 65536
inherit_environment = ["PATH"]

[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
    )
    .expect("adapter config");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;
    let mut graph = provider_review_graph("run-review-escalation");
    graph["nodes"][0]["worker"]["allowed_toolsets"] = json!(["file", "terminal"]);
    graph["nodes"][0]["parent_receipts"] = json!(["receipt:verify"]);

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": graph,
            "envelope": envelope("plan-review-escalation")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");

    let response = client
        .post(format!(
            "http://{bound}/v1/runs/run-review-escalation/nodes/review/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("execute-review-escalation"),
            "objective": "independently review the verified change"
        }))
        .send()
        .await
        .expect("execute provider request");
    assert_eq!(response.status(), reqwest::StatusCode::CONFLICT);

    let run: Value = client
        .get(format!("http://{bound}/v1/runs/run-review-escalation"))
        .send()
        .await
        .expect("get run")
        .error_for_status()
        .expect("get run status")
        .json()
        .await
        .expect("get run body");
    assert_eq!(run["graph"]["nodes"][0]["state"], "pending");
    assert_eq!(run["graph"]["nodes"][0]["checkpoint"]["sequence"], 0);

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");
}

#[tokio::test]
async fn operator_receipts_complete_execute_verify_review_and_close_in_order() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-complete"),
            "envelope": envelope("plan-run-complete")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    let planned: Value = client
        .get(format!("http://{bound}/v1/runs/run-complete"))
        .send()
        .await
        .expect("get planned run")
        .error_for_status()
        .expect("get planned run status")
        .json()
        .await
        .expect("get planned run body");
    let plan_node = planned["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "plan"))
        .expect("plan node");
    assert_eq!(plan_node["state"], "succeeded");
    assert_eq!(plan_node["output_digest"], "receipt:plan");
    assert!(plan_node["checkpoint"]["sequence"].as_u64().unwrap_or(0) > 0);
    assert!(plan_node["checkpoint"]["recovery_token"].is_string());
    assert!(plan_node["checkpoint"]["checkpoint_digest"].is_string());
    client
        .post(format!("http://{bound}/v1/runs/run-complete/approve"))
        .json(&json!({
            "node_id": "approval",
            "envelope": envelope("approve-run-complete")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    let malformed_digest = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("reject-malformed-digest"),
            "receipt_digest": "sha256:not-a-digest"
        }))
        .send()
        .await
        .expect("malformed digest request");
    assert_eq!(malformed_digest.status(), 400);

    let traversal_evidence = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("reject-traversal-evidence"),
            "receipt_digest": receipt_digest("execute"),
            "evidence": {
                "changes": [{
                    "path": "../outside",
                    "status": "modified",
                    "additions": 1,
                    "deletions": 0
                }]
            }
        }))
        .send()
        .await
        .expect("traversal evidence request");
    assert_eq!(traversal_evidence.status(), 400);

    for node_id in ["execute", "verify", "review", "close"] {
        let digest = receipt_digest(node_id);
        let response: Value = client
            .post(format!(
                "http://{bound}/v1/runs/run-complete/nodes/{node_id}/complete"
            ))
            .json(&json!({
                "envelope": envelope(&format!("complete-{node_id}")),
                "receipt_digest": digest,
                "evidence": match node_id {
                    "execute" => json!({
                        "changes": [{
                            "path": "src/lib.rs",
                            "status": "modified",
                            "additions": 2,
                            "deletions": 2,
                            "diff": "-hello\n+hello, Arda"
                        }],
                        "provider_receipt": {
                            "provider": "nous",
                            "model": "fixture-model",
                            "adapter": "hermes-workbench",
                            "receipt_digest": receipt_digest("execute"),
                            "summary": "Bounded fixture mutation completed."
                        }
                    }),
                    "verify" => json!({
                        "tests": [{
                            "name": "cargo test --quiet",
                            "status": "passed",
                            "duration_ms": 12,
                            "details": "exit 0"
                        }]
                    }),
                    _ => Value::Null,
                }
            }))
            .send()
            .await
            .expect("complete request")
            .error_for_status()
            .expect("complete status")
            .json()
            .await
            .expect("complete body");
        let node = response["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["id"] == node_id))
            .expect("completed node");
        assert_eq!(node["state"], "succeeded");
        assert_eq!(node["output_digest"], receipt_digest(node_id));
        assert!(node["checkpoint"]["sequence"].as_u64().unwrap_or(0) > 0);
        assert!(node["checkpoint"]["recovery_token"].is_string());
        assert!(node["checkpoint"]["checkpoint_digest"].is_string());
    }

    let recovered: Value = client
        .get(format!("http://{bound}/v1/runs/run-complete"))
        .send()
        .await
        .expect("get completed run")
        .error_for_status()
        .expect("get completed run status")
        .json()
        .await
        .expect("get completed run body");
    assert!(recovered["graph"]["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().all(|node| node["state"] == "succeeded")));
    let recovered_nodes = recovered["graph"]["nodes"].as_array().expect("nodes");
    for edge in recovered["graph"]["edges"].as_array().expect("edges") {
        let parent = recovered_nodes
            .iter()
            .find(|node| node["id"] == edge["from"])
            .expect("edge parent");
        let child = recovered_nodes
            .iter()
            .find(|node| node["id"] == edge["to"])
            .expect("edge child");
        assert_eq!(edge["parent_receipt"], parent["output_digest"]);
        assert!(child["parent_receipts"]
            .as_array()
            .is_some_and(|receipts| receipts.contains(&edge["parent_receipt"])));
    }
    assert_eq!(recovered["review"]["changes"][0]["path"], "src/lib.rs");
    assert_eq!(recovered["review"]["tests"][0]["status"], "passed");
    assert_eq!(recovered["review"]["provider_receipt"]["provider"], "nous");

    let evidence_changing_retry = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/close/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-close"),
            "receipt_digest": receipt_digest("close"),
            "evidence": {
                "changes": [{
                    "path": "src/forged.rs",
                    "status": "modified",
                    "additions": 1,
                    "deletions": 0
                }]
            }
        }))
        .send()
        .await
        .expect("evidence-changing complete retry");
    assert_eq!(evidence_changing_retry.status(), 409);

    let retry = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/close/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-close"),
            "receipt_digest": receipt_digest("close")
        }))
        .send()
        .await
        .expect("complete retry");
    assert_eq!(retry.status(), 200);

    let conflicting_retry = client
        .post(format!(
            "http://{bound}/v1/runs/run-complete/nodes/close/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-close"),
            "receipt_digest": receipt_digest("different-close")
        }))
        .send()
        .await
        .expect("conflicting complete retry");
    assert_eq!(conflicting_retry.status(), 409);

    shutdown.notify_waiters();
    handle.await.expect("harness join");
}

#[tokio::test]
async fn completed_workbench_mutation_survives_process_restart_and_replays_once() {
    let root = TempDir::new().expect("temp root");
    let client = reqwest::Client::new();
    let (bound, shutdown, handle) = start_harness(&root).await;
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-restart-recovery"),
            "envelope": envelope("plan-run-restart-recovery")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    client
        .post(format!(
            "http://{bound}/v1/runs/run-restart-recovery/approve"
        ))
        .json(&json!({
            "node_id": "approval",
            "envelope": envelope("approve-run-restart-recovery")
        }))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    let completion_body = json!({
        "envelope": envelope("complete-run-restart-recovery-execute"),
        "receipt_digest": receipt_digest("execute"),
        "evidence": {
            "changes": [{
                "path": "src/lib.rs",
                "status": "modified",
                "additions": 1,
                "deletions": 1
            }],
            "provider_receipt": {
                "provider": "nous",
                "model": "fixture-model",
                "adapter": "hermes-workbench",
                "receipt_digest": receipt_digest("execute"),
                "summary": "Restart acceptance mutation completed."
            }
        }
    });
    let completed: Value = client
        .post(format!(
            "http://{bound}/v1/runs/run-restart-recovery/nodes/execute/complete"
        ))
        .json(&completion_body)
        .send()
        .await
        .expect("complete request")
        .error_for_status()
        .expect("complete status")
        .json()
        .await
        .expect("complete body");
    let completed_node = completed["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
        .expect("completed execute node");
    assert_eq!(completed_node["state"], "succeeded");
    assert_eq!(completed_node["output_digest"], receipt_digest("execute"));
    let recovery_token = completed_node["checkpoint"]["recovery_token"]
        .as_str()
        .expect("recovery token")
        .to_string();
    let checkpoint_digest = completed_node["checkpoint"]["checkpoint_digest"]
        .as_str()
        .expect("checkpoint digest")
        .to_string();
    let events_path = root
        .path()
        .join("data/runs/run-restart-recovery/events.jsonl");
    let checkpoint_path = root
        .path()
        .join("data/runs/run-restart-recovery/checkpoint.json");
    let result_path = root
        .path()
        .join("data/runs/run-restart-recovery/result.json");
    assert!(events_path.exists(), "durable event receipt must exist");
    assert!(checkpoint_path.exists(), "durable checkpoint must exist");
    assert!(result_path.exists(), "durable review projection must exist");
    let events_before_restart = std::fs::read(&events_path).expect("read durable events");
    let checkpoint_before_restart =
        std::fs::read(&checkpoint_path).expect("read durable checkpoint");
    let result_before_restart = std::fs::read(&result_path).expect("read durable result");

    shutdown.notify_waiters();
    handle.await.expect("first harness shutdown");

    let (restarted_bound, restarted_shutdown, restarted_handle) = start_harness(&root).await;
    let recovered: Value = client
        .get(format!(
            "http://{restarted_bound}/v1/runs/run-restart-recovery"
        ))
        .send()
        .await
        .expect("recover run request")
        .error_for_status()
        .expect("recover run status")
        .json()
        .await
        .expect("recover run body");
    let recovered_node = recovered["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
        .expect("recovered execute node");
    assert_eq!(recovered_node["state"], "succeeded");
    assert_eq!(recovered_node["output_digest"], receipt_digest("execute"));
    assert_eq!(
        recovered_node["checkpoint"]["recovery_token"],
        recovery_token
    );
    assert_eq!(
        recovered_node["checkpoint"]["checkpoint_digest"],
        checkpoint_digest
    );
    assert_eq!(
        recovered["review"]["provider_receipt"]["receipt_digest"],
        receipt_digest("execute")
    );

    let replay: Value = client
        .post(format!(
            "http://{restarted_bound}/v1/runs/run-restart-recovery/nodes/execute/complete"
        ))
        .json(&completion_body)
        .send()
        .await
        .expect("completion replay")
        .error_for_status()
        .expect("completion replay status")
        .json()
        .await
        .expect("completion replay body");
    assert_eq!(
        replay["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
            .expect("replayed execute node")["checkpoint"]["recovery_token"],
        recovery_token
    );
    assert_eq!(
        std::fs::read(&events_path).expect("read replayed events"),
        events_before_restart,
        "idempotent replay must not append another mutation receipt"
    );
    assert_eq!(
        std::fs::read(&checkpoint_path).expect("read replayed checkpoint"),
        checkpoint_before_restart,
        "authoritative checkpoint must remain stable after replay"
    );
    assert_eq!(
        std::fs::read(&result_path).expect("read replayed result"),
        result_before_restart,
        "durable review projection must remain stable after replay"
    );

    restarted_shutdown.notify_waiters();
    restarted_handle.await.expect("restarted harness shutdown");
}

#[tokio::test]
async fn failed_verification_is_durable_and_blocks_review() {
    let root = TempDir::new().expect("temp root");
    let (bound, shutdown, handle) = start_harness(&root).await;
    let client = reqwest::Client::new();
    attach(&client, bound).await;

    client
        .post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({
            "project_id": PROJECT_ID,
            "graph": completion_graph("run-failed-verification"),
            "envelope": envelope("plan-failed-verification")
        }))
        .send()
        .await
        .expect("plan request")
        .error_for_status()
        .expect("plan status");
    client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/approve"
        ))
        .json(&json!({"node_id": "approval", "envelope": envelope("approve-failed-verification")}))
        .send()
        .await
        .expect("approve request")
        .error_for_status()
        .expect("approve status");

    client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/nodes/execute/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-failed-verification-execute"),
            "receipt_digest": receipt_digest("execute")
        }))
        .send()
        .await
        .expect("execute completion")
        .error_for_status()
        .expect("execute status");

    let failed: Value = client
        .post(format!(
            "http://{bound}/v1/runs/run-failed-verification/nodes/verify/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-failed-verification-verify"),
            "receipt_digest": receipt_digest("verify"),
            "evidence": {"tests": [{
                "name": "cargo test --quiet",
                "status": "failed",
                "duration_ms": 12,
                "details": "fixture assertion failed"
            }]}
        }))
        .send()
        .await
        .expect("verify completion")
        .error_for_status()
        .expect("verify status")
        .json()
        .await
        .expect("failed run response");
    assert_eq!(
        failed["graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == "verify")
            .unwrap()["state"],
        "failed"
    );
    assert_eq!(failed["review"]["tests"][0]["status"], "failed");
    assert_eq!(
        failed["recovery_diagnostics"]["failure_owner"],
        "arda-engine/workbench.verify"
    );
    assert_eq!(
        failed["recovery_diagnostics"]["failure_reason"],
        "Project-native verification failed: fixture assertion failed"
    );
    assert_eq!(
        failed["recovery_diagnostics"]["last_valid_state"]["node_id"],
        "execute"
    );
    assert_eq!(
        failed["recovery_diagnostics"]["last_valid_state"]["receipt_digest"],
        receipt_digest("execute")
    );
    assert_eq!(
        failed["recovery_diagnostics"]["post_recovery_receipt"],
        Value::Null
    );

    shutdown.notify_waiters();
    handle.await.expect("harness shutdown");

    let (restarted_bound, restarted_shutdown, restarted_handle) = start_harness(&root).await;
    let recovered: Value = client
        .get(format!(
            "http://{restarted_bound}/v1/runs/run-failed-verification"
        ))
        .send()
        .await
        .expect("recovery diagnostics request")
        .error_for_status()
        .expect("recovery diagnostics status")
        .json()
        .await
        .expect("recovery diagnostics body");
    assert_eq!(
        recovered["recovery_diagnostics"],
        failed["recovery_diagnostics"]
    );

    let blocked = client
        .post(format!(
            "http://{restarted_bound}/v1/runs/run-failed-verification/nodes/review/complete"
        ))
        .json(&json!({
            "envelope": envelope("complete-blocked-review"),
            "receipt_digest": receipt_digest("review")
        }))
        .send()
        .await
        .expect("blocked review request");
    assert_eq!(blocked.status(), 409);

    let recovered_after_retry: Value = client
        .post(format!(
            "http://{restarted_bound}/v1/runs/run-failed-verification/nodes/verify/complete"
        ))
        .json(&json!({
            "envelope": envelope("retry-failed-verification-verify"),
            "receipt_digest": receipt_digest("verify-recovered"),
            "evidence": {"tests": [{
                "name": "cargo test --quiet",
                "status": "passed",
                "duration_ms": 14,
                "details": "fixture assertion corrected"
            }]}
        }))
        .send()
        .await
        .expect("verification recovery request")
        .error_for_status()
        .expect("verification recovery status")
        .json()
        .await
        .expect("verification recovery body");
    assert_eq!(
        recovered_after_retry["recovery_diagnostics"]["post_recovery_receipt"],
        receipt_digest("verify-recovered")
    );

    restarted_shutdown.notify_waiters();
    restarted_handle.await.expect("restarted harness shutdown");

    let (final_bound, final_shutdown, final_handle) = start_harness(&root).await;
    let final_recovery: Value = client
        .get(format!(
            "http://{final_bound}/v1/runs/run-failed-verification"
        ))
        .send()
        .await
        .expect("final recovery request")
        .error_for_status()
        .expect("final recovery status")
        .json()
        .await
        .expect("final recovery body");
    assert_eq!(
        final_recovery["recovery_diagnostics"]["post_recovery_receipt"],
        receipt_digest("verify-recovered")
    );
    final_shutdown.notify_waiters();
    final_handle.await.expect("final harness shutdown");
}
