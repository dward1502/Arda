#![cfg(feature = "full-cli")]
//! Bounded adapter from operator-approved canonical queue work into Workbench.

use super::execution_outcome::project_terminal_outcome;
use super::task_queue::{ActiveQueueExecutor, ApprovedQueueClaim, QueueRecord};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const QUEUE_EXECUTION_RECEIPT_CONTRACT: &str = "arda.workbench.queue_execution_receipt.v1";
const DEFAULT_PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueExecutionReceipt {
    pub contract: String,
    pub task_id: Option<String>,
    pub workbench_run_id: Option<String>,
    pub status: String,
    pub result: String,
    pub execution_receipt_digest: Option<String>,
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
        let queue = ActiveQueueExecutor::new(&self.root);
        let Some(claim) = queue.claim_next_approved()? else {
            return Ok(QueueExecutionReceipt {
                contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                task_id: None,
                workbench_run_id: None,
                status: "idle".into(),
                result: "no_eligible_task".into(),
                execution_receipt_digest: None,
                detail: None,
                recorded_at_utc: Utc::now().to_rfc3339(),
            });
        };
        let run_id = claim.attempt.workbench_run_id.clone();
        match self.dispatch_claim(&claim).await {
            Ok((status, digest, detail)) => {
                if status == "in_progress" {
                    return Ok(QueueExecutionReceipt {
                        contract: QUEUE_EXECUTION_RECEIPT_CONTRACT.into(),
                        task_id: Some(claim.task.id),
                        workbench_run_id: Some(run_id),
                        status,
                        result: "existing_run_active".into(),
                        execution_receipt_digest: digest,
                        detail,
                        recorded_at_utc: Utc::now().to_rfc3339(),
                    });
                }
                let (queue_status, result) = match status.as_str() {
                    "succeeded" => ("completed", "completed"),
                    "cancelled" => ("failed", "cancelled"),
                    _ => ("failed", "failed"),
                };
                queue.append_workbench_terminal(
                    &claim.task,
                    queue_status,
                    result,
                    &run_id,
                    digest.as_deref(),
                    detail.as_deref(),
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
                    detail,
                    recorded_at_utc: Utc::now().to_rfc3339(),
                })
            }
            Err(error) => {
                let detail = format!("{error:#}");
                if detail.contains("was cancelled while provider execution was active") {
                    queue.append_workbench_terminal(
                        &claim.task,
                        "failed",
                        "cancelled",
                        &run_id,
                        None,
                        Some(&detail),
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
                        detail: Some(detail),
                        recorded_at_utc: Utc::now().to_rfc3339(),
                    });
                }
                queue.append_workbench_terminal(
                    &claim.task,
                    "failed",
                    "dispatch_failed",
                    &run_id,
                    None,
                    Some(&detail),
                )?;
                project_terminal_outcome(
                    &self.root,
                    &claim.task,
                    &run_id,
                    "failed",
                    "dispatch_failed",
                    None,
                    Some(&detail),
                )?;
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
        if let Some(outcome) = self.existing_run_outcome(run_id).await? {
            return Ok(outcome);
        }
        let envelope = approval_envelope(&claim.task, &format!("plan-{run_id}"))?;
        let approval_id = envelope["approval"]["approval_id"]
            .as_str()
            .ok_or_else(|| anyhow!("approval id missing"))?;
        let objective = claim
            .task
            .title
            .as_deref()
            .unwrap_or(claim.task.id.as_str());
        let graph = run_graph(run_id, &claim.task.id, objective, approval_id);

        let response = self
            .client
            .post(format!("{}/v1/runs/plan", self.harness_url))
            .json(&json!({
                "project_id": self.project_id,
                "graph": graph,
                "envelope": envelope,
            }))
            .send()
            .await
            .context("connect to the loopback Workbench harness")?;
        response_error(response, "plan approved queue run").await?;

        let envelope = approval_envelope(&claim.task, &format!("approve-{run_id}"))?;
        let response = self
            .client
            .post(format!("{}/v1/runs/{run_id}/approve", self.harness_url))
            .json(&json!({"node_id": "approval", "envelope": envelope}))
            .send()
            .await
            .context("submit Workbench approval")?;
        response_error(response, "approve queue run").await?;

        let envelope = approval_envelope(&claim.task, &format!("execute-{run_id}"))?;
        let response = self
            .client
            .post(format!(
                "{}/v1/runs/{run_id}/nodes/execute/execute-provider",
                self.harness_url
            ))
            .json(&json!({"objective": objective, "envelope": envelope}))
            .send()
            .await
            .context("dispatch approved Workbench provider")?;
        let value = response_error(response, "execute approved queue task").await?;
        let status = value["receipt"]["status"]
            .as_str()
            .unwrap_or("failed")
            .to_owned();
        let digest = value["receipt"]["receipt_digest"]
            .as_str()
            .map(str::to_owned);
        let detail = value["receipt"]["summary"].as_str().map(str::to_owned);
        Ok((status, digest, detail))
    }

    async fn existing_run_outcome(
        &self,
        run_id: &str,
    ) -> Result<Option<(String, Option<String>, Option<String>)>> {
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
        Ok(Some(classify_existing_run(&value)))
    }
}

fn classify_existing_run(value: &Value) -> (String, Option<String>, Option<String>) {
    let execute_state = value["graph"]["nodes"]
        .as_array()
        .and_then(|nodes| nodes.iter().find(|node| node["id"] == "execute"))
        .and_then(|node| node["state"].as_str())
        .unwrap_or("pending");
    let digest = value["review"]["provider_receipt"]["receipt_digest"]
        .as_str()
        .map(str::to_owned);
    let detail = value["review"]["provider_receipt"]["summary"]
        .as_str()
        .map(str::to_owned);
    let status = match execute_state {
        "succeeded" => "succeeded",
        "failed" => "failed",
        "cancelled" => "cancelled",
        _ => "in_progress",
    };
    (status.to_owned(), digest, detail)
}

fn approval_envelope(task: &QueueRecord, idempotency_key: &str) -> Result<Value> {
    let meta = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("task `{}` omitted governed queue metadata", task.id))?;
    let approval_id = required_meta(meta, "approval_packet_id", &task.id)?;
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

fn run_graph(run_id: &str, task_id: &str, objective: &str, approval_id: &str) -> Value {
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
            }))
        ],
        "edges": [
            {"id": "plan-approval", "from": "plan", "to": "approval", "parent_receipt": approval_id},
            {"id": "approval-execute", "from": "approval", "to": "execute", "parent_receipt": approval_id}
        ],
        "provenance": {
            "project_contract_digest": format!("sha256:{}", "0".repeat(64)),
            "created_by": "arda_workbench.queue_executor",
            "parent_receipts": [approval_id]
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

    #[test]
    fn graph_requires_the_approved_parent_and_bounded_worker() {
        let graph = run_graph("queue-task-1", "task-1", "bounded fixture", "approval-1");
        let raw = serde_json::to_string(&graph).unwrap();
        let parsed = arda_core::run_graph::RunGraph::from_json_str(&raw).unwrap();
        assert_eq!(parsed.nodes.len(), 3);
        let execute = parsed
            .nodes
            .iter()
            .find(|node| node.id.as_str() == "execute")
            .unwrap();
        assert_eq!(execute.retry.max_attempts, 2);
        assert_eq!(execute.parent_receipts, vec!["approval-1"]);
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
