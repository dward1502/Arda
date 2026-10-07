//! Explicit HTTP execution and receipt reconciliation, independent of queue scheduling.
use crate::compatibility::*;
use crate::execution_contract::*;
use anyhow::{anyhow, bail, Context, Result};
use arda_vaire::{ContextAssembly, MnemosyneService};
use chrono::{DateTime, Utc};
use futures::future::BoxFuture;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
mod provider_transport;
#[cfg(test)]
mod runtime_admission;
mod workspace_authority;
impl ExplicitWorkbenchWorkItem {
    pub fn completed_outcome(&self, root: &Path, run: &Value) -> Result<ExplicitExecutionOutcome> {
        workspace_authority::run(self, run)?;
        anyhow::ensure!(
            ["execute", "verify", "review", "close"]
                .iter()
                .all(|stage| node_state(run, stage) == Some("succeeded")),
            "retained run is not complete"
        );
        explicit_outcome_from_run(root, self, run)
    }

    /// Deterministic provider-free Close, shared by the executor and Harness.
    pub fn close_receipt(&self, parent: &str) -> Result<Value> {
        Ok(serde_json::to_value(canonical_explicit_close_receipt(
            self, parent,
        )?)?)
    }

    pub fn close_request_body(&self, parent: &str) -> Result<Value> {
        let receipt = canonical_explicit_close_receipt(self, parent)?;
        Ok(json!({
            "envelope": explicit_stage_envelope(self, "close")?,
            "receipt_digest": receipt.receipt_digest,
        }))
    }

    /// Call only while the caller holds the current objective mutation fence.
    pub fn persist_close_receipt(
        &self,
        root: &Path,
        parent: &str,
        owner: &dyn ExplicitRuntimeAdmission,
    ) -> Result<()> {
        let _runtime = owner.admit(root, self)?;
        let receipt = canonical_explicit_close_receipt(self, parent)?;
        persist_explicit_close_receipt(root, self, &receipt)
    }

    /// Recompute a stage binding without recalling memory or granting authority.
    pub fn stage_context(
        &self,
        root: &Path,
        stage: &str,
        parents: Vec<String>,
    ) -> Result<Option<ContextAssembly>> {
        let Some(original) = &self.context_assembly else {
            return Ok(None);
        };
        let purpose = match stage {
            "execute" => self.execution_prompt.clone(),
            "verify" => self.verification_prompt.clone(),
            "review" => review_prompt_with_dependency_receipts(root, self)?,
            "close" => return Ok(Some(original.clone())),
            _ => bail!("unsupported explicit Workbench context stage"),
        };
        // Vairë and the provider adapter share the bounded purpose projection.
        // The full stage payload remains separately retained in this work item.
        Ok(Some(original.for_run_stage(stage, &purpose, parents)?))
    }

    /// Deterministic payload construction only; this does not authorize dispatch.
    pub fn provider_request_body(
        &self,
        root: &Path,
        stage: &str,
        context: Option<ContextAssembly>,
    ) -> Result<Value> {
        let prompt = match stage {
            "execute" => self.execution_prompt.clone(),
            "verify" => self.verification_prompt.clone(),
            "review" => review_prompt_with_dependency_receipts(root, self)?,
            _ => bail!("unsupported explicit provider stage {stage}"),
        };
        Ok(json!({
            "objective": prompt,
            "envelope": explicit_stage_envelope(self, stage)?,
            "context_assembly": context,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct WorkbenchExecutionAdapter {
    root: PathBuf,
    harness_url: String,
    client: reqwest::Client,
}

impl ExplicitWorkbenchExecution for WorkbenchExecutionAdapter {
    fn inspect_retry_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<Option<ExplicitExecutionOutcome>>> {
        Box::pin(async move { self.reconcile_for_retry(item, owner).await })
    }
    fn execute_explicit_authorized<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
        authority: &'a dyn ExplicitWorkspaceAuthorization,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
        Box::pin(async move { self.execute_authorized(item, authority, owner).await })
    }
    fn reconcile_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<Option<ExplicitExecutionOutcome>>> {
        Box::pin(async move { self.reconcile(item, owner).await })
    }

    fn execute_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
        Box::pin(async move { self.execute(item, owner).await })
    }
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

    /// Inspect existing evidence before a separately authorized, budgeted retry.
    pub async fn reconcile_for_retry(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        owner: &dyn ExplicitRuntimeAdmission,
    ) -> Result<Option<ExplicitExecutionOutcome>> {
        let _runtime = owner.admit(&self.root, item)?;
        self.reconcile_internal(item, true).await
    }

    /// Read an already completed run. No planning, approval, binding or provider POST.
    pub async fn reconcile(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        owner: &dyn ExplicitRuntimeAdmission,
    ) -> Result<Option<ExplicitExecutionOutcome>> {
        let _runtime = owner.admit(&self.root, item)?;
        self.reconcile_internal(item, false).await
    }

    async fn reconcile_internal(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        for_retry: bool,
    ) -> Result<Option<ExplicitExecutionOutcome>> {
        validate_explicit_work_item_mode(
            &self.root,
            item,
            workspace_authority::Validation::Reconcile,
        )?;
        let response = self
            .client
            .get(format!("{}/v1/runs/{}", self.harness_url, item.run_id))
            .send()
            .await
            .context("inspect completed explicit Workbench run")?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let run = response_error(response, "inspect completed explicit Workbench run").await?;
        workspace_authority::run(item, &run)?;
        let nodes = run
            .pointer("/graph/nodes")
            .and_then(Value::as_array)
            .ok_or_else(|| anyhow!("malformed canonical run graph nodes"))?;
        for stage in ["execute", "verify", "review", "close"] {
            let mut matches = nodes
                .iter()
                .filter(|node| node.get("id").and_then(Value::as_str) == Some(stage));
            let node = matches
                .next()
                .ok_or_else(|| anyhow!("canonical run missing stage {stage}"))?;
            if matches.next().is_some() {
                return Err(anyhow!("canonical run duplicates stage {stage}"));
            }
            let state = node
                .get("state")
                .cloned()
                .ok_or_else(|| anyhow!("canonical run missing state for {stage}"))?;
            serde_json::from_value::<arda_core::run_graph::NodeState>(state)
                .with_context(|| format!("malformed canonical run state for {stage}"))?;
        }
        if !for_retry
            && ["execute", "verify", "review", "close"]
                .iter()
                .any(|stage| node_state(&run, stage) == Some("running"))
        {
            return Err(anyhow!(
                "canonical run is still running; defer receipt reconciliation"
            ));
        }
        if ["execute", "verify", "review", "close"]
            .iter()
            .any(|stage| node_state(&run, stage) != Some("succeeded"))
        {
            return Ok(None);
        }
        explicit_outcome_from_run(&self.root, item, &run).map(Some)
    }

    pub async fn execute(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        owner: &dyn ExplicitRuntimeAdmission,
    ) -> Result<ExplicitExecutionOutcome> {
        let _runtime = owner.admit(&self.root, item)?;
        self.execute_internal(item, None).await
    }

    /// Process-local callers must revalidate retained authority for this exact item.
    pub async fn execute_authorized(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        authority: &dyn ExplicitWorkspaceAuthorization,
        owner: &dyn ExplicitRuntimeAdmission,
    ) -> Result<ExplicitExecutionOutcome> {
        let _runtime = owner.admit(&self.root, item)?;
        self.execute_internal(item, Some(authority)).await
    }

    async fn execute_internal(
        &self,
        item: &ExplicitWorkbenchWorkItem,
        authorization: Option<&dyn ExplicitWorkspaceAuthorization>,
    ) -> Result<ExplicitExecutionOutcome> {
        validate_explicit_work_item_mode(
            &self.root,
            item,
            if authorization.is_some() {
                workspace_authority::Validation::Retained
            } else {
                workspace_authority::Validation::Fresh
            },
        )?;
        if let Some(authority) = authorization {
            authority.authorize(item)?;
        }
        let recovery = authorization
            .map(|authority| authority.recovery_window(item))
            .transpose()?
            .flatten();
        if let Some(window) = &recovery {
            window.validate()?;
            anyhow::ensure!(item.read_only, "recovery requires read-only scope");
            anyhow::ensure!(
                item.context_assembly.is_some(),
                "recovery requires retained context"
            );
        }
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
            anyhow::ensure!(recovery.is_none(), "recovery cannot plan a missing run");
            workspace_authority::reauthorize(authorization, item)?;
            let graph = explicit_run_graph(item, approval_id);
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
        workspace_authority::run(item, &run)?;
        if node_state(&run, "approval") != Some("succeeded") {
            anyhow::ensure!(
                recovery.is_none(),
                "recovery cannot approve an unfinished run"
            );
            workspace_authority::reauthorize(authorization, item)?;
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
            workspace_authority::run(item, &run)?;
        }

        if recovery.is_some() {
            anyhow::ensure!(
                node_state(&run, "execute") == Some("succeeded"),
                "recovery requires preserved successful execution"
            );
        }
        for stage in ["execute", "verify"] {
            if node_state(&run, stage) == Some("succeeded") {
                continue;
            }
            workspace_authority::reauthorize(authorization, item)?;
            let context =
                bind_explicit_stage_context(&self.root, item, &run, stage, recovery.as_ref())?;
            let provider_body = workspace_authority::provider_body(
                authorization,
                item,
                item.provider_request_body(&self.root, stage, context)?,
            )?;
            let response = provider_transport::send(
                &self.client,
                format!(
                    "{}/v1/runs/{}/nodes/{stage}/execute-provider",
                    self.harness_url, item.run_id
                ),
                &provider_body,
                &run,
                stage,
            )
            .await?;
            if response.status() == reqwest::StatusCode::CONFLICT {
                require_scheduler_admission_conflict(response, stage).await?;
                run = wait_for_explicit_stage(&self.client, &self.harness_url, &item.run_id, stage)
                    .await?;
                workspace_authority::run(item, &run)?;
                continue;
            }
            let value = response_error(
                response,
                &format!("execute explicit Workbench {stage} stage"),
            )
            .await?;
            if value["receipt"]["status"] != "succeeded" {
                workspace_authority::run(item, &value["run"])?;
                return explicit_failed_outcome(&self.root, item, &run, &value);
            }
            run = value["run"].clone();
            workspace_authority::run(item, &run)?;
        }

        if node_state(&run, "review") != Some("succeeded") {
            workspace_authority::reauthorize(authorization, item)?;
            let context =
                bind_explicit_stage_context(&self.root, item, &run, "review", recovery.as_ref())?;
            let provider_body = workspace_authority::provider_body(
                authorization,
                item,
                item.provider_request_body(&self.root, "review", context)?,
            )?;
            let response = provider_transport::send(
                &self.client,
                format!(
                    "{}/v1/runs/{}/nodes/review/execute-provider",
                    self.harness_url, item.run_id
                ),
                &provider_body,
                &run,
                "review",
            )
            .await?;
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
                    workspace_authority::run(item, &value["run"])?;
                    return explicit_failed_outcome(&self.root, item, &run, &value);
                }
                run = value["run"].clone();
            }
        }

        workspace_authority::run(item, &run)?;
        if node_state(&run, "close") != Some("succeeded") {
            workspace_authority::reauthorize(authorization, item)?;
            let parent = node_output_digest(&run, "review")
                .ok_or_else(|| anyhow!("explicit Workbench close omitted review receipt"))?;
            let close_receipt = canonical_explicit_close_receipt(item, parent)?;
            // Persist deterministic evidence before making the durable run
            // terminal. A crash may leave an unreferenced file, but cannot
            // leave a succeeded run whose canonical close receipt is absent.
            persist_explicit_close_receipt(&self.root, item, &close_receipt)?;
            workspace_authority::reauthorize(authorization, item)?;
            let mut close_body = json!({
                "envelope": explicit_stage_envelope(item, "close")?,
                "receipt_digest": close_receipt.receipt_digest,
            });
            if recovery.is_some() {
                close_body = workspace_authority::provider_body(authorization, item, close_body)?;
            }
            let response = self
                .client
                .post(format!(
                    "{}/v1/runs/{}/nodes/close/complete",
                    self.harness_url, item.run_id
                ))
                .json(&close_body)
                .send()
                .await
                .context("complete explicit Workbench close stage")?;
            run = response_error(response, "complete explicit Workbench close stage").await?;
            workspace_authority::run(item, &run)?;
            if node_output_digest(&run, "close") != Some(close_receipt.receipt_digest.as_str()) {
                return Err(anyhow!(
                    "explicit Workbench close node did not retain its canonical receipt digest"
                ));
            }
        }
        explicit_outcome_from_run(&self.root, item, &run)
    }
}

fn bind_explicit_stage_context(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    run: &Value,
    stage: &str,
    recovery: Option<&ExplicitRecoveryWindow>,
) -> Result<Option<ContextAssembly>> {
    let Some(original) = &item.context_assembly else {
        return Ok(None);
    };
    let parents: Vec<String> = serde_json::from_value(
        run["graph"]["nodes"]
            .as_array()
            .and_then(|nodes| nodes.iter().find(|node| node["id"] == stage))
            .ok_or_else(|| anyhow!("explicit run omitted {stage} node"))?["parent_receipts"]
            .clone(),
    )?;
    let expected = item
        .stage_context(root, stage, parents.clone())?
        .context("stage context missing")?;
    let memory = MnemosyneService::new(root.join("data/vaire"))?
        .with_contract_memory_root(root.join("core/state/memory"));
    if let Some(window) = recovery {
        window.validate()?;
        return Ok(Some(memory.bind_run_stage_context_for_recovery(
            original,
            stage,
            &expected.use_receipt.purpose,
            parents,
            Utc::now().timestamp_millis().max(0) as u128,
            u128::from(window.activated_at_unix_ms)..u128::from(window.expires_at_unix_ms),
        )?));
    }
    Ok(Some(memory.bind_run_stage_context(
        original,
        stage,
        &expected.use_receipt.purpose,
        parents,
        Utc::now().timestamp_millis().max(0) as u128,
    )?))
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

#[cfg(test)]
fn validate_explicit_work_item(root: &Path, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
    validate_explicit_work_item_mode(root, item, workspace_authority::Validation::Fresh)
}

fn validate_explicit_work_item_mode(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
    mode: workspace_authority::Validation,
) -> Result<()> {
    if !workspace_authority::safe_run_id(&item.run_id) {
        bail!("unsafe explicit run id");
    }
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
    workspace_authority::bound_workspace(root, item, mode)?;
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

fn explicit_run_graph(item: &ExplicitWorkbenchWorkItem, approval_id: &str) -> Value {
    let mut graph = run_graph_value(
        &item.run_id,
        &item.leaf_id,
        &item.objective,
        approval_id,
        Some(&item.objective_plan_receipt),
        None,
        &item.project_contract_digest,
    );
    if item.read_only {
        for node in graph["nodes"].as_array_mut().expect("generated nodes") {
            match node["id"].as_str() {
                Some("execute") => {
                    node["kind"] = json!("inspect");
                    node["authority"] = json!("read_only");
                    node["worker"]["role"] = json!("local_summary_classification");
                    node["worker"]["allowed_toolsets"] = json!(["file"]);
                }
                Some("verify") => node["worker"]["allowed_toolsets"] = json!(["file"]),
                _ => {}
            }
        }
    }
    graph
}

#[cfg(test)]
mod tests;
