use arda_core::project_contract::ProjectContract;
use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, NodeId, NodeKind, NodeState, ObjectiveId,
    Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode,
};
use arda_engine::adapters::{
    AdapterCancellation, AdapterProcessConfig, AdapterRequest, AdapterStatus, JsonlAdapter,
};
use arda_engine::runs::{apply_transition_once, RunEventDraft, RunEventKind, RunStore};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const RUN_ID: &str = "golden-python-run";
const APPROVAL_RECEIPT: &str = "receipt:golden-python-approval";

fn fixture_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workbench/python")
        .join(path)
}

fn copy_fixture(destination: &Path) {
    fs::create_dir_all(destination.join("src")).unwrap();
    fs::create_dir_all(destination.join("tests")).unwrap();
    for relative in [
        "pyproject.toml",
        "arda-project.json",
        "adapter.py",
        "src/greeting.py",
        "tests/test_greeting.py",
    ] {
        fs::copy(fixture_path(relative), destination.join(relative)).unwrap();
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn initialize_clean_repository(root: &Path) {
    git(root, &["init", "--quiet"]);
    git(root, &["config", "user.name", "Arda Golden Test"]);
    git(root, &["config", "user.email", "golden@arda.invalid"]);
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "clean fixture",
        ],
    );
    assert!(git(root, &["status", "--porcelain"]).is_empty());
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn node(id: &str, kind: NodeKind, authority: AuthorityClass, parent: Option<&str>) -> RunNode {
    RunNode {
        id: NodeId::new(id).unwrap(),
        kind,
        state: NodeState::Pending,
        authority,
        budget: Budget {
            max_joules: 1_000.0,
            max_cost_usd: 0.0,
        },
        retry: RetryPolicy { max_attempts: 2 },
        timeout_ms: 20_000,
        idempotency_key: format!("{RUN_ID}:{id}"),
        input_digest: Some(format!("sha256:{id}-input")),
        output_digest: None,
        parent_receipts: parent.into_iter().map(str::to_string).collect(),
        checkpoint: CheckpointMetadata::default(),
    }
}

fn graph(contract_digest: &str) -> RunGraph {
    let definitions = [
        ("inspect", NodeKind::Inspect, AuthorityClass::ReadOnly),
        ("plan", NodeKind::Plan, AuthorityClass::ReadOnly),
        (
            "approval",
            NodeKind::Approval,
            AuthorityClass::HumanApproval,
        ),
        (
            "execute",
            NodeKind::Execute,
            AuthorityClass::ExecuteWithApproval,
        ),
        ("verify", NodeKind::Verify, AuthorityClass::Verify),
        ("review", NodeKind::Review, AuthorityClass::Verify),
        ("close", NodeKind::Close, AuthorityClass::ReadOnly),
    ];
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (index, (id, kind, authority)) in definitions.iter().enumerate() {
        let parent = index
            .checked_sub(1)
            .map(|previous| format!("receipt:{}", definitions[previous].0));
        nodes.push(node(id, *kind, *authority, parent.as_deref()));
        if let Some(previous) = index.checked_sub(1) {
            let mut edge = RunEdge::new(
                format!("{}-to-{id}", definitions[previous].0),
                definitions[previous].0,
                *id,
            )
            .unwrap();
            edge.parent_receipt = parent;
            edges.push(edge);
        }
    }
    let graph = RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new(RUN_ID).unwrap(),
        objective_id: ObjectiveId::new("objective-golden-python").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: contract_digest.into(),
            created_by: "stage-4-python-golden-test".into(),
            parent_receipts: Vec::new(),
        },
    };
    graph.validate().unwrap();
    graph
}

fn node_id(id: &str) -> NodeId {
    NodeId::new(id).unwrap()
}

fn complete_node(store: &RunStore, graph: &mut RunGraph, id: &str, receipt: &str) {
    for (state, suffix) in [
        (NodeState::Ready, "ready"),
        (NodeState::Running, "running"),
        (NodeState::Succeeded, "succeeded"),
    ] {
        apply_transition_once(
            store,
            graph,
            &node_id(id),
            state,
            format!("{RUN_ID}:{id}:{suffix}"),
            Some(receipt.into()),
        )
        .unwrap();
    }
}

fn python_executable() -> PathBuf {
    let output = Command::new("python3")
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .unwrap();
    assert!(output.status.success());
    fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
}

#[tokio::test]
async fn clean_python_repository_completes_through_reference_adapter_outside_cargo_workspace() {
    let started = Instant::now();
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("python-project-outside-workspace");
    fs::create_dir(&project).unwrap();
    copy_fixture(&project);
    initialize_clean_repository(&project);
    assert!(!project.starts_with(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")));

    let contract_bytes = fs::read(project.join("arda-project.json")).unwrap();
    let contract = ProjectContract::from_json_str(std::str::from_utf8(&contract_bytes).unwrap())
        .expect("attach valid Python project contract");
    assert_eq!(contract.runtime.adapter, "python-reference");
    let contract_digest = digest(&contract_bytes);

    let store =
        RunStore::open(temp.path().join("arda-state"), RunId::new(RUN_ID).unwrap()).unwrap();
    let mut graph = graph(&contract_digest);
    store
        .append(RunEventDraft {
            node_id: node_id("inspect"),
            idempotency_key: format!("{RUN_ID}:attached-and-planned"),
            kind: RunEventKind::Planned {
                project_id: contract.identity.project_id.to_string(),
                approval_id: "golden-python-approval".into(),
            },
            receipt_digest: Some(APPROVAL_RECEIPT.into()),
        })
        .unwrap();
    store.write_checkpoint(&graph).unwrap();
    complete_node(&store, &mut graph, "inspect", "receipt:inspect");
    complete_node(&store, &mut graph, "plan", "receipt:plan");
    complete_node(&store, &mut graph, "approval", APPROVAL_RECEIPT);
    apply_transition_once(
        &store,
        &mut graph,
        &node_id("execute"),
        NodeState::Ready,
        format!("{RUN_ID}:execute:ready"),
        Some(APPROVAL_RECEIPT.into()),
    )
    .unwrap();

    let sdk = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sdk/python");
    let mutation_count = temp.path().join("python-mutation-count");
    let path = std::env::var("PATH").unwrap();
    let environment = BTreeMap::from([
        ("PATH".into(), path),
        ("PYTHONPATH".into(), sdk.display().to_string()),
        (
            "ARDA_GOLDEN_MUTATION_COUNT".into(),
            mutation_count.display().to_string(),
        ),
    ]);
    let adapter = JsonlAdapter::new(AdapterProcessConfig {
        executable: python_executable(),
        args: vec![project.join("adapter.py").display().to_string()],
        expected_adapter: "arda-python-golden".into(),
        expected_adapter_version: "1.0.0".into(),
        project_root: project.clone(),
        cwd: project.clone(),
        environment,
        environment_allowlist: BTreeSet::from([
            "PATH".into(),
            "PYTHONPATH".into(),
            "ARDA_GOLDEN_MUTATION_COUNT".into(),
        ]),
        capabilities: BTreeSet::from(["mutate_and_test".into()]),
        timeout: Duration::from_secs(20),
        cancellation_grace: Duration::from_millis(100),
        max_line_bytes: 64 * 1024,
    })
    .unwrap();

    let failed = adapter
        .execute(
            AdapterRequest {
                id: "python-golden-invalid-attempt".into(),
                operation: "mutate_and_test".into(),
                arguments: json!({"before": "wrong", "after": "hello, Arda"}),
                timeout: Duration::from_secs(20),
                required_capabilities: BTreeSet::from(["mutate_and_test".into()]),
                idempotency_key: format!("{RUN_ID}:execute:attempt-1"),
                recovery_token: None,
            },
            AdapterCancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(failed.status, AdapterStatus::Failed);
    assert!(!mutation_count.exists());

    let result = adapter
        .execute(
            AdapterRequest {
                id: "python-golden-recovered-attempt".into(),
                operation: "mutate_and_test".into(),
                arguments: json!({"before": "hello", "after": "hello, Arda"}),
                timeout: Duration::from_secs(20),
                required_capabilities: BTreeSet::from(["mutate_and_test".into()]),
                idempotency_key: format!("{RUN_ID}:execute:attempt-2"),
                recovery_token: Some("python-golden-recovery-1".into()),
            },
            AdapterCancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(result.status, AdapterStatus::Succeeded);
    assert_eq!(
        result.recovery_token.as_deref(),
        Some("python-golden-recovery-1")
    );
    assert_eq!(fs::read_to_string(&mutation_count).unwrap(), "1");

    apply_transition_once(
        &store,
        &mut graph,
        &node_id("execute"),
        NodeState::Running,
        format!("{RUN_ID}:execute:running"),
        Some(APPROVAL_RECEIPT.into()),
    )
    .unwrap();
    let adapter_receipt = digest(&serde_json::to_vec(&result).unwrap());
    apply_transition_once(
        &store,
        &mut graph,
        &node_id("execute"),
        NodeState::Succeeded,
        format!("{RUN_ID}:execute:succeeded"),
        Some(adapter_receipt.clone()),
    )
    .unwrap();
    complete_node(&store, &mut graph, "verify", "receipt:verify");
    complete_node(&store, &mut graph, "review", "receipt:review");
    complete_node(&store, &mut graph, "close", "receipt:close");

    let independent_test = Command::new("python3")
        .args(["-m", "unittest", "discover", "-s", "tests", "-v"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        independent_test.status.success(),
        "{}",
        String::from_utf8_lossy(&independent_test.stderr)
    );
    let diff = git(
        &project,
        &["diff", "--", "src/greeting.py", "tests/test_greeting.py"],
    );
    assert!(diff.contains("hello, Arda"));

    let golden_result = json!({
        "schema_version": "arda.workbench-golden-result.v1",
        "run_id": RUN_ID,
        "project": {
            "kind": "python",
            "contract_digest": contract_digest,
            "clean_before_attach": true,
            "outside_cargo_workspace": true
        },
        "graph": graph,
        "model_route": result.output["route"],
        "tools": [{
            "adapter": result.provenance.adapter,
            "operation": "mutate_and_test",
            "request_digest": result.provenance.request_digest
        }],
        "evidence": result.output["mutation"],
        "cost_usd": result.output["cost_usd"],
        "approval": {"receipt": APPROVAL_RECEIPT},
        "diff": diff,
        "tests": [result.output["test"].clone()],
        "memory": {
            "scope": "project",
            "summary": "Python greeting mutation verified and closed."
        },
        "metrics": {
            "install_to_result_ms": started.elapsed().as_millis(),
            "interventions": 1,
            "failures": [failed.output],
            "recovery_behavior": "invalid approved arguments failed before mutation; corrected request resumed with recovery token",
            "observable_mutation_count": 1
        }
    });
    store.write_result(&golden_result).unwrap();
    if let Some(directory) = std::env::var_os("ARDA_GOLDEN_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("python-golden-result.json"),
            serde_json::to_vec_pretty(&golden_result).unwrap(),
        )
        .unwrap();
    }
    store
        .append(RunEventDraft {
            node_id: node_id("close"),
            idempotency_key: format!("{RUN_ID}:result-projected"),
            kind: RunEventKind::ResultProjected,
            receipt_digest: Some(adapter_receipt),
        })
        .unwrap();

    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(store.result_path()).unwrap()).unwrap();
    assert_eq!(persisted["run_id"], RUN_ID);
    assert_eq!(persisted["project"]["outside_cargo_workspace"], true);
    assert_eq!(persisted["metrics"]["observable_mutation_count"], 1);
    assert_eq!(
        store
            .recover()
            .unwrap()
            .checkpoint
            .unwrap()
            .nodes
            .iter()
            .filter(|node| node.state == NodeState::Succeeded)
            .count(),
        7
    );
}
