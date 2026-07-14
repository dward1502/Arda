use annunimas_charon::types::{RouteGovernance, RouteLoveEquationGuard};
use annunimas_charon::RouteDecision;
use annunimas_core::agent::Agent;
use annunimas_core::error::Result as AnnunimasResult;
use annunimas_core::ledger::Ledger;
use annunimas_core::router::Router;
use annunimas_core::task::{Task, TaskStatus};
use annunimas_governance::{triad_validate, TriadConfig};
use annunimas_prometheus::autopilot::decomposer::{Objective, PlannedTask, Priority};
use annunimas_prometheus::autopilot::governance_policy::{
    GovernanceDecision, GovernanceGate, GovernancePolicy, TriadGateScore, TriadQuorumEvidence,
};
use annunimas_prometheus::Pipeline;
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use std::env;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const TASK_ID: &str = "33333333-3333-4333-8333-333333333333";
const HERMES_AGENT: &str = "hermes_mesh_smoke_agent";

struct HermesMeshSmokeAgent;

#[async_trait]
impl Agent for HermesMeshSmokeAgent {
    fn name(&self) -> &str {
        HERMES_AGENT
    }

    fn capabilities(&self) -> &[&str] {
        &["dispatch"]
    }

    async fn execute(&self, task: &mut Task) -> AnnunimasResult<()> {
        task.start_execution();
        task.complete(json!({
            "status": "completed",
            "surface": "hermes_simulated_dispatch",
            "message": "deterministic Gate 3 mesh smoke dispatch complete",
            "task_id": task.id
        }));
        Ok(())
    }
}

#[tokio::test]
async fn gate3_end_to_end_mesh_proof_is_reloadable_and_isolated() -> anyhow::Result<()> {
    let root = artifact_root()?;
    prepare_root(&root)?;
    env::set_var("ANNUNIMAS_ROOT", &root);

    let evidence_path = root.join("data/mesh_smoke/evidence.jsonl");
    let queue_path = root.join("core/projects/tasks/queue.jsonl");
    let route_path = root.join("core/state/charon_route_decision.json");
    let governance_path = root.join("core/state/governance_decision.json");
    let governance_gates_path = root.join("core/state/governance_gate_decisions.jsonl");
    let final_task_path = root.join("data/mesh_smoke/final_task.json");
    let reload_summary_path = root.join("data/mesh_smoke/reload_summary.json");
    create_parent_dirs(&[
        &evidence_path,
        &queue_path,
        &route_path,
        &governance_path,
        &governance_gates_path,
        &final_task_path,
        &reload_summary_path,
    ])?;

    let mut task = deterministic_task()?;
    append_jsonl(
        &queue_path,
        &json!({
            "event": "task_submitted",
            "task_id": task.id,
            "task_type": task.task_type,
            "description": task.description,
            "origin": "gate3_mesh_smoke"
        }),
    )?;
    append_evidence(
        &evidence_path,
        "task_submitted",
        &task.id.to_string(),
        json!({
            "queue_path": queue_path,
            "isolated_root": root,
        }),
    )?;

    let triad = triad_validate(&task, Some(&TriadConfig::default()));
    anyhow::ensure!(triad.passed, "fixture task must pass triad governance");

    let governance_gate_decisions = governance_gate_decisions();
    for decision in &governance_gate_decisions {
        append_jsonl(&governance_gates_path, &serde_json::to_value(decision)?)?;
    }
    let human_required = governance_gate_decisions
        .iter()
        .find(|decision| matches!(decision.gate, GovernanceGate::HumanRequired))
        .ok_or_else(|| anyhow::anyhow!("missing human-required governance fixture"))?;
    let triad_without_evidence = governance_gate_decisions
        .iter()
        .find(|decision| {
            matches!(decision.gate, GovernanceGate::TriadQuorumRequired)
                && decision.triad_quorum.is_none()
        })
        .ok_or_else(|| anyhow::anyhow!("missing triad-without-evidence governance fixture"))?;
    let triad_with_evidence = governance_gate_decisions
        .iter()
        .find(|decision| matches!(decision.gate, GovernanceGate::TriadQuorumApproved))
        .ok_or_else(|| anyhow::anyhow!("missing triad-approved governance fixture"))?;
    let safe_autonomous = governance_gate_decisions
        .iter()
        .find(|decision| matches!(decision.gate, GovernanceGate::SafeAutonomous))
        .ok_or_else(|| anyhow::anyhow!("missing safe-autonomous governance fixture"))?;
    anyhow::ensure!(
        !human_required.allowed_to_delegate && human_required.requires_human,
        "human-required fixture must block delegation before execution"
    );
    anyhow::ensure!(
        !triad_without_evidence.allowed_to_delegate && triad_without_evidence.requires_triad,
        "triad fixture without ORACLE evidence must block delegation before execution"
    );
    anyhow::ensure!(
        safe_autonomous.allowed_to_delegate && triad_with_evidence.allowed_to_delegate,
        "safe and approved triad fixtures must allow delegation"
    );
    append_evidence(
        &evidence_path,
        "ceo_loop_governance_gates_verified",
        &task.id.to_string(),
        json!({
            "governance_gates_path": governance_gates_path,
            "records": governance_gate_decisions.len(),
            "blocked_before_delegation": [
                human_required.objective_id,
                triad_without_evidence.objective_id,
            ],
            "approved_for_delegation": [
                safe_autonomous.objective_id,
                triad_with_evidence.objective_id,
            ],
        }),
    )?;

    let route_decision = RouteDecision {
        provider_id: "local-mesh-smoke".to_string(),
        model_id: "hermes-simulated-dispatch".to_string(),
        reason: "deterministic dispatch task routed to local Hermes-compatible smoke agent"
            .to_string(),
        route_class: "local_deterministic".to_string(),
        execution_lane: "ci_safe".to_string(),
        context_window_target: 4096,
        governance: RouteGovernance {
            triad_passed: triad.passed,
            triad_aurelius_score: triad.aurelius_score,
            triad_bacon_score: triad.bacon_score,
            triad_sun_tzu_score: triad.sun_tzu_score,
            love_equation_guard: RouteLoveEquationGuard {
                resonance: 0.91,
                attention: 0.88,
                reciprocity: 0.86,
                score: 0.8833,
            },
            ..RouteGovernance::default()
        },
        route_id: format!("gate3-route-{}", task.id),
    };
    write_pretty_json(&route_path, &route_decision)?;
    append_evidence(
        &evidence_path,
        "route_selected",
        &task.id.to_string(),
        json!({
            "route_path": route_path,
            "provider_id": route_decision.provider_id,
            "model_id": route_decision.model_id,
            "route_id": route_decision.route_id,
        }),
    )?;

    let governance_decision = json!({
        "decision_id": format!("gate3-governance-{}", task.id),
        "task_id": task.id,
        "actor": "prometheus_mesh_smoke",
        "scope": "gate3_end_to_end_mesh_proof",
        "decision": "approved",
        "reason": "fixture dispatch task has evidence-bearing description and deterministic local route",
        "timestamp": Utc::now().to_rfc3339(),
        "triad": triad,
        "mutating_actions_authorized": false
    });
    write_pretty_json(&governance_path, &governance_decision)?;
    append_evidence(
        &evidence_path,
        "governance_recorded",
        &task.id.to_string(),
        json!({
            "governance_path": governance_path,
            "decision": governance_decision["decision"],
            "mutating_actions_authorized": false,
        }),
    )?;

    let ledger = Ledger::new(root.join("data/ceo/pipeline_ledger"))?;
    let mut router = Router::new();
    router.register(Box::new(HermesMeshSmokeAgent));
    let pipeline = Pipeline::new(router, ledger, 1_000);
    let completed = pipeline.submit(task.clone()).await?;
    task = completed;
    anyhow::ensure!(
        matches!(task.status, TaskStatus::Complete),
        "pipeline task did not complete"
    );
    anyhow::ensure!(
        task.assigned_agent.as_deref() == Some(HERMES_AGENT),
        "task was not assigned to Hermes smoke agent"
    );
    write_pretty_json(&final_task_path, &task)?;
    append_evidence(
        &evidence_path,
        "pipeline_completed",
        &task.id.to_string(),
        json!({
            "final_task_path": final_task_path,
            "assigned_agent": task.assigned_agent,
            "status": task.status,
        }),
    )?;

    drop(pipeline);

    let reloaded_task: Task = read_json(&final_task_path)?;
    let reloaded_route: RouteDecision = read_json(&route_path)?;
    let reloaded_governance: Value = read_json(&governance_path)?;
    let reloaded_governance_gates = read_jsonl(&governance_gates_path)?;
    let evidence_events = read_jsonl(&evidence_path)?;
    let pipeline_events = read_pipeline_ledger_events(&root.join("data/ceo/pipeline_ledger"))?;
    let completion_count = pipeline_events
        .iter()
        .filter(|event| {
            event["payload"]["type"] == "task_complete"
                && event["payload"]["task_id"] == json!(TASK_ID)
        })
        .count();

    anyhow::ensure!(
        reloaded_task.id.to_string() == TASK_ID,
        "reloaded task id mismatch"
    );
    anyhow::ensure!(
        reloaded_route.route_id.ends_with(TASK_ID),
        "reloaded route id mismatch"
    );
    anyhow::ensure!(
        reloaded_governance["decision"] == "approved",
        "governance decision not approved"
    );
    anyhow::ensure!(evidence_events.len() >= 5, "missing smoke evidence events");
    anyhow::ensure!(
        reloaded_governance_gates.len() == 4,
        "expected four CEO_LOOP governance gate records, saw {}",
        reloaded_governance_gates.len()
    );
    anyhow::ensure!(
        reloaded_governance_gates.iter().any(|record| {
            record["gate"] == "human_required" && record["allowed_to_delegate"] == false
        }),
        "human-required gate record did not block delegation"
    );
    anyhow::ensure!(
        reloaded_governance_gates.iter().any(|record| {
            record["gate"] == "triad_quorum_required"
                && record["allowed_to_delegate"] == false
                && record["triad_quorum"].is_null()
        }),
        "triad-without-evidence gate record did not block delegation"
    );
    anyhow::ensure!(
        reloaded_governance_gates.iter().any(|record| {
            record["gate"] == "triad_quorum_approved"
                && record["allowed_to_delegate"] == true
                && !record["triad_quorum"].is_null()
        }),
        "triad-with-evidence gate record did not allow delegation"
    );
    anyhow::ensure!(
        completion_count == 1,
        "expected exactly one completion event, saw {completion_count}"
    );

    append_evidence(
        &evidence_path,
        "reload_verified",
        TASK_ID,
        json!({
            "reload_summary_path": reload_summary_path,
            "completion_count": completion_count,
        }),
    )?;
    let final_evidence_events = read_jsonl(&evidence_path)?;
    let reload_summary = json!({
        "ok": true,
        "task_id": TASK_ID,
        "route_id": reloaded_route.route_id,
        "governance_decision": reloaded_governance["decision"],
        "governance_gate_records": reloaded_governance_gates.len(),
        "evidence_events": final_evidence_events.len(),
        "pipeline_ledger_events": pipeline_events.len(),
        "completion_count": completion_count,
        "checked_at": Utc::now().to_rfc3339()
    });
    write_pretty_json(&reload_summary_path, &reload_summary)?;

    println!("Gate 3 mesh smoke artifacts: {}", root.display());
    Ok(())
}

fn artifact_root() -> anyhow::Result<PathBuf> {
    if let Ok(path) = env::var("ANNUNIMAS_MESH_SMOKE_ARTIFACT_DIR") {
        return Ok(PathBuf::from(path));
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| anyhow::anyhow!("could not resolve workspace root"))?;
    Ok(workspace_root.join("target/mesh_smoke/latest"))
}

fn prepare_root(root: &Path) -> anyhow::Result<()> {
    let keep = env::var("ANNUNIMAS_MESH_SMOKE_KEEP_ARTIFACTS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if root.exists() && !keep {
        fs::remove_dir_all(root)?;
    }
    fs::create_dir_all(root.join("core/state"))?;
    fs::create_dir_all(root.join("core/projects/tasks"))?;
    fs::create_dir_all(root.join("data/mesh_smoke"))?;
    fs::create_dir_all(root.join("data/ceo/pipeline_ledger"))?;
    Ok(())
}

fn deterministic_task() -> anyhow::Result<Task> {
    let mut task = Task::new(
        "route dispatch evidence via https://annunimas.local/gate3 because proof 3 needs deterministic source evidence",
        "dispatch",
    );
    task.id = Uuid::parse_str(TASK_ID)?;
    task.plan_id = Some("gate3_end_to_end_mesh_proof".to_string());
    task.plan_step_index = Some(0);
    task.joule_cost_estimated = 10.0;
    task.clarifications_resolved = 1;
    Ok(task)
}

fn governance_gate_decisions() -> Vec<GovernanceDecision> {
    let policy = GovernancePolicy::default();
    let safe = objective(
        "gate3-safe-status",
        "collect provider status for Gate 3 evidence",
        &["action_class:provider_status_check"],
    );
    let human = objective(
        "gate3-human-required",
        "rotate API key credential for a customer account",
        &["action_class:credential_rotation_or_disclosure"],
    );
    let triad = objective(
        "gate3-triad-required",
        "reroute provider traffic for Gate 3 mesh proof",
        &["action_class:provider_reroute"],
    );
    let safe_plan = vec![planned_task(
        "status",
        "Collect provider status",
        Priority::High,
    )];
    let human_plan = vec![planned_task(
        "security",
        "Rotate credential",
        Priority::Critical,
    )];
    let triad_plan = vec![planned_task(
        "routing",
        "Reroute provider traffic",
        Priority::Critical,
    )];
    vec![
        policy.classify_objective(&safe, &safe_plan),
        policy.classify_objective(&human, &human_plan),
        policy.classify_objective(&triad, &triad_plan),
        policy.classify_objective_with_triad_evidence(
            &triad,
            &triad_plan,
            Some(triad_quorum_evidence(&policy)),
        ),
    ]
}

fn objective(id: &str, statement: &str, tags: &[&str]) -> Objective {
    Objective {
        id: id.to_string(),
        statement: statement.to_string(),
        constraints: vec!["CEO_LOOP Phase 2 delegation gate must be machine-checkable".to_string()],
        deadline: None,
        success_criteria: vec![
            "decision record declares allowed_to_delegate before execution".to_string(),
        ],
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
    }
}

fn planned_task(task_type: &str, title: &str, priority: Priority) -> PlannedTask {
    PlannedTask {
        key: format!("gate3-{task_type}"),
        title: title.to_string(),
        task_type: task_type.to_string(),
        depends_on: Vec::new(),
        priority,
        joule_cost: 1.0,
        eta_seconds: 1,
        assigned_agent: Some("prometheus".to_string()),
    }
}

fn triad_quorum_evidence(policy: &GovernancePolicy) -> TriadQuorumEvidence {
    TriadQuorumEvidence {
        source: "oracle_gate".to_string(),
        query_id: "gate3_mesh_smoke::provider_reroute".to_string(),
        outcome: "pass".to_string(),
        resonance: 0.88,
        passed_gates: 3,
        total_gates: 3,
        quorum_ratio: 1.0,
        required_quorum_ratio: policy.triad_quorum_ratio,
        required_pass_rate: policy.triad_required_pass_rate,
        gate_scores: vec![
            TriadGateScore {
                gate: "aurelius".to_string(),
                passed: true,
                score: 0.91,
            },
            TriadGateScore {
                gate: "bacon".to_string(),
                passed: true,
                score: 0.87,
            },
            TriadGateScore {
                gate: "sun_tzu".to_string(),
                passed: true,
                score: 0.86,
            },
        ],
        concerns: Vec::new(),
        triad_philosopher: None,
    }
}

fn create_parent_dirs(paths: &[&PathBuf]) -> anyhow::Result<()> {
    for path in paths {
        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn append_evidence(path: &Path, event: &str, task_id: &str, payload: Value) -> anyhow::Result<()> {
    append_jsonl(
        path,
        &json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event,
            "task_id": task_id,
            "payload": payload
        }),
    )
}

fn append_jsonl(path: &Path, value: &Value) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn write_pretty_json<T: serde::Serialize>(path: &Path, value: &T) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("path has no parent: {}", path.display()))?;
    fs::create_dir_all(parent)?;
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let content = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&content)?)
}

fn read_jsonl(path: &Path) -> anyhow::Result<Vec<Value>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut values = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            values.push(serde_json::from_str(&line)?);
        }
    }
    Ok(values)
}

fn read_pipeline_ledger_events(dir: &Path) -> anyhow::Result<Vec<Value>> {
    let mut events = Vec::new();
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            events.extend(read_jsonl(&path)?);
        }
    }
    Ok(events)
}
