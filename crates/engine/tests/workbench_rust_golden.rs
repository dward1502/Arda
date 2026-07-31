use arda_core::project_contract::ProjectContract;
use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, NodeId, NodeKind, NodeState, ObjectiveId,
    Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode,
};
use arda_engine::adapters::{
    AdapterCancellation, HermesAdapter, HermesAdapterConfig, HermesNodeTask,
};
use arda_engine::runs::{apply_transition_once, RunEventDraft, RunEventKind, RunStore};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use tempfile::TempDir;

const RUN_ID: &str = "golden-rust-run";
const APPROVAL_RECEIPT: &str = "receipt:golden-rust-approval";

fn fixture_path(path: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workbench/rust")
        .join(path)
}

fn copy_fixture(destination: &Path) {
    fs::create_dir_all(destination.join("src")).unwrap();
    for relative in ["Cargo.toml", "arda-project.json", "src/lib.rs"] {
        fs::copy(fixture_path(relative), destination.join(relative)).unwrap();
    }
}

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
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
            max_cost_usd: if kind == NodeKind::Execute { 1.0 } else { 0.0 },
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
        objective_id: ObjectiveId::new("objective-golden-rust").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: contract_digest.into(),
            created_by: "stage-4-rust-golden-test".into(),
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

fn write_adapter_config(root: &Path, executable: &Path) -> PathBuf {
    let path = root.join("hermes-golden.toml");
    fs::write(
        &path,
        format!(
            r#"schema_version = "arda.hermes-adapter.v1"
adapter_version = "golden-1"
executable = "{}"
max_timeout_ms = 20000
cancellation_grace_ms = 100
max_turns = 8
max_prompt_bytes = 32768
max_output_bytes = 65536
inherit_environment = ["PATH", "ARDA_GOLDEN_TRANSCRIPT", "ARDA_GOLDEN_ATTEMPT", "ARDA_GOLDEN_MUTATION_COUNT"]

[toolsets]
read_only = ["file"]
human_approval = []
execute_with_approval = ["file", "terminal"]
verify = ["file", "terminal"]
compensate_with_approval = ["file", "terminal"]
"#,
            executable.display()
        ),
    )
    .unwrap();
    path
}

#[tokio::test]
async fn clean_rust_repository_completes_approved_vertical_slice_with_one_run_id() {
    let started = Instant::now();
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    copy_fixture(&project);
    initialize_clean_repository(&project);

    let contract_bytes = fs::read(project.join("arda-project.json")).unwrap();
    let contract = ProjectContract::from_json_str(std::str::from_utf8(&contract_bytes).unwrap())
        .expect("attach valid Rust project contract");
    assert_eq!(contract.identity.name, "arda-rust-golden-fixture");
    let contract_digest = digest(&contract_bytes);

    let state_root = temp.path().join("arda-state");
    let store = RunStore::open(&state_root, RunId::new(RUN_ID).unwrap()).unwrap();
    let mut graph = graph(&contract_digest);
    store
        .append(RunEventDraft {
            node_id: node_id("inspect"),
            idempotency_key: format!("{RUN_ID}:attached-and-planned"),
            kind: RunEventKind::Planned {
                project_id: contract.identity.project_id.to_string(),
                approval_id: "golden-rust-approval".into(),
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

    let executable = temp.path().join("hermes");
    fs::copy(fixture_path("fake_hermes.py"), &executable).unwrap();
    let mut permissions = fs::metadata(&executable).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&executable, permissions).unwrap();
    let config_path = write_adapter_config(temp.path(), &executable);
    let environment = BTreeMap::from([
        ("PATH".into(), std::env::var("PATH").unwrap()),
        (
            "ARDA_GOLDEN_TRANSCRIPT".into(),
            temp.path().join("transcript.json").display().to_string(),
        ),
        (
            "ARDA_GOLDEN_ATTEMPT".into(),
            temp.path().join("attempt").display().to_string(),
        ),
        (
            "ARDA_GOLDEN_MUTATION_COUNT".into(),
            temp.path().join("mutation-count").display().to_string(),
        ),
    ]);
    HermesAdapterConfig::from_toml_str(&fs::read_to_string(&config_path).unwrap()).unwrap();
    let adapter = HermesAdapter::load(&config_path, &project, &project, &environment).unwrap();
    let execute_node = graph
        .nodes
        .iter()
        .find(|node| node.id.as_str() == "execute")
        .unwrap()
        .clone();
    let task = HermesNodeTask {
        run_id: graph.run_id.clone(),
        node: execute_node,
        objective: "Change the Rust greeting from `hello` to `hello, Arda` and update its test."
            .into(),
        instructions:
            "Perform only the two bounded string replacements, then run cargo test --quiet.".into(),
        checks: vec!["test".into()],
        project_contract_digest: contract_digest.clone(),
    };

    let first_error = adapter
        .execute(&task, AdapterCancellation::new())
        .await
        .expect_err("fixture injects one recoverable adapter failure");
    let receipt = adapter
        .execute(&task, AdapterCancellation::new())
        .await
        .expect("retry recovers and returns canonical receipt");
    assert_eq!(receipt.run_id, RUN_ID);
    assert_eq!(receipt.usage.provider.as_deref(), Some("fixture-provider"));
    assert_eq!(receipt.usage.model.as_deref(), Some("fixture-model"));
    assert_eq!(receipt.usage.estimated_cost_usd, 0.001);
    assert_eq!(
        fs::read_to_string(temp.path().join("mutation-count")).unwrap(),
        "1"
    );

    apply_transition_once(
        &store,
        &mut graph,
        &node_id("execute"),
        NodeState::Running,
        format!("{RUN_ID}:execute:running"),
        Some(APPROVAL_RECEIPT.into()),
    )
    .unwrap();
    apply_transition_once(
        &store,
        &mut graph,
        &node_id("execute"),
        NodeState::Succeeded,
        format!("{RUN_ID}:execute:succeeded"),
        Some(receipt.receipt_digest.clone()),
    )
    .unwrap();
    complete_node(&store, &mut graph, "verify", "receipt:verify");
    complete_node(&store, &mut graph, "review", "receipt:review");
    complete_node(&store, &mut graph, "close", "receipt:close");

    let independent_test = Command::new("cargo")
        .args(["test", "--quiet"])
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        independent_test.status.success(),
        "{}",
        String::from_utf8_lossy(&independent_test.stderr)
    );
    let diff = git(&project, &["diff", "--", "src/lib.rs"]);
    assert!(diff.contains("hello, Arda"));
    assert!(!diff.contains("arda-project.json"));

    let result = json!({
        "schema_version": "arda.workbench-golden-result.v1",
        "run_id": RUN_ID,
        "project": {
            "kind": "rust",
            "contract_digest": contract_digest,
            "clean_before_attach": true
        },
        "graph": graph,
        "model_route": {
            "adapter": receipt.adapter,
            "provider": receipt.usage.provider,
            "model": receipt.usage.model
        },
        "tools": receipt.tool_evidence,
        "evidence": receipt.artifacts,
        "cost_usd": receipt.usage.estimated_cost_usd,
        "approval": {"receipt": APPROVAL_RECEIPT},
        "diff": diff,
        "tests": receipt.test_evidence,
        "memory": {
            "scope": "project",
            "summary": "Rust greeting mutation verified and closed."
        },
        "metrics": {
            "install_to_result_ms": started.elapsed().as_millis(),
            "interventions": 1,
            "failures": [first_error.to_string()],
            "recovery_behavior": "automatic retry after a recorded transient adapter failure",
            "observable_mutation_count": 1
        }
    });
    store.write_result(&result).unwrap();
    if let Some(directory) = std::env::var_os("ARDA_GOLDEN_EVIDENCE_DIR") {
        let directory = PathBuf::from(directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("rust-golden-result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
    store
        .append(RunEventDraft {
            node_id: node_id("close"),
            idempotency_key: format!("{RUN_ID}:result-projected"),
            kind: RunEventKind::ResultProjected,
            receipt_digest: Some(receipt.receipt_digest),
        })
        .unwrap();

    let persisted: serde_json::Value =
        serde_json::from_slice(&fs::read(store.result_path()).unwrap()).unwrap();
    assert_eq!(persisted["run_id"], RUN_ID);
    assert_eq!(persisted["metrics"]["observable_mutation_count"], 1);
    assert_eq!(persisted["model_route"]["model"], "fixture-model");
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
