use arda_core::capability_composition::{
    CompositionAuthorityClass, DataClass, EgressTarget, RoleKind,
};
use arda_core::contract::{MemoryKind, MemoryRecord};
use arda_core::run_graph::{ObjectiveId, RunId};
use arda_engine::harness::{
    presence::HarnessPresenceState, serve, HarnessState, DEFAULT_HARNESS_ADDR,
    DEFAULT_MANWE_PROXY_TIMEOUT, DEFAULT_WARDEN_SCOUT_TIMEOUT,
};
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use arda_vaire::{
    ContextAssembly, ContextConsumer, ContextLineage, ContextObjective, ContextReturnContract,
    MnemosyneService, OrganismContext,
};
use serde_json::{json, Value};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::TempDir;
use tokio::sync::{Notify, RwLock};

const PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const OBJECTIVE: &str = "Complete the bounded context-bootstrap check using only the governed capsule. Execute `python3 verify-context-bootstrap.py` as the first and only terminal command, then bind test evidence to that exact terminal call. Do not inspect the directory with ls or pwd.";

fn objective_id(run_id: &str) -> String {
    std::env::var("ARDA_CONTEXT_OBJECTIVE_ID").unwrap_or_else(|_| format!("objective-{run_id}"))
}

fn objective() -> String {
    std::env::var("ARDA_CONTEXT_OBJECTIVE").unwrap_or_else(|_| OBJECTIVE.into())
}

async fn start(
    root: &TempDir,
) -> (
    std::net::SocketAddr,
    Arc<Notify>,
    tokio::task::JoinHandle<()>,
) {
    let shutdown = Arc::new(Notify::new());
    let state = HarnessState {
        harness_addr: DEFAULT_HARNESS_ADDR.into(),
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
        operator_id: "operator-0".into(),
    };
    let (bound, handle) = serve(
        Some("127.0.0.1:0".parse().unwrap()),
        state,
        shutdown.clone(),
    )
    .await
    .unwrap();
    (bound, shutdown, handle)
}

fn envelope(key: &str) -> Value {
    json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": "proposal-context-bootstrap",
            "approval_id": "approval-context-bootstrap",
            "ledger_writes": ["context-bootstrap.jsonl"],
            "decision": "policy_safe",
            "created_at_utc": "2026-08-22T00:00:00Z"
        },
        "idempotency_key": key
    })
}

fn contract() -> Value {
    let mut contract: Value = serde_json::from_str(include_str!(
        "../../../spec/project-contract/v1/examples/rust-project.json"
    ))
    .unwrap();
    contract["runtime"]["adapter"] = json!("python3");
    contract["commands"] = json!([{
        "id": "test",
        "program": "python3",
        "args": ["verify-context-bootstrap.py"],
        "working_dir": "."
    }]);
    contract["artifacts"] = json!([]);
    contract["permissions"]["filesystem"]["write"] = json!(false);
    contract
}

fn approval_receipt() -> String {
    "receipt:approval".into()
}

fn graph(run_id: &str, node_id: &str, additional_parent: Option<&str>) -> Value {
    let mut execute_parents = vec![approval_receipt()];
    if let Some(parent) = additional_parent {
        execute_parents.push(parent.into());
    }
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": objective_id(run_id),
        "nodes": [
          {
            "id": "plan", "kind": "plan", "state": "pending", "authority": "read_only",
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0}, "retry": {"max_attempts": 1},
            "timeout_ms": 1000, "idempotency_key": format!("plan-{run_id}"),
            "input_digest": null, "output_digest": null, "parent_receipts": [],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
          },
          {
            "id": "approval", "kind": "approval", "state": "pending", "authority": "human_approval",
            "budget": {"max_joules": 1.0, "max_cost_usd": 0.0}, "retry": {"max_attempts": 1},
            "timeout_ms": 1000, "idempotency_key": format!("approval-{run_id}"),
            "input_digest": null, "output_digest": null, "parent_receipts": ["receipt:plan"],
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
          },
          {
            "id": node_id,
            "kind": "execute",
            "state": "pending",
            "authority": "execute_with_approval",
            "budget": {"max_joules": 100.0, "max_cost_usd": 1.0},
            "retry": {"max_attempts": 1},
            "timeout_ms": 600000,
            "idempotency_key": format!("provider-{run_id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": execute_parents,
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null}
          }
        ],
        "edges": [
          {"id":"plan-approval","from":"plan","to":"approval","parent_receipt":"receipt:plan"},
          {"id":"approval-execute","from":"approval","to":node_id,"parent_receipt":"receipt:approval"}
        ],
        "provenance": {
            "project_contract_digest": "sha256:project-fixture",
            "created_by": "context-bootstrap-test",
            "parent_receipts": []
        }
    })
}

fn install_fake_hermes(root: &Path) {
    let executable = root.join("fake-hermes-context");
    fs::write(
        &executable,
        r#"#!/usr/bin/python3
import json, sys
from pathlib import Path
root = Path(__file__).parent
transcript = root / "context-transcript.json"
args = sys.argv[1:]
if args[:2] == ["sessions", "export"]:
    print(transcript.read_text(encoding="utf-8"), flush=True)
    raise SystemExit(0)
prompt = args[args.index("-q") + 1]
count_path = root / "context-worker-count"
count = int(count_path.read_text() if count_path.exists() else "0") + 1
count_path.write_text(str(count), encoding="utf-8")
with (root / "context-prompts.jsonl").open("a", encoding="utf-8") as handle:
    handle.write(json.dumps({"worker":count,"prompt":prompt}) + "\n")
tool_result = json.dumps({"output":"ok\n","exit_code":0,"error":None}, separators=(",", ":"))
session = {
 "id":f"fresh-context-worker-{count}","source":"tool","model":"fixture-model",
 "billing_provider":"fixture-provider","estimated_cost_usd":0.0,"actual_cost_usd":0.0,
 "input_tokens":10,"output_tokens":10,"api_call_count":1,
 "messages":[
  {"role":"assistant","content":None,"tool_calls":[{"id":"call-test-1","type":"function","function":{"name":"terminal","arguments":json.dumps({"command":"python3 verify-context-bootstrap.py"})}}]},
  {"role":"tool","tool_call_id":"call-test-1","tool_name":"terminal","content":tool_result}
 ]
}
transcript.write_text(json.dumps(session), encoding="utf-8")
result={"schema_version":"arda.hermes-job-result.v1","status":"succeeded","summary":"Fresh worker completed the bounded task from governed context.","tool_evidence":[{"tool_call_id":"call-test-1"}],"test_evidence":[{"check_id":"test","tool_call_id":"call-test-1"}],"artifacts":[]}
print(f"session_id: fresh-context-worker-{count}")
print(json.dumps(result), flush=True)
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&executable, permissions).unwrap();
    let config = root.join("config/adapters/hermes-workbench.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        config,
        format!(
            "schema_version = \"arda.hermes-adapter.v1\"\nadapter_version = \"context-bootstrap-test\"\nexecutable = \"{}\"\nmax_timeout_ms = 10000\ncancellation_grace_ms = 100\nmax_turns = 4\nmax_prompt_bytes = 131072\nmax_output_bytes = 1048576\ninherit_environment = [\"PATH\"]\n\n[toolsets]\nread_only = [\"file\"]\nhuman_approval = []\nexecute_with_approval = [\"file\", \"terminal\"]\nverify = [\"file\", \"terminal\"]\ncompensate_with_approval = [\"file\", \"terminal\"]\n",
            executable.display()
        ),
    )
    .unwrap();
}

fn install_live_hermes(root: &Path) {
    let executable = std::env::var("ARDA_LIVE_HERMES_EXECUTABLE")
        .expect("ARDA_LIVE_HERMES_EXECUTABLE must name the real Hermes CLI");
    let config = root.join("config/adapters/hermes-workbench.toml");
    fs::create_dir_all(config.parent().unwrap()).unwrap();
    fs::write(
        config,
        format!(
            "schema_version = \"arda.hermes-adapter.v1\"\nadapter_version = \"context-bootstrap-live\"\nexecutable = \"{executable}\"\nmax_timeout_ms = 600000\ncancellation_grace_ms = 1000\nmax_turns = 8\nmax_prompt_bytes = 131072\nmax_output_bytes = 1048576\ninherit_environment = [\"HOME\", \"HERMES_HOME\", \"PATH\", \"LANG\", \"LC_ALL\", \"SSL_CERT_FILE\", \"SSL_CERT_DIR\"]\n\n[toolsets]\nread_only = [\"file\"]\nhuman_approval = []\nexecute_with_approval = [\"file\", \"terminal\"]\nverify = [\"file\", \"terminal\"]\ncompensate_with_approval = [\"file\", \"terminal\"]\n"
        ),
    )
    .unwrap();
}

fn assembly(root: &Path, run_id: &str, worker_id: &str, parents: Vec<String>) -> ContextAssembly {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let mut consumer = ConsumerContext::new(worker_id, vec![MemoryDomain::System]);
    consumer.purpose = Some(objective());
    let service = MnemosyneService::new(root.join("data/vaire"))
        .unwrap()
        .with_contract_memory_root(root.join("core/state/memory"));
    if service
        .recall_governed_memories(Some(&consumer))
        .unwrap()
        .is_empty()
    {
        let mut memory = MemoryRecord::new(
            "mem-bootstrap-next-action",
            MemoryKind::Semantic,
            "vaire",
            "Continue the objective: run the declared check and return typed evidence.",
        );
        memory
            .extensions
            .insert("memory_domain".into(), json!("system"));
        service
            .write_governed_memory(memory, Some(&consumer))
            .unwrap();
    }
    service
        .assemble_organism_context(
            OrganismContext {
                schema_version: OrganismContext::SCHEMA_VERSION.into(),
                organism_id: "arda:mythos:primary".into(),
                generated_at_unix_ms: now,
                expires_at_unix_ms: now + 300_000,
                consumer: ContextConsumer {
                    consumer_id: worker_id.into(),
                    role: RoleKind::Worker,
                    authority_ceiling: CompositionAuthorityClass::ExecuteWithApproval,
                    operator_authorized: false,
                    memory_domains: vec![MemoryDomain::System],
                    data_classes: vec![DataClass::Internal],
                    permitted_egress: vec![EgressTarget::LocalDevice],
                    compute_node_refs: vec!["node:arda-root".into()],
                    agent_ref: Some(format!("hermes:{worker_id}")),
                },
                lineage: ContextLineage {
                    objective_id: ObjectiveId::new(objective_id(run_id)).unwrap(),
                    project_id: Some(PROJECT_ID.parse().unwrap()),
                    run_id: Some(RunId::new(run_id).unwrap()),
                    task_id: Some("digital-organism-s1-context-bootstrap".into()),
                    session_ref: None,
                    parent_receipts: parents,
                },
                objective: ContextObjective {
                    requested_outcome: objective(),
                    acceptance_conditions: vec!["declared check passes".into()],
                    required_capabilities: vec!["terminal".into()],
                    forbidden_capabilities: vec!["ambient-transcript-read".into()],
                },
                evidence_refs: vec!["arda://varda/evidence/context-bootstrap".into()],
                memory_refs: vec!["mem-bootstrap-next-action".into()],
                unresolved_failures: Vec::new(),
                return_contract: ContextReturnContract {
                    schema_version: "arda.organism-outcome.v1".into(),
                    required_receipt_types: vec![
                        "arda.execution-receipt.v1".into(),
                        "arda.context-use-receipt.v1".into(),
                        "arda.handoff-receipt.v1".into(),
                    ],
                    max_output_bytes: 32768,
                },
            },
            &consumer,
            now,
        )
        .unwrap()
}

async fn attach_and_plan(
    client: &reqwest::Client,
    bound: std::net::SocketAddr,
    run_id: &str,
    node_id: &str,
    additional_parent: Option<&str>,
) -> Vec<String> {
    client
        .post(format!("http://{bound}/v1/projects/attach"))
        .json(&json!({"contract": contract(), "envelope": envelope(&format!("attach-{run_id}"))}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
    let response = client.post(format!("http://{bound}/v1/runs/plan"))
        .json(&json!({"project_id":PROJECT_ID,"graph":graph(run_id,node_id,additional_parent),"envelope":envelope(&format!("plan-{run_id}"))}))
        .send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert_eq!(status, 201, "{body}");
    let approved: Value = client
        .post(format!("http://{bound}/v1/runs/{run_id}/approve"))
        .json(&json!({"node_id":"approval","envelope":envelope(&format!("approve-{run_id}"))}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    approved["graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == node_id)
        .unwrap()["parent_receipts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|receipt| receipt.as_str().unwrap().to_owned())
        .collect()
}

async fn execute(
    client: &reqwest::Client,
    bound: std::net::SocketAddr,
    run_id: &str,
    node_id: &str,
    context: &ContextAssembly,
) -> Value {
    let response = client.post(format!("http://{bound}/v1/runs/{run_id}/nodes/{node_id}/execute-provider"))
        .json(&json!({"envelope":envelope(&format!("execute-{run_id}")),"objective":objective(),"context_assembly":context}))
        .send().await.unwrap();
    let status = response.status();
    let body = response.text().await.unwrap();
    assert!(
        status.is_success(),
        "provider execution failed ({status}): {body}"
    );
    serde_json::from_str(&body).unwrap()
}

#[tokio::test]
async fn another_fresh_worker_continues_after_root_restart_without_conversation_history() {
    let root = TempDir::new().unwrap();
    fs::write(
        root.path().join("context-bootstrap-input.txt"),
        "governed-next-action\n",
    )
    .unwrap();
    fs::write(
        root.path().join("verify-context-bootstrap.py"),
        "from pathlib import Path\nassert Path('context-bootstrap-input.txt').read_text(encoding='utf-8') == 'governed-next-action\\n'\nprint('context-bootstrap-check: ok')\n",
    )
    .unwrap();
    let live_hermes = std::env::var_os("ARDA_LIVE_HERMES_EXECUTABLE").is_some();
    if live_hermes {
        install_live_hermes(root.path());
    } else {
        install_fake_hermes(root.path());
    }
    let client = reqwest::Client::new();

    let (bound1, shutdown1, handle1) = start(&root).await;
    let first_parents =
        attach_and_plan(&client, bound1, "run-context-first", "execute-first", None).await;
    let first = assembly(
        root.path(),
        "run-context-first",
        "execute-first",
        first_parents,
    );
    let first_response = execute(
        &client,
        bound1,
        "run-context-first",
        "execute-first",
        &first,
    )
    .await;
    let first_receipt = &first_response["receipt"];
    assert_eq!(first_receipt["status"], "succeeded");
    assert_eq!(
        first_receipt["context_capsule_digest"],
        first.capsule.capsule_digest
    );
    let first_digest = first_receipt["receipt_digest"].as_str().unwrap().to_owned();
    shutdown1.notify_waiters();
    handle1.await.unwrap();

    let reopened = MnemosyneService::new(root.path().join("data/vaire")).unwrap();
    assert_eq!(
        reopened
            .context_use_receipt(&first.use_receipt.receipt_id)
            .unwrap(),
        Some(first.use_receipt.clone())
    );

    let (bound2, shutdown2, handle2) = start(&root).await;
    let second_parents = attach_and_plan(
        &client,
        bound2,
        "run-context-second",
        "execute-second",
        Some(&first_digest),
    )
    .await;
    let second = assembly(
        root.path(),
        "run-context-second",
        "execute-second",
        second_parents.clone(),
    );
    let second_response = execute(
        &client,
        bound2,
        "run-context-second",
        "execute-second",
        &second,
    )
    .await;
    let second_receipt = second_response["receipt"].clone();
    assert_eq!(second_receipt["status"], "succeeded");
    assert_eq!(
        second_receipt["context_handoff"]["destination_consumer"],
        "execute-second"
    );
    if !live_hermes {
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            "2"
        );
    }

    let replay = execute(
        &client,
        bound2,
        "run-context-second",
        "execute-second",
        &second,
    )
    .await;
    assert_eq!(replay["receipt"], second_receipt);
    if !live_hermes {
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            "2"
        );
    }

    let foreign_root = tempfile::tempdir().unwrap();
    std::thread::sleep(std::time::Duration::from_millis(2));
    let foreign = assembly(
        foreign_root.path(),
        "run-context-second",
        "execute-second",
        second_parents,
    );
    let foreign_response = client
        .post(format!(
            "http://{bound2}/v1/runs/run-context-second/nodes/execute-second/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("execute-run-context-second"),
            "objective": objective(),
            "context_assembly": foreign,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(foreign_response.status(), reqwest::StatusCode::CONFLICT);
    let foreign_body = foreign_response.text().await.unwrap();
    assert!(
        foreign_body.contains("context use receipt is not durably recorded"),
        "unexpected foreign-context rejection: {foreign_body}"
    );

    let mut mismatched = second.clone();
    mismatched.capsule.capsule_digest = format!("{}-mismatch", second.capsule.capsule_digest);
    let mismatch_response = client
        .post(format!(
            "http://{bound2}/v1/runs/run-context-second/nodes/execute-second/execute-provider"
        ))
        .json(&json!({
            "envelope": envelope("execute-run-context-second"),
            "objective": objective(),
            "context_assembly": mismatched,
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(mismatch_response.status(), reqwest::StatusCode::CONFLICT);
    assert!(mismatch_response
        .text()
        .await
        .unwrap()
        .contains("context capsule is invalid"));
    if !live_hermes {
        assert_eq!(
            fs::read_to_string(root.path().join("context-worker-count")).unwrap(),
            "2"
        );
    }

    if !live_hermes {
        let prompts: Vec<Value> = fs::read_to_string(root.path().join("context-prompts.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(prompts.len(), 2);
        assert!(prompts[0]["prompt"]
            .as_str()
            .unwrap()
            .contains("execute-first"));
        assert!(prompts[1]["prompt"]
            .as_str()
            .unwrap()
            .contains("execute-second"));
        assert!(!prompts[1]["prompt"]
            .as_str()
            .unwrap()
            .contains("fresh-context-worker-1"));
    }

    shutdown2.notify_waiters();
    handle2.await.unwrap();

    if let Some(path) = std::env::var_os("ARDA_CONTEXT_BOOTSTRAP_EVIDENCE_PATH") {
        let path = std::path::PathBuf::from(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let evidence = json!({
            "schema_version": "arda.context-bootstrap-runtime-proof.v1",
            "objective_id": objective_id("run-context-second"),
            "live_hermes": live_hermes,
            "fresh_process_count": 2,
            "first": {
                "run_id": "run-context-first",
                "worker_id": "execute-first",
                "receipt_digest": first_receipt["receipt_digest"],
                "capsule_digest": first_receipt["context_capsule_digest"],
                "context_use_receipt_ref": first_receipt["context_use_receipt_ref"],
                "handoff_receipt": first_receipt["context_handoff"],
            },
            "second": {
                "run_id": "run-context-second",
                "worker_id": "execute-second",
                "receipt_digest": second_receipt["receipt_digest"],
                "capsule_digest": second_receipt["context_capsule_digest"],
                "context_use_receipt_ref": second_receipt["context_use_receipt_ref"],
                "handoff_receipt": second_receipt["context_handoff"],
                "explicit_parent_receipt": first_digest,
            },
            "restart": {
                "first_harness_stopped_before_second_started": true,
                "vaire_context_use_receipt_reopened": true,
                "ambient_conversation_inherited": false,
            },
            "replay": {
                "canonical_receipt_returned": true,
                "mismatched_capsule_rejected": true,
            }
        });
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
}
