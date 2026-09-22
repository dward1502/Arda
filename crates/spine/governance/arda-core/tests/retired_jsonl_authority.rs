use arda_core::contract::TriadOutcome;
use arda_core::governance_gates::{AffordabilityPolicy, GovernanceGates};
use arda_core::loop_engine::{self, AgentBid, BidBoard, JouleEstimator, TriadConsultant};
use arda_core::state::{self, StateRoot};
use arda_core::task::Task;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            let directory = path.is_dir();
            out.insert(
                path.strip_prefix(root).unwrap().to_owned(),
                if directory {
                    None
                } else {
                    Some(std::fs::read(&path).unwrap())
                },
            );
            if directory {
                visit(root, &path, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

struct NeverCalled;
impl JouleEstimator for NeverCalled {
    fn estimate_for_task(&self, _: &Task) -> f64 {
        panic!("retired dispatcher called estimator")
    }
}
impl TriadConsultant for NeverCalled {
    fn consult(&self, _: &Task) -> TriadOutcome {
        panic!("retired dispatcher consulted triad")
    }
}
impl BidBoard for NeverCalled {
    fn bids_for(&self, _: &Task) -> Vec<AgentBid> {
        panic!("retired dispatcher requested bids")
    }
}
impl AffordabilityPolicy for NeverCalled {
    fn policy_name(&self) -> &'static str {
        panic!("retired dispatcher requested policy")
    }
    fn can_afford(&self, _: f64) -> bool {
        panic!("retired dispatcher evaluated affordability")
    }
}

#[test]
fn retired_core_queue_authority_refuses_before_any_side_effect() {
    for history in ["missing", "malformed", "pending", "completed", "halted"] {
        let root = tempfile::tempdir().unwrap();
        let state = StateRoot::new(root.path().join("core/state"));
        let queue = root.path().join("core/projects/tasks/queue.jsonl");
        let mut task = Task::new("historical fixture", "probe_provider");
        if history == "completed" {
            task.complete(serde_json::json!({"historical": true}));
        }
        if history != "missing" {
            std::fs::create_dir_all(queue.parent().unwrap()).unwrap();
            std::fs::write(
                &queue,
                if history == "malformed" {
                    "invalid historical line\n".into()
                } else {
                    format!("{}\n", serde_json::to_string(&task).unwrap())
                },
            )
            .unwrap();
        }
        if history == "halted" {
            std::fs::create_dir_all(state.root()).unwrap();
            std::fs::write(state.root().join(loop_engine::HALT_FILE_NAME), "halt").unwrap();
        }
        let before = snapshot(root.path());
        let mut errors = vec![state::append_task(&queue, &task).unwrap_err()];
        errors.push(loop_engine::dispatch(&state, &queue).unwrap_err());
        errors.push(loop_engine::dispatch_with_cap(&state, &queue, 0).unwrap_err());
        errors.push(
            loop_engine::dispatch_with_cap_and_estimator(&state, &queue, 1, &NeverCalled)
                .unwrap_err(),
        );
        errors.push(
            loop_engine::dispatch_full(
                &state,
                &queue,
                1,
                &NeverCalled,
                &NeverCalled,
                &NeverCalled,
                &GovernanceGates::permissive(),
            )
            .unwrap_err(),
        );
        errors.push(
            loop_engine::dispatch_full_with_affordability(
                &state,
                &queue,
                1,
                &NeverCalled,
                &NeverCalled,
                &NeverCalled,
                &GovernanceGates::permissive(),
                &NeverCalled,
            )
            .unwrap_err(),
        );
        for error in errors {
            assert!(error.to_string().contains("retired"), "{history}: {error}");
        }
        assert_eq!(
            snapshot(root.path()),
            before,
            "{history} authority mutated history"
        );
        let tasks = state::read_contract_tasks(&queue).unwrap();
        assert_eq!(
            tasks.len(),
            usize::from(!matches!(history, "missing" | "malformed"))
        );
        assert_eq!(
            snapshot(root.path()),
            before,
            "history read provisioned state"
        );
    }
}
