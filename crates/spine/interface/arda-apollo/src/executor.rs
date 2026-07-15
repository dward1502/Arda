// sigil: REPAIR
use arda_core::Task;
use annunimas_governance::{
    bacon_lite_validate, calculate_resonance_basic, triad_validate, BaconLiteResult,
    ResonanceScore, TriadResult,
};
use annunimas_plutus::LoveEquation;
use annunimas_warden::evaluate_execution_harness;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub const APOLLO_EXECUTOR_SCHEMA_VERSION: &str = "annunimas.apollo.executor.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    pub task_id: String,
    pub agent_id: String,
    pub payload: serde_json::Value,
    pub priority: ExecutionPriority,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ExecutionPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub task_id: String,
    pub agent_id: String,
    pub status: ExecutionStatus,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub joule_work: f64,
    pub governance: Option<ExecutionGovernance>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveEquationGuard {
    pub resonance: f64,
    pub attention: f64,
    pub reciprocity: f64,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionGovernance {
    pub triad: TriadResult,
    pub bacon_lite: BaconLiteResult,
    pub resonance: ResonanceScore,
    pub love_equation_guard: LoveEquationGuard,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterruptionAttachment {
    pub interruption_id: String,
    pub task_id: String,
    pub source: String,
    pub sender: String,
    pub content: String,
    pub disposition: String,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct InterruptionAttachmentRequest<'a> {
    pub task_id: &'a str,
    pub source: &'a str,
    pub sender: &'a str,
    pub content: &'a str,
    pub disposition: &'a str,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPlan {
    pub tool_id: String,
    pub purpose: String,
    pub phase: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Timeout,
}

pub struct ApolloExecutor {
    queue: Arc<RwLock<HashMap<String, ExecutionRequest>>>,
    results: Arc<RwLock<HashMap<String, ExecutionResult>>>,
    running: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    interruptions: Arc<RwLock<HashMap<String, Vec<InterruptionAttachment>>>>,
}

impl ApolloExecutor {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(HashMap::new())),
            interruptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn submit(&self, request: ExecutionRequest) -> String {
        let task_id = request.task_id.clone();
        let agent_id = request.agent_id.clone();
        let mut queue = self.queue.write().await;
        queue.insert(task_id.clone(), request);

        let result = ExecutionResult {
            task_id: task_id.clone(),
            agent_id,
            status: ExecutionStatus::Pending,
            output: None,
            error: None,
            started_at: chrono::Utc::now(),
            completed_at: None,
            joule_work: 0.0,
            governance: None,
        };

        let mut results = self.results.write().await;
        results.insert(task_id.clone(), result);

        task_id
    }

    pub async fn execute(&self, task_id: &str) -> Option<ExecutionResult> {
        let request = {
            let mut queue = self.queue.write().await;
            queue.remove(task_id)?
        };

        let mut result = self.results.write().await.get_mut(task_id)?.clone();

        result.status = ExecutionStatus::Running;
        result.started_at = chrono::Utc::now();

        {
            let mut results = self.results.write().await;
            results.insert(task_id.to_string(), result.clone());
        }

        let (output, governance, joule_work) = self.run_task(&request).await;

        result.output = Some(output);
        result.status = ExecutionStatus::Completed;
        result.completed_at = Some(chrono::Utc::now());
        result.joule_work = joule_work;
        result.governance = Some(governance);

        let mut results = self.results.write().await;
        results.insert(task_id.to_string(), result.clone());

        Some(result)
    }

    async fn run_task(
        &self,
        request: &ExecutionRequest,
    ) -> (serde_json::Value, ExecutionGovernance, f64) {
        let tool_plan = discover_tool_plan(&request.payload);
        let harness_policy =
            evaluate_execution_harness(&request.payload, Some(priority_label(request.priority)));
        let approval_required = harness_policy.approval_required;
        let joule_work = estimate_joule_work(request.priority, tool_plan.len(), approval_required);
        let governance = evaluate_governance(request, joule_work);
        let output = serde_json::json!({
            "task_id": request.task_id,
            "agent_id": request.agent_id,
            "executed": true,
            "priority": request.priority,
            "discovered_tools": tool_plan,
            "execution_sequence": execution_sequence(&request.payload),
            "harness_policy": harness_policy,
            "governance": governance,
        });
        (output, governance, joule_work)
    }

    pub async fn get_result(&self, task_id: &str) -> Option<ExecutionResult> {
        let results = self.results.read().await;
        results.get(task_id).cloned()
    }

    pub async fn all_results(&self) -> Vec<ExecutionResult> {
        let mut results = self
            .results
            .read()
            .await
            .values()
            .cloned()
            .collect::<Vec<_>>();
        results.sort_by(|a, b| a.task_id.cmp(&b.task_id));
        results
    }

    pub async fn cancel(&self, task_id: &str) -> bool {
        {
            let mut queue = self.queue.write().await;
            if queue.remove(task_id).is_some() {
                let mut results = self.results.write().await;
                if let Some(result) = results.get_mut(task_id) {
                    result.status = ExecutionStatus::Cancelled;
                    result.completed_at = Some(chrono::Utc::now());
                }
                return true;
            }
        }

        let mut running = self.running.write().await;
        if let Some(handle) = running.remove(task_id) {
            handle.abort();
            let mut results = self.results.write().await;
            if let Some(result) = results.get_mut(task_id) {
                result.status = ExecutionStatus::Cancelled;
                result.completed_at = Some(chrono::Utc::now());
            }
            return true;
        }

        false
    }

    pub async fn queue_size(&self) -> usize {
        let queue = self.queue.read().await;
        queue.len()
    }

    pub async fn pending_tasks(&self) -> Vec<String> {
        let queue = self.queue.read().await;
        let mut tasks = queue.keys().cloned().collect::<Vec<_>>();
        tasks.sort();
        tasks
    }

    pub async fn attach_interrupt(
        &self,
        request: InterruptionAttachmentRequest<'_>,
    ) -> Option<InterruptionAttachment> {
        let known = {
            let queue = self.queue.read().await;
            let results = self.results.read().await;
            let running = self.running.read().await;
            queue.contains_key(request.task_id)
                || results.contains_key(request.task_id)
                || running.contains_key(request.task_id)
        };
        if !known {
            return None;
        }
        let record = InterruptionAttachment {
            interruption_id: format!("int_{}", &uuid::Uuid::new_v4().simple().to_string()[..10]),
            task_id: request.task_id.to_string(),
            source: request.source.to_string(),
            sender: request.sender.to_string(),
            content: request.content.to_string(),
            disposition: request.disposition.to_string(),
            run_id: request.run_id,
            session_id: request.session_id,
            created_at: chrono::Utc::now(),
        };
        let mut interruptions = self.interruptions.write().await;
        interruptions
            .entry(request.task_id.to_string())
            .or_default()
            .push(record.clone());
        Some(record)
    }

    pub async fn interruptions_for(&self, task_id: &str) -> Vec<InterruptionAttachment> {
        let interruptions = self.interruptions.read().await;
        interruptions.get(task_id).cloned().unwrap_or_default()
    }

    pub async fn total_interruptions(&self) -> usize {
        let interruptions = self.interruptions.read().await;
        interruptions.values().map(|items| items.len()).sum()
    }

    pub async fn status_snapshot(&self) -> serde_json::Value {
        let mut pending = {
            let queue = self.queue.read().await;
            queue.keys().cloned().collect::<Vec<_>>()
        };
        pending.sort();
        let queue_depth = pending.len();

        let mut results = {
            let results = self.results.read().await;
            results.values().cloned().collect::<Vec<_>>()
        };
        results.sort_by(|a, b| a.task_id.cmp(&b.task_id));

        let interruptions = self.interruptions.read().await;
        let interruptions_total = interruptions
            .values()
            .map(|items| items.len())
            .sum::<usize>();
        let interruption_rows = interruptions
            .iter()
            .map(|(task_id, records)| {
                json!({
                    "task_id": task_id,
                    "count": records.len(),
                    "latest": records.last(),
                })
            })
            .collect::<Vec<_>>();
        drop(interruptions);

        let completed = results
            .iter()
            .filter(|result| result.status == ExecutionStatus::Completed)
            .count();
        let cancelled = results
            .iter()
            .filter(|result| result.status == ExecutionStatus::Cancelled)
            .count();
        let total_joule_work = results.iter().map(|result| result.joule_work).sum::<f64>();
        let discovered_tools_total = results
            .iter()
            .filter_map(|result| result.output.as_ref())
            .filter_map(|output| output.get("discovered_tools").and_then(|v| v.as_array()))
            .map(|tools| tools.len())
            .sum::<usize>();
        let approvals_required_total = results
            .iter()
            .filter_map(|result| result.output.as_ref())
            .filter_map(|output| output.get("harness_policy"))
            .filter(|policy| {
                policy
                    .get("approval_required")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
            })
            .count();
        let triad_passed_total = results
            .iter()
            .filter_map(|result| result.governance.as_ref())
            .filter(|governance| governance.triad.passed)
            .count();
        let bacon_lite_passed_total = results
            .iter()
            .filter_map(|result| result.governance.as_ref())
            .filter(|governance| governance.bacon_lite.passed)
            .count();
        let resonance_values = results
            .iter()
            .filter_map(|result| result.governance.as_ref())
            .map(|governance| governance.resonance.value)
            .collect::<Vec<_>>();
        let average_resonance = if resonance_values.is_empty() {
            0.0
        } else {
            resonance_values.iter().sum::<f64>() / resonance_values.len() as f64
        };
        let love_equation_average = {
            let values = results
                .iter()
                .filter_map(|result| result.governance.as_ref())
                .map(|governance| governance.love_equation_guard.score)
                .collect::<Vec<_>>();
            if values.is_empty() {
                0.0
            } else {
                values.iter().sum::<f64>() / values.len() as f64
            }
        };

        json!({
            "schema_version": APOLLO_EXECUTOR_SCHEMA_VERSION,
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "queue": {
                "pending_tasks": pending,
                "depth": queue_depth,
            },
            "results": results,
            "interruptions": interruption_rows,
            "summary": {
                "results_total": results.len(),
                "completed_total": completed,
                "cancelled_total": cancelled,
                "interruptions_total": interruptions_total,
                "joule_work_total": total_joule_work,
                "discovered_tools_total": discovered_tools_total,
                "approvals_required_total": approvals_required_total,
                "triad_passed_total": triad_passed_total,
                "bacon_lite_passed_total": bacon_lite_passed_total,
                "average_resonance": average_resonance,
                "love_equation_average": love_equation_average,
            }
        })
    }
}

fn estimate_joule_work(
    priority: ExecutionPriority,
    discovered_tools: usize,
    approval_required: bool,
) -> f64 {
    let base = match priority {
        ExecutionPriority::Low => 0.5,
        ExecutionPriority::Normal => 0.9,
        ExecutionPriority::High => 1.2,
        ExecutionPriority::Critical => 1.6,
    };
    let tool_load = discovered_tools as f64 * 0.15;
    let approval_load = if approval_required { 0.25 } else { 0.0 };
    (base + tool_load + approval_load).max(0.25)
}

fn evaluate_governance(request: &ExecutionRequest, joule_work: f64) -> ExecutionGovernance {
    let task = build_governance_task(request, joule_work);
    let triad = triad_validate(&task, None);
    let bacon_lite = bacon_lite_validate(&task);
    let resonance = calculate_resonance_basic(&task);
    let resonance_norm = (resonance.value / 100.0).clamp(0.0, 1.0);
    let attention = bacon_lite.confidence.clamp(0.0, 1.0);
    let reciprocity =
        ((triad.sun_tzu_score + if triad.passed { 0.85 } else { 0.45 }) / 2.0).clamp(0.0, 1.0);
    let score = LoveEquation::new().calculate(
        "apollo",
        &request.agent_id,
        resonance_norm,
        attention,
        reciprocity,
    );

    ExecutionGovernance {
        triad,
        bacon_lite,
        resonance,
        love_equation_guard: LoveEquationGuard {
            resonance: resonance_norm,
            attention,
            reciprocity,
            score,
        },
    }
}

fn build_governance_task(request: &ExecutionRequest, joule_work: f64) -> Task {
    let description = request
        .payload
        .get("description")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned)
        .or_else(|| {
            request
                .payload
                .get("task")
                .and_then(|v| v.as_str())
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| {
            format!(
                "execute {} for {}",
                inferred_task_type(&request.payload),
                request.agent_id
            )
        });
    let task_type = inferred_task_type(&request.payload);
    let mut task = Task::new(description, task_type);
    task.assign(request.agent_id.clone());
    task.execution_started_at = Some(task.created_at + chrono::TimeDelta::seconds(1));
    task.updated_at = task.created_at + chrono::TimeDelta::seconds(3);
    task.joule_cost_estimated = estimated_budget_for_priority(request.priority);
    task.joule_cost_actual = joule_work;
    task.clarifications_requested = request
        .payload
        .get("clarifications_requested")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    task.clarifications_resolved = request
        .payload
        .get("clarifications_resolved")
        .and_then(|v| v.as_u64())
        .unwrap_or(task.clarifications_requested as u64) as u32;
    task.status = arda_core::task::TaskStatus::Complete;
    task
}

fn inferred_task_type(payload: &serde_json::Value) -> String {
    payload
        .get("task_type")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("op").and_then(|v| v.as_str()))
        .unwrap_or("execution")
        .to_string()
}

fn estimated_budget_for_priority(priority: ExecutionPriority) -> f64 {
    match priority {
        ExecutionPriority::Low => 0.75,
        ExecutionPriority::Normal => 1.0,
        ExecutionPriority::High => 1.35,
        ExecutionPriority::Critical => 1.75,
    }
}

fn priority_label(priority: ExecutionPriority) -> &'static str {
    match priority {
        ExecutionPriority::Low => "low",
        ExecutionPriority::Normal => "normal",
        ExecutionPriority::High => "high",
        ExecutionPriority::Critical => "critical",
    }
}

fn discover_tool_plan(payload: &serde_json::Value) -> Vec<ToolPlan> {
    let mut plans = Vec::new();
    let text = payload.to_string().to_ascii_lowercase();
    let required_capabilities = payload
        .get("required_capabilities")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    if text.contains("repo")
        || text.contains("workspace")
        || text.contains("git")
        || required_capabilities
            .iter()
            .any(|v| v.as_str() == Some("repository"))
    {
        plans.push(ToolPlan {
            tool_id: "repo_inspector".to_string(),
            purpose: "Inspect repository state and changed surfaces".to_string(),
            phase: "discovery".to_string(),
        });
    }
    if text.contains("test")
        || text.contains("cargo")
        || text.contains("pytest")
        || required_capabilities
            .iter()
            .any(|v| v.as_str() == Some("verification"))
    {
        plans.push(ToolPlan {
            tool_id: "verification_runner".to_string(),
            purpose: "Run targeted verification after execution".to_string(),
            phase: "verification".to_string(),
        });
    }
    if text.contains("http://")
        || text.contains("https://")
        || required_capabilities
            .iter()
            .any(|v| v.as_str() == Some("network"))
    {
        plans.push(ToolPlan {
            tool_id: "network_probe".to_string(),
            purpose: "Access remote endpoints under restricted policy".to_string(),
            phase: "fetch".to_string(),
        });
    }
    if text.contains("shell")
        || text.contains("command")
        || text.contains("bash")
        || required_capabilities
            .iter()
            .any(|v| v.as_str() == Some("shell"))
    {
        plans.push(ToolPlan {
            tool_id: "shell_exec".to_string(),
            purpose: "Execute shell steps under harness constraints".to_string(),
            phase: "execution".to_string(),
        });
    }
    if plans.is_empty() {
        plans.push(ToolPlan {
            tool_id: "task_router".to_string(),
            purpose: "Sequence generic execution for the provided payload".to_string(),
            phase: "planning".to_string(),
        });
    }
    plans
}

fn execution_sequence(payload: &serde_json::Value) -> Vec<String> {
    let has_verification = payload.to_string().to_ascii_lowercase().contains("test");
    let mut sequence = vec![
        "inspect_inputs".to_string(),
        "discover_tools".to_string(),
        "execute_under_harness".to_string(),
    ];
    if has_verification {
        sequence.push("verify_outputs".to_string());
    }
    sequence.push("record_results".to_string());
    sequence
}

impl Default for ApolloExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ApolloExecutor, ExecutionPriority, ExecutionRequest, ExecutionStatus,
        InterruptionAttachmentRequest, APOLLO_EXECUTOR_SCHEMA_VERSION,
    };

    #[tokio::test]
    async fn attaches_interrupt_to_known_task() {
        let executor = ApolloExecutor::new();
        let task_id = "task_1".to_string();
        let _ = executor
            .submit(ExecutionRequest {
                task_id: task_id.clone(),
                agent_id: "athena".to_string(),
                payload: serde_json::json!({"op":"ingest"}),
                priority: ExecutionPriority::Normal,
                timeout_secs: 30,
            })
            .await;

        let rec = executor
            .attach_interrupt(InterruptionAttachmentRequest {
                task_id: &task_id,
                source: "voice",
                sender: "operator",
                content: "reroute to backlog cleanup",
                disposition: "reroute",
                run_id: Some("run_1".to_string()),
                session_id: None,
            })
            .await;
        assert!(rec.is_some());
        assert_eq!(executor.total_interruptions().await, 1);
        let entries = executor.interruptions_for(&task_id).await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].disposition, "reroute");
    }

    #[tokio::test]
    async fn executes_task_and_emits_runtime_snapshot() {
        let executor = ApolloExecutor::new();
        let task_id = "task_exec".to_string();
        executor
            .submit(ExecutionRequest {
                task_id: task_id.clone(),
                agent_id: "apollo".to_string(),
                payload: serde_json::json!({
                    "op":"workflow",
                    "task_type":"dispatch",
                    "description":"execute workflow using https://example.com evidence because source verification passed"
                }),
                priority: ExecutionPriority::High,
                timeout_secs: 60,
            })
            .await;

        let result = executor.execute(&task_id).await.expect("executed");
        assert_eq!(result.status, ExecutionStatus::Completed);
        assert!(result.joule_work >= 0.25);
        assert!(result.governance.is_some());

        let snapshot = executor.status_snapshot().await;
        assert_eq!(snapshot["schema_version"], APOLLO_EXECUTOR_SCHEMA_VERSION);
        assert_eq!(snapshot["summary"]["completed_total"], 1);
        assert!(
            snapshot["summary"]["joule_work_total"]
                .as_f64()
                .unwrap_or_default()
                >= 0.25
        );
        assert_eq!(snapshot["queue"]["depth"], 0);
        assert_eq!(snapshot["summary"]["triad_passed_total"], 1);
        assert_eq!(snapshot["summary"]["bacon_lite_passed_total"], 1);
    }

    #[tokio::test]
    async fn cancel_updates_snapshot_counts() {
        let executor = ApolloExecutor::new();
        let task_id = "task_cancel".to_string();
        executor
            .submit(ExecutionRequest {
                task_id: task_id.clone(),
                agent_id: "apollo".to_string(),
                payload: serde_json::json!({"op":"cancel"}),
                priority: ExecutionPriority::Normal,
                timeout_secs: 30,
            })
            .await;

        assert!(executor.cancel(&task_id).await);
        let result = executor.get_result(&task_id).await.expect("result");
        assert_eq!(result.status, ExecutionStatus::Cancelled);

        let snapshot = executor.status_snapshot().await;
        assert_eq!(snapshot["summary"]["cancelled_total"], 1);
    }
}
