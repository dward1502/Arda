//! Bounded Hermes CLI execution for approved run-graph nodes.
//!
//! Hermes session metadata is consumed only as transient adapter input. The
//! returned receipt contains normalized usage and evidence, while canonical run
//! state receives only the receipt digest through [`RunEventDraft`].

use super::AdapterCancellation;
use crate::runs::{RunEventDraft, RunEventKind};
use arda_core::run_graph::{
    AuthorityClass, NodeId, NodeKind, NodeState, RunId, RunNode, WorkerRouteClass,
};
use arda_orome::WorkerContextHandoffReceipt;
use arda_vaire::ContextAssembly;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
#[cfg(unix)]
use std::os::unix::process::CommandExt as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::time::timeout;

const CONFIG_SCHEMA_VERSION: &str = "arda.hermes-adapter.v1";
const RESULT_SCHEMA_VERSION: &str = "arda.hermes-job-result.v1";
const RECEIPT_SCHEMA_VERSION: &str = "arda.execution-receipt.v3";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesToolsets {
    pub read_only: Vec<String>,
    pub human_approval: Vec<String>,
    pub execute_with_approval: Vec<String>,
    pub verify: Vec<String>,
    pub compensate_with_approval: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesAdapterConfig {
    pub schema_version: String,
    pub adapter_version: String,
    pub executable: String,
    pub max_timeout_ms: u64,
    pub cancellation_grace_ms: u64,
    pub max_turns: u32,
    pub max_prompt_bytes: usize,
    pub max_output_bytes: usize,
    pub inherit_environment: Vec<String>,
    pub toolsets: HermesToolsets,
}

impl HermesAdapterConfig {
    pub fn from_toml_str(raw: &str) -> Result<Self, HermesAdapterError> {
        let config: Self = toml::from_str(raw)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<(), HermesAdapterError> {
        if self.schema_version != CONFIG_SCHEMA_VERSION {
            return Err(HermesAdapterError::UnsupportedConfigSchema(
                self.schema_version.clone(),
            ));
        }
        if self.adapter_version.trim().is_empty() {
            return Err(HermesAdapterError::InvalidConfig(
                "adapter_version cannot be empty".into(),
            ));
        }
        if self.executable.trim().is_empty()
            || self.max_timeout_ms == 0
            || self.cancellation_grace_ms == 0
            || self.max_turns == 0
            || self.max_prompt_bytes == 0
            || self.max_output_bytes == 0
        {
            return Err(HermesAdapterError::InvalidConfig(
                "executable and all configured bounds must be non-zero".into(),
            ));
        }
        for key in &self.inherit_environment {
            if key.is_empty()
                || key.contains('=')
                || key.as_bytes().contains(&0)
                || !key
                    .chars()
                    .all(|character| character == '_' || character.is_ascii_alphanumeric())
            {
                return Err(HermesAdapterError::InvalidEnvironmentKey(key.clone()));
            }
        }
        for toolset in [
            &self.toolsets.read_only,
            &self.toolsets.human_approval,
            &self.toolsets.execute_with_approval,
            &self.toolsets.verify,
            &self.toolsets.compensate_with_approval,
        ] {
            if toolset.iter().any(|name| {
                name.is_empty()
                    || !name.chars().all(|character| {
                        character == '-' || character == '_' || character.is_ascii_alphanumeric()
                    })
            }) {
                return Err(HermesAdapterError::InvalidConfig(
                    "toolset names must be non-empty ASCII identifiers".into(),
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesNodeTask {
    pub run_id: RunId,
    pub node: RunNode,
    pub objective: String,
    pub instructions: String,
    pub checks: Vec<String>,
    #[serde(default)]
    pub check_commands: BTreeMap<String, String>,
    pub project_contract_digest: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_assembly: Option<ContextAssembly>,
}

impl HermesNodeTask {
    /// Bind a provider receipt to the complete admitted task while excluding
    /// node fields that record mutable execution progress.
    pub fn authority_binding_digest(&self) -> Result<String, HermesAdapterError> {
        let mut admitted = self.clone();
        admitted.node.state = NodeState::Ready;
        admitted.node.output_digest = None;
        admitted.node.checkpoint = Default::default();
        digest_serializable(&admitted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HermesReceiptStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesToolEvidence {
    pub tool: String,
    pub action: String,
    pub exit_code: Option<i32>,
    pub output_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesTestEvidence {
    pub check_id: String,
    pub command: String,
    pub status: String,
    pub exit_code: i32,
    pub output_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesArtifactEvidence {
    pub path: String,
    pub digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizedHermesUsage {
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_calls: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    #[serde(default)]
    pub cost_measurement: CostMeasurement,
    pub completed: bool,
    pub failed: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostMeasurement {
    Observed,
    Estimated,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HermesExecutionReceipt {
    pub schema_version: String,
    pub receipt_digest: String,
    #[serde(default)]
    pub authority_binding_digest: String,
    pub run_id: String,
    pub node_id: String,
    pub idempotency_key: String,
    pub status: HermesReceiptStatus,
    pub summary: String,
    pub tool_evidence: Vec<HermesToolEvidence>,
    pub test_evidence: Vec<HermesTestEvidence>,
    pub artifacts: Vec<HermesArtifactEvidence>,
    pub usage: NormalizedHermesUsage,
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

impl HermesExecutionReceipt {
    /// Recompute the canonical digest from the typed receipt with the digest
    /// field cleared, matching receipt construction in [`HermesAdapter`].
    pub fn computed_digest(&self) -> Result<String, HermesAdapterError> {
        let mut unsigned = self.clone();
        unsigned.receipt_digest.clear();
        digest_serializable(&unsigned)
    }

    pub fn has_valid_digest(&self) -> Result<bool, HermesAdapterError> {
        Ok(
            is_sha256_digest(&self.receipt_digest)
                && self.computed_digest()? == self.receipt_digest,
        )
    }

    /// Project the normalized receipt into canonical run state. No Hermes
    /// session identifier or transcript location is present in this event.
    pub fn run_event_draft(&self) -> Result<RunEventDraft, HermesAdapterError> {
        let state = match self.status {
            HermesReceiptStatus::Succeeded => NodeState::Succeeded,
            HermesReceiptStatus::Failed => NodeState::Failed,
            HermesReceiptStatus::Cancelled => NodeState::Cancelled,
        };
        Ok(RunEventDraft {
            node_id: NodeId::new(&self.node_id)
                .map_err(|error| HermesAdapterError::InvalidResult(error.to_string()))?,
            idempotency_key: self.idempotency_key.clone(),
            kind: RunEventKind::NodeTransition { state },
            receipt_digest: Some(self.receipt_digest.clone()),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct HermesJobResult {
    schema_version: String,
    status: HermesReceiptStatus,
    summary: String,
    tool_evidence: Vec<ClaimedToolEvidence>,
    test_evidence: Vec<ClaimedTestEvidence>,
    artifacts: Vec<HermesArtifactEvidence>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimedToolEvidence {
    tool_call_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimedTestEvidence {
    check_id: String,
    tool_call_id: String,
}

#[derive(Debug, Default, Deserialize)]
struct HermesSessionExport {
    #[serde(default)]
    id: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    billing_provider: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    api_call_count: u64,
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    #[serde(default)]
    estimated_cost_usd: Option<f64>,
    #[serde(default)]
    actual_cost_usd: Option<f64>,
    #[serde(default)]
    messages: Vec<HermesSessionMessage>,
}

#[derive(Debug, Default, Deserialize)]
struct HermesSessionMessage {
    #[serde(default)]
    role: String,
    #[serde(default)]
    content: Option<serde_json::Value>,
    #[serde(default)]
    tool_call_id: Option<String>,
    #[serde(default)]
    tool_name: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<HermesSessionToolCall>>,
}

#[derive(Debug, Deserialize)]
struct HermesSessionToolCall {
    id: String,
    function: HermesSessionFunction,
}

#[derive(Debug, Deserialize)]
struct HermesSessionFunction {
    name: String,
    arguments: serde_json::Value,
}

impl From<&HermesSessionExport> for NormalizedHermesUsage {
    fn from(session: &HermesSessionExport) -> Self {
        let (estimated_cost_usd, cost_measurement) = session
            .actual_cost_usd
            .filter(|cost| cost.is_finite() && *cost >= 0.0)
            .map(|cost| (cost, CostMeasurement::Observed))
            .or_else(|| {
                session
                    .estimated_cost_usd
                    .filter(|cost| cost.is_finite() && *cost >= 0.0)
                    .map(|cost| (cost, CostMeasurement::Estimated))
            })
            .unwrap_or((0.0, CostMeasurement::Unknown));
        Self {
            provider: session.billing_provider.clone(),
            model: session.model.clone(),
            api_calls: session.api_call_count,
            input_tokens: session.input_tokens,
            output_tokens: session.output_tokens,
            total_tokens: session.input_tokens.saturating_add(session.output_tokens),
            estimated_cost_usd,
            cost_measurement,
            completed: true,
            failed: false,
        }
    }
}

#[derive(Debug)]
pub struct HermesAdapter {
    config: HermesAdapterConfig,
    executable: PathBuf,
    project_root: PathBuf,
    cwd: PathBuf,
    environment: BTreeMap<String, String>,
}

impl HermesAdapter {
    pub fn load(
        config_path: impl AsRef<Path>,
        project_root: impl AsRef<Path>,
        cwd: impl AsRef<Path>,
        host_environment: &BTreeMap<String, String>,
    ) -> Result<Self, HermesAdapterError> {
        let raw =
            fs::read_to_string(config_path.as_ref()).map_err(|source| HermesAdapterError::Io {
                context: format!("read config {}", config_path.as_ref().display()),
                source,
            })?;
        let config = HermesAdapterConfig::from_toml_str(&raw)?;
        let project_root = canonical_directory(project_root.as_ref(), "project root")?;
        let cwd = canonical_directory(cwd.as_ref(), "working directory")?;
        if !cwd.starts_with(&project_root) {
            return Err(HermesAdapterError::WorkingDirectoryEscape { cwd, project_root });
        }
        let executable = resolve_executable(&config.executable, host_environment)?;
        let mut environment = BTreeMap::new();
        for key in &config.inherit_environment {
            if let Some(value) = host_environment.get(key) {
                environment.insert(key.clone(), value.clone());
            }
        }
        Ok(Self {
            config,
            executable,
            project_root,
            cwd,
            environment,
        })
    }

    pub fn preflight(&self, task: &HermesNodeTask) -> Result<(), HermesAdapterError> {
        self.validate_task(task)?;
        let prompt = self.build_prompt(task)?;
        if prompt.len() > self.config.max_prompt_bytes {
            return Err(HermesAdapterError::PromptTooLarge {
                actual: prompt.len(),
                limit: self.config.max_prompt_bytes,
            });
        }
        Ok(())
    }

    pub fn validate_stored_receipt_authority(
        &self,
        task: &HermesNodeTask,
        receipt: &HermesExecutionReceipt,
    ) -> Result<(), HermesAdapterError> {
        if receipt.schema_version != RECEIPT_SCHEMA_VERSION {
            return Err(HermesAdapterError::InvalidResult(format!(
                "unsupported execution receipt schema {}",
                receipt.schema_version
            )));
        }
        if receipt.authority_binding_digest != task.authority_binding_digest()? {
            return Err(HermesAdapterError::InvalidResult(
                "stored provider receipt does not match the current admitted task".into(),
            ));
        }
        if receipt.adapter != "hermes-workbench"
            || receipt.adapter_version != self.config.adapter_version
        {
            return Err(HermesAdapterError::InvalidResult(
                "stored provider receipt does not match the configured adapter route".into(),
            ));
        }
        if receipt.usage.provider.as_deref().is_none_or(str::is_empty)
            || receipt.usage.model.as_deref().is_none_or(str::is_empty)
        {
            return Err(HermesAdapterError::InvalidResult(
                "stored provider receipt requires observed provider and model identity".into(),
            ));
        }
        Ok(())
    }

    pub async fn execute(
        &self,
        task: &HermesNodeTask,
        cancellation: AdapterCancellation,
    ) -> Result<HermesExecutionReceipt, HermesAdapterError> {
        self.preflight(task)?;
        let toolsets = self.validate_task(task)?;
        let prompt = self.build_prompt(task)?;
        if *cancellation.subscribe().borrow() {
            return Err(HermesAdapterError::Cancelled);
        }

        let mut timeout_ms = task.node.timeout_ms.min(self.config.max_timeout_ms);
        if let Some(worker) = &task.node.worker {
            let remaining = worker
                .deadline_unix_ms
                .checked_sub(unix_time_ms()?)
                .ok_or(HermesAdapterError::DeadlineExceeded)?;
            timeout_ms = timeout_ms.min(u64::try_from(remaining).unwrap_or(u64::MAX));
            if timeout_ms == 0 {
                return Err(HermesAdapterError::DeadlineExceeded);
            }
        }
        let total_timeout = Duration::from_millis(timeout_ms);
        let started = tokio::time::Instant::now();
        let mut command = Command::new(&self.executable);
        command
            .arg("chat")
            .arg("-Q")
            .arg("--source")
            .arg("tool")
            .arg("--max-turns")
            .arg(self.config.max_turns.to_string())
            .arg("--ignore-rules")
            .arg("-t")
            .arg(toolsets.join(","))
            .arg("-q")
            .arg(&prompt)
            .current_dir(&self.cwd)
            .env_clear()
            .envs(&self.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_process_group(&mut command);
        let chat_output = run_bounded(
            command,
            total_timeout,
            &cancellation,
            self.config.cancellation_grace_ms,
            self.config.max_output_bytes,
        )
        .await?;
        let (session_id, result_bytes) =
            split_chat_output(&chat_output.stdout, &chat_output.stderr)?;
        let result = parse_job_result(&result_bytes)?;
        self.validate_result(task, &result)?;

        let remaining = total_timeout
            .checked_sub(started.elapsed())
            .ok_or(HermesAdapterError::Timeout)?;
        let mut export = Command::new(&self.executable);
        export
            .arg("sessions")
            .arg("export")
            .arg("-")
            .arg("--format")
            .arg("jsonl")
            .arg("--session-id")
            .arg(&session_id)
            .arg("--redact")
            .arg("--yes")
            .current_dir(&self.cwd)
            .env_clear()
            .envs(&self.environment)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        configure_process_group(&mut export);
        let exported = run_bounded(
            export,
            remaining,
            &cancellation,
            self.config.cancellation_grace_ms,
            self.config.max_output_bytes,
        )
        .await?;
        let session: HermesSessionExport = serde_json::from_slice(trim_ascii(&exported.stdout))
            .map_err(|error| HermesAdapterError::InvalidUsage(error.to_string()))?;
        if session.id != session_id {
            return Err(HermesAdapterError::InvalidUsage(
                "exported Hermes session did not match the completed job".into(),
            ));
        }
        if session.source != "tool" {
            return Err(HermesAdapterError::InvalidUsage(
                "exported Hermes session did not retain tool-source attribution".into(),
            ));
        }
        let usage = NormalizedHermesUsage::from(&session);
        if usage
            .provider
            .as_deref()
            .is_none_or(|provider| provider.trim().is_empty())
            || usage
                .model
                .as_deref()
                .is_none_or(|model| model.trim().is_empty())
        {
            return Err(HermesAdapterError::InvalidResult(
                "provider receipt requires provider and model provenance".into(),
            ));
        }
        if !usage.estimated_cost_usd.is_finite()
            || usage.estimated_cost_usd < 0.0
            || usage.estimated_cost_usd > task.node.budget.max_cost_usd
        {
            return Err(HermesAdapterError::CostBudgetExceeded {
                actual: usage.estimated_cost_usd,
                limit: task.node.budget.max_cost_usd,
            });
        }
        let (tool_evidence, test_evidence) =
            translate_actual_evidence(task, &result, &session, &self.cwd)?;

        let recorded_at_unix_ms = unix_time_ms()?;
        let context_handoff = task
            .context_assembly
            .as_ref()
            .map(|assembly| {
                WorkerContextHandoffReceipt::issue(
                    assembly.capsule.context.lineage.objective_id.as_str(),
                    task.run_id.as_str(),
                    task.node.id.as_str(),
                    &assembly.capsule.context.consumer.consumer_id,
                    &assembly.capsule.capsule_id,
                    &assembly.capsule.capsule_digest,
                    assembly.use_receipt.receipt_ref(),
                    task.node.parent_receipts.clone(),
                    recorded_at_unix_ms,
                )
                .map_err(|error| HermesAdapterError::InvalidResult(error.to_string()))
            })
            .transpose()?;
        let mut receipt = HermesExecutionReceipt {
            schema_version: RECEIPT_SCHEMA_VERSION.into(),
            receipt_digest: String::new(),
            authority_binding_digest: task.authority_binding_digest()?,
            run_id: task.run_id.as_str().into(),
            node_id: task.node.id.as_str().into(),
            idempotency_key: task.node.idempotency_key.clone(),
            status: result.status,
            summary: result.summary,
            tool_evidence,
            test_evidence,
            artifacts: result.artifacts,
            usage,
            adapter: "hermes-workbench".into(),
            adapter_version: self.config.adapter_version.clone(),
            project_contract_digest: task.project_contract_digest.clone(),
            parent_receipts: task.node.parent_receipts.clone(),
            context_capsule_id: task
                .context_assembly
                .as_ref()
                .map(|assembly| assembly.capsule.capsule_id.clone()),
            context_capsule_digest: task
                .context_assembly
                .as_ref()
                .map(|assembly| assembly.capsule.capsule_digest.clone()),
            context_use_receipt_ref: task
                .context_assembly
                .as_ref()
                .map(|assembly| assembly.use_receipt.receipt_ref()),
            context_handoff,
            recorded_at_unix_ms,
        };
        receipt.receipt_digest = digest_serializable(&receipt)?;
        Ok(receipt)
    }

    fn validate_task(&self, task: &HermesNodeTask) -> Result<Vec<String>, HermesAdapterError> {
        if task.node.state != NodeState::Ready {
            return Err(HermesAdapterError::NodeNotReady(task.node.state));
        }
        if task.node.timeout_ms == 0 {
            return Err(HermesAdapterError::InvalidTask(
                "node timeout must be non-zero".into(),
            ));
        }
        if task.objective.trim().is_empty()
            || task.instructions.trim().is_empty()
            || task.project_contract_digest.trim().is_empty()
        {
            return Err(HermesAdapterError::InvalidTask(
                "objective, instructions, and project contract digest are required".into(),
            ));
        }
        if matches!(
            task.node.authority,
            AuthorityClass::ExecuteWithApproval | AuthorityClass::CompensateWithApproval
        ) && task.node.parent_receipts.is_empty()
        {
            return Err(HermesAdapterError::MissingApprovalReceipt);
        }
        if matches!(task.node.kind, NodeKind::Approval) {
            return Err(HermesAdapterError::HumanApprovalCannotExecute);
        }
        if let Some(assembly) = &task.context_assembly {
            let now = unix_time_ms()?;
            assembly
                .capsule
                .validate(now)
                .map_err(|error| HermesAdapterError::InvalidTask(error.to_string()))?;
            if !assembly
                .use_receipt
                .has_valid_digest()
                .map_err(|error| HermesAdapterError::InvalidTask(error.to_string()))?
                || assembly.use_receipt.capsule_id != assembly.capsule.capsule_id
                || assembly.use_receipt.capsule_digest != assembly.capsule.capsule_digest
                || assembly.use_receipt.consumer_id != assembly.capsule.context.consumer.consumer_id
                || assembly.use_receipt.objective_id
                    != assembly.capsule.context.lineage.objective_id.as_str()
                || assembly.use_receipt.run_id.as_deref() != Some(task.run_id.as_str())
                || assembly.capsule.context.lineage.run_id.as_ref() != Some(&task.run_id)
                || assembly.capsule.context.objective.requested_outcome != task.objective
                || assembly.capsule.context.lineage.parent_receipts != task.node.parent_receipts
            {
                return Err(HermesAdapterError::InvalidTask(
                    "Vairë context assembly does not match the Hermes run node".into(),
                ));
            }
        }
        let toolsets = match task.node.authority {
            AuthorityClass::ReadOnly => &self.config.toolsets.read_only,
            AuthorityClass::HumanApproval => &self.config.toolsets.human_approval,
            AuthorityClass::ExecuteWithApproval => &self.config.toolsets.execute_with_approval,
            AuthorityClass::Verify => &self.config.toolsets.verify,
            AuthorityClass::CompensateWithApproval => {
                &self.config.toolsets.compensate_with_approval
            }
        };
        if toolsets.is_empty() {
            return Err(HermesAdapterError::NoToolsForAuthority(task.node.authority));
        }
        let Some(worker) = &task.node.worker else {
            return Ok(toolsets.clone());
        };
        if !matches!(
            worker.route_class,
            WorkerRouteClass::Local | WorkerRouteClass::Hosted
        ) {
            return Err(HermesAdapterError::InvalidTask(
                "Hermes can execute only local or hosted worker routes".into(),
            ));
        }
        if worker.output_contract != RESULT_SCHEMA_VERSION {
            return Err(HermesAdapterError::InvalidTask(format!(
                "worker output contract must be {RESULT_SCHEMA_VERSION}"
            )));
        }
        if worker.allowed_toolsets.is_empty()
            || !worker
                .allowed_toolsets
                .iter()
                .all(|requested| toolsets.contains(requested))
        {
            return Err(HermesAdapterError::WorkerToolsetEscalation);
        }
        Ok(worker.allowed_toolsets.iter().cloned().collect())
    }

    fn build_prompt(&self, task: &HermesNodeTask) -> Result<String, HermesAdapterError> {
        let context = serde_json::json!({
            "run_id": task.run_id,
            "node": task.node,
            "objective": task.objective,
            "instructions": task.instructions,
            "checks": task.checks,
            "project_contract_digest": task.project_contract_digest,
            "project_root": self.project_root,
            "organism_context_capsule": task.context_assembly.as_ref().map(|assembly| &assembly.capsule),
            "context_use_receipt": task.context_assembly.as_ref().map(|assembly| &assembly.use_receipt),
        });
        Ok(format!(
            "Execute exactly one approved Arda run-graph node. Stay within the supplied project root, authority, instructions, and checks. Arda automatically derives tool and test evidence from Hermes' redacted session export, so leave tool_evidence and test_evidence empty and do not fail merely because opaque tool-call IDs are unavailable. Use status failed only when the governed work or evidence itself fails. Artifact entries are created outputs only: never list files merely read as artifacts, and use an empty artifacts array for read-only inspection or review work. Do not return a Hermes session id, transcript path, recovery token, or other vendor session state. Your final response must be one JSON object with no Markdown fences and exactly this shape: {{\"schema_version\":\"{RESULT_SCHEMA_VERSION}\",\"status\":\"succeeded|failed|cancelled\",\"summary\":\"...\",\"tool_evidence\":[],\"test_evidence\":[],\"artifacts\":[{{\"path\":\"project-relative/path\",\"digest\":\"sha256:<64 lowercase hex>\"}}]}}. Canonical node context follows:\n{}",
            serde_json::to_string(&context)?
        ))
    }

    fn validate_result(
        &self,
        _task: &HermesNodeTask,
        result: &HermesJobResult,
    ) -> Result<(), HermesAdapterError> {
        if result.schema_version != RESULT_SCHEMA_VERSION {
            return Err(HermesAdapterError::InvalidResult(format!(
                "unsupported result schema {}",
                result.schema_version
            )));
        }
        if result.summary.trim().is_empty() {
            return Err(HermesAdapterError::InvalidResult(
                "summary cannot be empty".into(),
            ));
        }
        let mut claimed_call_ids = std::collections::BTreeSet::new();
        for evidence in &result.tool_evidence {
            if evidence.tool_call_id.trim().is_empty()
                || !claimed_call_ids.insert(evidence.tool_call_id.as_str())
            {
                return Err(HermesAdapterError::InvalidResult(
                    "tool evidence requires unique, non-empty tool_call_id values".into(),
                ));
            }
        }
        for evidence in &result.test_evidence {
            if evidence.check_id.trim().is_empty() || evidence.tool_call_id.trim().is_empty() {
                return Err(HermesAdapterError::InvalidResult(
                    "test evidence requires check_id and tool_call_id".into(),
                ));
            }
        }
        for artifact in &result.artifacts {
            let path = Path::new(&artifact.path);
            if path.is_absolute()
                || artifact.path.is_empty()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
                || !is_sha256_digest(&artifact.digest)
            {
                return Err(HermesAdapterError::InvalidResult(
                    "artifact evidence requires a project-relative path and sha256 digest".into(),
                ));
            }
            let canonical = fs::canonicalize(self.project_root.join(path)).map_err(|error| {
                HermesAdapterError::InvalidResult(format!(
                    "artifact {} cannot be resolved: {error}",
                    artifact.path
                ))
            })?;
            if !canonical.starts_with(&self.project_root) || !canonical.is_file() {
                return Err(HermesAdapterError::InvalidResult(format!(
                    "artifact {} escapes the project or is not a file",
                    artifact.path
                )));
            }
            let actual = digest_bytes(&fs::read(&canonical).map_err(|error| {
                HermesAdapterError::InvalidResult(format!(
                    "artifact {} cannot be read: {error}",
                    artifact.path
                ))
            })?);
            if actual != artifact.digest {
                return Err(HermesAdapterError::InvalidResult(format!(
                    "artifact {} digest does not match the project file",
                    artifact.path
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
enum ProcessOutcome {
    Exited(std::process::ExitStatus),
    TimedOut,
    Cancelled,
}

struct BoundedProcessOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

async fn run_bounded(
    mut command: Command,
    process_timeout: Duration,
    cancellation: &AdapterCancellation,
    cancellation_grace_ms: u64,
    output_limit: usize,
) -> Result<BoundedProcessOutput, HermesAdapterError> {
    if *cancellation.subscribe().borrow() {
        return Err(HermesAdapterError::Cancelled);
    }
    let mut child = command.spawn().map_err(|source| HermesAdapterError::Io {
        context: "spawn Hermes command".into(),
        source,
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| HermesAdapterError::InvalidConfig("Hermes stdout was not piped".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| HermesAdapterError::InvalidConfig("Hermes stderr was not piped".into()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout, output_limit));
    let stderr_task = tokio::spawn(read_bounded(stderr, output_limit));
    let mut cancellation_signal = cancellation.subscribe();
    let outcome = tokio::select! {
        status = child.wait() => ProcessOutcome::Exited(status.map_err(|source| HermesAdapterError::Io {
            context: "wait for Hermes".into(),
            source,
        })?),
        _ = tokio::time::sleep(process_timeout) => {
            terminate_and_reap(&mut child, cancellation_grace_ms).await?;
            ProcessOutcome::TimedOut
        }
        changed = cancellation_signal.changed() => {
            if changed.is_ok() && *cancellation_signal.borrow() {
                terminate_and_reap(&mut child, cancellation_grace_ms).await?;
                ProcessOutcome::Cancelled
            } else {
                let status = child.wait().await.map_err(|source| HermesAdapterError::Io {
                    context: "wait for Hermes after cancellation channel closed".into(),
                    source,
                })?;
                ProcessOutcome::Exited(status)
            }
        }
    };
    if matches!(
        outcome,
        ProcessOutcome::TimedOut | ProcessOutcome::Cancelled
    ) {
        stdout_task.abort();
        stderr_task.abort();
        return match outcome {
            ProcessOutcome::TimedOut => Err(HermesAdapterError::Timeout),
            ProcessOutcome::Cancelled => Err(HermesAdapterError::Cancelled),
            ProcessOutcome::Exited(_) => unreachable!(),
        };
    }
    let stdout = stdout_task
        .await
        .map_err(|error| HermesAdapterError::OutputRead(error.to_string()))??;
    let stderr = stderr_task
        .await
        .map_err(|error| HermesAdapterError::OutputRead(error.to_string()))??;
    match outcome {
        ProcessOutcome::Exited(status) if status.success() => {
            Ok(BoundedProcessOutput { stdout, stderr })
        }
        ProcessOutcome::Exited(status) => Err(HermesAdapterError::ProcessFailed {
            code: status.code(),
            stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
        }),
        ProcessOutcome::TimedOut | ProcessOutcome::Cancelled => unreachable!(),
    }
}

fn split_chat_output(
    stdout: &[u8],
    stderr: &[u8],
) -> Result<(String, Vec<u8>), HermesAdapterError> {
    let output = std::str::from_utf8(trim_ascii(stdout))
        .map_err(|error| HermesAdapterError::InvalidResult(error.to_string()))?;
    let mut session_id = None;
    let mut result_lines = Vec::new();
    for line in output.lines() {
        if let Some(candidate) = parse_session_header(line) {
            if session_id.replace(candidate).is_some() {
                return Err(HermesAdapterError::InvalidResult(
                    "Hermes chat emitted multiple session headers".into(),
                ));
            }
        } else {
            result_lines.push(line);
        }
    }
    let stderr = std::str::from_utf8(trim_ascii(stderr))
        .map_err(|error| HermesAdapterError::InvalidResult(error.to_string()))?;
    for line in stderr.lines() {
        if let Some(candidate) = parse_session_header(line) {
            if session_id.replace(candidate).is_some() {
                return Err(HermesAdapterError::InvalidResult(
                    "Hermes chat emitted multiple session headers".into(),
                ));
            }
        }
    }
    let session_id = session_id.ok_or_else(|| {
        HermesAdapterError::InvalidResult("Hermes chat omitted its session header".into())
    })?;
    let result = result_lines.join("\n");
    if trim_ascii(result.as_bytes()).is_empty() {
        return Err(HermesAdapterError::InvalidResult(
            "Hermes chat omitted its result payload".into(),
        ));
    }
    Ok((session_id, trim_ascii(result.as_bytes()).to_vec()))
}

fn parse_session_header(line: &str) -> Option<String> {
    let trimmed = line.trim();
    let lower = trimmed.to_ascii_lowercase();
    let prefix_len = ["session_id:", "session id:", "session:"]
        .iter()
        .find_map(|prefix| lower.starts_with(prefix).then_some(prefix.len()))?;
    let value = trimmed[prefix_len..].trim();
    (!value.is_empty()
        && value.chars().all(|character| {
            character == '_' || character == '-' || character.is_ascii_alphanumeric()
        }))
    .then(|| value.to_owned())
}

fn parse_job_result(bytes: &[u8]) -> Result<HermesJobResult, HermesAdapterError> {
    if let Some(result) = parse_job_result_candidate(trim_ascii(bytes)) {
        return Ok(result);
    }
    // Some Hermes toolsets can still emit compact progress notices in quiet
    // mode. Accept only one terminal JSON object after that prefix.
    let trimmed = trim_ascii(bytes);
    let end = trimmed
        .iter()
        .rposition(|byte| *byte == b'}')
        .map(|index| index + 1)
        .ok_or_else(|| HermesAdapterError::InvalidResult("Hermes result was not JSON".into()))?;
    for start in trimmed[..end]
        .iter()
        .enumerate()
        .filter_map(|(index, byte)| (*byte == b'{').then_some(index))
        .rev()
    {
        if let Some(result) = parse_job_result_candidate(&trimmed[start..end]) {
            return Ok(result);
        }
    }
    Err(HermesAdapterError::InvalidResult(
        "Hermes result did not contain one terminal job-result object".into(),
    ))
}

fn parse_job_result_candidate(bytes: &[u8]) -> Option<HermesJobResult> {
    let mut value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    if let Some(entries) = value
        .get_mut("tool_evidence")
        .and_then(serde_json::Value::as_array_mut)
    {
        for entry in entries {
            if let Some(call_id) = entry.as_str().map(str::to_owned) {
                *entry = serde_json::json!({ "tool_call_id": call_id });
            }
        }
    }
    serde_json::from_value(value).ok()
}

fn translate_actual_evidence(
    task: &HermesNodeTask,
    result: &HermesJobResult,
    session: &HermesSessionExport,
    cwd: &Path,
) -> Result<(Vec<HermesToolEvidence>, Vec<HermesTestEvidence>), HermesAdapterError> {
    let mut calls = BTreeMap::<String, (String, String)>::new();
    let mut call_order = Vec::new();
    let mut outputs = BTreeMap::<String, (Option<String>, Vec<u8>, Option<i32>)>::new();
    for message in &session.messages {
        for call in message.tool_calls.iter().flatten() {
            call_order.push(call.id.clone());
            if calls
                .insert(
                    call.id.clone(),
                    (
                        call.function.name.clone(),
                        normalize_action(&call.function.arguments)?,
                    ),
                )
                .is_some()
            {
                return Err(HermesAdapterError::InvalidUsage(format!(
                    "duplicate tool call id {} in Hermes export",
                    call.id
                )));
            }
        }
        if message.role == "tool" {
            let call_id = message.tool_call_id.as_ref().ok_or_else(|| {
                HermesAdapterError::InvalidUsage("tool result omitted tool_call_id".into())
            })?;
            let content = canonical_tool_content(message.content.as_ref())?;
            let exit_code = parse_exit_code(&content);
            if outputs
                .insert(
                    call_id.clone(),
                    (message.tool_name.clone(), content, exit_code),
                )
                .is_some()
            {
                return Err(HermesAdapterError::InvalidUsage(format!(
                    "duplicate tool result for {call_id}"
                )));
            }
        }
    }

    // The provider-generated call IDs are not visible to models on every
    // Hermes backend. Build receipt evidence from the exported session itself;
    // the model's claims select check IDs but never manufacture evidence.
    let material_call_ids: Vec<_> = call_order
        .iter()
        .filter(|call_id| {
            calls.get(*call_id).is_some_and(|(tool, _)| {
                matches!(task.node.kind, NodeKind::Inspect | NodeKind::Review)
                    || !matches!(
                        tool.as_str(),
                        "read_file" | "search_files" | "browser_snapshot" | "browser_vision"
                    )
            })
        })
        .cloned()
        .collect();
    let resolve = |call_id: &str| -> Result<HermesToolEvidence, HermesAdapterError> {
        let (tool, action) = calls.get(call_id).ok_or_else(|| {
            HermesAdapterError::InvalidResult(format!(
                "tool call {call_id} is absent from Hermes export"
            ))
        })?;
        let (reported_tool, content, exit_code) = outputs.get(call_id).ok_or_else(|| {
            HermesAdapterError::InvalidResult(format!("tool call {call_id} has no actual result"))
        })?;
        if reported_tool.as_deref().is_some_and(|name| name != tool) {
            return Err(HermesAdapterError::InvalidUsage(format!(
                "tool name mismatch for {call_id}"
            )));
        }
        Ok(HermesToolEvidence {
            tool: tool.clone(),
            action: action.clone(),
            exit_code: *exit_code,
            output_digest: digest_bytes(content),
        })
    };

    if material_call_ids.is_empty() {
        return Err(HermesAdapterError::InvalidResult(
            "Hermes export contained no material tool evidence".into(),
        ));
    }
    let tool_evidence = material_call_ids
        .iter()
        .map(|call_id| resolve(call_id))
        .collect::<Result<Vec<_>, _>>()?;
    let terminal_call_ids: Vec<_> = call_order
        .iter()
        .filter(|call_id| {
            calls
                .get(*call_id)
                .is_some_and(|(tool, _)| tool == "terminal")
                && outputs
                    .get(*call_id)
                    .is_some_and(|(_, _, exit_code)| exit_code.is_some())
        })
        .collect();

    let expected_checks: std::collections::BTreeSet<_> = task.checks.iter().cloned().collect();
    if expected_checks.len() != task.checks.len() {
        return Err(HermesAdapterError::InvalidTask(
            "declared check ids must be unique".into(),
        ));
    }
    let mut actual_checks = std::collections::BTreeSet::new();
    let claimed_checks: Vec<_> = if result.test_evidence.is_empty() {
        task.checks
            .iter()
            .enumerate()
            .map(|(index, check_id)| {
                let call_id = terminal_call_ids.get(index).copied().ok_or_else(|| {
                    HermesAdapterError::InvalidResult(format!(
                        "check {check_id} has no actual terminal result in Hermes export"
                    ))
                })?;
                Ok((check_id.clone(), (*call_id).clone()))
            })
            .collect::<Result<Vec<_>, HermesAdapterError>>()?
    } else {
        result
            .test_evidence
            .iter()
            .map(|claim| (claim.check_id.clone(), claim.tool_call_id.clone()))
            .collect()
    };
    let mut test_evidence = Vec::with_capacity(claimed_checks.len());
    for (index, (check_id, claimed_call_id)) in claimed_checks.iter().enumerate() {
        if !actual_checks.insert(check_id.clone()) {
            return Err(HermesAdapterError::InvalidResult(format!(
                "duplicate test evidence for {}",
                check_id
            )));
        }
        let call_id = if calls
            .get(claimed_call_id)
            .is_some_and(|(tool, _)| tool == "terminal")
        {
            claimed_call_id
        } else {
            terminal_call_ids.get(index).copied().ok_or_else(|| {
                HermesAdapterError::InvalidResult(format!(
                    "check {} has no actual terminal result in Hermes export",
                    check_id
                ))
            })?
        };
        let evidence = resolve(call_id)?;
        if evidence.tool != "terminal" || evidence.exit_code.is_none() {
            return Err(HermesAdapterError::InvalidResult(format!(
                "check {} does not reference an actual terminal result with an exit code",
                check_id
            )));
        }
        let exit_code = evidence.exit_code.expect("checked above");
        if let Some(expected_command) = task.check_commands.get(check_id) {
            if !matches_declared_check_command(&evidence.action, expected_command, cwd) {
                return Err(HermesAdapterError::InvalidResult(format!(
                    "check {} referenced terminal command `{}`, expected `{}`",
                    check_id, evidence.action, expected_command
                )));
            }
        }
        test_evidence.push(HermesTestEvidence {
            check_id: check_id.clone(),
            command: evidence.action,
            status: if exit_code == 0 { "passed" } else { "failed" }.into(),
            exit_code,
            output_digest: evidence.output_digest,
        });
    }
    if actual_checks != expected_checks {
        return Err(HermesAdapterError::InvalidResult(
            "test evidence must cover every declared check exactly once".into(),
        ));
    }
    if result.status == HermesReceiptStatus::Succeeded
        && test_evidence
            .iter()
            .any(|evidence| evidence.status == "failed")
    {
        return Err(HermesAdapterError::InvalidResult(
            "a successful result cannot contain failed test evidence".into(),
        ));
    }
    Ok((tool_evidence, test_evidence))
}

fn matches_declared_check_command(actual: &str, expected: &str, cwd: &Path) -> bool {
    let actual = actual.trim();
    let expected = expected.trim();
    if actual == expected {
        return true;
    }
    let cwd = cwd.display().to_string();
    let quoted_cwd = format!("'{}'", cwd.replace('\'', "'\\''"));
    actual == format!("cd {cwd} && {expected}")
        || actual == format!("cd {quoted_cwd} && {expected}")
}

fn normalize_action(arguments: &serde_json::Value) -> Result<String, HermesAdapterError> {
    let normalized = match arguments {
        serde_json::Value::String(raw) => {
            serde_json::from_str(raw).unwrap_or_else(|_| serde_json::Value::String(raw.clone()))
        }
        value => value.clone(),
    };
    if let Some(command) = normalized
        .get("command")
        .and_then(serde_json::Value::as_str)
    {
        return Ok(command.into());
    }
    Ok(serde_json::to_string(&normalized)?)
}

fn canonical_tool_content(
    content: Option<&serde_json::Value>,
) -> Result<Vec<u8>, HermesAdapterError> {
    match content {
        Some(serde_json::Value::String(value)) => Ok(value.as_bytes().to_vec()),
        Some(value) => Ok(serde_json::to_vec(value)?),
        None => Ok(Vec::new()),
    }
}

fn parse_exit_code(content: &[u8]) -> Option<i32> {
    serde_json::from_slice::<serde_json::Value>(content)
        .ok()?
        .get("exit_code")?
        .as_i64()
        .and_then(|value| i32::try_from(value).ok())
}

async fn read_bounded<R: AsyncRead + Unpin>(
    reader: R,
    limit: usize,
) -> Result<Vec<u8>, HermesAdapterError> {
    let mut bytes = Vec::new();
    reader
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|source| HermesAdapterError::Io {
            context: "read Hermes process output".into(),
            source,
        })?;
    if bytes.len() > limit {
        return Err(HermesAdapterError::OutputTooLarge { limit });
    }
    Ok(bytes)
}

async fn terminate_and_reap(child: &mut Child, grace_ms: u64) -> Result<(), HermesAdapterError> {
    #[cfg(unix)]
    if let Some(pid) = child.id() {
        // Adapter children are process-group leaders. Terminating the group
        // keeps a tool subprocess from outliving its bounded graph-node job.
        // SAFETY: `pid` comes from the live child we spawned as process-group
        // leader, and `kill` does not dereference process memory.
        let result = unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        if result != 0 {
            let source = std::io::Error::last_os_error();
            if source.raw_os_error() != Some(libc::ESRCH) {
                return Err(HermesAdapterError::Io {
                    context: "terminate Hermes process group".into(),
                    source,
                });
            }
        }
    }
    #[cfg(not(unix))]
    child
        .start_kill()
        .map_err(|source| HermesAdapterError::Io {
            context: "terminate Hermes".into(),
            source,
        })?;
    timeout(Duration::from_millis(grace_ms), child.wait())
        .await
        .map_err(|_| HermesAdapterError::ReapTimeout)?
        .map_err(|source| HermesAdapterError::Io {
            context: "reap Hermes".into(),
            source,
        })?;
    Ok(())
}

fn configure_process_group(command: &mut Command) {
    #[cfg(unix)]
    command.as_std_mut().process_group(0);
}

fn canonical_directory(path: &Path, label: &str) -> Result<PathBuf, HermesAdapterError> {
    let canonical = fs::canonicalize(path).map_err(|source| HermesAdapterError::Io {
        context: format!("canonicalize {label} {}", path.display()),
        source,
    })?;
    if !canonical.is_dir() {
        return Err(HermesAdapterError::InvalidConfig(format!(
            "{label} is not a directory: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn resolve_executable(
    configured: &str,
    host_environment: &BTreeMap<String, String>,
) -> Result<PathBuf, HermesAdapterError> {
    let configured_path = Path::new(configured);
    let candidate = if configured_path.is_absolute() {
        configured_path.to_owned()
    } else {
        if configured_path.components().count() != 1 {
            return Err(HermesAdapterError::InvalidExecutable(configured.into()));
        }
        let path = host_environment
            .get("PATH")
            .ok_or(HermesAdapterError::MissingPath)?;
        std::env::split_paths(path)
            .map(|directory| directory.join(configured_path))
            .find(|candidate| candidate.is_file())
            .ok_or_else(|| HermesAdapterError::ExecutableNotFound(configured.into()))?
    };
    let executable = fs::canonicalize(&candidate).map_err(|source| HermesAdapterError::Io {
        context: format!("canonicalize executable {}", candidate.display()),
        source,
    })?;
    if !executable.is_file() {
        return Err(HermesAdapterError::InvalidExecutable(
            executable.display().to_string(),
        ));
    }
    Ok(executable)
}

fn unix_time_ms() -> Result<u128, HermesAdapterError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| HermesAdapterError::Clock(error.to_string()))
}

fn digest_serializable(value: &impl Serialize) -> Result<String, HermesAdapterError> {
    let bytes = serde_json::to_vec(value)?;
    Ok(digest_bytes(&bytes))
}

fn digest_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

fn is_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|hex| {
        hex.len() == 64
            && hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    })
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

#[derive(Debug, Error)]
pub enum HermesAdapterError {
    #[error("unsupported Hermes adapter config schema: {0}")]
    UnsupportedConfigSchema(String),
    #[error("invalid Hermes adapter config: {0}")]
    InvalidConfig(String),
    #[error("invalid inherited environment key: {0}")]
    InvalidEnvironmentKey(String),
    #[error("PATH is required to resolve a non-absolute Hermes executable")]
    MissingPath,
    #[error("Hermes executable was not found on PATH: {0}")]
    ExecutableNotFound(String),
    #[error("invalid Hermes executable: {0}")]
    InvalidExecutable(String),
    #[error("working directory {cwd} escapes project root {project_root}")]
    WorkingDirectoryEscape { cwd: PathBuf, project_root: PathBuf },
    #[error("run node is not ready: {0:?}")]
    NodeNotReady(NodeState),
    #[error("execute/compensate authority requires an approval receipt")]
    MissingApprovalReceipt,
    #[error("human approval nodes cannot be delegated to Hermes")]
    HumanApprovalCannotExecute,
    #[error("no Hermes toolsets configured for authority {0:?}")]
    NoToolsForAuthority(AuthorityClass),
    #[error("worker toolsets exceed the toolsets authorized for this node")]
    WorkerToolsetEscalation,
    #[error("worker deadline elapsed before Hermes execution")]
    DeadlineExceeded,
    #[error("invalid Hermes task: {0}")]
    InvalidTask(String),
    #[error("Hermes prompt is {actual} bytes, above configured limit {limit}")]
    PromptTooLarge { actual: usize, limit: usize },
    #[error("Hermes process output exceeded configured limit {limit}")]
    OutputTooLarge { limit: usize },
    #[error("Hermes process timed out")]
    Timeout,
    #[error("Hermes process was cancelled")]
    Cancelled,
    #[error("Hermes process could not be reaped within the cancellation grace period")]
    ReapTimeout,
    #[error("Hermes exited unsuccessfully with code {code:?}: {stderr}")]
    ProcessFailed { code: Option<i32>, stderr: String },
    #[error("invalid Hermes job result: {0}")]
    InvalidResult(String),
    #[error("invalid Hermes usage evidence: {0}")]
    InvalidUsage(String),
    #[error("Hermes cost {actual} exceeded node budget {limit}")]
    CostBudgetExceeded { actual: f64, limit: f64 },
    #[error("failed to read Hermes output: {0}")]
    OutputRead(String),
    #[error("system clock error: {0}")]
    Clock(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    Toml(#[from] toml::de::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
