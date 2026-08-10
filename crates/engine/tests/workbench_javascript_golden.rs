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

const RUN_ID: &str = "golden-javascript-run";
const APPROVAL_RECEIPT: &str = "receipt:javascript-approval";

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/workbench/javascript")
}

fn copy_fixture(destination: &Path) {
    for relative in [
        "arda-project.json",
        "package.json",
        "adapter.mjs",
        "src/greeting.js",
        "test/greeting.test.js",
    ] {
        let target = destination.join(relative);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(fixture_path().join(relative), target).unwrap();
    }
}

fn git(project: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn node_id(value: &str) -> NodeId {
    NodeId::new(value).unwrap()
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
    let mut nodes: Vec<RunNode> = definitions
        .iter()
        .map(|(id, kind, authority)| RunNode {
            id: node_id(id),
            kind: *kind,
            state: NodeState::Pending,
            authority: *authority,
            budget: Budget {
                max_joules: 25.0,
                max_cost_usd: 0.0,
            },
            retry: RetryPolicy { max_attempts: 2 },
            timeout_ms: 20_000,
            idempotency_key: format!("{RUN_ID}:{id}"),
            input_digest: Some("objective:javascript-golden".into()),
            output_digest: None,
            parent_receipts: Vec::new(),
            checkpoint: CheckpointMetadata::default(),
            worker: None,
        })
        .collect();
    for index in 1..nodes.len() {
        nodes[index].parent_receipts = vec![format!("receipt:{}", definitions[index - 1].0)];
    }
    let edges = definitions
        .windows(2)
        .map(|pair| {
            let mut edge =
                RunEdge::new(format!("{}-{}", pair[0].0, pair[1].0), pair[0].0, pair[1].0).unwrap();
            edge.parent_receipt = Some(format!("receipt:{}", pair[0].0));
            edge
        })
        .collect();
    RunGraph {
        schema_version: RunGraph::SCHEMA_VERSION.into(),
        run_id: RunId::new(RUN_ID).unwrap(),
        objective_id: ObjectiveId::new("objective-javascript-golden").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: contract_digest.into(),
            created_by: "u2-javascript-golden".into(),
            parent_receipts: Vec::new(),
        },
    }
}

fn complete_node(store: &RunStore, graph: &mut RunGraph, id: &str, receipt: &str) {
    for (suffix, state) in [
        ("ready", NodeState::Ready),
        ("running", NodeState::Running),
        ("succeeded", NodeState::Succeeded),
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

fn node_executable() -> PathBuf {
    let output = Command::new("which")
        .arg("node")
        .output()
        .expect("locate node");
    assert!(
        output.status.success(),
        "node is required for JavaScript golden proof"
    );
    fs::canonicalize(String::from_utf8(output.stdout).unwrap().trim()).unwrap()
}

#[tokio::test]
async fn clean_javascript_repository_completes_approved_vertical_slice() {
    let started = Instant::now();
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("project");
    fs::create_dir(&project).unwrap();
    copy_fixture(&project);
    git(&project, &["init", "--quiet"]);
    git(&project, &["config", "user.name", "Arda JavaScript Test"]);
    git(
        &project,
        &["config", "user.email", "javascript@arda.invalid"],
    );
    git(&project, &["add", "."]);
    git(&project, &["commit", "--quiet", "-m", "fixture"]);

    let contract_bytes = fs::read(project.join("arda-project.json")).unwrap();
    let contract = ProjectContract::from_json_str(std::str::from_utf8(&contract_bytes).unwrap())
        .expect("attach valid JavaScript project contract");
    assert_eq!(contract.identity.kind, "javascript");
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
                approval_id: "golden-javascript-approval".into(),
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

    let sdk_entry =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sdk/javascript/src/index.js");
    let mutation_count = temp.path().join("javascript-mutation-count");
    let adapter = JsonlAdapter::new(AdapterProcessConfig {
        executable: node_executable(),
        args: vec![project.join("adapter.mjs").display().to_string()],
        expected_adapter: "arda-javascript-golden".into(),
        expected_adapter_version: "1.0.0".into(),
        project_root: project.clone(),
        cwd: project.clone(),
        environment: BTreeMap::from([
            (
                "ARDA_JAVASCRIPT_SDK".into(),
                sdk_entry.display().to_string(),
            ),
            (
                "ARDA_GOLDEN_MUTATION_COUNT".into(),
                mutation_count.display().to_string(),
            ),
        ]),
        environment_allowlist: BTreeSet::from([
            "ARDA_JAVASCRIPT_SDK".into(),
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
                id: "javascript-invalid-attempt".into(),
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
                id: "javascript-recovered-attempt".into(),
                operation: "mutate_and_test".into(),
                arguments: json!({"before": "hello", "after": "hello, Arda"}),
                timeout: Duration::from_secs(20),
                required_capabilities: BTreeSet::from(["mutate_and_test".into()]),
                idempotency_key: format!("{RUN_ID}:execute:attempt-2"),
                recovery_token: Some("javascript-golden-recovery-1".into()),
            },
            AdapterCancellation::new(),
        )
        .await
        .unwrap();
    assert_eq!(result.status, AdapterStatus::Succeeded);
    assert_eq!(
        result.recovery_token.as_deref(),
        Some("javascript-golden-recovery-1")
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

    let independent = Command::new(node_executable())
        .arg("--test")
        .current_dir(&project)
        .output()
        .unwrap();
    assert!(
        independent.status.success(),
        "{}",
        String::from_utf8_lossy(&independent.stderr)
    );
    let diff = git(
        &project,
        &["diff", "--", "src/greeting.js", "test/greeting.test.js"],
    );
    assert!(diff.contains("hello, Arda"));
    assert!(graph
        .nodes
        .iter()
        .all(|node| node.state == NodeState::Succeeded));
    assert_eq!(store.recover().unwrap().checkpoint.unwrap(), graph);

    store.write_result(&json!({
        "schema_version": "arda.workbench-golden-result.v1",
        "run_id": RUN_ID,
        "project": {"kind": "javascript", "contract_digest": contract_digest, "clean_before_attach": true},
        "adapter": "javascript-reference",
        "receipt_digest": adapter_receipt,
        "approval": {"receipt": APPROVAL_RECEIPT},
        "diff": diff,
        "test": {"command": "node --test", "status": "passed"},
        "recovery_token": result.recovery_token,
        "observable_mutations": 1,
        "elapsed_ms": started.elapsed().as_millis(),
    })).unwrap();
    assert_eq!(
        store.read_result().unwrap().unwrap()["observable_mutations"],
        1
    );
}
