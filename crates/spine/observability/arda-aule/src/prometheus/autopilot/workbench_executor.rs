#![cfg(feature = "full-cli")]
//! Bounded adapter from operator-approved canonical queue work into Workbench.

use super::decomposer::{
    ExecutableLeafContract, Objective, ObjectiveContextSource, ObjectiveDecomposer, ObjectivePlan,
};
use super::execution_outcome::project_terminal_outcome;
use super::task_queue::{
    governance_authorization_id, workbench_run_id as attempt_workbench_run_id, ActiveQueueExecutor,
    ApprovedQueueClaim, QueueRecord,
};
use super::validator::PlanValidator;
use anyhow::{anyhow, Context, Result};
use arda_core::project_contract::ProjectContract;
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use arda_vaire::{
    ContextDisposition, ContextOutcomeInput, ContextOutcomeReceipt, MnemosyneService,
    OrganismContext,
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
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
    client: reqwest::Client,
}

impl WorkbenchQueueExecutor {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let harness_url =
            std::env::var("ARDA_HARNESS_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(1_200))
            .build()?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            harness_url: harness_url.trim_end_matches('/').to_owned(),
            client,
        })
    }

    pub async fn execute_once(&self) -> Result<QueueExecutionReceipt> {
        // Serialize only canonical reconciliation and claim selection. The
        // target locks then preserve project/worktree exclusion for dispatch.
        let executor_coordinator_lock = acquire_executor_lock(&self.root)?;
        reconcile_terminal_objective_leaves(&self.root)?;
        let queue = ActiveQueueExecutor::new(&self.root);
        queue.reconcile_schedules(Utc::now())?;
        let Some((claim, target_locks)) =
            claim_execution_with_available_target(&self.root, &queue)?
        else {
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
        drop(executor_coordinator_lock);
        let run_id = claim.attempt.workbench_run_id.clone();
        if is_decomposable_objective(&claim.task) {
            let (plan, plan_receipt) =
                persisted_objective_plan_for_task(&self.root, &run_id, &claim.task)?;
            materialize_objective_leaves(&self.root, &claim.task, &plan, &plan_receipt, &run_id)?;
            return Ok(QueueExecutionReceipt {
                contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                task_id: Some(claim.task.id),
                workbench_run_id: Some(run_id),
                status: "waiting".into(),
                result: "objective_decomposed".into(),
                execution_receipt_digest: Some(plan_receipt),
                continuation_decision: Some("continue_next_task".into()),
                detail: Some(format!(
                    "materialized {} independently durable objective leaves",
                    plan.tasks.len()
                )),
                recorded_at_utc: Utc::now().to_rfc3339(),
            });
        }
        match self.dispatch_claim(&claim, &target_locks.binding).await {
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
                let decision = continuation_decision_for_task(
                    &claim.task,
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
                if is_objective_leaf(&claim.task) {
                    if queue_status == "completed" {
                        advance_objective_after_leaf(&self.root, &claim.task)?;
                    } else if matches!(
                        decision,
                        "retry_same_task" | "revise_task" | "replan_objective" | "wait_until"
                    ) {
                        materialize_continuation(
                            &self.root,
                            &claim.task,
                            &run_id,
                            decision,
                            detail
                                .as_deref()
                                .unwrap_or("terminal leaf verification failed"),
                        )?;
                    }
                }
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
                    let decision = continuation_decision_for_task(
                        &claim.task,
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
        let _executor_lock = acquire_executor_lock(&self.root)?;
        let task = self
            .effective_task(task_id)?
            .ok_or_else(|| anyhow!("queue task `{task_id}` was not found"))?;
        if matches!(
            task.status.as_deref(),
            Some("completed" | "failed" | "cancelled")
        ) {
            return Err(anyhow!("queue task `{task_id}` is already terminal"));
        }
        let run_id = task
            .extra
            .get("workbench_run_id")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| workbench_run_id(task_id));
        let objective_id = super::task_queue::queue_objective_id(&task)
            .ok_or_else(|| anyhow!("queue task `{task_id}` omitted objective lineage"))?;
        let schedule_ledger = super::schedule::ScheduleLedger::new(
            self.root.join("core/projects/tasks/schedules.jsonl"),
        );
        let cancellation_already_persisted = schedule_ledger
            .effective()?
            .get(&task.id)
            .is_some_and(|schedule| {
                schedule.objective_id == objective_id
                    && schedule.state == super::schedule::ScheduleState::Cancelled
            });
        if cancellation_already_persisted {
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
            return Ok(json!({
                "status": "cancelled",
                "reconciled": true,
                "task_id": task.id,
                "workbench_run_id": run_id,
            }));
        }
        schedule_ledger.with_active_authority(&task.id, objective_id, || Ok(()))?;
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
        execution_target: &ExecutionTargetBinding,
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
        let (objective_plan, objective_plan_receipt) = if is_objective_leaf(&claim.task) {
            objective_plan_for_claim(&self.root, &claim.task)?
        } else {
            persisted_objective_plan_for_task(&self.root, run_id, &claim.task)?
        };
        let leaf_contract = if is_objective_leaf(&claim.task) {
            Some(objective_leaf_contract(&objective_plan, &claim.task)?)
        } else {
            None
        };
        let graph = run_graph_with_objective_plan_receipt(
            run_id,
            &claim.task.id,
            objective,
            approval_id,
            &objective_plan_receipt,
            leaf_contract,
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
            let response = self
                .client
                .post(format!("{}/v1/runs/plan", self.harness_url))
                .json(&json!({
                    "project_id": execution_target.project_id,
                    "expected_project_contract_digest": execution_target.project_contract_digest,
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
            ("verify", verification_prompt(&claim.task, leaf_contract)),
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
                    value["receipt"]["receipt_digest"]
                        .as_str()
                        .map(str::to_owned),
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

#[derive(Debug)]
struct ExecutionTargetLocks {
    _files: Vec<File>,
    binding: ExecutionTargetBinding,
}

#[derive(Debug)]
struct ExecutionTargetBinding {
    project_id: String,
    project_contract_digest: String,
    workspace_root: PathBuf,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionProjectRegistry {
    schema_version: String,
    projects: Vec<ExecutionAttachedProject>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionAttachedProject {
    contract: ProjectContract,
    #[serde(rename = "approval_id")]
    _approval_id: String,
    #[serde(rename = "proposal_id")]
    _proposal_id: String,
    #[serde(rename = "idempotency_key")]
    _idempotency_key: String,
}

fn resolve_execution_target(root: &Path, task: &QueueRecord) -> Result<ExecutionTargetBinding> {
    let project_id = task_project_id(task)
        .ok_or_else(|| anyhow!("task `{}` omitted `meta.project_id`", task.id))?;
    let registry_path = root.join("data/workbench/projects.json");
    const MAX_PROJECT_REGISTRY_BYTES: u64 = 1024 * 1024;
    let registry_metadata = std::fs::metadata(&registry_path).with_context(|| {
        format!(
            "read Workbench project registry metadata `{}`",
            registry_path.display()
        )
    })?;
    if registry_metadata.len() > MAX_PROJECT_REGISTRY_BYTES {
        return Err(anyhow!(
            "Workbench project registry `{}` exceeds maximum size of {} bytes",
            registry_path.display(),
            MAX_PROJECT_REGISTRY_BYTES
        ));
    }
    let raw = std::fs::read_to_string(&registry_path).with_context(|| {
        format!(
            "read Workbench project registry `{}`",
            registry_path.display()
        )
    })?;
    let registry: ExecutionProjectRegistry = serde_json::from_str(&raw).with_context(|| {
        format!(
            "parse Workbench project registry `{}`",
            registry_path.display()
        )
    })?;
    if registry.schema_version != "arda.workbench.project-registry.v1" {
        return Err(anyhow!(
            "unsupported Workbench project registry version `{}`",
            registry.schema_version
        ));
    }
    let mut matches = registry
        .projects
        .into_iter()
        .filter(|attached| attached.contract.identity.project_id.to_string() == project_id);
    let attached = matches
        .next()
        .ok_or_else(|| anyhow!("project `{project_id}` is not attached"))?;
    if matches.next().is_some() {
        return Err(anyhow!("project `{project_id}` is attached more than once"));
    }
    attached
        .contract
        .validate()
        .map_err(|error| anyhow!("invalid project contract for `{project_id}`: {error}"))?;

    let canonical_root = root
        .canonicalize()
        .with_context(|| format!("canonicalize Workbench root `{}`", root.display()))?;
    let workspace_root = canonical_root
        .join(attached.contract.workspace.root.as_str())
        .canonicalize()
        .with_context(|| format!("canonicalize registered workspace for project `{project_id}`"))?;
    if !workspace_root.starts_with(&canonical_root) {
        return Err(anyhow!(
            "registered workspace for project `{project_id}` escapes Workbench root"
        ));
    }
    if let Some(declared_worktree) = task_worktree_path(task) {
        let declared = Path::new(declared_worktree);
        let declared = if declared.is_absolute() {
            declared.to_path_buf()
        } else {
            canonical_root.join(declared)
        };
        let declared = declared.canonicalize().with_context(|| {
            format!(
                "canonicalize task `{}` metadata worktree `{declared_worktree}`",
                task.id
            )
        })?;
        if declared != workspace_root {
            return Err(anyhow!(
                "task `{}` metadata worktree `{}` disagrees with registered workspace `{}`",
                task.id,
                declared.display(),
                workspace_root.display()
            ));
        }
    }
    let contract_bytes = serde_json::to_vec(&attached.contract)
        .context("serialize resolved Workbench project contract")?;
    Ok(ExecutionTargetBinding {
        project_id: project_id.to_owned(),
        project_contract_digest: format!("sha256:{:x}", Sha256::digest(contract_bytes)),
        workspace_root,
    })
}

#[cfg(test)]
fn try_acquire_execution_target_locks(
    root: &Path,
    task: &QueueRecord,
) -> Result<Option<ExecutionTargetLocks>> {
    let binding = resolve_execution_target(root, task)?;
    try_acquire_execution_target_locks_for_binding(root, binding)
}

fn try_acquire_execution_target_locks_for_binding(
    root: &Path,
    binding: ExecutionTargetBinding,
) -> Result<Option<ExecutionTargetLocks>> {
    let lock_dir = root.join("core/projects/tasks/.workbench-executor-locks");
    std::fs::create_dir_all(&lock_dir)
        .with_context(|| format!("create executor lock directory `{}`", lock_dir.display()))?;
    let key = format!("workspace:{}", binding.workspace_root.display());
    let digest = format!("{:x}", Sha256::digest(key.as_bytes()));
    let lock_path = lock_dir.join(format!("target-{digest}.lock"));
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open execution target lock `{}`", lock_path.display()))?;
    match FileExt::try_lock_exclusive(&lock) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
        Err(error) => return Err(error).context("acquire execution target lock"),
    }
    Ok(Some(ExecutionTargetLocks {
        _files: vec![lock],
        binding,
    }))
}

fn claim_execution_with_available_target(
    root: &Path,
    queue: &ActiveQueueExecutor,
) -> Result<Option<(ApprovedQueueClaim, ExecutionTargetLocks)>> {
    let mut excluded_task_ids = BTreeSet::new();
    let mut excluded_workspace_roots = BTreeSet::new();
    loop {
        let Some(task) = queue.next_approved_reconciling_orphans_excluding(&excluded_task_ids)?
        else {
            return Ok(None);
        };
        let binding = resolve_execution_target(root, &task)?;
        if excluded_workspace_roots.contains(&binding.workspace_root) {
            excluded_task_ids.insert(task.id);
            continue;
        }
        let workspace_root = binding.workspace_root.clone();
        let Some(locks) = try_acquire_execution_target_locks_for_binding(root, binding)? else {
            excluded_workspace_roots.insert(workspace_root);
            excluded_task_ids.insert(task.id);
            continue;
        };
        if let Some(claim) = queue.claim_approved_candidate(&task.id, &excluded_task_ids)? {
            return Ok(Some((claim, locks)));
        }
        excluded_task_ids.insert(task.id);
    }
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
        .map(str::trim)
        .filter(|id| !id.is_empty())
}

fn task_worktree_path(task: &QueueRecord) -> Option<&str> {
    task.extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("worktree_path"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
}

fn is_objective_leaf(task: &QueueRecord) -> bool {
    task.extra["meta"]["objective_leaf"].as_bool() == Some(true)
}

fn is_decomposable_objective(task: &QueueRecord) -> bool {
    !is_objective_leaf(task)
        && task.extra["meta"]["acceptance_artifact"]
            .as_str()
            .is_some_and(|path| !path.trim().is_empty())
        && task.extra["meta"]["acceptance_markers"]
            .as_array()
            .is_some_and(|markers| !markers.is_empty())
}

fn objective_plan_for_claim(root: &Path, task: &QueueRecord) -> Result<(ObjectivePlan, String)> {
    let meta = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("objective leaf `{}` omitted metadata", task.id))?;
    let plan = serde_json::from_value(
        meta.get("objective_plan")
            .cloned()
            .ok_or_else(|| anyhow!("objective leaf `{}` omitted its durable plan", task.id))?,
    )
    .context("decode durable objective plan from queue leaf")?;
    let receipt = meta
        .get("objective_plan_receipt")
        .and_then(Value::as_str)
        .filter(|value| value.starts_with("sha256:"))
        .ok_or_else(|| anyhow!("objective leaf `{}` omitted its plan receipt", task.id))?;
    let plan_run_id = meta
        .get("objective_plan_run_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("objective leaf `{}` omitted its plan run ID", task.id))?;
    if plan_run_id.len() > 200
        || !plan_run_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(anyhow!(
            "objective leaf `{}` has an unsafe plan run ID",
            task.id
        ));
    }
    let objective_id = meta
        .get("objective_id")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective_id", task.id))?;
    let validation = PlanValidator::default().validate_objective_plan(&plan);
    if !validation.ok {
        return Err(anyhow!(
            "objective leaf `{}` embeds an invalid plan: {}",
            task.id,
            validation.errors.join("; ")
        ));
    }
    let persisted_path = root
        .join("audit/workbench-queue")
        .join(plan_run_id)
        .join("objective_plan_receipt.json");
    let persisted_metadata = std::fs::metadata(&persisted_path).with_context(|| {
        format!(
            "inspect objective-plan receipt `{}`",
            persisted_path.display()
        )
    })?;
    if persisted_metadata.len() > MAX_OBJECTIVE_PLAN_RECEIPT_BYTES as u64 {
        return Err(anyhow!(
            "objective-plan receipt `{}` exceeds {} bytes",
            persisted_path.display(),
            MAX_OBJECTIVE_PLAN_RECEIPT_BYTES
        ));
    }
    let mut persisted: Value =
        serde_json::from_slice(&std::fs::read(&persisted_path).with_context(|| {
            format!("read objective-plan receipt `{}`", persisted_path.display())
        })?)
        .with_context(|| {
            format!(
                "parse objective-plan receipt `{}`",
                persisted_path.display()
            )
        })?;
    if persisted["receipt_digest"] != receipt
        || persisted["run_id"] != plan_run_id
        || persisted["objective_id"] != objective_id
        || persisted["plan"] != serde_json::to_value(&plan)?
        || persisted["validation"] != serde_json::to_value(&validation)?
    {
        return Err(anyhow!(
            "objective leaf `{}` does not match its persisted plan receipt",
            task.id
        ));
    }
    persisted
        .as_object_mut()
        .ok_or_else(|| anyhow!("objective-plan receipt must be a JSON object"))?
        .remove("receipt_digest");
    let computed = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&persisted)?)
    );
    if computed != receipt {
        return Err(anyhow!("objective-plan receipt digest mismatch"));
    }
    Ok((plan, receipt.to_owned()))
}

fn objective_leaf_contract<'a>(
    plan: &'a ObjectivePlan,
    task: &QueueRecord,
) -> Result<&'a ExecutableLeafContract> {
    let leaf_key = task.extra["meta"]["objective_leaf_key"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective_leaf_key", task.id))?;
    plan.leaf_contracts.get(leaf_key).ok_or_else(|| {
        anyhow!(
            "objective leaf `{}` omitted its executable contract",
            task.id
        )
    })
}

fn verification_prompt(task: &QueueRecord, contract: Option<&ExecutableLeafContract>) -> String {
    let checks = contract
        .map(|contract| contract.verification_checks.join(", "))
        .filter(|checks| !checks.is_empty())
        .unwrap_or_else(|| "every check declared by the attached project contract".into());
    format!(
        "Verify task {} by running these project-native checks: {checks}; do not modify project files.",
        task.id
    )
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
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(anyhow!(
            "required acceptance artifact `{relative_path}` is not a safe repository-relative path"
        ));
    }
    let canonical_root = root
        .canonicalize()
        .context("canonicalize repository root for acceptance validation")?;
    let artifact_path = root
        .join(relative)
        .canonicalize()
        .with_context(|| format!("resolve required acceptance artifact `{relative_path}`"))?;
    if !artifact_path.starts_with(&canonical_root) {
        return Err(anyhow!(
            "required acceptance artifact `{relative_path}` escapes the repository root"
        ));
    }
    let content = std::fs::read_to_string(&artifact_path)
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

fn continuation_decision_for_task(
    task: &QueueRecord,
    queue_status: &str,
    result: &str,
    detail: Option<&str>,
    prior_continuations: u64,
) -> &'static str {
    let max_attempts = task.extra["meta"]["budget"]["max_attempts"]
        .as_u64()
        .unwrap_or(2)
        .max(1);
    let attempts_used = task
        .extra
        .get("retry_sequence")
        .and_then(Value::as_u64)
        .or_else(|| task.extra["meta"]["retry_sequence"].as_u64())
        .unwrap_or(0)
        + 1;
    if queue_status != "completed"
        && result != "cancelled"
        && task.extra["meta"]["wait_until_utc"]
            .as_str()
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
            .map(|value| value.with_timezone(&Utc))
            .is_some_and(|not_before| not_before > Utc::now())
    {
        return "wait_until";
    }
    if queue_status != "completed" && result != "cancelled" && attempts_used >= max_attempts {
        return "replan_objective";
    }
    continuation_decision(queue_status, result, detail, prior_continuations)
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

    let mut plan = ObjectiveDecomposer::default().decompose_grounded(
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
    let project_id = task_project_id(task).unwrap_or(DEFAULT_PROJECT_ID);
    for contract in plan.leaf_contracts.values_mut() {
        contract.project_id = project_id.to_owned();
    }
    let validation = PlanValidator::default().validate_objective_plan(&plan);
    if !validation.ok {
        return Err(anyhow!(
            "objective decomposition failed validation: {}",
            validation.errors.join("; ")
        ));
    }
    Ok(plan)
}

fn objective_leaf_id(objective_id: &str, leaf_key: &str) -> String {
    let raw = format!("{objective_id}__{leaf_key}");
    let normalized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    let prefix = normalized.chars().take(96).collect::<String>();
    let digest = format!("{:x}", Sha256::digest(raw.as_bytes()));
    format!("{prefix}--{}", &digest[..16])
}

fn append_queue_values(root: &Path, values: &[Value]) -> Result<()> {
    let path = root.join("core/projects/tasks/queue.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)?;
    file.lock_exclusive()?;
    let write_result = (|| -> Result<()> {
        use std::io::Write;
        for value in values {
            writeln!(file, "{value}")?;
        }
        file.sync_data()?;
        Ok(())
    })();
    let unlock_result = FileExt::unlock(&file);
    write_result?;
    unlock_result?;
    Ok(())
}

fn append_queue_value(root: &Path, value: Value) -> Result<()> {
    append_queue_values(root, &[value])
}

fn materialize_objective_leaves(
    root: &Path,
    objective: &QueueRecord,
    plan: &ObjectivePlan,
    plan_receipt: &str,
    plan_run_id: &str,
) -> Result<Vec<QueueRecord>> {
    let validation = PlanValidator::default().validate_objective_plan(plan);
    if !validation.ok {
        return Err(anyhow!(
            "refusing to materialize invalid objective plan: {}",
            validation.errors.join("; ")
        ));
    }
    let root_meta = objective
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .ok_or_else(|| anyhow!("objective `{}` omitted governed metadata", objective.id))?;
    let mut leaves: Vec<QueueRecord> = Vec::with_capacity(plan.tasks.len());
    for planned in &plan.tasks {
        let contract = plan
            .leaf_contracts
            .get(&planned.key)
            .ok_or_else(|| anyhow!("plan leaf `{}` omitted executable contract", planned.key))?;
        let id = objective_leaf_id(&objective.id, &planned.key);
        let dependencies = planned
            .depends_on
            .iter()
            .map(|key| objective_leaf_id(&objective.id, key))
            .collect::<Vec<_>>();
        let mut meta = root_meta.clone();
        if !matches!(
            planned.key.as_str(),
            "produce-outcome" | "verify-acceptance"
        ) {
            meta.remove("acceptance_artifact");
            meta.remove("acceptance_markers");
        }
        meta.insert("objective_leaf".into(), Value::Bool(true));
        meta.insert("objective_id".into(), Value::String(objective.id.clone()));
        meta.insert(
            "objective_title".into(),
            Value::String(
                objective
                    .title
                    .as_deref()
                    .unwrap_or(objective.id.as_str())
                    .to_owned(),
            ),
        );
        meta.insert(
            "objective_leaf_key".into(),
            Value::String(planned.key.clone()),
        );
        meta.insert("objective_plan".into(), serde_json::to_value(plan)?);
        meta.insert(
            "objective_plan_receipt".into(),
            Value::String(plan_receipt.to_owned()),
        );
        meta.insert(
            "objective_plan_run_id".into(),
            Value::String(plan_run_id.to_owned()),
        );
        meta.insert(
            "objective_root_meta".into(),
            Value::Object(root_meta.clone()),
        );
        meta.insert(
            "project_id".into(),
            Value::String(contract.project_id.clone()),
        );
        meta.insert(
            "authority_class".into(),
            Value::String(contract.authority_class.clone()),
        );
        meta.insert(
            "verification_checks".into(),
            serde_json::to_value(&contract.verification_checks)?,
        );
        meta.insert(
            "evidence_requirements".into(),
            serde_json::to_value(&contract.evidence_requirements)?,
        );
        meta.insert(
            "budget".into(),
            json!({
                "max_joules": contract.max_joules,
                "max_cost_usd": contract.max_cost_usd,
                "max_attempts": contract.max_attempts,
                "timeout_seconds": contract.timeout_seconds,
            }),
        );
        meta.insert("depends_on".into(), serde_json::to_value(&dependencies)?);
        leaves.push(serde_json::from_value(json!({
            "contract": "arda.workbench.objective_leaf.v1",
            "id": id,
            "source_record_id": id,
            "title": planned.title,
            "owner": objective.owner,
            "priority": planned.priority,
            "status": if dependencies.is_empty() { "queued" } else { "blocked" },
            "queued_at_utc": Utc::now().to_rfc3339(),
            "objective_id": objective.id,
            "objective_leaf_key": planned.key,
            "depends_on": dependencies,
            "revision_sequence": 0,
            "continuation_sequence": 0,
            "meta": meta,
        }))?);
    }
    let mut values = leaves
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    for value in &mut values {
        value["status"] = Value::String("blocked".into());
    }
    values.push(json!({
        "contract": "arda.workbench.objective_waiting.v1",
        "id": objective.id,
        "source_record_id": objective.id,
        "title": objective.title,
        "owner": objective.owner,
        "priority": objective.priority,
        "status": "waiting",
        "continuation_decision": "continue_next_task",
        "objective_plan_receipt": plan_receipt,
        "objective_plan_run_id": plan_run_id,
        "materialized_leaf_ids": leaves.iter().map(|leaf| leaf.id.as_str()).collect::<Vec<_>>(),
        "meta": root_meta,
    }));
    append_queue_values(root, &values)?;
    let schedule_ledger =
        super::schedule::ScheduleLedger::new(root.join("core/projects/tasks/schedules.jsonl"));
    for leaf in &leaves {
        let objective_id = super::task_queue::queue_objective_id(leaf)
            .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective lineage", leaf.id))?;
        schedule_ledger.append(&super::schedule::ScheduleRecord {
            contract: super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: leaf.id.clone(),
            objective_id: objective_id.into(),
            mode: super::schedule::ScheduleMode::Immediate,
            state: super::schedule::ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: Some("governed objective leaf materialized".into()),
        })?;
    }
    let activations = leaves
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    append_queue_values(root, &activations)?;
    Ok(leaves)
}

fn materialize_continuation(
    root: &Path,
    task: &QueueRecord,
    run_id: &str,
    decision: &str,
    detail: &str,
) -> Result<()> {
    let continuation = continuation_sequence(task) + 1;
    let retry = task
        .extra
        .get("retry_sequence")
        .and_then(Value::as_u64)
        .or_else(|| task.extra["meta"]["retry_sequence"].as_u64())
        .unwrap_or(0)
        + 1;
    let revision = task
        .extra
        .get("revision_sequence")
        .and_then(Value::as_u64)
        .or_else(|| task.extra["meta"]["revision_sequence"].as_u64())
        .unwrap_or(0)
        + u64::from(decision == "revise_task");
    let mut deferred_schedule = None;
    let value = match decision {
        "retry_same_task" | "revise_task" | "wait_until" => {
            let mut meta = task
                .extra
                .get("meta")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| anyhow!("objective leaf `{}` omitted metadata", task.id))?;
            meta.insert(
                "continuation_decision".into(),
                Value::String(decision.into()),
            );
            meta.insert("continuation_sequence".into(), Value::from(continuation));
            meta.insert("retry_sequence".into(), Value::from(retry));
            meta.insert("revision_sequence".into(), Value::from(revision));
            meta.insert("revision_directive".into(), Value::String(detail.into()));
            if decision == "wait_until" {
                let objective_id =
                    super::task_queue::queue_objective_id(task).ok_or_else(|| {
                        anyhow!("objective leaf `{}` omitted objective lineage", task.id)
                    })?;
                let not_before_utc = meta
                    .get("wait_until_utc")
                    .and_then(Value::as_str)
                    .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                    .map(|value| value.with_timezone(&Utc))
                    .filter(|value| *value > Utc::now())
                    .ok_or_else(|| {
                        anyhow!(
                            "objective leaf `{}` omitted a future wait_until_utc",
                            task.id
                        )
                    })?;
                deferred_schedule = Some(super::schedule::ScheduleRecord {
                    contract: super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                    task_id: task.id.clone(),
                    objective_id: objective_id.to_owned(),
                    mode: super::schedule::ScheduleMode::Deferred,
                    state: super::schedule::ScheduleState::Scheduled,
                    not_before_utc: Some(not_before_utc),
                    interval_seconds: None,
                    recorded_at_utc: Utc::now(),
                    reason: Some(detail.to_owned()),
                });
            }
            json!({
                "contract": "arda.workbench.executable_continuation.v1",
                "id": task.id,
                "source_record_id": task.id,
                "title": task.title,
                "owner": task.owner,
                "priority": task.priority,
                "status": if decision == "wait_until" { "blocked" } else { "queued" },
                "queued_at_utc": Utc::now().to_rfc3339(),
                "continuation_decision": decision,
                "continuation_sequence": continuation,
                "retry_sequence": retry,
                "revision_sequence": revision,
                "parent_workbench_run_id": run_id,
                "workbench_run_id": attempt_workbench_run_id(&task.id, retry),
                "revision_directive": detail,
                "meta": meta,
            })
        }
        "replan_objective" => {
            let meta = task
                .extra
                .get("meta")
                .and_then(Value::as_object)
                .ok_or_else(|| anyhow!("objective leaf `{}` omitted metadata", task.id))?;
            let objective_id = meta
                .get("objective_id")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective_id", task.id))?;
            let objective_title = meta
                .get("objective_title")
                .and_then(Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective_title", task.id))?;
            let objective_root_meta = meta
                .get("objective_root_meta")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| {
                    anyhow!("objective leaf `{}` omitted objective_root_meta", task.id)
                })?;
            json!({
                "contract": "arda.workbench.executable_continuation.v1",
                "id": objective_id,
                "source_record_id": objective_id,
                "title": objective_title,
                "status": "queued",
                "queued_at_utc": Utc::now().to_rfc3339(),
                "continuation_decision": decision,
                "continuation_sequence": continuation,
                "retry_sequence": retry,
                "parent_workbench_run_id": run_id,
                "revision_directive": detail,
                "meta": objective_root_meta,
            })
        }
        other => return Err(anyhow!("continuation decision `{other}` is not executable")),
    };
    append_queue_value(root, value.clone())?;
    if let Some(schedule) = deferred_schedule {
        super::schedule::ScheduleLedger::new(root.join("core/projects/tasks/schedules.jsonl"))
            .append(&schedule)?;
        let mut activation = value;
        activation["status"] = Value::String("queued".into());
        activation["queued_at_utc"] = Value::String(Utc::now().to_rfc3339());
        append_queue_value(root, activation)?;
    }
    Ok(())
}

fn reconcile_terminal_objective_leaves(root: &Path) -> Result<()> {
    let records =
        super::task_queue::TaskQueueAnalyzer::new(root.join("core/projects/tasks/queue.jsonl"))
            .load()?;
    let effective = super::task_queue::TaskQueueAnalyzer::effective_records(records);
    let terminal_objectives = effective
        .iter()
        .filter(|record| {
            !is_objective_leaf(record)
                && matches!(record.status.as_deref(), Some("completed" | "cancelled"))
        })
        .map(|record| record.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let schedule_ledger =
        super::schedule::ScheduleLedger::new(root.join("core/projects/tasks/schedules.jsonl"));
    let mut schedules = schedule_ledger.effective()?;
    let mut prepared_objectives = std::collections::BTreeMap::new();
    for leaf in effective.into_iter().filter(is_objective_leaf) {
        if leaf.extra["meta"]["objective_id"]
            .as_str()
            .is_some_and(|id| terminal_objectives.contains(id))
        {
            continue;
        }
        if leaf.status.as_deref() == Some("blocked")
            && leaf.extra.get("contract").and_then(Value::as_str)
                == Some("arda.workbench.executable_continuation.v1")
            && leaf
                .extra
                .get("continuation_decision")
                .and_then(Value::as_str)
                == Some("wait_until")
        {
            let objective_id = super::task_queue::queue_objective_id(&leaf).ok_or_else(|| {
                anyhow!(
                    "prepared wait continuation `{}` omitted objective lineage",
                    leaf.id
                )
            })?;
            let due = DateTime::parse_from_rfc3339(
                leaf.extra["meta"]["wait_until_utc"]
                    .as_str()
                    .ok_or_else(|| anyhow!("prepared wait continuation omitted wait_until_utc"))?,
            )?
            .with_timezone(&Utc);
            let authority_matches = schedules.get(&leaf.id).is_some_and(|schedule| {
                schedule.objective_id == objective_id
                    && schedule.mode == super::schedule::ScheduleMode::Deferred
                    && schedule.state == super::schedule::ScheduleState::Scheduled
                    && schedule.not_before_utc == Some(due)
            });
            if !authority_matches {
                let schedule = super::schedule::ScheduleRecord {
                    contract: super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                    task_id: leaf.id.clone(),
                    objective_id: objective_id.to_owned(),
                    mode: super::schedule::ScheduleMode::Deferred,
                    state: super::schedule::ScheduleState::Scheduled,
                    not_before_utc: Some(due),
                    interval_seconds: None,
                    recorded_at_utc: Utc::now(),
                    reason: Some("reconciled prepared wait_until continuation".into()),
                };
                schedule_ledger.append(&schedule)?;
                schedules.insert(leaf.id.clone(), schedule);
            }
            let mut activation = serde_json::to_value(&leaf)?;
            activation["status"] = Value::String("queued".into());
            activation["queued_at_utc"] = Value::String(Utc::now().to_rfc3339());
            append_queue_value(root, activation)?;
            continue;
        }
        if leaf.extra.get("contract").and_then(Value::as_str)
            == Some("arda.workbench.objective_leaf.v1")
        {
            let objective_id = super::task_queue::queue_objective_id(&leaf)
                .ok_or_else(|| anyhow!("objective leaf `{}` omitted objective lineage", leaf.id))?;
            if !schedules.contains_key(&leaf.id) {
                let schedule = super::schedule::ScheduleRecord {
                    contract: super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                    task_id: leaf.id.clone(),
                    objective_id: objective_id.to_owned(),
                    mode: super::schedule::ScheduleMode::Immediate,
                    state: super::schedule::ScheduleState::Scheduled,
                    not_before_utc: None,
                    interval_seconds: None,
                    recorded_at_utc: Utc::now(),
                    reason: Some("reconciled prepared objective leaf".into()),
                };
                schedule_ledger.append(&schedule)?;
                schedules.insert(leaf.id.clone(), schedule);
            }
            if leaf.status.as_deref() == Some("blocked") {
                prepared_objectives
                    .entry(objective_id.to_owned())
                    .or_insert_with(|| leaf.clone());
            }
        }
        match leaf.status.as_deref() {
            Some("completed") => advance_objective_after_leaf(root, &leaf)?,
            Some("failed") => {
                let decision = leaf
                    .extra
                    .get("continuation_decision")
                    .and_then(Value::as_str);
                if let Some(
                    decision @ ("retry_same_task" | "revise_task" | "replan_objective"
                    | "wait_until"),
                ) = decision
                {
                    let run_id = leaf
                        .extra
                        .get("workbench_run_id")
                        .and_then(Value::as_str)
                        .unwrap_or("reconciled-terminal-leaf");
                    let detail = leaf
                        .extra
                        .get("detail")
                        .and_then(Value::as_str)
                        .unwrap_or("reconciled terminal leaf continuation");
                    materialize_continuation(root, &leaf, run_id, decision, detail)?;
                }
            }
            _ => {}
        }
    }
    for leaf in prepared_objectives.into_values() {
        advance_objective_after_leaf(root, &leaf)?;
    }
    Ok(())
}

fn advance_objective_after_leaf(root: &Path, completed_leaf: &QueueRecord) -> Result<()> {
    let objective_id = completed_leaf.extra["meta"]["objective_id"]
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            anyhow!(
                "objective leaf `{}` omitted objective_id",
                completed_leaf.id
            )
        })?;
    let records =
        super::task_queue::TaskQueueAnalyzer::new(root.join("core/projects/tasks/queue.jsonl"))
            .load()?;
    let effective = super::task_queue::TaskQueueAnalyzer::effective_records(records);
    let leaves = effective
        .iter()
        .filter(|record| {
            record.extra["meta"]["objective_leaf"].as_bool() == Some(true)
                && record.extra["meta"]["objective_id"].as_str() == Some(objective_id)
        })
        .collect::<Vec<_>>();
    if leaves.is_empty() {
        return Err(anyhow!("objective `{objective_id}` has no durable leaves"));
    }
    let completed_ids = leaves
        .iter()
        .filter(|leaf| leaf.status.as_deref() == Some("completed"))
        .map(|leaf| leaf.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if completed_ids.len() == leaves.len() {
        let mut evidence = leaves
            .iter()
            .map(|leaf| {
                leaf.extra
                    .get("execution_receipt_digest")
                    .and_then(Value::as_str)
                    .filter(|digest| digest.starts_with("sha256:"))
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        anyhow!(
                            "objective leaf `{}` cannot close without an execution receipt",
                            leaf.id
                        )
                    })
            })
            .collect::<Result<Vec<_>>>()?;
        evidence.sort();
        let acceptance_leaf = leaves
            .iter()
            .find(|leaf| leaf.extra["meta"]["objective_leaf_key"] == "verify-acceptance")
            .ok_or_else(|| anyhow!("objective `{objective_id}` omitted acceptance leaf"))?;
        validate_task_acceptance_artifact(root, acceptance_leaf)?;
        let artifact = acceptance_leaf.extra["meta"]["acceptance_artifact"]
            .as_str()
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("objective `{objective_id}` omitted acceptance artifact"))?;
        let root_meta = acceptance_leaf.extra["meta"]["objective_root_meta"].clone();
        return append_queue_value(
            root,
            json!({
                "contract": "arda.workbench.objective_terminal.v1",
                "id": objective_id,
                "source_record_id": objective_id,
                "title": acceptance_leaf.extra["meta"]["objective_title"],
                "status": "completed",
                "result": "completed",
                "completed_at_utc": Utc::now().to_rfc3339(),
                "continuation_decision": "close_complete",
                "acceptance_artifact": artifact,
                "acceptance_leaf_id": acceptance_leaf.id,
                "closure_evidence_receipts": evidence,
                "meta": root_meta,
            }),
        );
    }

    let mut activated = Vec::new();
    for leaf in leaves {
        if leaf.status.as_deref() != Some("blocked") {
            continue;
        }
        let dependencies = leaf.extra["meta"]["depends_on"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();
        if dependencies
            .iter()
            .all(|dependency| completed_ids.contains(dependency))
        {
            let mut value = serde_json::to_value(leaf)?;
            value["status"] = Value::String("queued".into());
            value["queued_at_utc"] = Value::String(Utc::now().to_rfc3339());
            value["continuation_decision"] = Value::String("continue_next_task".into());
            activated.push(value);
        }
    }
    append_queue_values(root, &activated)
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
        .filter(|planned| {
            task.extra["meta"]["objective_leaf_key"]
                .as_str()
                .is_none_or(|key| key == planned.key)
        })
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
    let contract = objective_leaf_contract(plan, task).ok();
    let checks = contract
        .map(|contract| contract.verification_checks.join(", "))
        .filter(|checks| !checks.is_empty())
        .map(|checks| format!(" Required project-native checks: {checks}."))
        .unwrap_or_default();
    let revision_directive = task
        .extra
        .get("revision_directive")
        .and_then(Value::as_str)
        .or_else(|| task.extra["meta"]["revision_directive"].as_str())
        .filter(|directive| !directive.trim().is_empty())
        .map(|directive| {
            format!(" Correct the prior failed attempt before proceeding: {directive}.")
        })
        .unwrap_or_default();
    format!(
        "{}\n\nExecute this validated objective plan in dependency order:\n{}\n\nRead and cite these live authorities before changing anything:\n{}\n\nFinal output must be a concrete prioritized repair backlog with evidence, human-visible behavior, and the smallest authoritative implementation surface.{}{}{} Preserve unrelated dirty work and do not edit generated queue projections.",
        objective, tasks, sources, artifact_requirement, checks, revision_directive
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
    run_graph_value(run_id, task_id, objective, approval_id, None, None)
}

fn run_graph_with_objective_plan_receipt(
    run_id: &str,
    task_id: &str,
    objective: &str,
    approval_id: &str,
    objective_plan_receipt: &str,
    leaf_contract: Option<&ExecutableLeafContract>,
) -> Value {
    run_graph_value(
        run_id,
        task_id,
        objective,
        approval_id,
        Some(objective_plan_receipt),
        leaf_contract,
    )
}

fn run_graph_value(
    run_id: &str,
    task_id: &str,
    objective: &str,
    approval_id: &str,
    objective_plan_receipt: Option<&str>,
    leaf_contract: Option<&ExecutableLeafContract>,
) -> Value {
    let prompt_digest = format!("sha256:{:x}", Sha256::digest(objective.as_bytes()));
    let deadline = Utc::now().timestamp_millis().saturating_add(1_200_000) as u128;
    let execute_authority = leaf_contract
        .map(|contract| contract.authority_class.as_str())
        .unwrap_or("execute_with_approval");
    let node = |id: &str, kind: &str, authority: &str, parents: Vec<&str>, worker: Value| {
        let governed = matches!(id, "execute" | "verify");
        let max_joules = leaf_contract
            .filter(|_| governed)
            .map(|contract| contract.max_joules)
            .unwrap_or(5000.0);
        let max_cost_usd = leaf_contract
            .filter(|_| governed)
            .map(|contract| contract.max_cost_usd)
            .unwrap_or(2.0);
        let max_attempts = leaf_contract
            .filter(|_| governed)
            .map(|contract| contract.max_attempts)
            .unwrap_or(2);
        let timeout_ms = leaf_contract
            .filter(|_| governed)
            .map(|contract| contract.timeout_seconds.saturating_mul(1_000))
            .unwrap_or(900_000);
        json!({
            "id": id,
            "kind": kind,
            "state": "pending",
            "authority": authority,
            "budget": {"max_joules": max_joules, "max_cost_usd": max_cost_usd},
            "retry": {"max_attempts": max_attempts},
            "timeout_ms": timeout_ms,
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
            node("execute", "execute", execute_authority, vec![approval_id], json!({
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
    use crate::prometheus::autopilot::TaskQueueAnalyzer;
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

    fn execution_target_task(
        id: &str,
        project_id: Option<&str>,
        worktree_path: Option<&str>,
    ) -> QueueRecord {
        serde_json::from_value(json!({
            "id": id,
            "status": "in_progress",
            "meta": {
                "project_id": project_id,
                "worktree_path": worktree_path
            }
        }))
        .expect("execution target task")
    }

    fn write_execution_project_registry(root: &Path, projects: &[(&str, &str)]) {
        let entries = projects
            .iter()
            .map(|(project_id, workspace_root)| {
                json!({
                    "contract": {
                        "schema_version": "arda.project-contract.v1",
                        "identity": {
                            "project_id": project_id,
                            "name": format!("fixture-{project_id}"),
                            "kind": "rust"
                        },
                        "workspace": {"root": workspace_root},
                        "runtime": {"adapter": "cargo"},
                        "commands": [],
                        "checks": [],
                        "artifacts": [],
                        "permissions": {},
                        "rollback": {"strategy": "git_revert"},
                        "memory": {"scope": "project"},
                        "provenance": {
                            "declared_by": "execution-lock-test",
                            "declared_at": "2026-08-28T00:00:00Z"
                        }
                    },
                    "approval_id": format!("approval-{project_id}"),
                    "proposal_id": format!("proposal-{project_id}"),
                    "idempotency_key": format!("attach-{project_id}")
                })
            })
            .collect::<Vec<_>>();
        for (_, workspace_root) in projects {
            std::fs::create_dir_all(root.join(workspace_root)).unwrap();
        }
        let path = root.join("data/workbench/projects.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            path,
            serde_json::to_vec_pretty(&json!({
                "schema_version": "arda.workbench.project-registry.v1",
                "projects": entries
            }))
            .unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn execution_target_locks_block_distinct_projects_with_the_same_registered_workspace() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project_a = "550e8400-e29b-41d4-a716-446655440001";
        let project_b = "550e8400-e29b-41d4-a716-446655440002";
        write_execution_project_registry(dir.path(), &[(project_a, "."), (project_b, ".")]);
        let first_task = execution_target_task("first", Some(project_a), None);
        let second_task = execution_target_task("second", Some(project_b), None);
        let first = try_acquire_execution_target_locks(dir.path(), &first_task)
            .unwrap()
            .expect("first physical workspace lock");

        assert!(
            try_acquire_execution_target_locks(dir.path(), &second_task)
                .unwrap()
                .is_none(),
            "different metadata must not bypass a shared registered workspace lock"
        );
        drop(first);
    }

    #[test]
    fn execution_target_locks_reject_unknown_registered_project_identity() {
        let dir = tempfile::tempdir().expect("create lock root");
        write_execution_project_registry(
            dir.path(),
            &[("550e8400-e29b-41d4-a716-446655440001", ".")],
        );
        let task = execution_target_task(
            "unknown",
            Some("550e8400-e29b-41d4-a716-446655440099"),
            None,
        );

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[test]
    fn execution_target_locks_reject_oversized_project_registry_before_parsing() {
        let dir = tempfile::tempdir().expect("create lock root");
        let registry_path = dir.path().join("data/workbench/projects.json");
        std::fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
        std::fs::write(&registry_path, vec![b' '; 1_048_577]).unwrap();
        let task = execution_target_task(
            "oversized-registry",
            Some("550e8400-e29b-41d4-a716-446655440001"),
            None,
        );

        let error = try_acquire_execution_target_locks(dir.path(), &task)
            .expect_err("oversized registry must fail closed");
        assert!(
            error.to_string().contains("exceeds maximum size"),
            "registry must be rejected before unbounded parsing: {error:#}"
        );
    }

    #[test]
    fn execution_target_locks_reject_duplicate_registered_project_ids() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, "."), (project, ".")]);
        let task = execution_target_task("duplicate-project", Some(project), None);

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[test]
    fn execution_target_locks_reject_unknown_registry_fields() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let registry_path = dir.path().join("data/workbench/projects.json");
        let mut registry: Value =
            serde_json::from_slice(&std::fs::read(&registry_path).unwrap()).unwrap();
        registry["unexpected"] = json!(true);
        std::fs::write(&registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();
        let task = execution_target_task("unknown-field", Some(project), None);

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[test]
    fn execution_target_locks_reject_unsupported_registry_schema() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let registry_path = dir.path().join("data/workbench/projects.json");
        let mut registry: Value =
            serde_json::from_slice(&std::fs::read(&registry_path).unwrap()).unwrap();
        registry["schema_version"] = json!("arda.workbench.project-registry.v2");
        std::fs::write(&registry_path, serde_json::to_vec(&registry).unwrap()).unwrap();
        let task = execution_target_task("unsupported-schema", Some(project), None);

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn execution_target_locks_reject_registered_workspace_escaping_root() {
        let dir = tempfile::tempdir().expect("create lock root");
        let outside = tempfile::tempdir().expect("create external workspace");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("escape")).unwrap();
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, "escape")]);
        let task = execution_target_task("escaping-root", Some(project), None);

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[test]
    fn execution_target_locks_allow_distinct_registered_workspace_roots() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project_a = "550e8400-e29b-41d4-a716-446655440001";
        let project_b = "550e8400-e29b-41d4-a716-446655440002";
        write_execution_project_registry(
            dir.path(),
            &[
                (project_a, "worktrees/first"),
                (project_b, "worktrees/second"),
            ],
        );
        let first = execution_target_task("first-project-task", Some(project_a), None);
        let second = execution_target_task("second-project-task", Some(project_b), None);

        let _first_locks = try_acquire_execution_target_locks(dir.path(), &first)
            .expect("acquire first target locks")
            .expect("first target should be available");
        let second_locks = try_acquire_execution_target_locks(dir.path(), &second)
            .expect("inspect second target locks");

        assert!(
            second_locks.is_some(),
            "distinct registered physical workspaces must execute concurrently"
        );
    }

    #[test]
    fn execution_target_locks_trim_project_identity() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let first_task = execution_target_task("first", Some(project), None);
        let second_task = execution_target_task("second", Some(&format!(" {project} ")), None);
        let first = try_acquire_execution_target_locks(dir.path(), &first_task)
            .unwrap()
            .expect("first project lock");

        assert!(try_acquire_execution_target_locks(dir.path(), &second_task)
            .unwrap()
            .is_none());
        drop(first);
    }

    #[test]
    fn execution_target_locks_reject_metadata_worktree_mismatch() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, "registered")]);
        std::fs::create_dir_all(dir.path().join("claimed")).unwrap();
        let task = execution_target_task("mismatch", Some(project), Some("claimed"));

        assert!(try_acquire_execution_target_locks(dir.path(), &task).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn execution_target_locks_block_symlink_aliases_of_the_same_workspace() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project_a = "550e8400-e29b-41d4-a716-446655440001";
        let project_b = "550e8400-e29b-41d4-a716-446655440002";
        std::fs::create_dir_all(dir.path().join("physical")).unwrap();
        std::os::unix::fs::symlink("physical", dir.path().join("alias")).unwrap();
        write_execution_project_registry(
            dir.path(),
            &[(project_a, "physical"), (project_b, "alias")],
        );
        let first_task = execution_target_task("first", Some(project_a), None);
        let second_task = execution_target_task("second", Some(project_b), None);
        let first = try_acquire_execution_target_locks(dir.path(), &first_task)
            .unwrap()
            .expect("first worktree lock");

        assert!(try_acquire_execution_target_locks(dir.path(), &second_task)
            .unwrap()
            .is_none());
        drop(first);
    }

    #[test]
    fn execution_target_locks_fail_closed_without_project_identity() {
        let dir = tempfile::tempdir().expect("create lock root");
        let legacy = execution_target_task("legacy", None, None);

        assert!(try_acquire_execution_target_locks(dir.path(), &legacy).is_err());
    }

    #[test]
    fn coordinator_skips_busy_orphan_and_claims_distinct_project() {
        let dir = tempfile::tempdir().expect("create coordinator root");
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let record = |id: &str, project_id: &str, worktree_path: &str| -> QueueRecord {
            serde_json::from_value(json!({
                "id": id,
                "status": "queued",
                "meta": {
                    "action_class": "approved_autopilot_plan_step",
                    "mutation_risk": "operator-approved",
                    "execution_authority": "arda_workbench",
                    "source_objective_packet_id": format!("objective-{id}"),
                    "approval_packet_id": format!("approval-{id}"),
                    "project_id": project_id,
                    "worktree_path": worktree_path
                }
            }))
            .expect("queue record")
        };
        let project_a = "550e8400-e29b-41d4-a716-446655440001";
        let project_b = "550e8400-e29b-41d4-a716-446655440002";
        let project_c = "550e8400-e29b-41d4-a716-446655440003";
        write_execution_project_registry(
            dir.path(),
            &[
                (project_a, "worktrees/first"),
                (project_b, "worktrees/second"),
                (project_c, "worktrees/first"),
            ],
        );
        std::os::unix::fs::symlink("first", dir.path().join("worktrees/metadata-alias"))
            .expect("metadata symlink alias");
        let first = record("busy-orphan", project_a, "worktrees/first");
        let same_root = record("same-root-queued", project_c, "worktrees/metadata-alias");
        let second = record("available-project", project_b, "worktrees/second");
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&same_root).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        for task in [&first, &same_root, &second] {
            super::super::schedule::ScheduleLedger::new(
                dir.path().join("core/projects/tasks/schedules.jsonl"),
            )
            .append(&super::super::schedule::ScheduleRecord {
                contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: format!("objective-{}", task.id),
                mode: super::super::schedule::ScheduleMode::Immediate,
                state: super::super::schedule::ScheduleState::Scheduled,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: Some("coordinator lock test".into()),
            })
            .unwrap();
        }
        let queue = ActiveQueueExecutor::new(dir.path());
        let first_claim = queue
            .claim_next_approved()
            .unwrap()
            .expect("claim first project");
        let _busy_locks = try_acquire_execution_target_locks(dir.path(), &first_claim.task)
            .unwrap()
            .expect("hold first project locks");

        let (next, _next_locks) = claim_execution_with_available_target(dir.path(), &queue)
            .unwrap()
            .expect("claim distinct available project");

        assert_eq!(next.task.id, second.id);
        assert_eq!(next.attempt.task_id, second.id);
        let same_root_effective = TaskQueueAnalyzer::effective_records(
            TaskQueueAnalyzer::new(&queue_path)
                .load()
                .expect("queue records"),
        )
        .into_iter()
        .find(|record| record.id == same_root.id)
        .expect("same-root task remains present");
        assert_eq!(same_root_effective.status.as_deref(), Some("queued"));
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
                        "project_id": DEFAULT_PROJECT_ID,
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
        super::super::schedule::ScheduleLedger::new(
            root.join("core/projects/tasks/schedules.jsonl"),
        )
        .append(&super::super::schedule::ScheduleRecord {
            contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: task_id.into(),
            objective_id: "objective-reconciliation".into(),
            mode: super::super::schedule::ScheduleMode::Immediate,
            state: super::super::schedule::ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: Some("approved queue test fixture".into()),
        })
        .unwrap();
        write_execution_project_registry(root, &[(DEFAULT_PROJECT_ID, ".")]);
        for relative_path in [
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
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(2))
                .build()
                .unwrap(),
        }
    }

    #[tokio::test]
    async fn timer_execution_materializes_governed_objective_before_provider_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "timer-objective");
        let mut root: Value = serde_json::from_str(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        root["meta"]["acceptance_artifact"] =
            Value::String("docs/audits/timer-acceptance.md".into());
        root["meta"]["acceptance_markers"] = json!(["timer evidence"]);
        root["meta"]["project_id"] = Value::String(DEFAULT_PROJECT_ID.into());
        std::fs::write(&queue_path, format!("{root}\n")).unwrap();

        let receipt = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .execute_once()
            .await
            .unwrap();

        assert_eq!(receipt.status, "waiting");
        assert_eq!(receipt.result, "objective_decomposed");
        assert_eq!(
            receipt.continuation_decision.as_deref(),
            Some("continue_next_task")
        );
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(
            effective
                .iter()
                .filter(|record| record.extra["meta"]["objective_leaf"] == true)
                .count(),
            5
        );
        let schedules = super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .effective()
        .expect("replay materialized leaf schedules");
        let leaves = effective
            .iter()
            .filter(|record| record.extra["meta"]["objective_leaf"] == true)
            .collect::<Vec<_>>();
        assert_eq!(schedules.len(), leaves.len() + 1);
        for leaf in leaves {
            let schedule = schedules
                .get(&leaf.id)
                .expect("objective leaf schedule authority");
            assert_eq!(
                schedule.mode,
                super::super::schedule::ScheduleMode::Immediate
            );
            assert_eq!(
                schedule.state,
                super::super::schedule::ScheduleState::Scheduled
            );
            assert_eq!(
                Some(schedule.objective_id.as_str()),
                super::super::task_queue::queue_objective_id(leaf)
            );
        }
        assert_eq!(
            effective
                .iter()
                .find(|record| record.id == "timer-objective")
                .and_then(|record| record.status.as_deref()),
            Some("waiting")
        );
    }

    #[tokio::test]
    async fn timer_tick_reconciles_due_recurring_objective_before_claim() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "recurring-objective");
        let now = Utc::now();
        let mut root: Value = serde_json::from_str(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        root["status"] = Value::String("completed".into());
        root["result"] = Value::String("completed".into());
        root["completed_at_utc"] = Value::String((now - chrono::Duration::minutes(2)).to_rfc3339());
        root["meta"]["acceptance_artifact"] =
            Value::String("docs/audits/recurring-acceptance.md".into());
        root["meta"]["acceptance_markers"] = json!(["recurring evidence"]);
        root["meta"]["project_id"] = Value::String(DEFAULT_PROJECT_ID.into());
        std::fs::write(&queue_path, format!("{root}\n")).unwrap();
        super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .append(&super::super::schedule::ScheduleRecord {
            contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "recurring-objective".into(),
            objective_id: "objective-reconciliation".into(),
            mode: super::super::schedule::ScheduleMode::Recurring,
            state: super::super::schedule::ScheduleState::Scheduled,
            not_before_utc: Some(now - chrono::Duration::minutes(3)),
            interval_seconds: Some(60),
            recorded_at_utc: now - chrono::Duration::minutes(3),
            reason: None,
        })
        .unwrap();

        let receipt = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .execute_once()
            .await
            .unwrap();

        assert_eq!(receipt.status, "waiting");
        assert_eq!(receipt.result, "objective_decomposed");
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

        let binding = resolve_execution_target(dir.path(), &claim.task).unwrap();
        let outcome = test_executor(dir.path(), harness_url)
            .dispatch_claim(&claim, &binding)
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
    async fn execute_once_materializes_future_wait_until_after_terminal_failure() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "waiting-objective");
        let due = Utc::now() + chrono::Duration::minutes(30);
        let mut objective: Value = serde_json::from_str(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        objective["meta"]["acceptance_artifact"] =
            Value::String("docs/audits/waiting-acceptance.md".into());
        objective["meta"]["acceptance_markers"] = json!(["waiting evidence"]);
        objective["meta"]["project_id"] = Value::String(DEFAULT_PROJECT_ID.into());
        std::fs::write(&queue_path, format!("{objective}\n")).unwrap();
        let decomposition = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .execute_once()
            .await
            .unwrap();
        assert_eq!(decomposition.result, "objective_decomposed");
        let mut waiting_leaf = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(&queue_path)
                .load()
                .unwrap(),
        )
        .into_iter()
        .find(|record| record.extra["meta"]["objective_leaf"] == true)
        .expect("durable objective leaf");
        waiting_leaf.extra["meta"]["wait_until_utc"] = Value::String(due.to_rfc3339());
        waiting_leaf.extra["meta"]["budget"] = json!({"max_attempts": 2});
        let waiting_leaf_id = waiting_leaf.id.clone();
        let mut queue_file = std::fs::OpenOptions::new()
            .append(true)
            .open(&queue_path)
            .unwrap();
        serde_json::to_writer(&mut queue_file, &waiting_leaf).unwrap();
        std::io::Write::write_all(&mut queue_file, b"\n").unwrap();
        queue_file.sync_data().unwrap();
        let (harness_url, server) = scripted_harness(vec![Some((
            200,
            json!({
                "graph": {"nodes": [
                    {"id": "execute", "state": "failed"},
                    {"id": "verify", "state": "blocked"},
                    {"id": "review", "state": "blocked"},
                    {"id": "close", "state": "failed", "output_digest": "sha256:failed"}
                ]},
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

        assert_eq!(
            receipt.continuation_decision.as_deref(),
            Some("wait_until"),
            "unexpected executor receipt: {receipt:?}"
        );
        assert_eq!(receipt.task_id.as_deref(), Some(waiting_leaf_id.as_str()));
        let schedule = super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .effective()
        .unwrap()
        .remove(&waiting_leaf_id)
        .expect("deferred schedule authority");
        assert_eq!(
            schedule.mode,
            super::super::schedule::ScheduleMode::Deferred
        );
        assert_eq!(schedule.not_before_utc, Some(due));
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(
            effective
                .iter()
                .find(|record| record.id == waiting_leaf_id)
                .and_then(|record| record.status.as_deref()),
            Some("queued")
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
        super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .append(&super::super::schedule::ScheduleRecord {
            contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "pre-dispatch-crash-task".into(),
            objective_id: "objective-crash-proof".into(),
            mode: super::super::schedule::ScheduleMode::Immediate,
            state: super::super::schedule::ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: Some("process restart fixture schedule authority".into()),
        })
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

        let one_attempt: QueueRecord = serde_json::from_value(json!({
            "id": "bounded-leaf",
            "meta": {"budget": {"max_attempts": 1}}
        }))
        .unwrap();
        assert_eq!(
            continuation_decision_for_task(
                &one_attempt,
                "failed",
                "failed",
                Some("provider timeout"),
                0,
            ),
            "replan_objective"
        );
    }

    #[test]
    fn future_wait_until_materializes_deferred_schedule_authority() {
        let dir = tempfile::tempdir().unwrap();
        let due = Utc::now() + chrono::Duration::minutes(30);
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "waiting-leaf",
            "title": "Wait for an external dependency",
            "status": "failed",
            "meta": {
                "objective_leaf": true,
                "objective_id": "objective-wait",
                "source_objective_packet_id": "objective-wait",
                "wait_until_utc": due.to_rfc3339(),
                "budget": {"max_attempts": 2}
            }
        }))
        .unwrap();

        let decision = continuation_decision_for_task(
            &task,
            "failed",
            "failed",
            Some("external dependency unavailable"),
            0,
        );
        assert_eq!(decision, "wait_until");
        materialize_continuation(
            dir.path(),
            &task,
            "queue-waiting-leaf",
            decision,
            "external dependency unavailable",
        )
        .unwrap();

        let schedule = super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .effective()
        .unwrap()
        .remove("waiting-leaf")
        .expect("deferred schedule");
        assert_eq!(
            schedule.mode,
            super::super::schedule::ScheduleMode::Deferred
        );
        assert_eq!(
            schedule.state,
            super::super::schedule::ScheduleState::Scheduled
        );
        assert_eq!(schedule.not_before_utc, Some(due));
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(
                dir.path().join("core/projects/tasks/queue.jsonl"),
            )
            .load()
            .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("queued"));
        assert_eq!(
            effective[0].extra["continuation_decision"].as_str(),
            Some("wait_until")
        );
    }

    #[test]
    fn restart_reconciles_terminal_leaf_successor_activation() {
        let dir = tempfile::tempdir().unwrap();
        let first_id = objective_leaf_id("objective-crash", "first");
        let second_id = objective_leaf_id("objective-crash", "second");
        append_queue_values(
            dir.path(),
            &[
                json!({
                    "id": first_id,
                    "source_record_id": first_id,
                    "status": "completed",
                    "execution_receipt_digest": "sha256:first",
                    "meta": {
                        "objective_leaf": true,
                        "objective_id": "objective-crash",
                        "objective_leaf_key": "first",
                        "depends_on": []
                    }
                }),
                json!({
                    "id": second_id,
                    "source_record_id": second_id,
                    "status": "blocked",
                    "meta": {
                        "objective_leaf": true,
                        "objective_id": "objective-crash",
                        "objective_leaf_key": "second",
                        "depends_on": [first_id]
                    }
                }),
            ],
        )
        .unwrap();

        reconcile_terminal_objective_leaves(dir.path()).unwrap();

        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(
                dir.path().join("core/projects/tasks/queue.jsonl"),
            )
            .load()
            .unwrap(),
        );
        assert_eq!(
            effective
                .iter()
                .find(|record| record.id == second_id)
                .unwrap()
                .status
                .as_deref(),
            Some("queued")
        );
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        let records_after_repair = std::fs::read_to_string(&queue_path)
            .unwrap()
            .lines()
            .count();
        reconcile_terminal_objective_leaves(dir.path()).unwrap();
        assert_eq!(
            std::fs::read_to_string(queue_path).unwrap().lines().count(),
            records_after_repair,
            "restart reconciliation must not append duplicate successor records"
        );
    }

    #[test]
    fn objective_leaves_and_corrected_revision_survive_executor_restart() {
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
            let contents = if path.ends_with("core/projects/tasks/queue.jsonl") {
                ""
            } else {
                "authoritative fixture\n"
            };
            std::fs::write(path, contents).unwrap();
        }
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "objective-durable",
            "title": "Produce the required acceptance artifact",
            "status": "in_progress",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "packet-durable",
                "approval_packet_id": "approval-durable",
                "project_id": DEFAULT_PROJECT_ID,
                "acceptance_artifact": "docs/audits/acceptance.md",
                "acceptance_markers": ["evidence"]
            }
        }))
        .unwrap();
        let (plan, plan_receipt) =
            persisted_objective_plan_for_task(dir.path(), "queue-objective-durable", &task)
                .unwrap();

        let leaves = materialize_objective_leaves(
            dir.path(),
            &task,
            &plan,
            &plan_receipt,
            "queue-objective-durable",
        )
        .unwrap();
        assert_eq!(leaves.len(), 5);

        let restarted = super::super::task_queue::TaskQueueAnalyzer::new(
            dir.path().join("core/projects/tasks/queue.jsonl"),
        );
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            restarted.load().unwrap(),
        );
        assert_eq!(
            effective
                .iter()
                .filter(|record| record.extra["meta"]["objective_leaf"] == true)
                .count(),
            5
        );
        assert_eq!(
            effective
                .iter()
                .filter(|record| record.status.as_deref() == Some("queued"))
                .count(),
            1
        );
        assert!(leaves.iter().all(|leaf| {
            let meta = &leaf.extra["meta"];
            meta["objective_id"] == "objective-durable"
                && meta["project_id"] == DEFAULT_PROJECT_ID
                && meta["authority_class"].is_string()
                && meta["verification_checks"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                && meta["evidence_requirements"]
                    .as_array()
                    .is_some_and(|v| !v.is_empty())
                && meta["budget"]["max_joules"]
                    .as_f64()
                    .is_some_and(|v| v > 0.0)
        }));
        let producer = leaves
            .iter()
            .find(|leaf| leaf.extra["meta"]["objective_leaf_key"] == "produce-outcome")
            .unwrap();
        assert_eq!(
            producer.extra["meta"]["authority_class"],
            "execute_with_approval"
        );
        assert_eq!(
            producer.extra["meta"]["acceptance_artifact"],
            "docs/audits/acceptance.md"
        );
        let acceptance = leaves
            .iter()
            .find(|leaf| leaf.extra["meta"]["objective_leaf_key"] == "verify-acceptance")
            .unwrap();
        assert_eq!(acceptance.extra["meta"]["authority_class"], "read_only");
        assert_eq!(
            acceptance.extra["meta"]["acceptance_artifact"],
            "docs/audits/acceptance.md"
        );
        assert_eq!(
            objective_plan_for_claim(dir.path(), acceptance).unwrap().1,
            plan_receipt
        );
        let mut tampered = acceptance.clone();
        tampered.extra["meta"]["objective_plan"]["tasks"][0]["title"] =
            Value::String("tampered task".into());
        assert!(objective_plan_for_claim(dir.path(), &tampered)
            .unwrap_err()
            .to_string()
            .contains("does not match its persisted plan receipt"));

        let failed_leaf = leaves
            .iter()
            .find(|leaf| leaf.extra["meta"]["objective_leaf_key"] == "verify-acceptance")
            .unwrap();
        append_queue_value(
            dir.path(),
            json!({
                "id": failed_leaf.id,
                "source_record_id": failed_leaf.id,
                "title": failed_leaf.title,
                "owner": failed_leaf.owner,
                "priority": failed_leaf.priority,
                "status": "failed",
                "result": "failed",
                "workbench_run_id": "queue-objective-durable__verify-acceptance",
                "meta": failed_leaf.extra["meta"],
            }),
        )
        .unwrap();
        materialize_continuation(
            dir.path(),
            failed_leaf,
            "queue-objective-durable__verify-acceptance",
            "revise_task",
            "acceptance criteria were not satisfied",
        )
        .unwrap();

        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            restarted.load().unwrap(),
        );
        let revised = effective
            .iter()
            .find(|record| record.id == failed_leaf.id)
            .unwrap();
        assert_eq!(revised.status.as_deref(), Some("queued"));
        assert_eq!(revised.extra["continuation_decision"], "revise_task");
        assert_eq!(revised.extra["revision_sequence"], 1);
        assert_eq!(
            revised.extra["workbench_run_id"],
            attempt_workbench_run_id(&failed_leaf.id, 1)
        );
        assert_eq!(revised.extra["meta"]["objective_id"], "objective-durable");
        assert!(revised.extra["revision_directive"]
            .as_str()
            .unwrap()
            .contains("acceptance criteria"));
        assert!(revised.extra["meta"]["revision_directive"]
            .as_str()
            .unwrap()
            .contains("acceptance criteria"));
        let revised_prompt =
            objective_execution_prompt(&plan, revised.title.as_deref().unwrap(), revised);
        assert!(revised_prompt.contains("Correct the prior failed attempt"));
        assert!(revised_prompt.contains("acceptance criteria were not satisfied"));
        assert!(revised_prompt.contains("Required project-native checks: test"));

        let revised_contract = objective_leaf_contract(&plan, revised).unwrap();
        let revised_graph = run_graph_with_objective_plan_receipt(
            "queue-objective-durable__verify-acceptance-attempt-2",
            &revised.id,
            revised.title.as_deref().unwrap(),
            "approval-durable",
            &plan_receipt,
            Some(revised_contract),
        );
        let execute = revised_graph["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["id"] == "execute")
            .unwrap();
        assert_eq!(
            execute["budget"]["max_joules"],
            revised.extra["meta"]["budget"]["max_joules"]
        );
        assert_eq!(
            execute["budget"]["max_cost_usd"],
            revised.extra["meta"]["budget"]["max_cost_usd"]
        );
        assert_eq!(
            execute["retry"]["max_attempts"],
            revised.extra["meta"]["budget"]["max_attempts"]
        );
        assert_eq!(
            execute["timeout_ms"].as_u64(),
            revised.extra["meta"]["budget"]["timeout_seconds"]
                .as_u64()
                .map(|seconds| seconds * 1_000)
        );

        for (index, planned) in plan.tasks.iter().enumerate() {
            let leaf = leaves
                .iter()
                .find(|leaf| leaf.extra["meta"]["objective_leaf_key"] == planned.key)
                .unwrap();
            if planned.key == "verify-acceptance" {
                let artifact = dir.path().join("docs/audits/acceptance.md");
                std::fs::create_dir_all(artifact.parent().unwrap()).unwrap();
                std::fs::write(artifact, "evidence: structurally bound\n").unwrap();
            }
            append_queue_value(
                dir.path(),
                json!({
                    "id": leaf.id,
                    "source_record_id": leaf.id,
                    "title": leaf.title,
                    "status": "completed",
                    "result": "completed",
                    "execution_receipt_digest": format!("sha256:leaf-{index}"),
                    "meta": leaf.extra["meta"],
                }),
            )
            .unwrap();
            advance_objective_after_leaf(dir.path(), leaf).unwrap();
            if let Some(next) = plan.tasks.get(index + 1) {
                let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
                    restarted.load().unwrap(),
                );
                let next_id = objective_leaf_id("objective-durable", &next.key);
                assert_eq!(
                    effective
                        .iter()
                        .find(|record| record.id == next_id)
                        .and_then(|record| record.status.as_deref()),
                    Some("queued")
                );
            }
        }
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            restarted.load().unwrap(),
        );
        let closed = effective
            .iter()
            .find(|record| record.id == "objective-durable")
            .unwrap();
        assert_eq!(closed.status.as_deref(), Some("completed"));
        assert_eq!(closed.extra["continuation_decision"], "close_complete");
        assert_eq!(
            closed.extra["closure_evidence_receipts"]
                .as_array()
                .unwrap()
                .len(),
            5
        );
        assert_eq!(
            closed.extra["acceptance_artifact"],
            "docs/audits/acceptance.md"
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

        let mut traversal = task.clone();
        traversal.extra["meta"]["acceptance_artifact"] = Value::String("../outside.md".into());
        assert!(validate_task_acceptance_artifact(dir.path(), &traversal)
            .unwrap_err()
            .to_string()
            .contains("safe repository-relative path"));
        let mut absolute = task;
        absolute.extra["meta"]["acceptance_artifact"] = Value::String("/etc/passwd".into());
        assert!(validate_task_acceptance_artifact(dir.path(), &absolute)
            .unwrap_err()
            .to_string()
            .contains("safe repository-relative path"));
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

    #[tokio::test]
    async fn cancellation_rejects_missing_schedule_before_network_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        approved_queue_fixture(dir.path(), "cancel-without-schedule");
        std::fs::remove_file(dir.path().join("core/projects/tasks/schedules.jsonl")).unwrap();

        let error = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .cancel_task("cancel-without-schedule", "operator cancellation")
            .await
            .unwrap_err();

        assert!(
            error.to_string().contains("schedule not found"),
            "authority must fail before network dispatch: {error:#}"
        );
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(
                dir.path().join("core/projects/tasks/queue.jsonl"),
            )
            .load()
            .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("queued"));
    }

    #[tokio::test]
    async fn cancellation_retry_repairs_queue_terminal_without_network_dispatch() {
        let dir = tempfile::tempdir().unwrap();
        approved_queue_fixture(dir.path(), "cancel-after-schedule");
        let schedule_path = dir.path().join("core/projects/tasks/schedules.jsonl");
        let ledger = super::super::schedule::ScheduleLedger::new(&schedule_path);
        let append_error = ledger
            .with_cancellation_transition(
                "cancel-after-schedule",
                "objective-reconciliation",
                Utc::now(),
                Some("operator cancellation"),
                || Err::<(), _>(std::io::Error::other("simulated queue append failure")),
            )
            .unwrap_err();
        assert_eq!(append_error.kind(), std::io::ErrorKind::Other);
        assert_eq!(
            ledger.effective().unwrap()["cancel-after-schedule"].state,
            super::super::schedule::ScheduleState::Cancelled
        );

        let receipt = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .cancel_task("cancel-after-schedule", "operator cancellation")
            .await
            .unwrap();

        assert_eq!(receipt["status"], "cancelled");
        assert_eq!(receipt["reconciled"], true);
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(
                dir.path().join("core/projects/tasks/queue.jsonl"),
            )
            .load()
            .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("failed"));
        assert_eq!(effective[0].result.as_deref(), Some("cancelled"));
    }

    #[test]
    fn wait_until_queue_failure_does_not_publish_deferred_schedule() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(&queue_path).unwrap();
        let wait_until = Utc::now() + chrono::Duration::hours(1);
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "wait-queue-failure",
            "title": "Wait without orphaning authority",
            "status": "failed",
            "workbench_run_id": "wait-run",
            "continuation_sequence": 0,
            "retry_sequence": 0,
            "revision_sequence": 0,
            "meta": {
                "objective_leaf": true,
                "objective_id": "objective-wait",
                "source_objective_packet_id": "objective-wait",
                "wait_until_utc": wait_until.to_rfc3339()
            }
        }))
        .unwrap();
        let ledger = super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        );
        ledger
            .append(&super::super::schedule::ScheduleRecord {
                contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
                task_id: task.id.clone(),
                objective_id: "objective-wait".into(),
                mode: super::super::schedule::ScheduleMode::Immediate,
                state: super::super::schedule::ScheduleState::Scheduled,
                not_before_utc: None,
                interval_seconds: None,
                recorded_at_utc: Utc::now(),
                reason: Some("initial authority".into()),
            })
            .unwrap();

        assert!(materialize_continuation(
            dir.path(),
            &task,
            "wait-run",
            "wait_until",
            "retry later"
        )
        .is_err());

        let effective = ledger.effective().unwrap();
        assert_eq!(
            effective[&task.id].mode,
            super::super::schedule::ScheduleMode::Immediate
        );
        assert!(effective[&task.id].not_before_utc.is_none());
    }

    #[test]
    fn objective_queue_failure_does_not_publish_leaf_schedules() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(&queue_path).unwrap();
        let mut plan = ObjectiveDecomposer::default().decompose_grounded(
            &Objective {
                id: "atomic-objective".into(),
                statement: "Improve reliability with verified acceptance".into(),
                constraints: vec!["Preserve unrelated changes".into()],
                deadline: None,
                success_criteria: vec!["Verified acceptance evidence exists".into()],
                tags: vec!["reliability".into()],
            },
            vec![ObjectiveContextSource::new("plan", "docs/plans/test.md")],
        );
        for contract in plan.leaf_contracts.values_mut() {
            contract.project_id = DEFAULT_PROJECT_ID.into();
        }
        let objective: QueueRecord = serde_json::from_value(json!({
            "id": "atomic-objective",
            "title": "Atomic objective",
            "owner": "prometheus",
            "priority": "high",
            "status": "in_progress",
            "meta": {
                "objective_id": "atomic-objective",
                "source_objective_packet_id": "atomic-objective",
                "project_id": DEFAULT_PROJECT_ID
            }
        }))
        .unwrap();

        let error = materialize_objective_leaves(
            dir.path(),
            &objective,
            &plan,
            "sha256:plan-receipt",
            "plan-run",
        )
        .unwrap_err();
        assert!(
            error.to_string().contains("directory") || error.to_string().contains("queue.jsonl"),
            "expected queue publication failure, got: {error:#}"
        );

        let schedule_path = dir.path().join("core/projects/tasks/schedules.jsonl");
        assert!(
            !schedule_path.exists()
                || super::super::schedule::ScheduleLedger::new(&schedule_path)
                    .effective()
                    .unwrap()
                    .is_empty(),
            "queue publication failure must not publish canonical leaf schedules"
        );
    }

    #[test]
    fn restart_reconciles_prepared_wait_until_continuation() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let due = Utc::now() + chrono::Duration::minutes(30);
        std::fs::write(
            &queue_path,
            format!(
                "{}\n",
                json!({
                    "contract": "arda.workbench.executable_continuation.v1",
                    "id": "prepared-wait",
                    "source_record_id": "prepared-wait",
                    "title": "Prepared wait continuation",
                    "owner": "prometheus",
                    "priority": "high",
                    "status": "blocked",
                    "continuation_decision": "wait_until",
                    "meta": {
                        "objective_leaf": true,
                        "objective_id": "objective-wait",
                        "source_objective_packet_id": "objective-wait",
                        "wait_until_utc": due.to_rfc3339(),
                        "depends_on": []
                    }
                })
            ),
        )
        .unwrap();

        reconcile_terminal_objective_leaves(dir.path()).unwrap();

        let schedule = super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .effective()
        .unwrap()
        .remove("prepared-wait")
        .expect("reconciled deferred schedule");
        assert_eq!(
            schedule.mode,
            super::super::schedule::ScheduleMode::Deferred
        );
        assert_eq!(schedule.not_before_utc, Some(due));
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        assert_eq!(effective[0].status.as_deref(), Some("queued"));
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
