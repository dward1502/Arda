#![cfg(feature = "full-cli")]
//! Bounded adapter from operator-approved canonical queue work into Workbench.

use super::decomposer::{Objective, ObjectiveContextSource, ObjectiveDecomposer, ObjectivePlan};
use super::execution_outcome::project_terminal_outcome;
use super::task_queue::{
    governance_authorization_id, ActiveQueueExecutor, ApprovedQueueClaim, QueueRecord,
};
use super::validator::PlanValidator;
use anyhow::{anyhow, Context, Result};
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use arda_vaire::{
    ContextDisposition, ContextOutcomeInput, ContextOutcomeReceipt, MnemosyneService,
    OrganismContext,
};
use chrono::Utc;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub const QUEUE_EXECUTION_RECEIPT_CONTRACT: &str = "arda.workbench.queue_execution_receipt.v1";
const DEFAULT_PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const MAX_OBJECTIVE_PLAN_RECEIPT_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExecutionReceipt {
    pub contract: String,
    pub task_id: Option<String>,
    pub workbench_run_id: Option<String>,
    pub status: String,
    pub result: String,
    pub execution_receipt_digest: Option<String>,
    pub continuation_decision: Option<String>,
    pub detail: Option<String>,
    pub recorded_at_utc: String,
}

#[derive(Debug, Clone)]
pub struct WorkbenchQueueExecutor {
    root: PathBuf,
    harness_url: String,
    project_id: String,
    client: reqwest::Client,
}

impl WorkbenchQueueExecutor {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let harness_url =
            std::env::var("ARDA_HARNESS_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into());
        let project_id = std::env::var("ARDA_WORKBENCH_PROJECT_ID")
            .unwrap_or_else(|_| DEFAULT_PROJECT_ID.into());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1_200))
            .build()?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            harness_url: harness_url.trim_end_matches('/').to_owned(),
            project_id,
            client,
        })
    }

    pub async fn execute_once(&self) -> Result<QueueExecutionReceipt> {
        // Hold one root-scoped process lock through claim reconciliation and
        // dispatch. A crash releases it, so the next invocation can recover an
        // unexpired claim without mistaking a live executor for an orphan.
        let _executor_lock = acquire_executor_lock(&self.root)?;
        let queue = ActiveQueueExecutor::new(&self.root);
        let Some(claim) = queue.claim_next_approved_reconciling_orphans()? else {
            return Ok(QueueExecutionReceipt {
                contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                task_id: None,
                workbench_run_id: None,
                status: "idle".into(),
                result: "no_eligible_task".into(),
                execution_receipt_digest: None,
                continuation_decision: None,
                detail: None,
                recorded_at_utc: Utc::now().to_rfc3339(),
            });
        };
        let run_id = claim.attempt.workbench_run_id.clone();
        match self.dispatch_claim(&claim).await {
            Ok((mut status, digest, mut detail)) => {
                if status == "in_progress" {
                    return Ok(QueueExecutionReceipt {
                        contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                        task_id: Some(claim.task.id),
                        workbench_run_id: Some(run_id),
                        status,
                        result: "existing_run_active".into(),
                        execution_receipt_digest: digest,
                        continuation_decision: None,
                        detail,
                        recorded_at_utc: Utc::now().to_rfc3339(),
                    });
                }
                if status == "succeeded" {
                    if let Err(error) = validate_task_acceptance_artifact(&self.root, &claim.task) {
                        status = "failed".into();
                        detail = Some(format!("acceptance criteria not satisfied: {error:#}"));
                    }
                }
                let (queue_status, result) = match status.as_str() {
                    "succeeded" => ("completed", "completed"),
                    "cancelled" => ("failed", "cancelled"),
                    _ => ("failed", "failed"),
                };
                let decision = continuation_decision(
                    queue_status,
                    result,
                    detail.as_deref(),
                    continuation_sequence(&claim.task),
                );
                let mut outcome_evidence = digest.iter().cloned().collect::<Vec<_>>();
                if let Some(receipt) = record_context_outcome(
                    &self.root,
                    &claim.task,
                    &run_id,
                    result,
                    &outcome_evidence,
                )? {
                    outcome_evidence.push(receipt.receipt_digest);
                }
                queue.append_workbench_terminal_with_continuation(
                    &claim.task,
                    queue_status,
                    result,
                    &run_id,
                    digest.as_deref(),
                    detail.as_deref(),
                    Some(decision),
                )?;
                project_terminal_outcome(
                    &self.root,
                    &claim.task,
                    &run_id,
                    queue_status,
                    result,
                    digest.as_deref(),
                    detail.as_deref(),
                )?;
                Ok(QueueExecutionReceipt {
                    contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                    task_id: Some(claim.task.id),
                    workbench_run_id: Some(run_id),
                    status: queue_status.into(),
                    result: result.into(),
                    execution_receipt_digest: digest,
                    continuation_decision: Some(decision.into()),
                    detail,
                    recorded_at_utc: Utc::now().to_rfc3339(),
                })
            }
            Err(error) => {
                let detail = format!("{error:#}");
                if detail.contains("was cancelled while provider execution was active") {
                    let decision = continuation_decision(
                        "failed",
                        "cancelled",
                        Some(&detail),
                        continuation_sequence(&claim.task),
                    );
                    record_context_outcome(&self.root, &claim.task, &run_id, "cancelled", &[])?;
                    queue.append_workbench_terminal_with_continuation(
                        &claim.task,
                        "failed",
                        "cancelled",
                        &run_id,
                        None,
                        Some(&detail),
                        Some(decision),
                    )?;
                    project_terminal_outcome(
                        &self.root,
                        &claim.task,
                        &run_id,
                        "failed",
                        "cancelled",
                        None,
                        Some(&detail),
                    )?;
                    return Ok(QueueExecutionReceipt {
                        contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                        task_id: Some(claim.task.id),
                        workbench_run_id: Some(run_id),
                        status: "failed".into(),
                        result: "cancelled".into(),
                        execution_receipt_digest: None,
                        continuation_decision: Some(decision.into()),
                        detail: Some(detail),
                        recorded_at_utc: Utc::now().to_rfc3339(),
                    });
                }
                // A transport error may occur after the harness durably
                // accepted or completed a node. Preserve the claim and let
                // the deterministic run id reconcile the journal next time.
                Err(error)
            }
        }
    }

    pub async fn cancel_task(&self, task_id: &str, reason: &str) -> Result<Value> {
        let task = self
            .effective_task(task_id)?
            .ok_or_else(|| anyhow!("queue task `{task_id}` was not found"))?;
        let run_id = task
            .extra
            .get("workbench_run_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| workbench_run_id(task_id));
        let envelope = approval_envelope(&task, &format!("cancel-{run_id}"))?;
        let response = self
            .client
            .post(format!("{}/v1/runs/{run_id}/cancel", self.harness_url))
            .json(&json!({"reason": reason, "envelope": envelope}))
            .send()
            .await
            .context("send Workbench cancellation")?;
        let value = response_error(response, "cancel Workbench run").await?;
        ActiveQueueExecutor::new(&self.root).append_workbench_terminal(
            &task,
            "failed",
            "cancelled",
            &run_id,
            None,
            Some(reason),
        )?;
        project_terminal_outcome(
            &self.root,
            &task,
            &run_id,
            "failed",
            "cancelled",
            None,
            Some(reason),
        )?;
        Ok(value)
    }

    fn effective_task(&self, task_id: &str) -> Result<Option<QueueRecord>> {
        let records = super::task_queue::TaskQueueAnalyzer::new(
            self.root.join("core/projects/tasks/queue.jsonl"),
        )
        .load()?;
        Ok(
            super::task_queue::TaskQueueAnalyzer::effective_records(records)
                .into_iter()
                .find(|record| record.id == task_id),
        )
    }

    async fn dispatch_claim(
        &self,
        claim: &ApprovedQueueClaim,
    ) -> Result<(String, Option<String>, Option<String>)> {
        let run_id = &claim.attempt.workbench_run_id;
        let envelope = approval_envelope(&claim.task, &format!("plan-{run_id}"))?;
        let approval_id = envelope["approval"]["approval_id"]
            .as_str()
            .ok_or_else(|| anyhow!("approval id missing"))?;
        let objective = claim
            .task
            .title
            .as_deref()
            .unwrap_or(claim.task.id.as_str());
        let (objective_plan, objective_plan_receipt) =
            persisted_objective_plan_for_task(&self.root, run_id, &claim.task)?;
        let graph = run_graph_with_objective_plan_receipt(
            run_id,
            &claim.task.id,
            objective,
            approval_id,
            &objective_plan_receipt,
        );
        let mut run = if let Some(existing) = self.existing_run(run_id).await? {
            let outcome = classify_existing_run(&existing);
            if outcome.0 != "in_progress" {
                return Ok(outcome);
            }
            if run_has_running_node(&existing) {
                return Ok(outcome);
            }
            existing
        } else {
            let project_id = task_project_id(&claim.task).unwrap_or(&self.project_id);
            let response = self
                .client
                .post(format!("{}/v1/runs/plan", self.harness_url))
                .json(&json!({
                    "project_id": project_id,
                    "graph": graph,
                    "envelope": envelope,
                }))
                .send()
                .await
                .context("connect to the loopback Workbench harness")?;
            response_error(response, "plan approved queue run").await?
        };

        if node_state(&run, "approval") != Some("succeeded") {
            let envelope = approval_envelope(&claim.task, &format!("approve-{run_id}"))?;
            let response = self
                .client
                .post(format!("{}/v1/runs/{run_id}/approve", self.harness_url))
                .json(&json!({"node_id": "approval", "envelope": envelope}))
                .send()
                .await
                .context("submit Workbench approval")?;
            run = response_error(response, "approve queue run").await?;
        }

        for (node_id, stage_objective) in [
            (
                "execute",
                objective_execution_prompt(&objective_plan, objective, &claim.task),
            ),
            (
                "verify",
                format!(
                    "Verify task {} by running every project-native check from the attached project contract; do not modify project files.",
                    claim.task.id
                ),
            ),
        ] {
            if node_state(&run, node_id) == Some("succeeded") {
                continue;
            }
            let envelope = approval_envelope(&claim.task, &format!("{node_id}-{run_id}"))?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{run_id}/nodes/{node_id}/execute-provider",
                    self.harness_url
                ))
                .json(&json!({"objective": stage_objective, "envelope": envelope}))
                .send()
                .await
                .with_context(|| format!("dispatch approved Workbench {node_id} provider"))?;
            let value = response_error(response, &format!("{node_id} approved queue task")).await?;
            if value["receipt"]["status"] != "succeeded" {
                return Ok((
                    value["receipt"]["status"]
                        .as_str()
                        .unwrap_or("failed")
                        .to_owned(),
                    value["receipt"]["receipt_digest"].as_str().map(str::to_owned),
                    value["receipt"]["summary"].as_str().map(str::to_owned),
                ));
            }
            run = value["run"].clone();
            ActiveQueueExecutor::new(&self.root).append_workbench_continuation(
                &claim.task,
                run_id,
                node_id,
                node_output_digest(&run, node_id),
                if node_id == "execute" {
                    "continue_verify"
                } else {
                    "continue_review"
                },
            )?;
            forced_restart_after_stage(node_id);
        }

        require_closure_evidence(&run)?;
        for node_id in ["review", "close"] {
            if node_state(&run, node_id) == Some("succeeded") {
                continue;
            }
            let parent = node_output_digest(
                &run,
                if node_id == "review" {
                    "verify"
                } else {
                    "review"
                },
            )
            .ok_or_else(|| anyhow!("{node_id} omitted its durable parent receipt"))?;
            let receipt_digest = completion_digest(run_id, &claim.task.id, node_id, parent);
            let envelope = approval_envelope(&claim.task, &format!("{node_id}-{run_id}"))?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{run_id}/nodes/{node_id}/complete",
                    self.harness_url
                ))
                .json(&json!({
                    "envelope": envelope,
                    "receipt_digest": receipt_digest,
                }))
                .send()
                .await
                .with_context(|| format!("complete Workbench {node_id} node"))?;
            run = response_error(response, &format!("complete queue {node_id}")).await?;
            if node_id == "review" {
                ActiveQueueExecutor::new(&self.root).append_workbench_continuation(
                    &claim.task,
                    run_id,
                    node_id,
                    node_output_digest(&run, node_id),
                    "continue_close",
                )?;
            }
        }
        Ok(classify_existing_run(&run))
    }

    async fn existing_run(&self, run_id: &str) -> Result<Option<Value>> {
        let response = self
            .client
            .get(format!("{}/v1/runs/{run_id}", self.harness_url))
            .send()
            .await
            .context("inspect existing Workbench run")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let value = response_error(response, "inspect existing Workbench run").await?;
        Ok(Some(value))
    }
}

fn forced_restart_after_stage(stage: &str) {
    if std::env::var("ARDA_WORKBENCH_FORCE_RESTART_AFTER_STAGE")
        .ok()
        .as_deref()
        == Some(stage)
    {
        // Acceptance/recovery failpoint: the provider receipt and run graph are
        // durable, but no later node or queue terminal has been written yet.
        // Process exit is intentional so recovery exercises the same persisted
        // boundary as an unplanned executor crash.
        std::process::exit(86);
    }
}

fn acquire_executor_lock(root: &Path) -> Result<File> {
    let lock_path = root.join("core/projects/tasks/.workbench-queue-executor.lock");
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create executor lock directory `{}`", parent.display()))?;
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open executor lock `{}`", lock_path.display()))?;
    lock.lock_exclusive()
        .with_context(|| format!("acquire executor lock `{}`", lock_path.display()))?;
    Ok(lock)
}

fn classify_existing_run(value: &Value) -> (String, Option<String>, Option<String>) {
    let nodes = value["graph"]["nodes"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let close = nodes.iter().find(|node| node["id"] == "close");
    let digest = close
        .and_then(|node| node["output_digest"].as_str())
        .map(str::to_owned);
    let detail = value["review"]["provider_receipt"]["summary"]
        .as_str()
        .map(str::to_owned);
    let states: Vec<&str> = nodes
        .iter()
        .filter_map(|node| node["state"].as_str())
        .collect();
    let status = if states.contains(&"failed") {
        "failed"
    } else if states.contains(&"cancelled") {
        "cancelled"
    } else if close.is_some_and(|node| node["state"] == "succeeded")
        && closure_evidence_present(value)
    {
        "succeeded"
    } else {
        "in_progress"
    };
    (status.to_owned(), digest, detail)
}

fn node_state<'a>(run: &'a Value, node_id: &str) -> Option<&'a str> {
    run["graph"]["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["id"] == node_id)?["state"]
        .as_str()
}

fn run_has_running_node(run: &Value) -> bool {
    run["graph"]["nodes"]
        .as_array()
        .is_some_and(|nodes| nodes.iter().any(|node| node["state"] == "running"))
}

fn node_output_digest<'a>(run: &'a Value, node_id: &str) -> Option<&'a str> {
    run["graph"]["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["id"] == node_id)?["output_digest"]
        .as_str()
}

fn closure_evidence_present(run: &Value) -> bool {
    run["review"]["provider_receipt"]["receipt_digest"].is_string()
        && run["review"]["tests"].as_array().is_some_and(|tests| {
            !tests.is_empty() && tests.iter().all(|test| test["status"] == "passed")
        })
}

fn require_closure_evidence(run: &Value) -> Result<()> {
    if closure_evidence_present(run) {
        Ok(())
    } else {
        Err(anyhow!(
            "refusing Workbench closure without a provider receipt and passing project-native test evidence"
        ))
    }
}

fn completion_digest(run_id: &str, task_id: &str, stage: &str, parent: &str) -> String {
    format!(
        "sha256:{:x}",
        Sha256::digest(format!("{run_id}\n{task_id}\n{stage}\n{parent}").as_bytes())
    )
}

fn task_project_id(task: &QueueRecord) -> Option<&str> {
    task.extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("project_id"))
        .and_then(Value::as_str)
        .filter(|id| !id.trim().is_empty())
}

fn continuation_sequence(task: &QueueRecord) -> u64 {
    task.extra
        .get("continuation_sequence")
        .and_then(Value::as_u64)
        .unwrap_or(0)
}

fn validate_task_acceptance_artifact(root: &Path, task: &QueueRecord) -> Result<()> {
    let Some(meta) = task.extra.get("meta").and_then(Value::as_object) else {
        return Ok(());
    };
    let Some(relative_path) = meta.get("acceptance_artifact").and_then(Value::as_str) else {
        return Ok(());
    };
    let content = std::fs::read_to_string(root.join(relative_path))
        .with_context(|| format!("read required acceptance artifact `{relative_path}`"))?;
    if content.trim().is_empty() {
        return Err(anyhow!(
            "required acceptance artifact `{relative_path}` is empty"
        ));
    }
    for marker in meta
        .get("acceptance_markers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        if !content
            .to_ascii_lowercase()
            .contains(&marker.to_ascii_lowercase())
        {
            return Err(anyhow!(
                "required acceptance artifact `{relative_path}` omitted marker `{marker}`"
            ));
        }
    }
    Ok(())
}

fn continuation_decision(
    queue_status: &str,
    result: &str,
    detail: Option<&str>,
    prior_continuations: u64,
) -> &'static str {
    if queue_status == "completed" {
        return "close_complete";
    }
    if prior_continuations > 0 {
        return "replan_objective";
    }
    let detail = detail.unwrap_or_default().to_ascii_lowercase();
    if result == "cancelled" {
        "request_operator_decision"
    } else if ["acceptance", "criteria", "scope", "invalid plan"]
        .iter()
        .any(|marker| detail.contains(marker))
    {
        "revise_task"
    } else {
        "retry_same_task"
    }
}

fn record_context_outcome(
    root: &Path,
    task: &QueueRecord,
    run_id: &str,
    result: &str,
    evidence_refs: &[String],
) -> Result<Option<ContextOutcomeReceipt>> {
    let receipt_id = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("context_use_receipt_id"))
        .and_then(Value::as_str);
    let service = MnemosyneService::new(root.join("data/vaire"))?
        .with_contract_memory_root(root.join("core/state/memory"));
    let use_receipt = if let Some(receipt_id) = receipt_id {
        service
            .context_use_receipt(receipt_id)?
            .ok_or_else(|| anyhow!("context use receipt `{receipt_id}` was not found"))?
    } else {
        let now = Utc::now().timestamp_millis().max(0) as u128;
        let context: OrganismContext = serde_json::from_value(json!({
            "schema_version": "arda.organism-context.v1",
            "organism_id": "arda",
            "generated_at_unix_ms": now,
            "expires_at_unix_ms": now.saturating_add(3_600_000),
            "consumer": {
                "consumer_id": "arda_workbench.queue_executor",
                "role": "worker",
                "authority_ceiling": "execute_with_approval",
                "operator_authorized": true,
                "memory_domains": ["system"],
                "data_classes": ["internal"],
                "permitted_egress": ["local_device"],
                "compute_node_refs": [],
                "agent_ref": "aule"
            },
            "lineage": {
                "objective_id": task.id,
                "project_id": task_project_id(task),
                "run_id": run_id,
                "task_id": task.id,
                "session_ref": null,
                "parent_receipts": []
            },
            "objective": {
                "requested_outcome": task.title.as_deref().unwrap_or(task.id.as_str()),
                "acceptance_conditions": ["Record an evidence-backed terminal outcome"],
                "required_capabilities": ["objective_execution"],
                "forbidden_capabilities": []
            },
            "evidence_refs": evidence_refs,
            "memory_refs": [],
            "unresolved_failures": [],
            "return_contract": {
                "schema_version": "arda.context-return.v1",
                "required_receipt_types": ["arda.context-outcome-receipt.v1"],
                "max_output_bytes": 65536
            }
        }))?;
        let mut consumer =
            ConsumerContext::new("arda_workbench.queue_executor", vec![MemoryDomain::System]);
        consumer.purpose = Some(task.title.as_deref().unwrap_or(task.id.as_str()).to_owned());
        consumer.operator_authorized = true;
        service
            .assemble_organism_context(context, &consumer, now)?
            .use_receipt
    };
    if use_receipt.objective_id != task.id || use_receipt.run_id.as_deref() != Some(run_id) {
        return Err(anyhow!(
            "context use receipt `{}` is not bound to objective `{}` and run `{run_id}`",
            use_receipt.receipt_id,
            task.id
        ));
    }
    let used = result == "completed" || result == "failed";
    Ok(Some(service.record_context_outcome(
        &use_receipt,
        ContextOutcomeInput {
            consumer_id: use_receipt.consumer_id.clone(),
            disposition: if used {
                ContextDisposition::Used
            } else {
                ContextDisposition::Deferred
            },
            influenced_memory_refs: if used {
                use_receipt.memory_refs.clone()
            } else {
                Vec::new()
            },
            evidence_refs: evidence_refs.to_vec(),
            rationale: format!(
                "Workbench terminal result `{result}` recorded for governed objective `{}`",
                task.id
            ),
            recorded_at_unix_ms: Utc::now().timestamp_millis().max(0) as u128,
        },
    )?))
}

fn objective_plan_for_task(root: &Path, task: &QueueRecord) -> Result<ObjectivePlan> {
    let objective = task.title.as_deref().unwrap_or(task.id.as_str());
    let detail = task
        .extra
        .get("detail")
        .and_then(Value::as_str)
        .unwrap_or(
            "Inspect live behavior, compare it with operator-authored intent, and produce a prioritized repair backlog backed by reproducible evidence.",
        );
    let mut sources = Vec::new();
    for (kind, relative_path) in [
        ("project_contract", "data/workbench/projects.json"),
        (
            "operator_plan",
            "docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md",
        ),
        (
            "operator_plan",
            "docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md",
        ),
        ("soterion_result", "ARDA_SYSTEM_STATUS_REPORT.md"),
        ("recent_receipt", "core/projects/tasks/queue.jsonl"),
    ] {
        let bytes = std::fs::read(root.join(relative_path))
            .with_context(|| format!("read objective context source `{relative_path}`"))?;
        sources.push(ObjectiveContextSource {
            kind: kind.into(),
            reference: relative_path.into(),
            digest: Some(format!("sha256:{:x}", Sha256::digest(bytes))),
        });
    }
    let repository_state = std::process::Command::new("git")
        .args(["status", "--short"])
        .current_dir(root)
        .output()
        .map(|output| output.stdout)
        .unwrap_or_else(|_| b"repository state unavailable".to_vec());
    sources.push(ObjectiveContextSource {
        kind: "repository_state".into(),
        reference: "git status --short".into(),
        digest: Some(format!("sha256:{:x}", Sha256::digest(repository_state))),
    });

    let plan = ObjectiveDecomposer::default().decompose_grounded(
        &Objective {
            id: task.id.clone(),
            statement: objective.into(),
            constraints: vec![detail.into(), "Preserve unrelated working-tree changes".into()],
            deadline: None,
            success_criteria: vec![
                "A prioritized repair backlog names concrete human-visible behavior, source evidence, and the smallest authoritative repair surface.".into(),
                "Every claim distinguishes implemented capability, configured runtime, and live deployed proof.".into(),
                "No unrelated working-tree artifacts or generated queue projections are modified.".into(),
            ],
            tags: vec!["operator-vision".into(), "comprehensive-review".into()],
        },
        sources,
    );
    let validation = PlanValidator::default().validate_objective_plan(&plan);
    if !validation.ok {
        return Err(anyhow!(
            "objective decomposition failed validation: {}",
            validation.errors.join("; ")
        ));
    }
    Ok(plan)
}

fn persisted_objective_plan_for_task(
    root: &Path,
    run_id: &str,
    task: &QueueRecord,
) -> Result<(ObjectivePlan, String)> {
    if run_id.is_empty()
        || run_id.len() > 200
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(anyhow!(
            "workbench run ID must be a bounded safe path component"
        ));
    }
    let receipt_path = root
        .join("audit/workbench-queue")
        .join(run_id)
        .join("objective_plan_receipt.json");
    if receipt_path.exists() {
        let receipt_size = std::fs::metadata(&receipt_path)
            .with_context(|| format!("stat objective-plan receipt `{}`", receipt_path.display()))?
            .len();
        if receipt_size > MAX_OBJECTIVE_PLAN_RECEIPT_BYTES as u64 {
            return Err(anyhow!("objective-plan receipt exceeds size limit"));
        }
        let bytes = std::fs::read(&receipt_path)
            .with_context(|| format!("read objective-plan receipt `{}`", receipt_path.display()))?;
        let mut receipt: Value = serde_json::from_slice(&bytes).with_context(|| {
            format!("parse objective-plan receipt `{}`", receipt_path.display())
        })?;
        let receipt_digest = receipt
            .get("receipt_digest")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("objective-plan receipt omitted receipt_digest"))?
            .to_owned();
        receipt
            .as_object_mut()
            .ok_or_else(|| anyhow!("objective-plan receipt must be a JSON object"))?
            .remove("receipt_digest");
        let computed_digest = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(&receipt)?));
        if computed_digest != receipt_digest {
            return Err(anyhow!("objective-plan receipt digest mismatch"));
        }
        if receipt["contract"] != "arda.workbench.objective_plan_receipt.v1"
            || receipt["run_id"] != run_id
            || receipt["objective_id"] != task.id
        {
            return Err(anyhow!(
                "objective-plan receipt identity does not match the claimed queue task"
            ));
        }
        let plan: ObjectivePlan = serde_json::from_value(receipt["plan"].clone())
            .context("decode persisted objective plan")?;
        let validation = PlanValidator::default().validate_objective_plan(&plan);
        if !validation.ok || serde_json::to_value(validation)? != receipt["validation"] {
            return Err(anyhow!(
                "persisted objective plan no longer matches its validation receipt"
            ));
        }
        return Ok((plan, receipt_digest));
    }

    let plan = objective_plan_for_task(root, task)?;
    let validation = PlanValidator::default().validate_objective_plan(&plan);
    if !validation.ok {
        return Err(anyhow!(
            "refusing to persist invalid objective plan: {}",
            validation.errors.join("; ")
        ));
    }
    let mut receipt = json!({
        "contract": "arda.workbench.objective_plan_receipt.v1",
        "run_id": run_id,
        "objective_id": task.id,
        "plan": plan,
        "validation": validation,
    });
    let receipt_payload = serde_json::to_vec(&receipt)?;
    if receipt_payload.len() > MAX_OBJECTIVE_PLAN_RECEIPT_BYTES {
        return Err(anyhow!("objective-plan receipt exceeds size limit"));
    }
    let receipt_digest = format!("sha256:{:x}", Sha256::digest(receipt_payload));
    receipt["receipt_digest"] = Value::String(receipt_digest.clone());
    let parent = receipt_path
        .parent()
        .ok_or_else(|| anyhow!("objective-plan receipt path has no parent"))?;
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "create objective-plan receipt directory `{}`",
            parent.display()
        )
    })?;
    let temporary_path = receipt_path.with_extension("json.tmp");
    std::fs::write(&temporary_path, serde_json::to_vec_pretty(&receipt)?).with_context(|| {
        format!(
            "write objective-plan receipt `{}`",
            temporary_path.display()
        )
    })?;
    std::fs::rename(&temporary_path, &receipt_path).with_context(|| {
        format!(
            "install objective-plan receipt `{}`",
            receipt_path.display()
        )
    })?;
    Ok((plan, receipt_digest))
}

fn objective_execution_prompt(plan: &ObjectivePlan, objective: &str, task: &QueueRecord) -> String {
    let sources = plan
        .context_sources
        .iter()
        .map(|source| {
            format!(
                "- {:?}: {} ({})",
                source.kind,
                source.reference,
                source.digest.as_deref().unwrap_or("digest unavailable")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let tasks = plan
        .tasks
        .iter()
        .map(|task| format!("- {}: {}", task.key, task.title))
        .collect::<Vec<_>>()
        .join("\n");
    let artifact_requirement = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("acceptance_artifact"))
        .and_then(Value::as_str)
        .map(|path| format!(" Write the final accepted backlog to `{path}`."))
        .unwrap_or_default();
    format!(
        "{}\n\nExecute this validated objective plan in dependency order:\n{}\n\nRead and cite these live authorities before changing anything:\n{}\n\nFinal output must be a concrete prioritized repair backlog with evidence, human-visible behavior, and the smallest authoritative implementation surface.{} Preserve unrelated dirty work and do not edit generated queue projections.",
        objective, tasks, sources, artifact_requirement
    )
}

fn approval_envelope(task: &QueueRecord, idempotency_key: &str) -> Result<Value> {
    let meta = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("task `{}` omitted governed queue metadata", task.id))?;
    let mutation_risk = required_meta(meta, "mutation_risk", &task.id)?;
    if required_meta(meta, "execution_authority", &task.id)? != "arda_workbench"
        || required_meta(meta, "action_class", &task.id)? != "approved_autopilot_plan_step"
    {
        return Err(anyhow!(
            "task `{}` has invalid Workbench execution authority",
            task.id
        ));
    }
    let approval_id = match mutation_risk {
        "operator-approved" => required_meta(meta, "approval_packet_id", &task.id)?,
        "governance-authorized-reversible" => {
            governance_authorization_id(meta).ok_or_else(|| {
                anyhow!(
                    "task `{}` has unbound governance authorization metadata",
                    task.id
                )
            })?
        }
        other => {
            return Err(anyhow!(
                "task `{}` has unsupported Workbench mutation authority `{other}`",
                task.id
            ));
        }
    };
    let proposal_id = required_meta(meta, "source_objective_packet_id", &task.id)?;
    Ok(json!({
        "approval": {
            "schema_version": "arda.orome.task_approval.v1",
            "proposal_id": proposal_id,
            "approval_id": approval_id,
            "ledger_writes": ["core/projects/tasks/queue.jsonl", "data/runs"],
            "decision": "policy_safe",
            "created_at_utc": Utc::now().to_rfc3339(),
        },
        "idempotency_key": idempotency_key,
    }))
}

fn required_meta<'a>(
    meta: &'a serde_json::Map<String, Value>,
    key: &str,
    task_id: &str,
) -> Result<&'a str> {
    meta.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("task `{task_id}` omitted `{key}`"))
}

#[cfg(test)]
fn run_graph(run_id: &str, task_id: &str, objective: &str, approval_id: &str) -> Value {
    run_graph_value(run_id, task_id, objective, approval_id, None)
}

fn run_graph_with_objective_plan_receipt(
    run_id: &str,
    task_id: &str,
    objective: &str,
    approval_id: &str,
    objective_plan_receipt: &str,
) -> Value {
    run_graph_value(
        run_id,
        task_id,
        objective,
        approval_id,
        Some(objective_plan_receipt),
    )
}

fn run_graph_value(
    run_id: &str,
    task_id: &str,
    objective: &str,
    approval_id: &str,
    objective_plan_receipt: Option<&str>,
) -> Value {
    let prompt_digest = format!("sha256:{:x}", Sha256::digest(objective.as_bytes()));
    let deadline = Utc::now().timestamp_millis().saturating_add(1_200_000) as u128;
    let node = |id: &str, kind: &str, authority: &str, parents: Vec<&str>, worker: Value| {
        json!({
            "id": id,
            "kind": kind,
            "state": "pending",
            "authority": authority,
            "budget": {"max_joules": 5000.0, "max_cost_usd": 2.0},
            "retry": {"max_attempts": 2},
            "timeout_ms": 900000,
            "idempotency_key": format!("queue-{task_id}-{id}"),
            "input_digest": null,
            "output_digest": null,
            "parent_receipts": parents,
            "checkpoint": {"sequence": 0, "recovery_token": null, "checkpoint_digest": null},
            "worker": worker,
        })
    };
    let mut provenance_receipts = vec![approval_id];
    if let Some(receipt) = objective_plan_receipt {
        provenance_receipts.push(receipt);
    }
    json!({
        "schema_version": "arda.run-graph.v1",
        "run_id": run_id,
        "objective_id": task_id,
        "nodes": [
            node("plan", "plan", "read_only", vec![], Value::Null),
            node("approval", "approval", "human_approval", vec![approval_id], Value::Null),
            node("execute", "execute", "execute_with_approval", vec![approval_id], json!({
                "role": "implementer",
                "worker_id": format!("hermes:queue:{task_id}"),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": prompt_digest,
                "allowed_toolsets": ["file", "terminal"],
                "dependencies": ["approval"],
                "deadline_unix_ms": deadline,
                "output_contract": "arda.hermes-job-result.v1",
                "evidence_policy": "worker_report"
            })),
            node("verify", "verify", "verify", vec![], json!({
                "role": "independent_verifier",
                "worker_id": format!("hermes:queue:{task_id}:verify"),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": prompt_digest,
                "allowed_toolsets": ["terminal"],
                "dependencies": ["execute"],
                "deadline_unix_ms": deadline,
                "output_contract": "arda.hermes-job-result.v1",
                "evidence_policy": "project_native_checks"
            })),
            node("review", "review", "read_only", vec![], Value::Null),
            node("close", "close", "read_only", vec![], Value::Null)
        ],
        "edges": [
            {"id": "plan-approval", "from": "plan", "to": "approval", "parent_receipt": approval_id},
            {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": approval_id},
            {"id": "execute-verify", "from": "execute", "to": "verify", "parent_receipt": null},
            {"id": "verify-review", "from": "verify", "to": "review", "parent_receipt": null},
            {"id": "review-close", "from": "review", "to": "close", "parent_receipt": null}
        ],
        "provenance": {
            "project_contract_digest": format!("sha256:{}", "0".repeat(64)),
            "created_by": "arda_workbench.queue_executor",
            "parent_receipts": provenance_receipts
        }
    })
}

fn workbench_run_id(task_id: &str) -> String {
    let normalized = task_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("queue-{normalized}")
}

async fn response_error(response: reqwest::Response, action: &str) -> Result<Value> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("{action} returned {status}: {body}"));
    }
    serde_json::from_str(&body).with_context(|| format!("decode {action} response"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn governance_authorization_builds_workbench_approval_envelope() {
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "governed-task",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "packet-1",
                "mutation_risk": "governance-authorized-reversible",
                "governance_action_class": "safe_local",
                "governance_gate": "safe_autonomous",
                "governance_authorization_id": "governance:packet-1:safe_local"
            }
        }))
        .expect("queue record");

        let envelope = approval_envelope(&task, "governed-task-attempt")
            .expect("binding governance should authorize Workbench execution");

        assert_eq!(
            envelope["approval"]["approval_id"],
            "governance:packet-1:safe_local"
        );
        assert_eq!(envelope["approval"]["decision"], "policy_safe");
    }

    #[test]
    fn mismatched_governance_authorization_is_rejected() {
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "forged-governed-task",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "packet-1",
                "mutation_risk": "governance-authorized-reversible",
                "governance_action_class": "safe_local",
                "governance_gate": "safe_autonomous",
                "governance_authorization_id": "governance:another-packet:safe_local"
            }
        }))
        .expect("queue record");

        assert!(approval_envelope(&task, "forged-attempt").is_err());
    }

    fn approved_queue_fixture(root: &Path, task_id: &str) -> PathBuf {
        let queue_path = root.join("core/projects/tasks/queue.jsonl");
        let active_path = root.join("core/state/queue_active.json");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(active_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(root.join("core/state/memory")).unwrap();
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                json!({
                    "id": task_id,
                    "title": "Reconcile deterministic Workbench run",
                    "status": "queued",
                    "meta": {
                        "action_class": "approved_autopilot_plan_step",
                        "mutation_risk": "operator-approved",
                        "execution_authority": "arda_workbench",
                        "source_objective_packet_id": "objective-reconciliation",
                        "approval_packet_id": "approval-reconciliation"
                    }
                })
            ),
        )
        .unwrap();
        std::fs::write(
            &active_path,
            format!("{{\"active\":[{{\"id\":\"{task_id}\"}}]}}\n"),
        )
        .unwrap();
        for relative_path in [
            "data/workbench/projects.json",
            "docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md",
            "docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md",
            "ARDA_SYSTEM_STATUS_REPORT.md",
        ] {
            let path = root.join(relative_path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, "authoritative fixture\n").unwrap();
        }
        queue_path
    }

    fn test_executor(root: &Path, harness_url: String) -> WorkbenchQueueExecutor {
        WorkbenchQueueExecutor {
            root: root.to_path_buf(),
            harness_url,
            project_id: DEFAULT_PROJECT_ID.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap(),
        }
    }

    async fn scripted_harness(
        responses: Vec<Option<(u16, String)>>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let mut requests = Vec::new();
            for response in responses {
                let Ok(Ok((mut stream, _))) =
                    tokio::time::timeout(std::time::Duration::from_secs(1), listener.accept())
                        .await
                else {
                    break;
                };
                let mut request = Vec::new();
                let mut buffer = [0_u8; 4096];
                loop {
                    let count = stream.read(&mut buffer).await.unwrap();
                    if count == 0 {
                        break;
                    }
                    request.extend_from_slice(&buffer[..count]);
                    let Some(headers_end) = request.windows(4).position(|part| part == b"\r\n\r\n")
                    else {
                        continue;
                    };
                    let headers = String::from_utf8_lossy(&request[..headers_end + 4]);
                    let content_length = headers
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .and_then(|value| value.trim().parse::<usize>().ok())
                        })
                        .unwrap_or(0);
                    if request.len() >= headers_end + 4 + content_length {
                        break;
                    }
                }
                requests.push(
                    String::from_utf8_lossy(&request)
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .to_owned(),
                );
                let Some((status, body)) = response else {
                    break;
                };
                let reason = if status == 200 { "OK" } else { "Not Found" };
                stream
                    .write_all(
                        format!(
                            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        )
                        .as_bytes(),
                    )
                    .await
                    .unwrap();
            }
            requests
        });
        (format!("http://{address}"), server)
    }

    #[tokio::test]
    async fn transient_harness_outage_during_restart_preserves_claim() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "outage-task");
        ActiveQueueExecutor::new(dir.path())
            .claim_next_approved()
            .unwrap()
            .expect("initial claim");
        let before = std::fs::read(&queue_path).unwrap();
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let unavailable_url = format!("http://{}", listener.local_addr().unwrap());
        drop(listener);

        let error = test_executor(dir.path(), unavailable_url)
            .execute_once()
            .await
            .unwrap_err();

        assert!(format!("{error:#}").contains("inspect existing Workbench run"));
        assert_eq!(std::fs::read(&queue_path).unwrap(), before);
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("in_progress"));
    }

    #[tokio::test]
    async fn lost_execute_response_preserves_claim_for_run_inspection() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "lost-response-task");
        let (harness_url, server) = scripted_harness(vec![
            Some((404, "{}".into())),
            Some((200, "{}".into())),
            Some((200, "{}".into())),
            None,
        ])
        .await;

        let error = test_executor(dir.path(), harness_url)
            .execute_once()
            .await
            .unwrap_err();
        let requests = server.await.unwrap();

        assert!(
            format!("{error:#}").contains("dispatch approved Workbench execute provider"),
            "unexpected error: {error:#}"
        );
        assert_eq!(requests.len(), 4);
        assert!(requests[3]
            .starts_with("POST /v1/runs/queue-lost-response-task/nodes/execute/execute-provider "));
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("in_progress"));
    }

    #[tokio::test]
    async fn existing_deterministic_run_still_running_remains_recoverable() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "running-task");
        let (harness_url, server) = scripted_harness(vec![Some((
            200,
            json!({
                "graph": {"nodes": [{"id": "execute", "state": "running"}]},
                "review": {"provider_receipt": null}
            })
            .to_string(),
        ))])
        .await;

        let receipt = test_executor(dir.path(), harness_url)
            .execute_once()
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(receipt.status, "in_progress");
        assert_eq!(receipt.result, "existing_run_active");
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("in_progress"));
    }

    #[tokio::test]
    async fn existing_deterministic_run_terminal_is_definitive() {
        let dir = tempfile::tempdir().unwrap();
        approved_queue_fixture(dir.path(), "terminal-task");
        let claim = ActiveQueueExecutor::new(dir.path())
            .claim_next_approved()
            .unwrap()
            .expect("initial claim");
        let (harness_url, server) = scripted_harness(vec![Some((
            200,
            json!({
                "graph": {"nodes": [
                    {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                    {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                    {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                    {"id": "close", "state": "succeeded", "output_digest": "sha256:terminal"}
                ]},
                "review": {
                    "tests": [{"name": "project-check", "status": "passed"}],
                    "provider_receipt": {
                        "receipt_digest": "sha256:execute",
                        "summary": "provider completed before restart"
                    }
                }
            })
            .to_string(),
        ))])
        .await;

        let outcome = test_executor(dir.path(), harness_url)
            .dispatch_claim(&claim)
            .await
            .unwrap();
        server.await.unwrap();

        assert_eq!(outcome.0, "succeeded");
        assert_eq!(outcome.1.as_deref(), Some("sha256:terminal"));
        assert_eq!(
            outcome.2.as_deref(),
            Some("provider completed before restart")
        );
    }

    #[tokio::test]
    async fn restart_after_execute_resumes_verify_review_close_and_persists_decision() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "restart-completion-task");
        let (harness_url, server) = scripted_harness(vec![
            Some((
                200,
                json!({
                    "graph": {"nodes": [
                        {"id": "plan", "state": "succeeded"},
                        {"id": "approval", "state": "succeeded"},
                        {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                        {"id": "verify", "state": "ready"},
                        {"id": "review", "state": "blocked"},
                        {"id": "close", "state": "blocked"}
                    ]},
                    "review": {"provider_receipt": {"receipt_digest": "sha256:execute", "summary": "executed"}}
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "receipt": {"status": "succeeded", "receipt_digest": "sha256:verify", "summary": "verified"},
                    "run": {
                        "graph": {"nodes": [
                            {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                            {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                            {"id": "review", "state": "ready"},
                            {"id": "close", "state": "blocked"}
                        ]},
                        "review": {
                            "tests": [{"name": "cargo test", "status": "passed"}],
                            "provider_receipt": {"receipt_digest": "sha256:verify", "summary": "verified"}
                        }
                    }
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "graph": {"nodes": [
                        {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                        {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                        {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                        {"id": "close", "state": "ready"}
                    ]},
                    "review": {
                        "tests": [{"name": "cargo test", "status": "passed"}],
                        "provider_receipt": {"receipt_digest": "sha256:verify", "summary": "verified"}
                    }
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "graph": {"nodes": [
                        {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                        {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                        {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                        {"id": "close", "state": "succeeded", "output_digest": "sha256:close"}
                    ]},
                    "review": {
                        "tests": [{"name": "cargo test", "status": "passed"}],
                        "provider_receipt": {"receipt_digest": "sha256:verify", "summary": "verified"}
                    }
                })
                .to_string(),
            )),
        ])
        .await;

        let receipt = test_executor(dir.path(), harness_url)
            .execute_once()
            .await
            .unwrap();
        let requests = server.await.unwrap();

        assert_eq!(receipt.status, "completed");
        assert_eq!(
            receipt.execution_receipt_digest.as_deref(),
            Some("sha256:close")
        );
        assert_eq!(
            receipt.continuation_decision.as_deref(),
            Some("close_complete")
        );
        assert!(requests[1].contains("/nodes/verify/execute-provider"));
        assert!(requests[2].contains("/nodes/review/complete"));
        assert!(requests[3].contains("/nodes/close/complete"));
        let records = super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
            .load()
            .unwrap();
        assert!(records.iter().any(|record| {
            record
                .extra
                .get("continuation_decision")
                .and_then(Value::as_str)
                == Some("continue_review")
                && record.extra.get("workbench_run_id").and_then(Value::as_str)
                    == Some("queue-restart-completion-task")
        }));
        assert!(records.iter().any(|record| {
            record
                .extra
                .get("continuation_decision")
                .and_then(Value::as_str)
                == Some("continue_close")
                && record.extra.get("workbench_run_id").and_then(Value::as_str)
                    == Some("queue-restart-completion-task")
        }));
        let terminal = records.last().unwrap();
        assert_eq!(terminal.status.as_deref(), Some("completed"));
        assert_eq!(terminal.extra["continuation_decision"], "close_complete");
        assert_eq!(terminal.extra["closure_receipt_digest"], "sha256:close");
    }

    #[test]
    fn claim_before_dispatch_crash_child() {
        let Ok(root) = std::env::var("ARDA_CLAIM_CRASH_FIXTURE_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let _executor_lock = acquire_executor_lock(&root).expect("acquire child executor lock");
        let claim = ActiveQueueExecutor::new(&root)
            .claim_next_approved_reconciling_orphans()
            .expect("claim fixture task")
            .expect("approved fixture claim");
        assert_eq!(claim.task.id, "pre-dispatch-crash-task");
        std::process::exit(86);
    }

    #[test]
    fn process_restart_recovers_claim_before_lease_expiry() {
        let dir = tempfile::tempdir().expect("create tempdir");
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        let active_path = dir.path().join("core/state/queue_active.json");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        std::fs::create_dir_all(active_path.parent().unwrap()).unwrap();
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                json!({
                    "id": "pre-dispatch-crash-task",
                    "title": "Recover before lease expiry",
                    "status": "queued",
                    "meta": {
                        "action_class": "approved_autopilot_plan_step",
                        "mutation_risk": "operator-approved",
                        "execution_authority": "arda_workbench",
                        "source_objective_packet_id": "objective-crash-proof",
                        "approval_packet_id": "approval-crash-proof"
                    }
                })
            ),
        )
        .unwrap();
        std::fs::write(
            &active_path,
            "{\"active\":[{\"id\":\"pre-dispatch-crash-task\"}]}\n",
        )
        .unwrap();

        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("prometheus::autopilot::workbench_executor::tests::claim_before_dispatch_crash_child")
            .arg("--nocapture")
            .env("ARDA_CLAIM_CRASH_FIXTURE_ROOT", dir.path())
            .status()
            .expect("run crash child");
        assert_eq!(child.code(), Some(86));

        let claimed_bytes = std::fs::read(&queue_path).unwrap();
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(&queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("in_progress"));
        let lease = effective[0].extra["lease_expires_at_utc"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .expect("future lease");
        assert!(lease > Utc::now());

        let _executor_lock =
            acquire_executor_lock(dir.path()).expect("crash released executor lock");
        let recovered = ActiveQueueExecutor::new(dir.path())
            .claim_next_approved_reconciling_orphans()
            .expect("recover claimed task")
            .expect("unexpired claim recovered");
        assert_eq!(recovered.task.id, "pre-dispatch-crash-task");
        assert_eq!(
            recovered.attempt.workbench_run_id,
            "queue-pre-dispatch-crash-task"
        );
        assert_eq!(std::fs::read(&queue_path).unwrap(), claimed_bytes);
    }

    #[test]
    fn graph_requires_the_approved_parent_and_bounded_worker() {
        let graph = run_graph("queue-task-1", "task-1", "bounded fixture", "approval-1");
        let raw = serde_json::to_string(&graph).unwrap();
        let parsed = arda_core::run_graph::RunGraph::from_json_str(&raw).unwrap();
        assert_eq!(parsed.nodes.len(), 6);
        assert_eq!(parsed.objective_id.as_str(), "task-1");
        assert_eq!(
            parsed
                .nodes
                .iter()
                .map(|node| format!("{:?}", node.kind).to_ascii_lowercase())
                .collect::<Vec<_>>(),
            ["plan", "approval", "execute", "verify", "review", "close"]
        );
        let execute = parsed
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "execute")
            .unwrap();
        assert_eq!(execute.retry.max_attempts, 2);
        assert_eq!(execute.parent_receipts, vec!["approval-1"]);
        let verify = parsed
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "verify")
            .unwrap();
        assert_eq!(
            format!("{:?}", verify.worker.as_ref().unwrap().evidence_policy).to_ascii_lowercase(),
            "projectnativechecks"
        );
    }

    #[test]
    fn evidence_backed_close_is_the_only_successful_terminal_state() {
        let partial = classify_existing_run(&json!({
            "graph": {"nodes": [
                {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                {"id": "review", "state": "pending", "output_digest": null},
                {"id": "close", "state": "pending", "output_digest": null}
            ]},
            "review": {"tests": [{"name": "project-check", "status": "passed"}]}
        }));
        assert_eq!(partial.0, "in_progress");

        let closed = classify_existing_run(&json!({
            "graph": {"nodes": [
                {"id": "execute", "state": "succeeded", "output_digest": "sha256:execute"},
                {"id": "verify", "state": "succeeded", "output_digest": "sha256:verify"},
                {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                {"id": "close", "state": "succeeded", "output_digest": "sha256:close"}
            ]},
            "review": {
                "tests": [{"name": "project-check", "status": "passed"}],
                "provider_receipt": {
                    "receipt_digest": "sha256:execute",
                    "summary": "bounded mutation completed"
                }
            }
        }));
        assert_eq!(closed.0, "succeeded");
        assert_eq!(closed.1.as_deref(), Some("sha256:close"));
    }

    #[test]
    fn objective_plan_is_grounded_persisted_and_digest_bound_outside_the_graph() {
        let dir = tempfile::tempdir().unwrap();
        for path in [
            "data/workbench/projects.json",
            "docs/plans/AUTONOMOUS_TASK_COMPLETION_LOOP.md",
            "docs/plans/ARDA_WHOLE_SYSTEM_COMPLETION_PROGRAM.md",
            "ARDA_SYSTEM_STATUS_REPORT.md",
            "core/projects/tasks/queue.jsonl",
        ] {
            let path = dir.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, "authoritative fixture\n").unwrap();
        }
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "objective-1",
            "title": "Review Arda against the operator vision",
            "detail": "Inspect live behavior and produce the smallest authoritative repairs"
        }))
        .unwrap();

        let plan = objective_plan_for_task(dir.path(), &task).unwrap();
        let validation =
            super::super::validator::PlanValidator::default().validate_objective_plan(&plan);
        assert!(validation.ok, "{:?}", validation.errors);
        assert_eq!(plan.context_sources.len(), 6);
        assert!(plan.context_sources.iter().all(|source| source
            .digest
            .as_deref()
            .is_some_and(|value| value.starts_with("sha256:"))));

        let (persisted_plan, receipt_digest) =
            persisted_objective_plan_for_task(dir.path(), "queue-objective-1", &task).unwrap();
        assert_eq!(persisted_plan.objective_id, "objective-1");
        assert!(receipt_digest.starts_with("sha256:"));
        let receipt_path = dir
            .path()
            .join("audit/workbench-queue/queue-objective-1/objective_plan_receipt.json");
        assert!(receipt_path.is_file());

        std::fs::write(
            dir.path().join("ARDA_SYSTEM_STATUS_REPORT.md"),
            "changed after the plan was accepted\n",
        )
        .unwrap();
        let (reloaded_plan, reloaded_digest) =
            persisted_objective_plan_for_task(dir.path(), "queue-objective-1", &task).unwrap();
        assert_eq!(
            serde_json::to_value(reloaded_plan).unwrap(),
            serde_json::to_value(persisted_plan).unwrap()
        );
        assert_eq!(reloaded_digest, receipt_digest);

        let graph = run_graph(
            "queue-objective-1",
            "objective-1",
            "Review Arda",
            "approval-1",
        );
        assert!(graph["provenance"].get("objective_plan").is_none());
        assert!(graph["provenance"]
            .get("objective_plan_validation")
            .is_none());
        assert_eq!(
            graph["provenance"]["parent_receipts"],
            json!(["approval-1"])
        );

        assert!(
            persisted_objective_plan_for_task(dir.path(), "../escape", &task)
                .unwrap_err()
                .to_string()
                .contains("safe path component")
        );
        assert!(!dir.path().join("audit/escape").exists());

        let mut oversized_task = task;
        oversized_task.title = Some("x".repeat(MAX_OBJECTIVE_PLAN_RECEIPT_BYTES));
        assert!(persisted_objective_plan_for_task(
            dir.path(),
            "queue-objective-oversized",
            &oversized_task,
        )
        .unwrap_err()
        .to_string()
        .contains("size limit"));
    }

    #[test]
    fn failed_attempts_choose_durable_retry_revise_and_replan_decisions() {
        assert_eq!(
            continuation_decision("failed", "failed", Some("provider timeout"), 0),
            "retry_same_task"
        );
        assert_eq!(
            continuation_decision(
                "failed",
                "failed",
                Some("acceptance criteria were not satisfied"),
                0,
            ),
            "revise_task"
        );
        assert_eq!(
            continuation_decision("failed", "failed", Some("provider timeout"), 1),
            "replan_objective"
        );
        assert_eq!(
            continuation_decision("completed", "completed", None, 0),
            "close_complete"
        );
    }

    #[test]
    fn declared_acceptance_artifact_must_exist_and_cover_required_markers() {
        let dir = tempfile::tempdir().unwrap();
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "objective-1",
            "meta": {
                "acceptance_artifact": "docs/audits/backlog.md",
                "acceptance_markers": ["human-visible behavior", "evidence", "priority"]
            }
        }))
        .unwrap();
        assert!(validate_task_acceptance_artifact(dir.path(), &task).is_err());
        let path = dir.path().join("docs/audits/backlog.md");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            "# Priority backlog\n\nHuman-visible behavior with source evidence.\n",
        )
        .unwrap();
        validate_task_acceptance_artifact(dir.path(), &task).unwrap();
    }

    #[test]
    fn terminal_result_records_vaire_context_outcome_when_use_receipt_is_bound() {
        use arda_vaire::{ContextUseReceipt, MnemosyneService};

        let dir = tempfile::tempdir().unwrap();
        let memory_root = dir.path().join("data/vaire");
        std::fs::create_dir_all(&memory_root).unwrap();
        let mut use_receipt = ContextUseReceipt {
            schema_version: "arda.context-use-receipt.v1".into(),
            receipt_id: "context-use:fixture".into(),
            receipt_digest: String::new(),
            capsule_id: "capsule:fixture".into(),
            capsule_digest: "sha256:fixture".into(),
            objective_id: "objective-1".into(),
            run_id: Some("queue-objective-1".into()),
            consumer_id: "arda_workbench.queue_executor".into(),
            purpose: "objective execution".into(),
            memory_refs: vec!["memory-1".into()],
            recorded_at_unix_ms: 10,
            expires_at_unix_ms: 20,
        };
        use_receipt.receipt_digest = use_receipt.computed_digest().unwrap();
        std::fs::write(
            memory_root.join("context_use_receipts.jsonl"),
            format!("{}\n", serde_json::to_string(&use_receipt).unwrap()),
        )
        .unwrap();
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "objective-1",
            "meta": {"context_use_receipt_id": "context-use:fixture"}
        }))
        .unwrap();

        let receipt = record_context_outcome(
            dir.path(),
            &task,
            "queue-objective-1",
            "completed",
            &["sha256:closure".into()],
        )
        .unwrap()
        .unwrap();

        assert_eq!(receipt.objective_id, "objective-1");
        assert_eq!(receipt.influenced_memory_refs, vec!["memory-1"]);
        assert_eq!(
            MnemosyneService::new(memory_root)
                .unwrap()
                .context_outcome_receipts()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn cancellation_endpoint_preserves_governed_run_identity() {
        let run_id = workbench_run_id("task/one");
        assert_eq!(run_id, "queue-task-one");
        let graph = run_graph(&run_id, "task/one", "bounded fixture", "approval-1");
        assert_eq!(graph["run_id"], run_id);
        assert_eq!(graph["provenance"]["parent_receipts"][0], "approval-1");
    }

    #[test]
    fn existing_active_run_is_classified_for_reconciliation() {
        let outcome = classify_existing_run(&json!({
            "graph": {"nodes": [{"id": "execute", "state": "running"}]},
            "review": {"provider_receipt": null}
        }));
        assert_eq!(outcome.0, "in_progress");
        assert!(outcome.1.is_none());
    }
}
