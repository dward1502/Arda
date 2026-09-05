#![cfg(feature = "full-cli")]
#![allow(
    dead_code,
    reason = "legacy queue execution is crate-private and retained only for migration tests"
)]
//! Bounded adapter from operator-approved canonical queue work into Workbench.

use super::decomposer::{
    ExecutableLeafContract, Objective, ObjectiveContextSource, ObjectiveDecomposer, ObjectivePlan,
};
use super::execution_outcome::project_terminal_outcome;
use super::task_queue::{
    governance_authorization_id, has_read_only_execution_authority,
    workbench_run_id as attempt_workbench_run_id, ActiveQueueExecutor, ApprovedQueueClaim,
    QueueRecord, MAX_PARALLEL_READ_ONLY_PER_WORKSPACE,
};
use super::validator::PlanValidator;
use anyhow::{anyhow, bail, Context, Result};
use arda_core::project_contract::ProjectContract;
use arda_orome::WorkerContextHandoffReceipt;
use arda_vaire::ContextAssembly;
use arda_vaire::{
    ContextDisposition, ContextOutcomeInput, ContextOutcomeReceipt, MnemosyneService,
};
use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const QUEUE_EXECUTION_RECEIPT_CONTRACT: &str = "arda.workbench.queue_execution_receipt.v1";
const DEFAULT_PROJECT_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const MAX_OBJECTIVE_PLAN_RECEIPT_BYTES: usize = 256 * 1024;
const MAX_PARALLEL_QUEUE_EXECUTIONS: usize = 4;

async fn execute_round<F>(futures: Vec<F>) -> Vec<F::Output>
where
    F: Future,
{
    futures::future::join_all(futures).await
}

async fn execute_prepared_round<P, F, T>(prepare: P, futures: Vec<F>) -> Result<Vec<T>>
where
    P: Future<Output = Result<()>>,
    F: Future<Output = Result<T>>,
{
    prepare.await?;
    execute_round(futures).await.into_iter().collect()
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplicitWorkbenchWorkItem {
    pub objective_id: String,
    pub leaf_id: String,
    pub run_id: String,
    pub objective: String,
    pub execution_prompt: String,
    pub verification_prompt: String,
    pub review_prompt: String,
    pub project_id: String,
    pub project_contract_digest: String,
    pub workspace_root: PathBuf,
    pub approval_envelope: Value,
    pub objective_plan_receipt: String,
    #[serde(default)]
    pub dependency_receipts: Vec<ExplicitReceiptReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_assembly: Option<ContextAssembly>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExplicitReceiptReference {
    pub stage: String,
    pub digest: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExplicitExecutionOutcome {
    pub run_id: String,
    pub status: String,
    pub root_receipt_digest: Option<String>,
    pub receipts: Vec<ExplicitReceiptReference>,
}

#[derive(Debug, Clone)]
pub struct WorkbenchExecutionAdapter {
    root: PathBuf,
    harness_url: String,
    client: reqwest::Client,
}

impl WorkbenchExecutionAdapter {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let harness_url =
            std::env::var("ARDA_HARNESS_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into());
        Self::with_harness_url(root, harness_url)
    }

    pub fn with_harness_url(
        root: impl AsRef<Path>,
        harness_url: impl Into<String>,
    ) -> Result<Self> {
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            harness_url: harness_url.into().trim_end_matches('/').to_owned(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()?,
        })
    }

    pub async fn execute(
        &self,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<ExplicitExecutionOutcome> {
        validate_explicit_work_item(&self.root, item)?;
        let response = self
            .client
            .get(format!("{}/v1/runs/{}", self.harness_url, item.run_id))
            .send()
            .await
            .context("inspect explicit Workbench run")?;
        let approval_id = item.approval_envelope["approval"]["approval_id"]
            .as_str()
            .ok_or_else(|| anyhow!("explicit Workbench approval id missing"))?;
        let mut run = if response.status() == reqwest::StatusCode::NOT_FOUND {
            let graph = run_graph_with_objective_plan_receipt(
                &item.run_id,
                &item.leaf_id,
                &item.objective,
                approval_id,
                &item.objective_plan_receipt,
                None,
            );
            let response = self
                .client
                .post(format!("{}/v1/runs/plan", self.harness_url))
                .json(&json!({
                    "project_id": item.project_id,
                    "expected_project_contract_digest": item.project_contract_digest,
                    "graph": graph,
                    "envelope": explicit_stage_envelope(item, "plan")?,
                }))
                .send()
                .await
                .context("plan explicit Workbench run")?;
            response_error(response, "plan explicit Workbench run").await?
        } else {
            response_error(response, "inspect explicit Workbench run").await?
        };

        if node_state(&run, "approval") != Some("succeeded") {
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{}/approve",
                    self.harness_url, item.run_id
                ))
                .json(&json!({
                    "node_id": "approval",
                    "envelope": explicit_stage_envelope(item, "approve")?,
                }))
                .send()
                .await
                .context("approve explicit Workbench run")?;
            run = response_error(response, "approve explicit Workbench run").await?;
        }

        for (stage, prompt) in [
            ("execute", item.execution_prompt.as_str()),
            ("verify", item.verification_prompt.as_str()),
        ] {
            if node_state(&run, stage) == Some("succeeded") {
                continue;
            }
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{}/nodes/{stage}/execute-provider",
                    self.harness_url, item.run_id
                ))
                .json(&json!({
                    "objective": prompt,
                    "envelope": explicit_stage_envelope(item, stage)?,
                    "context_assembly": item.context_assembly,
                }))
                .send()
                .await
                .with_context(|| format!("execute explicit Workbench {stage} stage"))?;
            if response.status() == reqwest::StatusCode::CONFLICT {
                require_scheduler_admission_conflict(response, stage).await?;
                run = wait_for_explicit_stage(&self.client, &self.harness_url, &item.run_id, stage)
                    .await?;
                continue;
            }
            let value = response_error(
                response,
                &format!("execute explicit Workbench {stage} stage"),
            )
            .await?;
            if value["receipt"]["status"] != "succeeded" {
                return explicit_failed_outcome(&self.root, item, &run, &value);
            }
            run = value["run"].clone();
        }

        if node_state(&run, "review") != Some("succeeded") {
            let review_prompt = review_prompt_with_dependency_receipts(&self.root, item)?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{}/nodes/review/execute-provider",
                    self.harness_url, item.run_id
                ))
                .json(&json!({
                    "objective": review_prompt,
                    "envelope": explicit_stage_envelope(item, "review")?,
                    "context_assembly": item.context_assembly,
                }))
                .send()
                .await
                .context("execute explicit Workbench review stage")?;
            if response.status() == reqwest::StatusCode::CONFLICT {
                require_scheduler_admission_conflict(response, "review").await?;
                run = wait_for_explicit_stage(
                    &self.client,
                    &self.harness_url,
                    &item.run_id,
                    "review",
                )
                .await?;
            } else {
                let value =
                    response_error(response, "execute explicit Workbench review stage").await?;
                if value["receipt"]["status"] != "succeeded" {
                    return explicit_failed_outcome(&self.root, item, &run, &value);
                }
                run = value["run"].clone();
            }
        }

        if node_state(&run, "close") != Some("succeeded") {
            let parent = node_output_digest(&run, "review")
                .ok_or_else(|| anyhow!("explicit Workbench close omitted review receipt"))?;
            let close_receipt = canonical_explicit_close_receipt(item, parent)?;
            // Persist deterministic evidence before making the durable run
            // terminal. A crash may leave an unreferenced file, but cannot
            // leave a succeeded run whose canonical close receipt is absent.
            persist_explicit_close_receipt(&self.root, item, &close_receipt)?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{}/nodes/close/complete",
                    self.harness_url, item.run_id
                ))
                .json(&json!({
                    "envelope": explicit_stage_envelope(item, "close")?,
                    "receipt_digest": close_receipt.receipt_digest,
                }))
                .send()
                .await
                .context("complete explicit Workbench close stage")?;
            run = response_error(response, "complete explicit Workbench close stage").await?;
            if node_output_digest(&run, "close") != Some(close_receipt.receipt_digest.as_str()) {
                return Err(anyhow!(
                    "explicit Workbench close node did not retain its canonical receipt digest"
                ));
            }
        }
        explicit_outcome_from_run(&self.root, item, &run)
    }
}

async fn wait_for_explicit_stage(
    client: &reqwest::Client,
    harness_url: &str,
    run_id: &str,
    stage: &str,
) -> Result<Value> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let response = client
            .get(format!("{harness_url}/v1/runs/{run_id}"))
            .send()
            .await
            .with_context(|| format!("reconcile conflicted explicit Workbench {stage} stage"))?;
        let run = response_error(
            response,
            &format!("reconcile conflicted explicit Workbench {stage} stage"),
        )
        .await?;
        match node_state(&run, stage) {
            Some("succeeded") => return Ok(run),
            Some("failed" | "cancelled") => {
                bail!("conflicted explicit Workbench {stage} stage became terminal without success")
            }
            _ if tokio::time::Instant::now() >= deadline => {
                bail!("timed out waiting for Workbench {stage} stage after 30s — manwe may not be connected to the harness at http://127.0.0.1:7878. Set ARDA_HARNESS_URL env var to override.")
            }
            _ => tokio::time::sleep(std::time::Duration::from_millis(250)).await,
        }
    }
}

fn review_prompt_with_dependency_receipts(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
) -> Result<String> {
    if item.dependency_receipts.is_empty() {
        return Ok(item.review_prompt.clone());
    }
    let mut receipts = Vec::with_capacity(item.dependency_receipts.len());
    for reference in &item.dependency_receipts {
        if reference.stage != "close" {
            return Err(anyhow!(
                "explicit dependency receipt `{}` is not a close receipt",
                reference.path
            ));
        }
        let path = root.join(&reference.path);
        let bytes = std::fs::read(&path)
            .with_context(|| format!("read explicit dependency receipt `{}`", path.display()))?;
        let receipt: CanonicalHermesExecutionReceipt = serde_json::from_slice(&bytes)
            .with_context(|| format!("decode explicit dependency receipt `{}`", path.display()))?;
        if receipt.receipt_digest != reference.digest || !receipt.has_valid_digest()? {
            return Err(anyhow!(
                "explicit dependency receipt `{}` failed canonical digest validation",
                path.display()
            ));
        }
        if receipt.node_id != "close" || receipt.status != json!("succeeded") {
            return Err(anyhow!(
                "explicit dependency receipt `{}` is not a successful close receipt",
                path.display()
            ));
        }
        receipts.push(json!({
            "run_id": receipt.run_id,
            "node_id": receipt.node_id,
            "receipt_digest": receipt.receipt_digest,
            "status": receipt.status,
            "summary": receipt.summary,
            "project_contract_digest": receipt.project_contract_digest,
            "parent_receipts": receipt.parent_receipts,
        }));
    }
    Ok(format!(
        "{} Canonical dependency close-receipt payloads (loaded from durable run paths and digest-validated before review): {}",
        item.review_prompt,
        serde_json::to_string(&receipts)?
    ))
}

fn explicit_stage_envelope(item: &ExplicitWorkbenchWorkItem, stage: &str) -> Result<Value> {
    let mut envelope = item.approval_envelope.clone();
    let object = envelope
        .as_object_mut()
        .ok_or_else(|| anyhow!("explicit Workbench approval envelope must be an object"))?;
    object.insert(
        "idempotency_key".into(),
        Value::String(format!("{}-{stage}", item.run_id)),
    );
    Ok(envelope)
}

fn canonical_explicit_close_receipt(
    item: &ExplicitWorkbenchWorkItem,
    parent: &str,
) -> Result<CanonicalHermesExecutionReceipt> {
    let authority_binding_digest = format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(&json!({
            "objective_id": item.objective_id,
            "leaf_id": item.leaf_id,
            "project_id": item.project_id,
            "project_contract_digest": item.project_contract_digest,
            "approval": item.approval_envelope["approval"],
        }))?)
    );
    let mut receipt = CanonicalHermesExecutionReceipt {
        schema_version: "arda.execution-receipt.v3".into(),
        receipt_digest: String::new(),
        authority_binding_digest,
        run_id: item.run_id.clone(),
        node_id: "close".into(),
        idempotency_key: format!("{}-close", item.run_id),
        status: json!("succeeded"),
        summary: "Workbench closed the reviewed objective leaf.".into(),
        tool_evidence: Vec::new(),
        test_evidence: Vec::new(),
        artifacts: Vec::new(),
        usage: CanonicalHermesUsage {
            provider: Some("arda-workbench".into()),
            model: Some("deterministic-close".into()),
            api_calls: 0,
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            estimated_cost_usd: 0.0,
            cost_measurement: json!("observed"),
            completed: true,
            failed: false,
        },
        adapter: "arda-workbench".into(),
        adapter_version: "1".into(),
        project_contract_digest: item.project_contract_digest.clone(),
        parent_receipts: vec![parent.to_owned()],
        context_capsule_id: item
            .context_assembly
            .as_ref()
            .map(|assembly| assembly.capsule.capsule_id.clone()),
        context_capsule_digest: item
            .context_assembly
            .as_ref()
            .map(|assembly| assembly.capsule.capsule_digest.clone()),
        context_use_receipt_ref: item
            .context_assembly
            .as_ref()
            .map(|assembly| assembly.use_receipt.receipt_ref()),
        context_handoff: None,
        recorded_at_unix_ms: item.approval_envelope["approval"]
            .get("created_at_utc")
            .or_else(|| item.approval_envelope["approval"].get("approved_at_utc"))
            .and_then(Value::as_str)
            .and_then(|value| DateTime::parse_from_rfc3339(value).ok())
            .and_then(|value| u128::try_from(value.timestamp_millis()).ok())
            .unwrap_or_default(),
    };
    receipt.receipt_digest = receipt.computed_digest()?;
    Ok(receipt)
}

fn persist_explicit_close_receipt(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    receipt: &CanonicalHermesExecutionReceipt,
) -> Result<()> {
    let receipt_root = root
        .join("data/runs")
        .join(&item.run_id)
        .join("execution-receipts");
    std::fs::create_dir_all(&receipt_root).with_context(|| {
        format!(
            "create explicit close receipt directory `{}`",
            receipt_root.display()
        )
    })?;
    let path = receipt_root.join("close.json");
    let temporary = receipt_root.join(format!(".close.json.tmp.{}", std::process::id()));
    std::fs::write(&temporary, serde_json::to_vec_pretty(receipt)?)
        .with_context(|| format!("write explicit close receipt `{}`", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .with_context(|| format!("install explicit close receipt `{}`", path.display()))?;
    Ok(())
}

fn validate_explicit_close_receipt(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    run: &Value,
) -> Result<()> {
    let review_parent = node_output_digest(run, "review")
        .ok_or_else(|| anyhow!("explicit Workbench close omitted review receipt"))?;
    let close_digest = node_output_digest(run, "close")
        .ok_or_else(|| anyhow!("explicit Workbench close node omitted receipt digest"))?;
    let path = root
        .join("data/runs")
        .join(&item.run_id)
        .join("execution-receipts/close.json");
    let bytes = std::fs::read(&path)
        .with_context(|| format!("read explicit close receipt `{}`", path.display()))?;
    let receipt: CanonicalHermesExecutionReceipt = serde_json::from_slice(&bytes)
        .with_context(|| format!("decode explicit close receipt `{}`", path.display()))?;
    let expected = canonical_explicit_close_receipt(item, review_parent)?;
    if receipt.receipt_digest != close_digest {
        return Err(anyhow!(
            "explicit Workbench close node did not retain its canonical receipt digest"
        ));
    }
    if !receipt.has_valid_digest()? {
        return Err(anyhow!(
            "explicit Workbench close receipt has an invalid digest"
        ));
    }
    if serde_json::to_value(&receipt)? != serde_json::to_value(&expected)? {
        return Err(anyhow!(
            "explicit Workbench close receipt does not equal the recomputed canonical payload"
        ));
    }
    Ok(())
}

fn explicit_failed_outcome(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    run: &Value,
    response: &Value,
) -> Result<ExplicitExecutionOutcome> {
    let mut outcome = explicit_outcome_from_run(root, item, run)?;
    outcome.status = response["receipt"]["status"]
        .as_str()
        .unwrap_or("failed")
        .to_owned();
    Ok(outcome)
}

fn validate_explicit_work_item(root: &Path, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
    for (name, value) in [
        ("objective_id", item.objective_id.as_str()),
        ("leaf_id", item.leaf_id.as_str()),
        ("run_id", item.run_id.as_str()),
        ("objective", item.objective.as_str()),
        ("execution_prompt", item.execution_prompt.as_str()),
        ("verification_prompt", item.verification_prompt.as_str()),
        ("review_prompt", item.review_prompt.as_str()),
        ("project_id", item.project_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(anyhow!("explicit work item omitted `{name}`"));
        }
    }
    for (name, digest) in [
        (
            "project_contract_digest",
            item.project_contract_digest.as_str(),
        ),
        (
            "objective_plan_receipt",
            item.objective_plan_receipt.as_str(),
        ),
    ] {
        if !digest.strip_prefix("sha256:").is_some_and(|value| {
            value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
        }) {
            return Err(anyhow!("explicit work item has invalid `{name}`"));
        }
    }
    let approval = item
        .approval_envelope
        .get("approval")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("explicit work item omitted authenticated approval envelope"))?;
    if approval.get("schema_version").and_then(Value::as_str) != Some("arda.orome.task_approval.v1")
        || approval
            .get("approval_id")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        return Err(anyhow!(
            "explicit work item has invalid authenticated approval envelope"
        ));
    }
    let ledger_writes = approval
        .get("ledger_writes")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("explicit work item approval omitted ledger authority"))?;
    if ledger_writes.iter().any(|path| {
        path.as_str()
            .is_some_and(|path| path.contains("core/projects/tasks"))
    }) || !ledger_writes
        .iter()
        .any(|path| path.as_str() == Some("data/arda/objectives.sqlite3"))
        || !ledger_writes
            .iter()
            .any(|path| path.as_str() == Some("data/runs"))
    {
        return Err(anyhow!(
            "explicit work item approval does not authorize only resident Arda state"
        ));
    }
    let canonical_root = root.canonicalize().context("canonicalize Arda root")?;
    let workspace_root = item
        .workspace_root
        .canonicalize()
        .context("canonicalize explicit work-item workspace")?;
    if !workspace_root.starts_with(&canonical_root) {
        return Err(anyhow!("explicit work-item workspace escapes Arda root"));
    }
    Ok(())
}

fn explicit_outcome_from_run(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    run: &Value,
) -> Result<ExplicitExecutionOutcome> {
    let mut receipts = Vec::new();
    for stage in ["execute", "verify", "review", "close"] {
        if node_state(run, stage) != Some("succeeded") {
            return Ok(ExplicitExecutionOutcome {
                run_id: item.run_id.clone(),
                status: "in_progress".into(),
                root_receipt_digest: None,
                receipts,
            });
        }
        let digest = node_output_digest(run, stage)
            .ok_or_else(|| anyhow!("explicit Workbench `{stage}` node omitted receipt digest"))?;
        receipts.push(ExplicitReceiptReference {
            stage: stage.into(),
            digest: digest.to_owned(),
            path: root
                .join("data/runs")
                .join(&item.run_id)
                .join("execution-receipts")
                .join(format!("{stage}.json"))
                .display()
                .to_string(),
        });
    }
    validate_explicit_close_receipt(root, item, run)?;
    Ok(ExplicitExecutionOutcome {
        run_id: item.run_id.clone(),
        status: "succeeded".into(),
        root_receipt_digest: receipts.last().map(|receipt| receipt.digest.clone()),
        receipts,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalHermesToolEvidence {
    tool: String,
    action: String,
    exit_code: Option<i32>,
    output_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalHermesTestEvidence {
    check_id: String,
    command: String,
    status: String,
    exit_code: i32,
    output_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalHermesArtifactEvidence {
    path: String,
    digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalHermesUsage {
    provider: Option<String>,
    model: Option<String>,
    api_calls: u64,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
    estimated_cost_usd: f64,
    #[serde(default)]
    cost_measurement: Value,
    completed: bool,
    failed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalHermesExecutionReceipt {
    schema_version: String,
    receipt_digest: String,
    #[serde(default)]
    authority_binding_digest: String,
    run_id: String,
    node_id: String,
    idempotency_key: String,
    status: Value,
    summary: String,
    tool_evidence: Vec<CanonicalHermesToolEvidence>,
    test_evidence: Vec<CanonicalHermesTestEvidence>,
    artifacts: Vec<CanonicalHermesArtifactEvidence>,
    usage: CanonicalHermesUsage,
    adapter: String,
    adapter_version: String,
    project_contract_digest: String,
    parent_receipts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_capsule_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_capsule_digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_use_receipt_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    context_handoff: Option<WorkerContextHandoffReceipt>,
    recorded_at_unix_ms: u128,
}

impl CanonicalHermesExecutionReceipt {
    fn computed_digest(&self) -> Result<String> {
        let mut unsigned = self.clone();
        unsigned.receipt_digest.clear();
        Ok(format!(
            "sha256:{:x}",
            Sha256::digest(serde_json::to_vec(&unsigned)?)
        ))
    }

    fn has_valid_digest(&self) -> Result<bool> {
        Ok(self
            .receipt_digest
            .strip_prefix("sha256:")
            .is_some_and(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
            && self.computed_digest()? == self.receipt_digest)
    }
}

async fn response_error(response: reqwest::Response, action: &str) -> Result<Value> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(anyhow!("{action} returned {status}: {body}"));
    }
    serde_json::from_str(&body).with_context(|| format!("decode {action} response"))
}

async fn require_scheduler_admission_conflict(
    response: reqwest::Response,
    stage: &str,
) -> Result<()> {
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("read explicit Workbench {stage} conflict response"))?;
    let code = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|value| value["code"].as_str().map(str::to_owned));
    if code.as_deref() == Some("scheduler_not_admitted") {
        return Ok(());
    }
    Err(anyhow!(
        "execute explicit Workbench {stage} stage returned {status}: {body}"
    ))
}

#[derive(Debug, Clone)]
pub(crate) struct WorkbenchQueueExecutor {
    root: PathBuf,
    harness_url: String,
    client: reqwest::Client,
}

impl WorkbenchQueueExecutor {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let harness_url =
            std::env::var("ARDA_HARNESS_URL").unwrap_or_else(|_| "http://127.0.0.1:7878".into());
        Self::with_harness_url(root, harness_url)
    }

    pub fn with_harness_url(
        root: impl AsRef<Path>,
        harness_url: impl Into<String>,
    ) -> Result<Self> {
        let harness_url = harness_url.into();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
            harness_url: harness_url.trim_end_matches('/').to_owned(),
            client,
        })
    }

    pub async fn execute_once(&self) -> Result<QueueExecutionReceipt> {
        self.prepare_execution_round()?;
        self.execute_prepared_once().await
    }

    fn prepare_execution_round(&self) -> Result<()> {
        // Serialize only canonical reconciliation and claim selection. The
        // target locks then preserve project/worktree exclusion for dispatch.
        let executor_coordinator_lock = acquire_executor_lock(&self.root)?;
        reconcile_terminal_objective_leaves(&self.root)?;
        let queue = ActiveQueueExecutor::new(&self.root);
        queue.reconcile_schedules(Utc::now())?;
        drop(executor_coordinator_lock);
        Ok(())
    }

    async fn execute_prepared_once(&self) -> Result<QueueExecutionReceipt> {
        let executor_coordinator_lock = acquire_executor_lock(&self.root)?;
        let queue = ActiveQueueExecutor::new(&self.root);
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
                if is_objective_leaf(&claim.task) && queue_status == "completed" {
                    advance_objective_after_leaf(&self.root, &claim.task)?;
                } else if queue_status != "completed"
                    && matches!(
                        decision,
                        "retry_same_task" | "revise_task" | "replan_objective" | "wait_until"
                    )
                {
                    materialize_continuation(
                        &self.root,
                        &claim.task,
                        &run_id,
                        decision,
                        detail
                            .as_deref()
                            .unwrap_or("terminal task verification failed"),
                    )?;
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

    pub async fn execute_available(&self) -> Result<Vec<QueueExecutionReceipt>> {
        execute_prepared_round(
            async { self.prepare_execution_round() },
            (0..MAX_PARALLEL_QUEUE_EXECUTIONS)
                .map(|_| self.execute_prepared_once())
                .collect::<Vec<_>>(),
        )
        .await
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
        if self.existing_run(&run_id).await?.is_none() {
            // This helper owns the schedule transition too: cancelled results
            // are appended through ScheduleLedger::with_cancellation_transition.
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
                "reconciled": false,
                "task_id": task.id,
                "workbench_run_id": run_id,
                "run_existed": false,
            }));
        }
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
        if node_state(&run, "review") != Some("succeeded") {
            let envelope = approval_envelope(&claim.task, &format!("review-{run_id}"))?;
            let review_evidence = pre_review_receipt_projection(&self.root, run_id, &run)?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{run_id}/nodes/review/execute-provider",
                    self.harness_url
                ))
                .json(&json!({
                    "objective": review_prompt(&claim.task, leaf_contract, &review_evidence),
                    "envelope": envelope,
                }))
                .send()
                .await
                .context("dispatch independent Workbench review provider")?;
            let value = response_error(response, "review approved queue task").await?;
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
                "review",
                node_output_digest(&run, "review"),
                "continue_close",
            )?;
            forced_restart_after_stage("review");
        }

        if node_state(&run, "close") != Some("succeeded") {
            let parent = node_output_digest(&run, "review")
                .ok_or_else(|| anyhow!("close omitted its durable parent receipt"))?;
            let receipt_digest = completion_digest(run_id, &claim.task.id, "close", parent);
            let envelope = approval_envelope(&claim.task, &format!("close-{run_id}"))?;
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{run_id}/nodes/close/complete",
                    self.harness_url
                ))
                .json(&json!({
                    "envelope": envelope,
                    "receipt_digest": receipt_digest,
                }))
                .send()
                .await
                .context("complete Workbench close node")?;
            run = response_error(response, "complete queue close").await?;
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

fn acquire_objective_advance_lock(root: &Path) -> Result<File> {
    let lock_path = root.join("core/projects/tasks/.objective-advance.lock");
    if let Some(parent) = lock_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "create objective advance lock directory `{}`",
                parent.display()
            )
        })?;
    }
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open objective advance lock `{}`", lock_path.display()))?;
    lock.lock_exclusive()
        .with_context(|| format!("acquire objective advance lock `{}`", lock_path.display()))?;
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
    read_only: bool,
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
        read_only: has_read_only_execution_authority(task),
    })
}

#[cfg(test)]
fn try_acquire_execution_target_locks(
    root: &Path,
    task: &QueueRecord,
) -> Result<Option<ExecutionTargetLocks>> {
    let binding = resolve_execution_target(root, task)?;
    let locks = try_acquire_execution_target_locks_for_binding(root, binding)?;
    if let Some(locks) = locks.as_ref() {
        ensure_fresh_mutation_workspace_clean(&locks.binding, false)?;
    }
    Ok(locks)
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
    let target_lock_result = if binding.read_only {
        FileExt::try_lock_shared(&lock)
    } else {
        FileExt::try_lock_exclusive(&lock)
    };
    match target_lock_result {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(None),
        Err(error) => return Err(error).context("acquire execution target lock"),
    }

    let mut files = vec![lock];
    if binding.read_only {
        let mut slot = None;
        for index in 0..MAX_PARALLEL_READ_ONLY_PER_WORKSPACE {
            let slot_path = lock_dir.join(format!("read-slot-{digest}-{index}.lock"));
            let slot_file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&slot_path)
                .with_context(|| {
                    format!("open read-only execution slot `{}`", slot_path.display())
                })?;
            match FileExt::try_lock_exclusive(&slot_file) {
                Ok(()) => {
                    slot = Some(slot_file);
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
                Err(error) => return Err(error).context("acquire read-only execution slot"),
            }
        }
        let Some(slot) = slot else {
            return Ok(None);
        };
        files.push(slot);
    }
    Ok(Some(ExecutionTargetLocks {
        _files: files,
        binding,
    }))
}

fn ensure_fresh_mutation_workspace_clean(
    binding: &ExecutionTargetBinding,
    exact_persisted_attempt: bool,
) -> Result<()> {
    if binding.read_only || exact_persisted_attempt {
        return Ok(());
    }
    let output = Command::new("git")
        .args([
            "-c",
            "core.fsmonitor=false",
            "status",
            "--porcelain=v1",
            "--untracked-files=all",
            "--ignore-submodules=none",
        ])
        .current_dir(&binding.workspace_root)
        .env("LC_ALL", "C")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_CONFIG_GLOBAL")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_COUNT")
        .output()
        .with_context(|| {
            format!(
                "inspect Git worktree `{}` before mutation admission",
                binding.workspace_root.display()
            )
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !has_git_control_marker(&binding.workspace_root)? {
            return Ok(());
        }
        return Err(anyhow!(
            "could not inspect Git worktree `{}` before mutation admission: {}",
            binding.workspace_root.display(),
            stderr.trim()
        ));
    }
    if !output.stdout.is_empty() {
        return Err(anyhow!(
            "refusing mutation in dirty Git worktree `{}`; preserve unrelated local changes in a clean registered worktree",
            binding.workspace_root.display()
        ));
    }
    Ok(())
}

fn has_git_control_marker(workspace_root: &Path) -> Result<bool> {
    for ancestor in workspace_root.ancestors() {
        match std::fs::symlink_metadata(ancestor.join(".git")) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "inspect Git control metadata above `{}`",
                        workspace_root.display()
                    )
                });
            }
        }
    }
    Ok(false)
}

fn try_acquire_task_execution_lock(root: &Path, task_id: &str) -> Result<Option<File>> {
    let lock_dir = root.join("core/projects/tasks/.workbench-executor-locks");
    std::fs::create_dir_all(&lock_dir)
        .with_context(|| format!("create executor lock directory `{}`", lock_dir.display()))?;
    let key = format!("task:{task_id}");
    let digest = format!("{:x}", Sha256::digest(key.as_bytes()));
    let lock_path = lock_dir.join(format!("task-{digest}.lock"));
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("open execution task lock `{}`", lock_path.display()))?;
    match FileExt::try_lock_exclusive(&lock) {
        Ok(()) => Ok(Some(lock)),
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(error) => Err(error).context("acquire execution task lock"),
    }
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
        let Some(task_lock) = try_acquire_task_execution_lock(root, &task.id)? else {
            excluded_task_ids.insert(task.id);
            continue;
        };
        let binding = resolve_execution_target(root, &task)?;
        if excluded_workspace_roots.contains(&binding.workspace_root) {
            excluded_task_ids.insert(task.id);
            continue;
        }
        let workspace_root = binding.workspace_root.clone();
        let Some(mut locks) = try_acquire_execution_target_locks_for_binding(root, binding)? else {
            excluded_workspace_roots.insert(workspace_root);
            excluded_task_ids.insert(task.id);
            continue;
        };
        let exact_persisted_attempt = queue.is_exact_persisted_workbench_attempt(&task)?;
        ensure_fresh_mutation_workspace_clean(&locks.binding, exact_persisted_attempt)?;
        locks._files.push(task_lock);
        if let Some(claim) = queue.claim_approved_candidate(&task, &excluded_task_ids)? {
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

fn task_project_ids(task: &QueueRecord) -> Vec<&str> {
    let mut project_ids = task
        .extra
        .get("meta")
        .and_then(Value::as_object)
        .and_then(|meta| meta.get("project_ids"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    if project_ids.is_empty() {
        project_ids.extend(task_project_id(task));
    }
    let mut unique = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        if !unique.contains(&project_id) {
            unique.push(project_id);
        }
    }
    unique
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
        && (task_project_ids(task).len() > 1
            || (task.extra["meta"]["acceptance_artifact"]
                .as_str()
                .is_some_and(|path| !path.trim().is_empty())
                && task.extra["meta"]["acceptance_markers"]
                    .as_array()
                    .is_some_and(|markers| !markers.is_empty())))
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
    if !is_canonical_workbench_run_id(plan_run_id) {
        return Err(anyhow!(
            "objective leaf `{}` used an invalid objective-plan run id",
            task.id
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

fn review_prompt(
    task: &QueueRecord,
    contract: Option<&ExecutableLeafContract>,
    receipt_projection: &Value,
) -> String {
    let evidence = contract
        .map(|contract| contract.evidence_requirements.join("; "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "the canonical execution and verification receipts".to_string());
    let checks = contract
        .map(|contract| contract.verification_checks.join(", "))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "the attached project contract checks".to_string());
    let receipt_projection = serde_json::to_string(receipt_projection)
        .unwrap_or_else(|_| "{\"state\":\"unavailable\"}".into());
    format!(
        "Independently review task `{}` after execution and verification. Inspect the changed project state and this canonical credential-free receipt projection: {}. Evaluate it against these evidence requirements: {}. Confirm that the prior verification receipt covers these declared checks: {}. Do not rerun checks or modify files. Name any concrete defect, unsupported completion claim, security issue, or missing evidence. Return success only when the objective is satisfied and the evidence is sufficient.",
        task.title.as_deref().unwrap_or(task.id.as_str()),
        receipt_projection,
        evidence,
        checks
    )
}

fn pre_review_receipt_projection(root: &Path, run_id: &str, run: &Value) -> Result<Value> {
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

fn exact_pre_review_receipt(path: &Path, stage: &str, expected_digest: &str) -> Result<Value> {
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
    let Some(receipt_id) = receipt_id else {
        // Legacy queue execution is not a Vairë context authority. Only work
        // that consumed a previously persisted governed context may report a
        // context outcome.
        return Ok(None);
    };
    let use_receipt = service
        .context_use_receipt(receipt_id)?
        .ok_or_else(|| anyhow!("context use receipt `{receipt_id}` was not found"))?;
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
    let project_ids = task_project_ids(task);
    let project_id = project_ids.first().copied().unwrap_or(DEFAULT_PROJECT_ID);
    let read_only_template = plan
        .leaf_contracts
        .get("inspect-authorities")
        .cloned()
        .ok_or_else(|| anyhow!("objective decomposition omitted inspection contract"))?;
    if project_ids.len() > 1 {
        for contract in plan.leaf_contracts.values_mut() {
            contract.project_id = DEFAULT_PROJECT_ID.to_owned();
        }
        let inspection_index = plan
            .tasks
            .iter()
            .position(|planned| planned.key == "inspect-authorities")
            .ok_or_else(|| anyhow!("objective decomposition omitted inspection leaf"))?;
        let inspection = plan.tasks.remove(inspection_index);
        let inspection_contract = plan
            .leaf_contracts
            .remove("inspect-authorities")
            .ok_or_else(|| anyhow!("objective decomposition omitted inspection contract"))?;
        let inspection_keys = project_ids
            .iter()
            .enumerate()
            .map(|(index, _)| format!("inspect-authorities-project-{}", index + 1))
            .collect::<Vec<_>>();
        for planned in &mut plan.tasks {
            if planned
                .depends_on
                .iter()
                .any(|dependency| dependency == "inspect-authorities")
            {
                planned
                    .depends_on
                    .retain(|dependency| dependency != "inspect-authorities");
                planned.depends_on.extend(inspection_keys.iter().cloned());
            }
        }
        let mut inspections = Vec::with_capacity(project_ids.len());
        for (project_id, key) in project_ids.iter().zip(&inspection_keys) {
            let mut planned = inspection.clone();
            planned.key = key.clone();
            planned.title = format!(
                "Inspect only bound project `{project_id}` for project-local evidence needed by: {objective}. Do not require sibling project files in this leaf; joined synthesis compares all project leaves."
            );
            inspections.push(planned);

            let mut contract = inspection_contract.clone();
            contract.project_id = (*project_id).to_owned();
            contract.verification_checks = vec!["git status --short --branch".to_owned()];
            plan.leaf_contracts.insert(key.clone(), contract);
        }
        plan.tasks
            .splice(inspection_index..inspection_index, inspections);
    } else {
        for contract in plan.leaf_contracts.values_mut() {
            contract.project_id = project_id.to_owned();
        }
    }
    let outcome = plan
        .leaf_contracts
        .get_mut("produce-outcome")
        .expect("grounded decomposition always provides an outcome contract");
    let project_id = outcome.project_id.clone();
    let mut read_only_outcome = read_only_template;
    read_only_outcome.project_id = project_id;
    read_only_outcome.authority_class = "read_only".into();
    read_only_outcome.verification_checks = vec!["git status --short --branch".into()];
    read_only_outcome.evidence_requirements = vec!["worker_report".into()];
    *outcome = read_only_outcome;
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

fn is_canonical_workbench_run_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn append_objective_terminal_once(root: &Path, objective_id: &str, value: Value) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};

    let path = root.join("core/projects/tasks/queue.jsonl");
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(&path)?;
    file.lock_exclusive()?;
    let result = (|| -> Result<()> {
        let mut terminal_exists = false;
        for line in BufReader::new(File::open(&path)?).lines() {
            let line = line?;
            let record: Value = serde_json::from_str(&line).with_context(|| {
                format!("parse locked canonical queue record for objective `{objective_id}`")
            })?;
            if record["id"] == objective_id
                && record["contract"] == "arda.workbench.objective_terminal.v1"
            {
                terminal_exists = true;
                break;
            }
        }
        if terminal_exists {
            return Ok(());
        }
        writeln!(file, "{value}")?;
        file.sync_data()?;
        Ok(())
    })();
    let unlock = FileExt::unlock(&file);
    result?;
    unlock?;
    Ok(())
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
    for task in effective.iter().filter(|record| {
        !is_objective_leaf(record)
            && matches!(
                record.status.as_deref(),
                Some("completed" | "failed" | "cancelled")
            )
    }) {
        let run_id = task
            .extra
            .get("workbench_run_id")
            .and_then(Value::as_str)
            .unwrap_or("reconciled-terminal-task");
        if !is_canonical_workbench_run_id(run_id) {
            return Err(anyhow!(
                "terminal task `{}` used an invalid canonical run id",
                task.id
            ));
        }
        let outcome_path = root
            .join("audit/workbench-queue")
            .join(run_id)
            .join("execution_receipt.json");
        if !outcome_path.exists() {
            project_terminal_outcome(
                root,
                task,
                run_id,
                task.status.as_deref().unwrap_or("failed"),
                task.result.as_deref().unwrap_or("failed"),
                task.extra
                    .get("execution_receipt_digest")
                    .and_then(Value::as_str),
                task.extra.get("detail").and_then(Value::as_str),
            )?;
        }
        if task.status.as_deref() == Some("failed") {
            if let Some(decision @ ("retry_same_task" | "revise_task" | "wait_until")) = task
                .extra
                .get("continuation_decision")
                .and_then(Value::as_str)
            {
                materialize_continuation(
                    root,
                    task,
                    run_id,
                    decision,
                    task.extra
                        .get("detail")
                        .and_then(Value::as_str)
                        .unwrap_or("reconciled terminal task continuation"),
                )?;
            }
        }
    }
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
    // Queue append locks do not protect an earlier reader from observing a
    // concurrent terminal append between writes. Serialize this compatibility
    // close transaction across its read, decision, and append boundary.
    let _advance_lock = acquire_objective_advance_lock(root)?;
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
    if effective.iter().any(|record| {
        record.id == objective_id
            && matches!(
                record.status.as_deref(),
                Some("completed" | "failed" | "cancelled")
            )
    }) {
        return Ok(());
    }
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
        let acceptance_run_id = acceptance_leaf
            .extra
            .get("workbench_run_id")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                anyhow!("objective `{objective_id}` acceptance leaf omitted its run id")
            })?;
        if !is_canonical_workbench_run_id(acceptance_run_id) {
            return Err(anyhow!(
                "objective `{objective_id}` acceptance leaf used an invalid canonical run id"
            ));
        }
        let artifact = format!("data/runs/{acceptance_run_id}/execution-receipts/review.json");
        let artifact_path = root.join(&artifact);
        if !artifact_path.is_file() {
            return Err(anyhow!(
                "objective `{objective_id}` acceptance receipt is missing at {}",
                artifact_path.display()
            ));
        }
        let artifact_bytes = std::fs::read(&artifact_path).with_context(|| {
            format!(
                "read objective acceptance receipt `{}`",
                artifact_path.display()
            )
        })?;
        let artifact_receipt: CanonicalHermesExecutionReceipt =
            serde_json::from_slice(&artifact_bytes).with_context(|| {
                format!(
                    "parse objective acceptance receipt `{}`",
                    artifact_path.display()
                )
            })?;
        if artifact_receipt.schema_version != "arda.execution-receipt.v3"
            || artifact_receipt.run_id != acceptance_run_id
            || artifact_receipt.node_id != "review"
            || artifact_receipt.status != "succeeded"
            || !artifact_receipt.has_valid_digest()?
        {
            return Err(anyhow!(
                "objective `{objective_id}` acceptance receipt is not a successful canonical review receipt"
            ));
        }
        let run_root = root.join("data/runs").join(acceptance_run_id);
        let result_path = run_root.join("result.json");
        let result: Value =
            serde_json::from_slice(&std::fs::read(&result_path).with_context(|| {
                format!("read acceptance run result `{}`", result_path.display())
            })?)
            .with_context(|| format!("parse acceptance run result `{}`", result_path.display()))?;
        if result["provider_receipt"]["receipt_digest"] != artifact_receipt.receipt_digest
            || result["provider_receipt"]["authority_binding_digest"]
                != artifact_receipt.authority_binding_digest
        {
            return Err(anyhow!(
                "objective `{objective_id}` acceptance receipt digest mismatch"
            ));
        }
        let checkpoint_path = run_root.join("checkpoint.json");
        let checkpoint: Value =
            serde_json::from_slice(&std::fs::read(&checkpoint_path).with_context(|| {
                format!(
                    "read acceptance run checkpoint `{}`",
                    checkpoint_path.display()
                )
            })?)
            .with_context(|| {
                format!(
                    "parse acceptance run checkpoint `{}`",
                    checkpoint_path.display()
                )
            })?;
        if checkpoint["run_id"] != acceptance_run_id
            || checkpoint["objective_id"] != acceptance_leaf.id
        {
            return Err(anyhow!(
                "objective `{objective_id}` acceptance receipt is not bound to its canonical run"
            ));
        }
        let acceptance_artifact_digest = format!("sha256:{:x}", Sha256::digest(&artifact_bytes));
        let root_meta = acceptance_leaf.extra["meta"]["objective_root_meta"].clone();
        return append_objective_terminal_once(
            root,
            objective_id,
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
                "acceptance_artifact_digest": acceptance_artifact_digest,
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
    let leaf_key = task.extra["meta"]["objective_leaf_key"].as_str();
    let authority_instructions = if leaf_key
        .is_some_and(|key| key.starts_with("inspect-authorities"))
    {
        "Inspect only the bound project and cite project-local material source evidence. Do not require root-level or sibling-project authority paths that are outside this worker boundary.".to_owned()
    } else {
        format!("Read and cite these live authorities before changing anything:\n{sources}")
    };
    let outcome_requirement = match leaf_key {
        Some("recover-context") => {
            "Final output must be an evidence-backed context summary sufficient for downstream objective leaves."
        }
        Some(key) if key.starts_with("inspect-authorities") => {
            "Final output must be a project-scoped inspection report anchored to material source evidence."
        }
        Some("synthesize-findings") => {
            "Final output must synthesize the completed project inspections into prioritized findings."
        }
        Some("produce-outcome") => {
            "Final output must be the concrete operator-visible outcome required by the reviewed objective."
        }
        Some("verify-acceptance") => {
            "Final output must be an evidence-backed acceptance verdict for the reviewed objective."
        }
        _ => "Final output must satisfy this validated objective-plan leaf with material evidence.",
    };
    format!(
        "{}\n\nExecute this validated objective plan in dependency order:\n{}\n\n{}\n\n{}{}{}{} Preserve unrelated dirty work and do not edit generated queue projections.",
        objective,
        tasks,
        authority_instructions,
        outcome_requirement,
        artifact_requirement,
        checks,
        revision_directive
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prometheus::autopilot::TaskQueueAnalyzer;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn execution_round_polls_independent_claims_concurrently() {
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let futures = (0..2)
            .map(|_| {
                let active = Arc::clone(&active);
                let maximum = Arc::clone(&maximum);
                async move {
                    let current = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                }
            })
            .collect::<Vec<_>>();

        execute_round(futures).await;

        assert_eq!(maximum.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn prepared_execution_round_runs_reconciliation_once_before_claims() {
        let preparations = Arc::new(AtomicUsize::new(0));
        let prepare_count = Arc::clone(&preparations);
        let prepare = async move {
            prepare_count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };
        let claims = vec![
            std::future::ready(Ok::<_, anyhow::Error>(1_u8)),
            std::future::ready(Ok::<_, anyhow::Error>(2_u8)),
        ];

        let receipts = execute_prepared_round(prepare, claims).await.unwrap();

        assert_eq!(preparations.load(Ordering::SeqCst), 1);
        assert_eq!(receipts, vec![1, 2]);
    }

    #[test]
    fn join_review_loads_and_validates_canonical_dependency_close_receipts() {
        let root = tempfile::tempdir().unwrap();
        let mut dependency = ExplicitWorkbenchWorkItem {
            objective_id: "objective-1".into(),
            leaf_id: "leaf-a".into(),
            run_id: "run-a".into(),
            objective: "dependency".into(),
            execution_prompt: "execute".into(),
            verification_prompt: "verify".into(),
            review_prompt: "review".into(),
            project_id: "project-a".into(),
            project_contract_digest: format!("sha256:{}", "a".repeat(64)),
            workspace_root: root.path().to_path_buf(),
            approval_envelope: json!({"approval": {"approval_id": "approval-1"}}),
            objective_plan_receipt: format!("sha256:{}", "b".repeat(64)),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };
        let receipt =
            canonical_explicit_close_receipt(&dependency, &format!("sha256:{}", "c".repeat(64)))
                .unwrap();
        let relative_path = "data/runs/run-a/execution-receipts/close.json";
        let path = root.path().join(relative_path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();

        dependency.leaf_id = "join".into();
        dependency.run_id = "join-run".into();
        dependency.review_prompt = "review joined evidence".into();
        dependency.dependency_receipts = vec![ExplicitReceiptReference {
            stage: "close".into(),
            digest: receipt.receipt_digest.clone(),
            path: relative_path.into(),
        }];

        let prompt = review_prompt_with_dependency_receipts(root.path(), &dependency).unwrap();
        assert!(prompt.contains("review joined evidence"));
        assert!(prompt.contains("run-a"));
        assert!(prompt.contains(&receipt.receipt_digest));

        dependency.dependency_receipts[0].digest = format!("sha256:{}", "d".repeat(64));
        assert!(
            review_prompt_with_dependency_receipts(root.path(), &dependency)
                .unwrap_err()
                .to_string()
                .contains("canonical digest validation")
        );
    }

    #[tokio::test]
    async fn explicit_work_item_recovers_receipts_without_reading_or_writing_queue() {
        let dir = tempfile::tempdir().unwrap();
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let item = ExplicitWorkbenchWorkItem {
            objective_id: "objective-1".into(),
            leaf_id: "leaf-1".into(),
            run_id: "objective-1-leaf-1-attempt-1".into(),
            objective: "Inspect exact project authority".into(),
            execution_prompt: "Inspect only the bound project.".into(),
            verification_prompt: "Verify the inspection receipt.".into(),
            review_prompt: "Independently review the evidence.".into(),
            project_id: DEFAULT_PROJECT_ID.into(),
            project_contract_digest: digest('f'),
            workspace_root: dir.path().to_path_buf(),
            approval_envelope: json!({
                "approval": {
                    "schema_version": "arda.orome.task_approval.v1",
                    "proposal_id": "operator-objective-1",
                    "approval_id": "operator-approval-1",
                    "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                    "decision": "policy_safe",
                    "created_at_utc": "2026-09-01T00:00:00Z"
                },
                "idempotency_key": "objective-1-leaf-1"
            }),
            objective_plan_receipt: digest('1'),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };
        let close_receipt = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
        persist_explicit_close_receipt(dir.path(), &item, &close_receipt).unwrap();
        let (harness_url, server) = scripted_harness(vec![Some((
            200,
            json!({
                "graph": {"nodes": [
                    {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
                    {"id": "execute", "state": "succeeded", "output_digest": digest('b')},
                    {"id": "verify", "state": "succeeded", "output_digest": digest('c')},
                    {"id": "review", "state": "succeeded", "output_digest": digest('d')},
                    {"id": "close", "state": "succeeded", "output_digest": close_receipt.receipt_digest}
                ]}
            })
            .to_string(),
        ))])
        .await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();

        let outcome = adapter.execute(&item).await.unwrap();
        let requests = server.await.unwrap();

        assert_eq!(outcome.status, "succeeded");
        assert_eq!(outcome.receipts.len(), 4);
        assert_eq!(
            outcome.root_receipt_digest.as_deref(),
            Some(close_receipt.receipt_digest.as_str())
        );
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("GET /v1/runs/objective-1-leaf-1-attempt-1 "));
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
        assert!(!dir
            .path()
            .join("core/projects/tasks/schedules.jsonl")
            .exists());
    }

    #[tokio::test]
    async fn explicit_work_item_reconciles_scheduler_conflict_from_terminal_run() {
        let dir = tempfile::tempdir().unwrap();
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let initial = json!({
            "graph": {"nodes": [
                {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
                {"id": "execute", "state": "pending", "output_digest": null},
                {"id": "verify", "state": "pending", "output_digest": null},
                {"id": "review", "state": "pending", "output_digest": null},
                {"id": "close", "state": "pending", "output_digest": null}
            ]}
        });
        let item = ExplicitWorkbenchWorkItem {
            objective_id: "objective-conflict".into(),
            leaf_id: "leaf-conflict".into(),
            run_id: "objective-conflict-leaf-conflict-attempt-1".into(),
            objective: "Inspect exact project authority".into(),
            execution_prompt: "Inspect only the bound project.".into(),
            verification_prompt: "Verify the inspection receipt.".into(),
            review_prompt: "Independently review the evidence.".into(),
            project_id: DEFAULT_PROJECT_ID.into(),
            project_contract_digest: digest('f'),
            workspace_root: dir.path().to_path_buf(),
            approval_envelope: json!({
                "approval": {
                    "schema_version": "arda.orome.task_approval.v1",
                    "proposal_id": "operator-objective-conflict",
                    "approval_id": "operator-approval-conflict",
                    "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                    "decision": "policy_safe",
                    "created_at_utc": "2026-09-01T00:00:00Z"
                },
                "idempotency_key": "objective-conflict-leaf-conflict"
            }),
            objective_plan_receipt: digest('1'),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };
        let close_receipt = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
        persist_explicit_close_receipt(dir.path(), &item, &close_receipt).unwrap();
        let terminal = json!({
            "graph": {"nodes": [
                {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
                {"id": "execute", "state": "succeeded", "output_digest": digest('b')},
                {"id": "verify", "state": "succeeded", "output_digest": digest('c')},
                {"id": "review", "state": "succeeded", "output_digest": digest('d')},
                {"id": "close", "state": "succeeded", "output_digest": close_receipt.receipt_digest}
            ]}
        });
        let (harness_url, server) = scripted_harness(vec![
            Some((200, initial.to_string())),
            Some((
                409,
                json!({"code": "scheduler_not_admitted", "message": "not selected by deterministic scheduler"})
                    .to_string(),
            )),
            Some((200, terminal.to_string())),
        ])
        .await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();

        let outcome = adapter.execute(&item).await.unwrap();
        let requests = server.await.unwrap();

        assert_eq!(outcome.status, "succeeded");
        assert_eq!(
            outcome.root_receipt_digest.as_deref(),
            Some(close_receipt.receipt_digest.as_str())
        );
        assert_eq!(requests.len(), 3);
        assert!(requests[1].contains("/nodes/execute/execute-provider "));
        assert!(requests[2].starts_with("GET /v1/runs/objective-conflict-leaf-conflict-attempt-1 "));

        let mut forged = close_receipt.clone();
        forged.recorded_at_unix_ms = forged.recorded_at_unix_ms.saturating_add(1);
        forged.receipt_digest = forged.computed_digest().unwrap();
        persist_explicit_close_receipt(dir.path(), &item, &forged).unwrap();
        let mut forged_terminal = terminal;
        forged_terminal["graph"]["nodes"][4]["output_digest"] =
            Value::String(forged.receipt_digest.clone());
        let error =
            validate_explicit_close_receipt(dir.path(), &item, &forged_terminal).unwrap_err();
        assert!(error.to_string().contains("recomputed canonical payload"));
    }

    #[tokio::test]
    async fn non_scheduler_conflict_is_not_reconciled() {
        let (harness_url, server) = scripted_harness(vec![Some((
            409,
            json!({"code": "conflict", "message": "different canonical payload"}).to_string(),
        ))])
        .await;
        let response = reqwest::get(harness_url).await.unwrap();
        let error = require_scheduler_admission_conflict(response, "execute")
            .await
            .unwrap_err();
        assert!(error.to_string().contains("different canonical payload"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn conflicted_close_is_not_treated_as_scheduler_owned_completion() {
        let dir = tempfile::tempdir().unwrap();
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let run = |close_state: &str, close_digest: Option<String>| {
            json!({
                "graph": {"nodes": [
                    {"id": "approval", "state": "succeeded", "output_digest": digest('a')},
                    {"id": "execute", "state": "succeeded", "output_digest": digest('b')},
                    {"id": "verify", "state": "succeeded", "output_digest": digest('c')},
                    {"id": "review", "state": "succeeded", "output_digest": digest('d')},
                    {"id": "close", "state": close_state, "output_digest": close_digest}
                ]}
            })
        };
        let initial = run("pending", None);
        let (harness_url, server) = scripted_harness(vec![
            Some((200, initial.to_string())),
            Some((409, json!({"code": "conflict"}).to_string())),
        ])
        .await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();
        let item = ExplicitWorkbenchWorkItem {
            objective_id: "objective-close-conflict".into(),
            leaf_id: "leaf-close-conflict".into(),
            run_id: "objective-close-conflict-leaf-close-conflict-attempt-1".into(),
            objective: "Inspect exact project authority".into(),
            execution_prompt: "Inspect only the bound project.".into(),
            verification_prompt: "Verify the inspection receipt.".into(),
            review_prompt: "Independently review the evidence.".into(),
            project_id: DEFAULT_PROJECT_ID.into(),
            project_contract_digest: digest('f'),
            workspace_root: dir.path().to_path_buf(),
            approval_envelope: json!({
                "approval": {
                    "schema_version": "arda.orome.task_approval.v1",
                    "proposal_id": "operator-objective-close-conflict",
                    "approval_id": "operator-approval-close-conflict",
                    "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                    "decision": "policy_safe",
                    "created_at_utc": "2026-09-01T00:00:00Z"
                },
                "idempotency_key": "objective-close-conflict-leaf-close-conflict"
            }),
            objective_plan_receipt: digest('1'),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };
        let error = adapter.execute(&item).await.unwrap_err();
        let requests = server.await.unwrap();

        assert!(error.to_string().contains("returned 409"));
        assert_eq!(requests.len(), 2);
        let staged_receipt = dir
            .path()
            .join("data/runs")
            .join(&item.run_id)
            .join("execution-receipts/close.json");
        assert!(staged_receipt.exists());
        let staged: CanonicalHermesExecutionReceipt =
            serde_json::from_slice(&std::fs::read(staged_receipt).unwrap()).unwrap();
        assert!(staged.has_valid_digest().unwrap());
    }

    #[test]
    fn terminal_close_rejects_locally_valid_noncanonical_payload() {
        let dir = tempfile::tempdir().unwrap();
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let item = ExplicitWorkbenchWorkItem {
            objective_id: "objective-forged-close".into(),
            leaf_id: "leaf-forged-close".into(),
            run_id: "objective-forged-close-leaf-forged-close-attempt-1".into(),
            objective: "Inspect exact project authority".into(),
            execution_prompt: "Inspect only the bound project.".into(),
            verification_prompt: "Verify the inspection receipt.".into(),
            review_prompt: "Independently review the evidence.".into(),
            project_id: DEFAULT_PROJECT_ID.into(),
            project_contract_digest: digest('f'),
            workspace_root: dir.path().to_path_buf(),
            approval_envelope: json!({
                "approval": {
                    "schema_version": "arda.orome.task_approval.v1",
                    "proposal_id": "operator-objective-forged-close",
                    "approval_id": "operator-approval-forged-close",
                    "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                    "decision": "policy_safe",
                    "created_at_utc": "2026-09-01T00:00:00Z"
                },
                "idempotency_key": "objective-forged-close-leaf-forged-close"
            }),
            objective_plan_receipt: digest('1'),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };
        let mut forged = canonical_explicit_close_receipt(&item, &digest('d')).unwrap();
        forged.summary = "forged but self-consistent close".into();
        forged.receipt_digest.clear();
        forged.receipt_digest = forged.computed_digest().unwrap();
        persist_explicit_close_receipt(dir.path(), &item, &forged).unwrap();
        let run = json!({"graph": {"nodes": [
            {"id": "review", "state": "succeeded", "output_digest": digest('d')},
            {"id": "close", "state": "succeeded", "output_digest": forged.receipt_digest}
        ]}});
        assert_eq!(
            node_output_digest(&run, "close"),
            Some(forged.receipt_digest.as_str())
        );

        let error = validate_explicit_close_receipt(dir.path(), &item, &run).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("does not equal the recomputed canonical payload"),
            "unexpected validation error: {error:#}"
        );
    }

    #[tokio::test]
    async fn explicit_work_item_executes_all_stages_without_queue_continuations() {
        let dir = tempfile::tempdir().unwrap();
        let digest = |byte: char| format!("sha256:{}", byte.to_string().repeat(64));
        let run = |completed: &[&str]| {
            let nodes = ["approval", "execute", "verify", "review", "close"]
                .into_iter()
                .map(|stage| {
                    let state = if completed.contains(&stage) { "succeeded" } else { "pending" };
                    json!({
                        "id": stage,
                        "state": state,
                        "output_digest": if state == "succeeded" { Some(digest(stage.chars().next().unwrap())) } else { None }
                    })
                })
                .collect::<Vec<_>>();
            json!({"graph": {"nodes": nodes}})
        };
        let provider = |completed: &[&str], stage: &str| {
            json!({
                "receipt": {
                    "status": "succeeded",
                    "receipt_digest": digest(stage.chars().next().unwrap()),
                    "summary": format!("{stage} completed")
                },
                "run": run(completed)
            })
        };
        let (harness_url, server) = scripted_harness(vec![
            Some((404, "{}".into())),
            Some((200, run(&[]).to_string())),
            Some((200, run(&["approval"]).to_string())),
            Some((
                200,
                provider(&["approval", "execute"], "execute").to_string(),
            )),
            Some((
                200,
                provider(&["approval", "execute", "verify"], "verify").to_string(),
            )),
            Some((
                200,
                provider(&["approval", "execute", "verify", "review"], "review").to_string(),
            )),
            Some((
                200,
                run(&["approval", "execute", "verify", "review", "close"])
                    .to_string()
                    .replace(&digest('c'), "__REQUEST_RECEIPT_DIGEST__"),
            )),
        ])
        .await;
        let adapter = WorkbenchExecutionAdapter::with_harness_url(dir.path(), harness_url).unwrap();
        let item = ExplicitWorkbenchWorkItem {
            objective_id: "objective-2".into(),
            leaf_id: "leaf-2".into(),
            run_id: "objective-2-leaf-2-attempt-1".into(),
            objective: "Inspect exact project authority".into(),
            execution_prompt: "Inspect only the bound project.".into(),
            verification_prompt: "Verify the inspection receipt.".into(),
            review_prompt: "Independently review the evidence.".into(),
            project_id: DEFAULT_PROJECT_ID.into(),
            project_contract_digest: digest('f'),
            workspace_root: dir.path().to_path_buf(),
            approval_envelope: json!({
                "approval": {
                    "schema_version": "arda.orome.task_approval.v1",
                    "proposal_id": "operator-objective-2",
                    "approval_id": "operator-approval-2",
                    "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"],
                    "decision": "policy_safe",
                    "created_at_utc": "2026-09-01T00:00:00Z"
                },
                "idempotency_key": "objective-2-leaf-2"
            }),
            objective_plan_receipt: digest('1'),
            dependency_receipts: Vec::new(),
            context_assembly: None,
        };

        let outcome = adapter.execute(&item).await.unwrap();
        let requests = server.await.unwrap();

        assert_eq!(outcome.status, "succeeded");
        assert_eq!(requests.len(), 7);
        assert!(requests[1].starts_with("POST /v1/runs/plan "));
        assert!(requests[3].contains("/nodes/execute/execute-provider "));
        assert!(requests[4].contains("/nodes/verify/execute-provider "));
        assert!(requests[5].contains("/nodes/review/execute-provider "));
        assert!(requests[6].contains("/nodes/close/complete "));
        assert!(!dir.path().join("core/projects/tasks/queue.jsonl").exists());
    }

    #[test]
    fn critic_projection_uses_exact_execute_and_verify_receipts() {
        let root = tempfile::tempdir().expect("create run root");
        let receipt_root = root.path().join("data/runs/run-review/execution-receipts");
        std::fs::create_dir_all(&receipt_root).expect("create receipt directory");
        let make_receipt =
            |node_id: &str,
             authority_binding_digest: &str,
             provider: &str,
             model: &str,
             summary: &str,
             test_evidence: Vec<CanonicalHermesTestEvidence>| {
                let mut receipt = CanonicalHermesExecutionReceipt {
                    schema_version: "arda.execution-receipt.v3".into(),
                    receipt_digest: String::new(),
                    authority_binding_digest: authority_binding_digest.into(),
                    run_id: "run-review".into(),
                    node_id: node_id.into(),
                    idempotency_key: format!("run-review:{node_id}"),
                    status: json!("succeeded"),
                    summary: summary.into(),
                    tool_evidence: vec![],
                    test_evidence,
                    artifacts: vec![],
                    usage: CanonicalHermesUsage {
                        api_calls: 1,
                        input_tokens: 1,
                        output_tokens: 1,
                        total_tokens: 2,
                        estimated_cost_usd: 0.0,
                        cost_measurement: json!("provider_reported"),
                        completed: true,
                        failed: false,
                        provider: Some(provider.into()),
                        model: Some(model.into()),
                    },
                    adapter: "hermes".into(),
                    adapter_version: "1".into(),
                    project_contract_digest: "sha256:project".into(),
                    parent_receipts: vec![],
                    context_capsule_id: None,
                    context_capsule_digest: None,
                    context_use_receipt_ref: None,
                    context_handoff: None,
                    recorded_at_unix_ms: 1,
                };
                receipt.receipt_digest = receipt.computed_digest().expect("digest receipt");
                receipt
            };
        let execute = make_receipt(
            "execute",
            "sha256:execute-authority",
            "provider-a",
            "model-a",
            "Project-local inspection found bounded authority evidence.",
            vec![],
        );
        let verify = make_receipt(
            "verify",
            "sha256:verify-authority",
            "provider-b",
            "model-b",
            "Verification passed.",
            vec![CanonicalHermesTestEvidence {
                check_id: "tests".into(),
                command: "cargo test".into(),
                status: "passed".into(),
                exit_code: 0,
                output_digest: "sha256:test".into(),
            }],
        );
        std::fs::write(
            receipt_root.join("execute.json"),
            serde_json::to_vec(&execute).expect("serialize execute receipt"),
        )
        .expect("write execute receipt");
        std::fs::write(
            receipt_root.join("verify.json"),
            serde_json::to_vec(&verify).expect("serialize verify receipt"),
        )
        .expect("write verify receipt");
        let run = json!({
            "graph": {"nodes": [
                {"id": "execute", "output_digest": execute.receipt_digest},
                {"id": "verify", "output_digest": verify.receipt_digest}
            ]},
            "review": {"provider_receipt": null, "tests": []}
        });

        let projection = pre_review_receipt_projection(root.path(), "run-review", &run)
            .expect("project exact receipts");
        assert_eq!(
            projection["execute_receipt"]["usage"]["provider"],
            "provider-a"
        );
        assert_eq!(projection["execute_receipt"]["usage"]["model"], "model-a");
        assert_eq!(
            projection["execute_receipt"]["summary"],
            "Project-local inspection found bounded authority evidence."
        );
        assert_eq!(
            projection["execute_receipt"]["authority_binding_digest"],
            "sha256:execute-authority"
        );
        assert_eq!(
            projection["verify_receipt"]["receipt_digest"],
            verify.receipt_digest
        );
        assert_eq!(
            projection["verify_receipt"]["test_evidence"][0]["status"],
            "passed"
        );
        assert_eq!(projection["verify_output_digest"], verify.receipt_digest);

        let mut tampered = serde_json::to_value(&execute).expect("encode receipt for tamper");
        tampered["summary"] = json!("forged summary retained the old receipt digest");
        std::fs::write(
            receipt_root.join("execute.json"),
            serde_json::to_vec(&tampered).expect("serialize tampered receipt"),
        )
        .expect("write tampered receipt");
        let error = pre_review_receipt_projection(root.path(), "run-review", &run)
            .expect_err("tampered receipt must fail canonical digest validation");
        assert!(error.to_string().contains("invalid canonical digest"));
    }

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

    fn execution_target_task_with_authority(
        id: &str,
        project_id: &str,
        authority_class: &str,
    ) -> QueueRecord {
        let mut task = execution_target_task(id, Some(project_id), None);
        task.extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .expect("execution target metadata")
            .insert("authority_class".into(), json!(authority_class));
        task
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

    fn initialize_git_workspace(workspace: &Path) {
        let git = |args: &[&str]| {
            let output = std::process::Command::new("git")
                .args(args)
                .current_dir(workspace)
                .output()
                .expect("run git fixture command");
            assert!(
                output.status.success(),
                "git fixture command failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        };
        git(&["init", "--quiet"]);
        std::fs::write(workspace.join("owned.txt"), "baseline\n").unwrap();
        git(&["add", "owned.txt"]);
        git(&[
            "-c",
            "user.name=Arda Test",
            "-c",
            "user.email=arda-test@example.invalid",
            "commit",
            "--quiet",
            "-m",
            "fixture baseline",
        ]);
    }

    #[test]
    fn execution_target_admission_rejects_dirty_mutation_but_allows_read_only() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        let workspace_path = "worktrees/dirty";
        write_execution_project_registry(dir.path(), &[(project, workspace_path)]);
        let workspace = dir.path().join(workspace_path);
        initialize_git_workspace(&workspace);
        std::fs::write(workspace.join("owned.txt"), "unrelated local edit\n").unwrap();
        let mut mutation =
            execution_target_task_with_authority("mutation", project, "execute_with_approval");
        mutation.status = Some("queued".into());
        let read_only = execution_target_task_with_authority("review", project, "read_only");
        let mut orphan_predecessor =
            execution_target_task_with_authority("orphan", project, "execute_with_approval");
        orphan_predecessor.status = Some("queued".into());
        orphan_predecessor
            .extra
            .get_mut("meta")
            .and_then(Value::as_object_mut)
            .unwrap()
            .extend([
                ("mutation_risk".into(), json!("operator-approved")),
                ("approval_packet_id".into(), json!("approval-orphan")),
                ("execution_authority".into(), json!("arda_workbench")),
                (
                    "source_objective_packet_id".into(),
                    json!("objective-orphan"),
                ),
                ("action_class".into(), json!("approved_autopilot_plan_step")),
            ]);
        let forged_orphan =
            execution_target_task_with_authority("forged-orphan", project, "execute_with_approval");

        let error = try_acquire_execution_target_locks(dir.path(), &mutation)
            .expect_err("mutation must not enter an already-dirty worktree");
        assert!(
            error.to_string().contains("dirty Git worktree"),
            "dirty-worktree rejection should be explicit: {error:#}"
        );
        assert!(
            try_acquire_execution_target_locks(dir.path(), &read_only)
                .expect("read-only admission should inspect the same workspace")
                .is_some(),
            "byte-exact read-only work must remain eligible in a dirty worktree"
        );
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        std::fs::write(
            &queue_path,
            format!("{}\n", serde_json::to_string(&orphan_predecessor).unwrap()),
        )
        .unwrap();
        let queue = ActiveQueueExecutor::new(dir.path());
        queue
            .append_attempt_fixture(&orphan_predecessor)
            .expect("canonical writer should append persisted attempt fixture");
        crate::prometheus::autopilot::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .append(&crate::prometheus::autopilot::schedule::ScheduleRecord {
            contract: crate::prometheus::autopilot::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: orphan_predecessor.id.clone(),
            objective_id: "objective-orphan".into(),
            mode: crate::prometheus::autopilot::schedule::ScheduleMode::Immediate,
            state: crate::prometheus::autopilot::schedule::ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: Some("persisted recovery fixture".into()),
        })
        .expect("append authoritative immediate schedule");
        let persisted: QueueRecord = std::fs::read_to_string(&queue_path)
            .unwrap()
            .lines()
            .last()
            .map(serde_json::from_str)
            .transpose()
            .unwrap()
            .expect("persisted attempt row");
        let binding = resolve_execution_target(dir.path(), &persisted).unwrap();
        let _locks = try_acquire_execution_target_locks_for_binding(dir.path(), binding)
            .unwrap()
            .expect("persisted mutation attempt target lock");
        assert!(queue
            .is_exact_persisted_workbench_attempt(&persisted)
            .unwrap());
        drop(_locks);
        let queue_rows_before_recovery = std::fs::read_to_string(&queue_path)
            .unwrap()
            .lines()
            .count();
        assert_eq!(
            queue
                .next_approved_reconciling_orphans_excluding(&BTreeSet::new())
                .expect("production selector should read persisted recovery candidate"),
            Some(persisted.clone()),
            "the exact persisted attempt must survive effective-record selection"
        );
        let (recovered, recovery_locks) = claim_execution_with_available_target(dir.path(), &queue)
            .expect("production claim path should evaluate persisted recovery authority")
            .expect("exact persisted mutation attempt should remain eligible in a dirty worktree");
        assert_eq!(recovered.task, persisted);
        assert_eq!(
            recovered.attempt.workbench_run_id,
            persisted
                .extra
                .get("workbench_run_id")
                .and_then(Value::as_str)
                .expect("persisted attempt run id")
        );
        assert_eq!(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .count(),
            queue_rows_before_recovery,
            "persisted recovery must not append a duplicate execution attempt"
        );
        drop(recovery_locks);
        let error = try_acquire_execution_target_locks(dir.path(), &forged_orphan)
            .expect_err("status alone must not grant dirty-worktree recovery authority");
        assert!(
            error.to_string().contains("dirty Git worktree"),
            "forged in-progress status must follow fresh mutation admission: {error:#}"
        );
    }

    #[test]
    fn execution_target_admission_fails_closed_for_broken_git_control_file() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440002";
        let workspace_path = "worktrees/broken-git";
        write_execution_project_registry(dir.path(), &[(project, workspace_path)]);
        let workspace = dir.path().join(workspace_path);
        std::fs::write(workspace.join(".git"), "gitdir: /missing/arda-git-dir\n").unwrap();
        let mut mutation =
            execution_target_task_with_authority("broken-git", project, "execute_with_approval");
        mutation.status = Some("queued".into());

        let error = try_acquire_execution_target_locks(dir.path(), &mutation)
            .expect_err("broken Git control metadata must fail closed");
        assert!(
            error.to_string().contains("could not inspect Git worktree"),
            "broken Git control metadata must not be classified as a non-Git root: {error:#}"
        );
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
    fn execution_target_locks_allow_bounded_read_only_access_to_one_workspace() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let first_task = execution_target_task_with_authority("read-first", project, "read_only");
        let second_task = execution_target_task_with_authority("read-second", project, "read_only");
        let third_task = execution_target_task_with_authority("read-third", project, "read_only");
        let first = try_acquire_execution_target_locks(dir.path(), &first_task)
            .unwrap()
            .expect("first read-only slot");
        let second = try_acquire_execution_target_locks(dir.path(), &second_task)
            .unwrap()
            .expect("second read-only slot");

        assert!(
            try_acquire_execution_target_locks(dir.path(), &third_task)
                .unwrap()
                .is_none(),
            "read-only overlap must stop at the configured slot bound"
        );
        drop((first, second));
    }

    #[test]
    fn execution_target_locks_block_mutation_while_read_only_access_is_active() {
        let dir = tempfile::tempdir().expect("create lock root");
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let read_only = execution_target_task_with_authority("read", project, "read_only");
        let mutation =
            execution_target_task_with_authority("mutation", project, "execute_with_approval");
        let read_lock = try_acquire_execution_target_locks(dir.path(), &read_only)
            .unwrap()
            .expect("read-only lock");

        assert!(
            try_acquire_execution_target_locks(dir.path(), &mutation)
                .unwrap()
                .is_none(),
            "mutation must remain exclusive against active read-only work"
        );
        drop(read_lock);
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

    #[test]
    fn concurrent_read_only_executors_claim_distinct_tasks_in_one_workspace() {
        let dir = tempfile::tempdir().expect("create coordinator root");
        let queue_path = dir.path().join("core/projects/tasks/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let project = "550e8400-e29b-41d4-a716-446655440001";
        write_execution_project_registry(dir.path(), &[(project, ".")]);
        let record = |id: &str| -> QueueRecord {
            serde_json::from_value(json!({
                "id": id,
                "status": "queued",
                "meta": {
                    "action_class": "approved_autopilot_plan_step",
                    "mutation_risk": "operator-approved",
                    "execution_authority": "arda_workbench",
                    "authority_class": "read_only",
                    "source_objective_packet_id": format!("objective-{id}"),
                    "approval_packet_id": format!("approval-{id}"),
                    "project_id": project,
                    "worktree_path": "."
                }
            }))
            .expect("queue record")
        };
        let first = record("read-first");
        let second = record("read-second");
        std::fs::write(
            &queue_path,
            format!(
                "{}\n{}\n",
                serde_json::to_string(&first).unwrap(),
                serde_json::to_string(&second).unwrap()
            ),
        )
        .unwrap();
        for task in [&first, &second] {
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
                reason: Some("read-only claim ownership test".into()),
            })
            .unwrap();
        }
        let queue = ActiveQueueExecutor::new(dir.path());

        let (first_claim, _first_locks) = claim_execution_with_available_target(dir.path(), &queue)
            .unwrap()
            .expect("first read-only claim");
        let (second_claim, _second_locks) =
            claim_execution_with_available_target(dir.path(), &queue)
                .unwrap()
                .expect("second read-only claim");

        assert_eq!(first_claim.task.id, first.id);
        assert_eq!(second_claim.task.id, second.id);
        assert_ne!(
            first_claim.attempt.workbench_run_id, second_claim.attempt.workbench_run_id,
            "concurrent executors must not replay one live Workbench run"
        );
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
    async fn timer_execution_decomposes_multi_project_objective_without_artifact_markers() {
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "multi-project-objective");
        let project_one = "550e8400-e29b-41d4-a716-446655440001";
        let project_two = "550e8400-e29b-41d4-a716-446655440002";
        let mut root: Value = serde_json::from_str(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        root["meta"]["project_ids"] = json!([project_one, project_two, project_one]);
        root["meta"]["project_id"] = Value::String(project_one.into());
        root["title"] = Value::String(
            "Inspect both declared projects with project-bound read-only workers".into(),
        );
        std::fs::write(&queue_path, format!("{root}\n")).unwrap();
        write_execution_project_registry(dir.path(), &[(project_one, "."), (project_two, ".")]);

        let receipt = test_executor(dir.path(), "http://127.0.0.1:9".into())
            .execute_once()
            .await
            .expect("multi-project objective must decompose before provider dispatch");

        assert_eq!(receipt.status, "waiting");
        assert_eq!(receipt.result, "objective_decomposed");
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        let leaves = effective
            .iter()
            .filter(|record| record.extra["meta"]["objective_leaf"] == true)
            .collect::<Vec<_>>();
        assert_eq!(leaves.len(), 6);
        assert_eq!(
            leaves
                .iter()
                .filter(|leaf| leaf.id.contains("__inspect-authorities-project-"))
                .filter_map(|leaf| leaf.extra["meta"]["project_id"].as_str())
                .filter(|project_id| matches!(*project_id, value if value == project_one || value == project_two))
                .count(),
            2
        );
        assert_eq!(
            leaves
                .iter()
                .filter(|leaf| !leaf.id.contains("__inspect-authorities-project-"))
                .filter(|leaf| leaf.extra["meta"]["project_id"] == DEFAULT_PROJECT_ID)
                .count(),
            4
        );
        assert!(leaves
            .iter()
            .filter(|record| record.id.contains("__inspect-authorities-project-"))
            .all(|record| record.extra["meta"]["verification_checks"]
                .as_array()
                .is_some_and(|checks| {
                    checks
                        .iter()
                        .any(|check| check.as_str() == Some("git status --short --branch"))
                        && checks
                            .iter()
                            .all(|check| check.as_str() != Some("cargo test -p arda-core"))
                })));
        let outcome = leaves
            .iter()
            .find(|record| record.id.contains("__produce-outcome--"))
            .expect("outcome leaf");
        assert_eq!(outcome.extra["meta"]["authority_class"], "read_only");
        let recover = leaves
            .iter()
            .find(|record| record.id.contains("__recover-context--"))
            .unwrap();
        let plan: ObjectivePlan =
            serde_json::from_value(recover.extra["meta"]["objective_plan"].clone()).unwrap();
        let prompt = objective_execution_prompt(&plan, "reviewed objective", recover);
        assert!(prompt.contains("evidence-backed context summary"));
        assert!(!prompt.contains("prioritized repair backlog"));

        let inspection_leaf = leaves
            .iter()
            .find(|record| record.id.contains("inspect-authorities-project-1"))
            .unwrap();
        let inspection_prompt =
            objective_execution_prompt(&plan, "reviewed objective", inspection_leaf);
        assert!(inspection_prompt.contains("Inspect only the bound project"));
        assert!(!inspection_prompt.contains("Context sources:\n"));
        assert!(!inspection_prompt.contains("data/workbench/projects.json=sha256:"));
    }

    #[tokio::test]
    async fn joined_objective_uses_acceptance_leaf_receipt_without_synthetic_marker() {
        assert!(is_canonical_workbench_run_id("queue-objective-run-1"));
        assert!(!is_canonical_workbench_run_id("../escape"));
        assert!(!is_canonical_workbench_run_id("/absolute"));
        let dir = tempfile::tempdir().unwrap();
        let queue_path = approved_queue_fixture(dir.path(), "receipt-backed-join");
        let mut root: Value = serde_json::from_str(
            std::fs::read_to_string(&queue_path)
                .unwrap()
                .lines()
                .next()
                .unwrap(),
        )
        .unwrap();
        let project_one = "550e8400-e29b-41d4-a716-446655440001";
        let project_two = "550e8400-e29b-41d4-a716-446655440002";
        root["meta"]["project_ids"] = json!([project_one, project_two]);
        root["meta"]["project_id"] = Value::String(project_one.into());
        std::fs::write(&queue_path, format!("{root}\n")).unwrap();
        write_execution_project_registry(dir.path(), &[(project_one, "."), (project_two, ".")]);
        test_executor(dir.path(), "http://127.0.0.1:9".into())
            .execute_once()
            .await
            .expect("objective decomposition");
        let leaves = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(&queue_path)
                .load()
                .unwrap(),
        )
        .into_iter()
        .filter(|record| record.extra["meta"]["objective_leaf"] == true)
        .collect::<Vec<_>>();
        let mut acceptance_provider_digest = String::new();
        let mut acceptance_authority_binding = String::new();
        for (index, leaf) in leaves.iter().enumerate() {
            let run_id = format!("joined-leaf-{index}");
            let task_digest = format!("sha256:{:064x}", index + 101);
            let receipt_path = dir
                .path()
                .join("data/runs")
                .join(&run_id)
                .join("execution-receipts/review.json");
            std::fs::create_dir_all(receipt_path.parent().unwrap()).unwrap();
            let mut receipt: CanonicalHermesExecutionReceipt = serde_json::from_value(json!({
                "schema_version": "arda.execution-receipt.v3",
                "run_id": run_id,
                "node_id": "review",
                "idempotency_key": format!("review-{index}"),
                "status": "succeeded",
                "receipt_digest": "",
                "authority_binding_digest": task_digest,
                "summary": "approved",
                "tool_evidence": [],
                "test_evidence": [],
                "artifacts": [],
                "usage": {
                    "provider": null,
                    "model": null,
                    "api_calls": 0,
                    "input_tokens": 0,
                    "output_tokens": 0,
                    "total_tokens": 0,
                    "estimated_cost_usd": 0.0,
                    "cost_measurement": "unknown",
                    "completed": true,
                    "failed": false
                },
                "adapter": "hermes",
                "adapter_version": "test",
                "project_contract_digest": format!("sha256:{:064x}", 777),
                "parent_receipts": [],
                "recorded_at_unix_ms": 1
            }))
            .unwrap();
            receipt.receipt_digest = receipt.computed_digest().unwrap();
            let provider_digest = receipt.receipt_digest.clone();
            if leaf.id.contains("__verify-acceptance--") {
                acceptance_provider_digest = provider_digest.clone();
                acceptance_authority_binding = task_digest.clone();
            }
            std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
            std::fs::write(
                dir.path()
                    .join("data/runs")
                    .join(&run_id)
                    .join("result.json"),
                serde_json::to_vec_pretty(&json!({
                    "provider_receipt": {
                        "receipt_digest": provider_digest,
                        "authority_binding_digest": task_digest
                    }
                }))
                .unwrap(),
            )
            .unwrap();
            std::fs::write(
                dir.path()
                    .join("data/runs")
                    .join(&run_id)
                    .join("checkpoint.json"),
                serde_json::to_vec_pretty(&json!({
                    "run_id": run_id,
                    "objective_id": leaf.id
                }))
                .unwrap(),
            )
            .unwrap();
            let mut value = serde_json::to_value(leaf).unwrap();
            value["status"] = Value::String("completed".into());
            value["result"] = Value::String("completed".into());
            value["workbench_run_id"] = Value::String(run_id);
            value["execution_receipt_digest"] = Value::String(format!(
                "sha256:{:x}",
                Sha256::digest(std::fs::read(&receipt_path).unwrap())
            ));
            append_queue_value(dir.path(), value).unwrap();
        }
        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(&queue_path)
                .load()
                .unwrap(),
        );
        let completed = effective
            .iter()
            .find(|record| record.id.contains("__verify-acceptance--"))
            .unwrap();

        let acceptance_result = dir.path().join("data/runs/joined-leaf-5/result.json");
        std::fs::write(
            &acceptance_result,
            serde_json::to_vec_pretty(&json!({
                "provider_receipt": {
                    "receipt_digest": format!("sha256:{:064x}", 999),
                    "authority_binding_digest": format!("sha256:{:064x}", 106)
                }
            }))
            .unwrap(),
        )
        .unwrap();
        let error = advance_objective_after_leaf(dir.path(), completed).unwrap_err();
        assert!(error.to_string().contains("receipt digest mismatch"));
        std::fs::write(
            &acceptance_result,
            serde_json::to_vec_pretty(&json!({
                "provider_receipt": {
                    "receipt_digest": acceptance_provider_digest,
                    "authority_binding_digest": acceptance_authority_binding
                }
            }))
            .unwrap(),
        )
        .unwrap();

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));
        let root_path = dir.path().to_path_buf();
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..8 {
                let barrier = barrier.clone();
                let root_path = &root_path;
                handles.push(scope.spawn(move || {
                    barrier.wait();
                    advance_objective_after_leaf(root_path, completed)
                }));
            }
            for handle in handles {
                handle.join().unwrap().expect("receipt-backed join");
            }
        });

        let effective = super::super::task_queue::TaskQueueAnalyzer::effective_records(
            super::super::task_queue::TaskQueueAnalyzer::new(queue_path)
                .load()
                .unwrap(),
        );
        let root = effective
            .iter()
            .find(|record| record.id == "receipt-backed-join")
            .unwrap();
        assert_eq!(root.status.as_deref(), Some("completed"));
        assert_eq!(
            root.extra["acceptance_artifact"],
            "data/runs/joined-leaf-5/execution-receipts/review.json"
        );
        for leaf in &leaves {
            advance_objective_after_leaf(dir.path(), leaf).unwrap();
        }
        let terminal_roots = super::super::task_queue::TaskQueueAnalyzer::new(
            dir.path().join("core/projects/tasks/queue.jsonl"),
        )
        .load()
        .unwrap()
        .into_iter()
        .filter(|record| {
            record.id == "receipt-backed-join"
                && record
                    .extra
                    .get("contract")
                    .is_some_and(|contract| contract == "arda.workbench.objective_terminal.v1")
        })
        .count();
        assert_eq!(terminal_roots, 1);
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
                let request = String::from_utf8_lossy(&request).into_owned();
                requests.push(request.clone());
                let Some((status, body)) = response else {
                    break;
                };
                let body = if body.contains("__REQUEST_RECEIPT_DIGEST__") {
                    let digest = request
                        .split_once("\r\n\r\n")
                        .and_then(|(_, body)| serde_json::from_str::<Value>(body).ok())
                        .and_then(|body| body["receipt_digest"].as_str().map(str::to_owned))
                        .expect("scripted response requested the submitted receipt digest");
                    body.replace("__REQUEST_RECEIPT_DIGEST__", &digest)
                } else {
                    body
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
        let receipt_root = dir
            .path()
            .join("data/runs/queue-restart-completion-task/execution-receipts");
        std::fs::create_dir_all(&receipt_root).unwrap();
        let mut receipt_digests = std::collections::BTreeMap::new();
        for (stage, provider, model, tests) in [
            ("execute", "provider-execute", "model-execute", vec![]),
            (
                "verify",
                "provider-verify",
                "model-verify",
                vec![CanonicalHermesTestEvidence {
                    check_id: "cargo-test".into(),
                    command: "cargo test".into(),
                    status: "passed".into(),
                    exit_code: 0,
                    output_digest: "sha256:test-output".into(),
                }],
            ),
        ] {
            let mut canonical = CanonicalHermesExecutionReceipt {
                schema_version: "arda.execution-receipt.v3".into(),
                receipt_digest: String::new(),
                authority_binding_digest: format!("sha256:{stage}-authority"),
                run_id: "queue-restart-completion-task".into(),
                node_id: stage.into(),
                idempotency_key: format!("queue-restart-completion-task:{stage}"),
                status: json!("succeeded"),
                summary: format!("{stage} completed"),
                tool_evidence: vec![],
                test_evidence: tests,
                artifacts: vec![],
                usage: CanonicalHermesUsage {
                    provider: Some(provider.into()),
                    model: Some(model.into()),
                    api_calls: 1,
                    input_tokens: 1,
                    output_tokens: 1,
                    total_tokens: 2,
                    estimated_cost_usd: 0.0,
                    cost_measurement: json!("provider_reported"),
                    completed: true,
                    failed: false,
                },
                adapter: "hermes".into(),
                adapter_version: "1".into(),
                project_contract_digest: "sha256:project-contract".into(),
                parent_receipts: vec![],
                context_capsule_id: None,
                context_capsule_digest: None,
                context_use_receipt_ref: None,
                context_handoff: None,
                recorded_at_unix_ms: 1,
            };
            canonical.receipt_digest = canonical.computed_digest().unwrap();
            receipt_digests.insert(stage, canonical.receipt_digest.clone());
            std::fs::write(
                receipt_root.join(format!("{stage}.json")),
                serde_json::to_vec(&canonical).unwrap(),
            )
            .unwrap();
        }
        let execute_digest = receipt_digests["execute"].clone();
        let verify_digest = receipt_digests["verify"].clone();
        let (harness_url, server) = scripted_harness(vec![
            Some((
                200,
                json!({
                    "graph": {"nodes": [
                        {"id": "plan", "state": "succeeded"},
                        {"id": "approval", "state": "succeeded"},
                        {"id": "execute", "state": "succeeded", "output_digest": execute_digest},
                        {"id": "verify", "state": "ready"},
                        {"id": "review", "state": "blocked"},
                        {"id": "close", "state": "blocked"}
                    ]},
                    "review": {"provider_receipt": {"receipt_digest": execute_digest, "summary": "executed"}}
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "receipt": {"status": "succeeded", "receipt_digest": verify_digest, "summary": "verified"},
                    "run": {
                        "graph": {"nodes": [
                            {"id": "execute", "state": "succeeded", "output_digest": execute_digest},
                            {"id": "verify", "state": "succeeded", "output_digest": verify_digest},
                            {"id": "review", "state": "ready"},
                            {"id": "close", "state": "blocked"}
                        ]},
                        "review": {
                            "tests": [{"name": "cargo test", "status": "passed"}],
                            "provider_receipt": {"receipt_digest": verify_digest, "summary": "verified"}
                        }
                    }
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "receipt": {
                        "status": "succeeded",
                        "receipt_digest": "sha256:review",
                        "summary": "independent critic passed"
                    },
                    "run": {
                        "graph": {"nodes": [
                            {"id": "execute", "state": "succeeded", "output_digest": execute_digest},
                            {"id": "verify", "state": "succeeded", "output_digest": verify_digest},
                            {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                            {"id": "close", "state": "ready"}
                        ]},
                        "review": {
                            "tests": [{"name": "cargo test", "status": "passed"}],
                            "provider_receipt": {"receipt_digest": "sha256:review", "summary": "independent critic passed"}
                        }
                    }
                })
                .to_string(),
            )),
            Some((
                200,
                json!({
                    "graph": {"nodes": [
                        {"id": "execute", "state": "succeeded", "output_digest": execute_digest},
                        {"id": "verify", "state": "succeeded", "output_digest": verify_digest},
                        {"id": "review", "state": "succeeded", "output_digest": "sha256:review"},
                        {"id": "close", "state": "succeeded", "output_digest": "sha256:close"}
                    ]},
                    "review": {
                        "tests": [{"name": "cargo test", "status": "passed"}],
                        "provider_receipt": {"receipt_digest": verify_digest, "summary": "verified"}
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
        assert!(requests[2].contains("/nodes/review/execute-provider"));
        assert!(requests[2].contains("provider-execute"));
        assert!(requests[2].contains("model-verify"));
        assert!(requests[2].contains("sha256:execute-authority"));
        assert!(requests[2].contains("sha256:verify"));
        assert!(requests[2].contains("cargo-test"));
        assert!(requests[2].contains("passed"));
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
    fn run_graph_assigns_review_to_a_distinct_independent_critic() {
        let graph = run_graph(
            "queue-critic-task",
            "critic-task",
            "Review me",
            "approval-1",
        );
        let nodes = graph["nodes"].as_array().expect("run nodes");
        let execute = nodes.iter().find(|node| node["id"] == "execute").unwrap();
        let verify = nodes.iter().find(|node| node["id"] == "verify").unwrap();
        let review = nodes.iter().find(|node| node["id"] == "review").unwrap();

        assert_eq!(review["worker"]["role"], "security_privacy_critic");
        assert_eq!(review["worker"]["evidence_policy"], "worker_report");
        assert_eq!(review["worker"]["dependencies"], json!(["verify"]));
        assert_eq!(
            review["worker"]["allowed_toolsets"],
            json!(["file"]),
            "read-only critics must fit the configured read_only capability set"
        );
        assert_ne!(
            review["worker"]["worker_id"],
            execute["worker"]["worker_id"]
        );
        assert_ne!(review["worker"]["worker_id"], verify["worker"]["worker_id"]);
    }

    #[test]
    fn read_only_leaf_uses_an_inspection_worker_contract() {
        let contract = ExecutableLeafContract {
            project_id: DEFAULT_PROJECT_ID.to_string(),
            authority_class: "read_only".to_string(),
            verification_checks: vec!["git status --short".to_string()],
            evidence_requirements: vec!["worker_report".to_string()],
            max_joules: 1000.0,
            max_cost_usd: 1.0,
            max_attempts: 2,
            timeout_seconds: 300,
        };
        let graph = run_graph_with_objective_plan_receipt(
            "queue-read-only-leaf",
            "read-only-leaf",
            "Inspect the bound project",
            "approval-read-only-leaf",
            "sha256:plan",
            Some(&contract),
        );
        let execute = graph["nodes"]
            .as_array()
            .expect("run nodes")
            .iter()
            .find(|node| node["id"] == "execute")
            .expect("execute node");

        assert_eq!(execute["kind"], "inspect");
        assert_eq!(execute["authority"], "read_only");
        assert_eq!(execute["worker"]["role"], "local_summary_classification");
        assert_eq!(
            execute["worker"]["allowed_toolsets"],
            json!(["file"]),
            "read-only inspection must not request tools outside read_only authority"
        );
    }

    #[test]
    fn claim_before_dispatch_crash_child() {
        let Ok(root) = std::env::var("ARDA_CLAIM_CRASH_FIXTURE_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let _executor_lock = acquire_executor_lock(&root).expect("acquire child executor lock");
        let (claim, _execution_locks) =
            claim_execution_with_available_target(&root, &ActiveQueueExecutor::new(&root))
                .expect("claim fixture task")
                .expect("approved fixture claim");
        assert_eq!(claim.task.id, "pre-dispatch-crash-task");
        std::fs::write(root.join("claim-ready"), b"ready\n").expect("signal held claim");
        std::thread::sleep(std::time::Duration::from_secs(30));
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
                "{}\n{}\n",
                json!({
                    "id": "pre-dispatch-crash-task",
                    "title": "Recover before lease expiry",
                    "status": "queued",
                    "meta": {
                        "action_class": "approved_autopilot_plan_step",
                        "mutation_risk": "operator-approved",
                        "execution_authority": "arda_workbench",
                        "authority_class": "read_only",
                        "project_id": DEFAULT_PROJECT_ID,
                        "worktree_path": ".",
                        "source_objective_packet_id": "objective-crash-proof",
                        "approval_packet_id": "approval-crash-proof"
                    }
                }),
                json!({
                    "id": "post-crash-distinct-task",
                    "title": "Run beside the held task",
                    "status": "queued",
                    "meta": {
                        "action_class": "approved_autopilot_plan_step",
                        "mutation_risk": "operator-approved",
                        "execution_authority": "arda_workbench",
                        "authority_class": "read_only",
                        "project_id": DEFAULT_PROJECT_ID,
                        "worktree_path": ".",
                        "source_objective_packet_id": "objective-distinct-proof",
                        "approval_packet_id": "approval-distinct-proof"
                    }
                })
            ),
        )
        .unwrap();
        std::fs::write(
            &active_path,
            "{\"active\":[{\"id\":\"pre-dispatch-crash-task\"},{\"id\":\"post-crash-distinct-task\"}]}\n",
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
        super::super::schedule::ScheduleLedger::new(
            dir.path().join("core/projects/tasks/schedules.jsonl"),
        )
        .append(&super::super::schedule::ScheduleRecord {
            contract: super::super::schedule::SCHEDULE_RECORD_CONTRACT.into(),
            task_id: "post-crash-distinct-task".into(),
            objective_id: "objective-distinct-proof".into(),
            mode: super::super::schedule::ScheduleMode::Immediate,
            state: super::super::schedule::ScheduleState::Scheduled,
            not_before_utc: None,
            interval_seconds: None,
            recorded_at_utc: Utc::now(),
            reason: Some("distinct process fixture schedule authority".into()),
        })
        .unwrap();
        write_execution_project_registry(dir.path(), &[(DEFAULT_PROJECT_ID, ".")]);

        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("prometheus::autopilot::workbench_executor::tests::claim_before_dispatch_crash_child")
            .arg("--nocapture")
            .env("ARDA_CLAIM_CRASH_FIXTURE_ROOT", dir.path())
            .spawn()
            .expect("run crash child");
        let ready_path = dir.path().join("claim-ready");
        for _ in 0..100 {
            if ready_path.exists() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(ready_path.exists(), "child did not acquire execution locks");

        let queue = ActiveQueueExecutor::new(dir.path());
        let (distinct, _distinct_locks) = claim_execution_with_available_target(dir.path(), &queue)
            .expect("claim beside live owner")
            .expect("second read-only slot");
        assert_eq!(
            distinct.task.id, "post-crash-distinct-task",
            "a live task owner must force the next executor onto a distinct task"
        );
        assert!(
            try_acquire_task_execution_lock(dir.path(), "pre-dispatch-crash-task")
                .expect("probe live task ownership")
                .is_none(),
            "the child must retain its task lock through execution"
        );

        child.kill().expect("terminate lock owner");
        let child = child.wait().expect("reap lock owner");
        assert!(!child.success());
        let released_task_lock =
            try_acquire_task_execution_lock(dir.path(), "pre-dispatch-crash-task")
                .expect("probe released task ownership")
                .expect("process exit must release the task lock");
        drop(released_task_lock);

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
        let (recovered, _execution_locks) =
            claim_execution_with_available_target(dir.path(), &queue)
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
        assert_eq!(plan.context_sources.len(), 5);
        assert!(!plan
            .context_sources
            .iter()
            .any(|source| source.kind == "repository_state"));
        assert!(plan.context_sources.iter().all(|source| {
            source
                .digest
                .as_deref()
                .is_some_and(|value| value.starts_with("sha256:"))
        }));

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
    fn restart_does_not_materialize_leaf_replan_for_terminal_root_task() {
        let dir = tempfile::tempdir().unwrap();
        append_queue_value(
            dir.path(),
            json!({
                "id": "terminal-root-objective",
                "source_record_id": "terminal-root-objective",
                "title": "Terminal root objective",
                "status": "failed",
                "result": "failed",
                "continuation_decision": "replan_objective",
                "workbench_run_id": "queue-terminal-root-objective",
                "meta": {
                    "action_class": "approved_autopilot_plan_step",
                    "source_objective_packet_id": "objective-terminal-root",
                    "approval_packet_id": "approval-terminal-root"
                }
            }),
        )
        .unwrap();

        reconcile_terminal_objective_leaves(dir.path())
            .expect("terminal root reconciliation must not require leaf-only metadata");

        let queue =
            std::fs::read_to_string(dir.path().join("core/projects/tasks/queue.jsonl")).unwrap();
        assert_eq!(queue.lines().count(), 1);
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
    fn multi_project_objective_creates_parallel_project_bound_inspection_leaves() {
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
            "id": "objective-multi-project",
            "title": "Inspect two real projects and join the evidence",
            "status": "in_progress",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "packet-multi-project",
                "approval_packet_id": "approval-multi-project",
                "project_id": "project-one",
                "project_ids": ["project-one", "project-two"]
            }
        }))
        .unwrap();

        let plan = objective_plan_for_task(dir.path(), &task).unwrap();
        let inspections = plan
            .tasks
            .iter()
            .filter(|task| task.key.starts_with("inspect-authorities-project-"))
            .collect::<Vec<_>>();
        assert_eq!(inspections.len(), 2);
        assert!(inspections
            .iter()
            .all(|task| task.depends_on == ["recover-context"]));
        assert_eq!(
            inspections
                .iter()
                .map(|task| task.title.as_str())
                .collect::<Vec<_>>(),
            [
                "Inspect only bound project `project-one` for project-local evidence needed by: Inspect two real projects and join the evidence. Do not require sibling project files in this leaf; joined synthesis compares all project leaves.",
                "Inspect only bound project `project-two` for project-local evidence needed by: Inspect two real projects and join the evidence. Do not require sibling project files in this leaf; joined synthesis compares all project leaves."
            ]
        );
        let project_ids = inspections
            .iter()
            .map(|task| plan.leaf_contracts[&task.key].project_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(project_ids, ["project-one", "project-two"]);
        let synthesis = plan
            .tasks
            .iter()
            .find(|task| task.key == "synthesize-findings")
            .unwrap();
        assert_eq!(
            synthesis.depends_on,
            [
                "inspect-authorities-project-1",
                "inspect-authorities-project-2"
            ]
        );
        let leaves = materialize_objective_leaves(
            dir.path(),
            &task,
            &plan,
            "sha256:multi-project-plan",
            "queue-objective-multi-project",
        )
        .unwrap();
        let materialized_project_ids = leaves
            .iter()
            .filter(|leaf| {
                leaf.extra["meta"]["objective_leaf_key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with("inspect-authorities-project-"))
            })
            .map(|leaf| leaf.extra["meta"]["project_id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(materialized_project_ids, ["project-one", "project-two"]);
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
        assert_eq!(producer.extra["meta"]["authority_class"], "read_only");
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
            let run_id = format!("durable-leaf-{index}");
            let mut receipt_digest = format!("sha256:leaf-{index}");
            if planned.key == "verify-acceptance" {
                let receipt_path = dir
                    .path()
                    .join("data/runs")
                    .join(&run_id)
                    .join("execution-receipts/review.json");
                std::fs::create_dir_all(receipt_path.parent().unwrap()).unwrap();
                let authority_binding =
                    "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
                let mut receipt: CanonicalHermesExecutionReceipt =
                    serde_json::from_value(json!({
                        "schema_version": "arda.execution-receipt.v3",
                        "run_id": run_id,
                        "node_id": "review",
                        "idempotency_key": "durable-review",
                        "status": "succeeded",
                        "receipt_digest": "",
                        "authority_binding_digest": authority_binding,
                        "summary": "approved",
                        "tool_evidence": [],
                        "test_evidence": [],
                        "artifacts": [],
                        "usage": {
                            "provider": null,
                            "model": null,
                            "api_calls": 0,
                            "input_tokens": 0,
                            "output_tokens": 0,
                            "total_tokens": 0,
                            "estimated_cost_usd": 0.0,
                            "cost_measurement": "unknown",
                            "completed": true,
                            "failed": false
                        },
                        "adapter": "hermes",
                        "adapter_version": "test",
                        "project_contract_digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
                        "parent_receipts": [],
                        "recorded_at_unix_ms": 1
                    }))
                    .unwrap();
                receipt.receipt_digest = receipt.computed_digest().unwrap();
                let provider_digest = receipt.receipt_digest.clone();
                std::fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap())
                    .unwrap();
                std::fs::write(
                    dir.path()
                        .join("data/runs")
                        .join(&run_id)
                        .join("result.json"),
                    serde_json::to_vec_pretty(&json!({
                        "provider_receipt": {
                            "receipt_digest": provider_digest,
                            "authority_binding_digest": authority_binding
                        }
                    }))
                    .unwrap(),
                )
                .unwrap();
                std::fs::write(
                    dir.path()
                        .join("data/runs")
                        .join(&run_id)
                        .join("checkpoint.json"),
                    serde_json::to_vec_pretty(&json!({
                        "run_id": run_id,
                        "objective_id": leaf.id
                    }))
                    .unwrap(),
                )
                .unwrap();
                receipt_digest = format!(
                    "sha256:{:x}",
                    Sha256::digest(std::fs::read(receipt_path).unwrap())
                );
            }
            append_queue_value(
                dir.path(),
                json!({
                    "id": leaf.id,
                    "source_record_id": leaf.id,
                    "title": leaf.title,
                    "status": "completed",
                    "result": "completed",
                    "workbench_run_id": run_id,
                    "execution_receipt_digest": receipt_digest,
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
        let acceptance_index = plan
            .tasks
            .iter()
            .position(|task| task.key == "verify-acceptance")
            .unwrap();
        assert_eq!(
            closed.extra["acceptance_artifact"],
            format!("data/runs/durable-leaf-{acceptance_index}/execution-receipts/review.json")
        );
    }

    #[test]
    fn direct_task_critic_rejection_materializes_revise_continuation() {
        let dir = tempfile::tempdir().unwrap();
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "direct-task",
            "title": "Verify the attached project",
            "status": "in_progress",
            "meta": {
                "action_class": "approved_autopilot_plan_step",
                "mutation_risk": "operator-approved",
                "execution_authority": "arda_workbench",
                "source_objective_packet_id": "direct-objective",
                "approval_packet_id": "approval-direct"
            }
        }))
        .unwrap();
        append_queue_value(dir.path(), serde_json::to_value(&task).unwrap()).unwrap();
        append_queue_value(
            dir.path(),
            json!({
                "id": task.id,
                "source_record_id": task.id,
                "title": task.title,
                "status": "failed",
                "result": "failed",
                "continuation_decision": "revise_task",
                "workbench_run_id": "queue-direct-task",
                "meta": task.extra["meta"],
            }),
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
        let revised = effective
            .iter()
            .find(|record| record.id == "direct-task")
            .unwrap();
        assert_eq!(revised.status.as_deref(), Some("queued"));
        assert_eq!(revised.extra["continuation_decision"], "revise_task");
        assert_eq!(revised.extra["revision_sequence"], 1);
    }

    #[test]
    fn review_prompt_embeds_canonical_receipt_projection() {
        let task: QueueRecord = serde_json::from_value(json!({
            "id": "review-task",
            "title": "Review the verified project"
        }))
        .unwrap();
        let prompt = review_prompt(
            &task,
            None,
            &json!({
                "provider_receipt": {
                    "receipt_digest": "sha256:execute-receipt",
                    "summary": "execution completed"
                },
                "tests": [{"name": "test", "status": "passed"}]
            }),
        );

        assert!(prompt.contains("sha256:execute-receipt"));
        assert!(prompt.contains("\"name\":\"test\",\"status\":\"passed\""));
        assert!(prompt.contains("credential-free receipt projection"));
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
