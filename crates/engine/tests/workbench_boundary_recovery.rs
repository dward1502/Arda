use arda_core::run_graph::{
    AuthorityClass, Budget, CheckpointMetadata, NodeId, NodeKind, NodeState, ObjectiveId,
    Provenance, RetryPolicy, RunEdge, RunGraph, RunId, RunNode,
};
use arda_engine::runs::{apply_transition_once, RunEventDraft, RunEventKind, RunStore};
use serde_json::json;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const RUN_ID: &str = "golden-boundary-recovery";
const CRASH_EXIT: i32 = 86;

fn node(id: &str, kind: NodeKind, authority: AuthorityClass, parent: Option<&str>) -> RunNode {
    RunNode {
        id: NodeId::new(id).unwrap(),
        kind,
        state: NodeState::Pending,
        authority,
        budget: Budget {
            max_joules: 100.0,
            max_cost_usd: 0.0,
        },
        retry: RetryPolicy { max_attempts: 2 },
        timeout_ms: 5_000,
        idempotency_key: format!("{RUN_ID}:{id}"),
        input_digest: Some(format!("sha256:{id}-input")),
        output_digest: None,
        parent_receipts: parent.into_iter().map(str::to_string).collect(),
        checkpoint: CheckpointMetadata::default(),
    }
}

fn recovery_graph() -> RunGraph {
    let definitions = [
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
        objective_id: ObjectiveId::new("objective-boundary-recovery").unwrap(),
        nodes,
        edges,
        provenance: Provenance {
            project_contract_digest: "sha256:boundary-recovery-contract".into(),
            created_by: "stage-4-recovery-test".into(),
            parent_receipts: Vec::new(),
        },
    };
    graph.validate().unwrap();
    graph
}

fn node_id(id: &str) -> NodeId {
    NodeId::new(id).unwrap()
}

fn complete_node(store: &RunStore, graph: &mut RunGraph, id: &str) {
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
            Some(format!("receipt:{id}")),
        )
        .unwrap();
    }
}

fn durable_write(path: &Path, bytes: &[u8]) {
    let mut file = File::create(path).unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}

fn read_counter(path: &Path) -> u32 {
    fs::read_to_string(path)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(0)
}

fn increment(path: &Path) {
    durable_write(path, (read_counter(path) + 1).to_string().as_bytes());
}

fn apply_idempotent_mutation(root: &Path) {
    let attempts = root.join("mutation-attempts");
    increment(&attempts);
    let source = root.join("project/src/lib.rs");
    let before = fs::read_to_string(&source).unwrap();
    if before.contains("hello, Arda") {
        return;
    }
    let after = before.replace("hello", "hello, Arda");
    assert_ne!(before, after);
    durable_write(&source, after.as_bytes());
    increment(&root.join("observable-mutations"));
}

fn worker_root() -> PathBuf {
    PathBuf::from(std::env::var_os("ARDA_RECOVERY_ROOT").expect("worker root"))
}

#[test]
#[ignore = "spawned by restart_at_every_graph_boundary_preserves_exact_once_mutation"]
fn boundary_worker() {
    if std::env::var_os("ARDA_RECOVERY_ROOT").is_none() {
        return;
    }
    let root = worker_root();
    let state_root = root.join("state");
    let step_path = root.join("step");
    let step = read_counter(&step_path);
    let store = RunStore::open(&state_root, RunId::new(RUN_ID).unwrap()).unwrap();

    if step == 0 {
        let graph = recovery_graph();
        store
            .append(RunEventDraft {
                node_id: node_id("plan"),
                idempotency_key: format!("{RUN_ID}:planned"),
                kind: RunEventKind::Planned {
                    project_id: "boundary-fixture".into(),
                    approval_id: "boundary-approval".into(),
                },
                receipt_digest: Some("receipt:boundary-approval".into()),
            })
            .unwrap();
        store.write_checkpoint(&graph).unwrap();
        durable_write(&step_path, b"1");
        std::process::exit(CRASH_EXIT);
    }

    let mut graph = store.recover().unwrap().checkpoint.unwrap();
    match step {
        1 => complete_node(&store, &mut graph, "plan"),
        2 => complete_node(&store, &mut graph, "approval"),
        3 => {
            apply_idempotent_mutation(&root);
            let crash_marker = root.join("execute-crashed-before-receipt");
            if !crash_marker.exists() {
                durable_write(&crash_marker, b"uncertain execute boundary\n");
                std::process::exit(CRASH_EXIT);
            }
            complete_node(&store, &mut graph, "execute");
        }
        4 => complete_node(&store, &mut graph, "verify"),
        5 => complete_node(&store, &mut graph, "review"),
        6 => complete_node(&store, &mut graph, "close"),
        7 => {
            let result = json!({
                "schema_version": "arda.workbench-recovery-result.v1",
                "run_id": RUN_ID,
                "crash_boundaries": [
                    "planned", "plan", "approval", "execute-before-receipt",
                    "execute", "verify", "review", "close", "result"
                ],
                "mutation_attempts": read_counter(&root.join("mutation-attempts")),
                "observable_mutations": read_counter(&root.join("observable-mutations")),
                "compensation": null,
                "recovery_behavior": "uncertain execute replayed the idempotent bounded mutation and observed the already-applied target state"
            });
            store.write_result(&result).unwrap();
            store
                .append(RunEventDraft {
                    node_id: node_id("close"),
                    idempotency_key: format!("{RUN_ID}:result-projected"),
                    kind: RunEventKind::ResultProjected,
                    receipt_digest: Some("receipt:result".into()),
                })
                .unwrap();
        }
        8 => return,
        other => panic!("unexpected recovery step {other}"),
    }
    durable_write(&step_path, (step + 1).to_string().as_bytes());
    std::process::exit(CRASH_EXIT);
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

fn initialize_fixture(root: &Path) {
    let project = root.join("project");
    fs::create_dir_all(project.join("src")).unwrap();
    fs::write(
        project.join("src/lib.rs"),
        "pub fn greeting() -> &'static str {\n    \"hello\"\n}\n",
    )
    .unwrap();
    git(&project, &["init", "--quiet"]);
    git(&project, &["config", "user.name", "Arda Recovery Test"]);
    git(&project, &["config", "user.email", "recovery@arda.invalid"]);
    git(&project, &["add", "."]);
    git(
        &project,
        &[
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "clean fixture",
        ],
    );
    assert!(git(&project, &["status", "--porcelain"]).is_empty());
}

fn spawn_worker(executable: &Path, root: &Path) -> std::process::ExitStatus {
    Command::new(executable)
        .args([
            "--ignored",
            "--exact",
            "boundary_worker",
            "--test-threads=1",
        ])
        .env("ARDA_RECOVERY_ROOT", root)
        .status()
        .unwrap()
}

#[test]
fn restart_at_every_graph_boundary_preserves_exact_once_mutation() {
    let temp = TempDir::new().unwrap();
    initialize_fixture(temp.path());
    let executable = std::env::current_exe().unwrap();
    let mut forced_terminations = 0_u32;

    for _ in 0..16 {
        let status = spawn_worker(&executable, temp.path());
        if status.success() {
            break;
        }
        assert_eq!(status.code(), Some(CRASH_EXIT));
        forced_terminations += 1;
    }
    assert_eq!(read_counter(&temp.path().join("step")), 8);
    assert_eq!(forced_terminations, 9);
    assert_eq!(read_counter(&temp.path().join("mutation-attempts")), 2);
    assert_eq!(read_counter(&temp.path().join("observable-mutations")), 1);

    let source = fs::read_to_string(temp.path().join("project/src/lib.rs")).unwrap();
    assert_eq!(source.matches("hello, Arda").count(), 1);
    assert!(!source.contains("Arda, Arda"));
    let diff = git(&temp.path().join("project"), &["diff", "--", "src/lib.rs"]);
    assert_eq!(diff.matches("+    \"hello, Arda\"").count(), 1);

    let store = RunStore::open(temp.path().join("state"), RunId::new(RUN_ID).unwrap()).unwrap();
    let before = store.recover().unwrap();
    let event_count = before.events.len();
    assert_eq!(
        before
            .checkpoint
            .unwrap()
            .nodes
            .iter()
            .filter(|node| node.state == NodeState::Succeeded)
            .count(),
        6
    );
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(store.result_path()).unwrap()).unwrap();
    assert_eq!(result["run_id"], RUN_ID);
    assert_eq!(result["mutation_attempts"], 2);
    assert_eq!(result["observable_mutations"], 1);

    assert!(spawn_worker(&executable, temp.path()).success());
    assert_eq!(store.recover().unwrap().events.len(), event_count);
    assert_eq!(read_counter(&temp.path().join("observable-mutations")), 1);
}
