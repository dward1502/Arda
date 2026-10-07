//! Shared graph, receipt and project-binding operations for the retired queue adapter.
//! No scheduling, queue persistence or observability is owned here.
use anyhow::{anyhow, Context, Result};
use arda_core::project_contract::ProjectContract;
use arda_orome::WorkerContextHandoffReceipt;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
mod strict_value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHermesToolEvidence {
    pub tool: String,
    pub action: String,
    pub exit_code: Option<i32>,
    pub output_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHermesTestEvidence {
    pub check_id: String,
    pub command: String,
    pub status: String,
    pub exit_code: i32,
    pub output_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHermesArtifactEvidence {
    pub path: String,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHermesUsage {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    #[serde(default)]
    pub cost_measurement: Value,
    pub completed: bool,
    pub failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalHermesExecutionReceipt {
    pub schema_version: String,
    pub receipt_digest: String,
    #[serde(default)]
    pub authority_binding_digest: String,
    pub run_id: String,
    pub node_id: String,
    pub idempotency_key: String,
    pub status: Value,
    pub summary: String,
    pub tool_evidence: Vec<CanonicalHermesToolEvidence>,
    pub test_evidence: Vec<CanonicalHermesTestEvidence>,
    pub artifacts: Vec<CanonicalHermesArtifactEvidence>,
    pub usage: CanonicalHermesUsage,
    pub adapter: String,
    pub adapter_version: String,
    pub project_contract_digest: String,
    pub parent_receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_capsule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_capsule_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_use_receipt_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_handoff: Option<WorkerContextHandoffReceipt>,
    pub recorded_at_unix_ms: u128,
}

impl CanonicalHermesExecutionReceipt {
    pub fn computed_digest(&self) -> Result<String> {
        let mut unsigned = self.clone();
        unsigned.receipt_digest.clear();
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&unsigned)?)
        ))
    }

    pub fn has_valid_digest(&self) -> Result<bool> {
        Ok(self
            .receipt_digest
            .strip_prefix("sha256:")
            .is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            && self.computed_digest()? == self.receipt_digest)
    }
}

pub async fn response_error(response: reqwest::Response, action: &str) -> Result<Value> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("{action} returned {status}: {body}"));
    }
    serde_json::from_str(&body).with_context(|| format!("decode {action} response"))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExecutionProjectRegistry {
    schema_version: String,
    projects: Vec<strict_value::StrictValue>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionAttachedProject {
    pub contract: ProjectContract,
    #[serde(rename = "approval_id")]
    _approval_id: String,
    #[serde(rename = "proposal_id")]
    _proposal_id: String,
    #[serde(rename = "idempotency_key")]
    _idempotency_key: String,
}

pub fn attached_execution_project(
    root: &Path,
    project_id: &str,
) -> Result<ExecutionAttachedProject> {
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
    let mut selected = None;
    for strict_value::StrictValue(entry) in registry.projects {
        let id = entry
            .pointer("/contract/identity/project_id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("stored project identity is missing or invalid"))?;
        let parsed_id = uuid::Uuid::parse_str(id).context("invalid stored project UUID")?;
        anyhow::ensure!(
            id == parsed_id.to_string(),
            "stored project UUID is not canonical"
        );
        if id == project_id {
            anyhow::ensure!(
                selected.is_none(),
                "project `{project_id}` is attached more than once"
            );
            selected = Some(
                serde_json::from_value::<ExecutionAttachedProject>(entry)
                    .context("selected project contract is malformed")?,
            );
        }
    }
    let attached = selected.ok_or_else(|| anyhow!("project `{project_id}` is not attached"))?;
    attached
        .contract
        .validate()
        .map_err(|error| anyhow!("invalid project contract for `{project_id}`: {error}"))?;
    Ok(attached)
}

pub fn node_state<'a>(run: &'a Value, node_id: &str) -> Option<&'a str> {
    run["graph"]["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["id"] == node_id)?["state"]
        .as_str()
}

pub fn node_output_digest<'a>(run: &'a Value, node_id: &str) -> Option<&'a str> {
    run["graph"]["nodes"]
        .as_array()?
        .iter()
        .find(|node| node["id"] == node_id)?["output_digest"]
        .as_str()
}

pub fn pre_review_receipt_projection(root: &Path, run_id: &str, run: &Value) -> Result<Value> {
    let receipt_root = root
        .join("data/runs")
        .join(run_id)
        .join("execution-receipts");
    let execute_digest = node_output_digest(run, "execute")
        .ok_or_else(|| anyhow!("completed execute node omitted its output digest"))?;
    let verify_digest = node_output_digest(run, "verify")
        .ok_or_else(|| anyhow!("completed verify node omitted its output digest"))?;
    let execute_receipt = exact_pre_review_receipt(
        &receipt_root.join("execute.json"),
        "execute",
        execute_digest,
    )?;
    let verify_receipt =
        exact_pre_review_receipt(&receipt_root.join("verify.json"), "verify", verify_digest)?;
    Ok(json!({
        "execute_output_digest": execute_digest,
        "verify_output_digest": verify_digest,
        "execute_receipt": execute_receipt,
        "verify_receipt": verify_receipt,
    }))
}

pub fn exact_pre_review_receipt(path: &Path, stage: &str, expected_digest: &str) -> Result<Value> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("read exact {stage} execution receipt `{}`", path.display()))?;
    let receipt: Value = serde_json::from_str(&raw).with_context(|| {
        format!(
            "decode exact {stage} execution receipt `{}`",
            path.display()
        )
    })?;
    let canonical: CanonicalHermesExecutionReceipt =
        serde_json::from_str(&raw).with_context(|| {
            format!(
                "decode canonical {stage} execution receipt `{}`",
                path.display()
            )
        })?;
    if canonical.schema_version != "arda.execution-receipt.v3" {
        return Err(anyhow!(
            "exact {stage} receipt uses unsupported schema `{}`",
            canonical.schema_version
        ));
    }
    if !canonical.has_valid_digest()? {
        return Err(anyhow!(
            "exact {stage} receipt has an invalid canonical digest"
        ));
    }
    let expected_run_id = path
        .parent()
        .and_then(Path::parent)
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        .ok_or_else(|| anyhow!("exact {stage} receipt path omitted its canonical run id"))?;
    if canonical.run_id != expected_run_id || canonical.node_id != stage {
        return Err(anyhow!(
            "exact {stage} receipt run or node identity does not match its canonical path"
        ));
    }
    if canonical.receipt_digest != expected_digest {
        return Err(anyhow!(
            "exact {stage} receipt digest does not match the completed node output"
        ));
    }
    if canonical.status != "succeeded" {
        return Err(anyhow!("exact {stage} receipt is not successful"));
    }
    for field in [
        "authority_binding_digest",
        "adapter",
        "project_contract_digest",
    ] {
        if !receipt[field]
            .as_str()
            .is_some_and(|value| !value.trim().is_empty())
        {
            return Err(anyhow!("exact {stage} receipt omitted `{field}`"));
        }
    }
    Ok(json!({
        "receipt_digest": receipt["receipt_digest"],
        "authority_binding_digest": receipt["authority_binding_digest"],
        "status": receipt["status"],
        "summary": receipt["summary"],
        "usage": {
            "provider": receipt["usage"]["provider"],
            "model": receipt["usage"]["model"],
        },
        "adapter": receipt["adapter"],
        "project_contract_digest": receipt["project_contract_digest"],
        "tool_evidence": receipt["tool_evidence"],
        "test_evidence": receipt["test_evidence"],
        "artifacts": receipt["artifacts"],
    }))
}

/// Execution limits projected from legacy planning, with no queue ownership.
pub struct ExecutionGraphLimits<'a> {
    pub authority_class: &'a str,
    pub max_joules: f64,
    pub max_cost_usd: f64,
    pub max_attempts: u32,
    pub timeout_seconds: u64,
}

pub fn run_graph_value(
    run_id: &str,
    task_id: &str,
    objective: &str,
    approval_id: &str,
    objective_plan_receipt: Option<&str>,
    leaf_contract: Option<&ExecutionGraphLimits<'_>>,
    project_contract_digest: &str,
) -> Value {
    let prompt_digest = format!("sha256:{:x}", Sha256::digest(objective.as_bytes()));
    let deadline = Utc::now().timestamp_millis().saturating_add(1_200_000) as u128;
    let execute_authority = leaf_contract
        .map(|contract| contract.authority_class)
        .unwrap_or("execute_with_approval");
    let read_only_execute = execute_authority == "read_only";
    let execute_kind = if read_only_execute {
        "inspect"
    } else {
        "execute"
    };
    let execute_role = if read_only_execute {
        "local_summary_classification"
    } else {
        "implementer"
    };
    let execute_toolsets = if read_only_execute {
        json!(["file"])
    } else {
        json!(["file", "terminal"])
    };
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
            node("execute", execute_kind, execute_authority, vec![approval_id], json!({
                "role": execute_role,
                "worker_id": format!("hermes:queue:{task_id}"),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": prompt_digest,
                "allowed_toolsets": execute_toolsets,
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
            node("review", "review", "read_only", vec![], json!({
                "role": "security_privacy_critic",
                "worker_id": format!("hermes:queue:{task_id}:critic"),
                "route_id": "hosted:hermes-workbench",
                "route_class": "hosted",
                "prompt_digest": prompt_digest,
                "allowed_toolsets": ["file"],
                "dependencies": ["verify"],
                "deadline_unix_ms": deadline,
                "output_contract": "arda.hermes-job-result.v1",
                "evidence_policy": "worker_report"
            })),
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
            "project_contract_digest": project_contract_digest,
            "created_by": "arda_workbench.queue_executor",
            "parent_receipts": provenance_receipts
        }
    })
}
