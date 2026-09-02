use super::model::{ClaimedLeaf, ReceiptStage, StageReceipt};
use super::runtime::{LeafExecution, LeafExecutionResult};
use crate::adapters::{HermesExecutionReceipt, HermesReceiptStatus};
use anyhow::{anyhow, bail, Context, Result};
use arda_aule::prometheus::autopilot::{
    ExplicitExecutionOutcome, ExplicitReceiptReference, ExplicitWorkbenchWorkItem,
    WorkbenchExecutionAdapter,
};
use futures::future::BoxFuture;
use std::path::{Path, PathBuf};

pub trait ExplicitWorkbenchExecution: Send + Sync {
    fn execute_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>>;
}

impl ExplicitWorkbenchExecution for WorkbenchExecutionAdapter {
    fn execute_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
        Box::pin(async move { self.execute(item).await })
    }
}

#[derive(Clone, Debug)]
pub struct WorkbenchLeafExecution<E = WorkbenchExecutionAdapter> {
    root: PathBuf,
    adapter: E,
}

impl WorkbenchLeafExecution<WorkbenchExecutionAdapter> {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let adapter = WorkbenchExecutionAdapter::new(&root)?;
        Ok(Self { root, adapter })
    }
}

impl<E> WorkbenchLeafExecution<E> {
    pub fn with_adapter(root: impl AsRef<Path>, adapter: E) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
            adapter,
        }
    }
}

impl<E> LeafExecution for WorkbenchLeafExecution<E>
where
    E: ExplicitWorkbenchExecution + Clone + 'static,
{
    fn execute(&self, claim: ClaimedLeaf) -> BoxFuture<'static, Result<LeafExecutionResult>> {
        let root = self.root.clone();
        let adapter = self.adapter.clone();
        Box::pin(async move {
            let execution = claim.execution.as_ref().ok_or_else(|| {
                anyhow!("claimed leaf `{}` omitted execution payload", claim.leaf_id)
            })?;
            let project_id = claim.project_id.as_deref().ok_or_else(|| {
                anyhow!("claimed leaf `{}` omitted exact project id", claim.leaf_id)
            })?;
            let project_contract_digest =
                claim.project_contract_digest.as_deref().ok_or_else(|| {
                    anyhow!(
                        "claimed leaf `{}` omitted exact project contract digest",
                        claim.leaf_id
                    )
                })?;
            let run_id = format!(
                "objective-{}-leaf-{}-attempt-{}",
                claim.objective_id, claim.leaf_id, claim.attempt
            );
            let item = ExplicitWorkbenchWorkItem {
                objective_id: claim.objective_id.clone(),
                leaf_id: claim.leaf_id.clone(),
                run_id: run_id.clone(),
                objective: execution.objective.clone(),
                execution_prompt: execution.execution_prompt.clone(),
                verification_prompt: execution.verification_prompt.clone(),
                review_prompt: execution.review_prompt.clone(),
                project_id: project_id.to_owned(),
                project_contract_digest: project_contract_digest.to_owned(),
                workspace_root: PathBuf::from(&claim.workspace_root),
                approval_envelope: execution.approval_envelope.clone(),
                objective_plan_receipt: execution.objective_plan_receipt.clone(),
                dependency_receipts: claim
                    .dependency_receipts
                    .iter()
                    .map(|receipt| ExplicitReceiptReference {
                        stage: receipt.stage.as_str().to_owned(),
                        digest: receipt.digest.clone(),
                        path: receipt.run_path.clone(),
                    })
                    .collect(),
            };
            let outcome = adapter.execute_explicit(&item).await?;
            if outcome.run_id != run_id {
                bail!(
                    "explicit Workbench outcome run `{}` did not match claimed run `{run_id}`",
                    outcome.run_id
                );
            }
            if outcome.status != "succeeded" {
                bail!(
                    "explicit Workbench run `{run_id}` ended with status `{}`",
                    outcome.status
                );
            }
            let receipts = project_receipts(&root, &run_id, project_contract_digest, &outcome)?;
            Ok(LeafExecutionResult { receipts })
        })
    }
}

fn project_receipts(
    root: &Path,
    run_id: &str,
    project_contract_digest: &str,
    outcome: &ExplicitExecutionOutcome,
) -> Result<Vec<StageReceipt>> {
    let expected_stages = [
        ("execute", ReceiptStage::Execute),
        ("verify", ReceiptStage::Verify),
        ("review", ReceiptStage::Review),
        ("close", ReceiptStage::Close),
    ];
    if outcome.receipts.len() != expected_stages.len() {
        bail!(
            "explicit Workbench run `{run_id}` returned {} receipts instead of four",
            outcome.receipts.len()
        );
    }
    let canonical_root = root.canonicalize().context("canonicalize Arda root")?;
    let mut predecessor = None;
    let mut projected = Vec::with_capacity(expected_stages.len());
    for (reference, (expected_name, stage)) in outcome.receipts.iter().zip(expected_stages) {
        if reference.stage != expected_name {
            bail!(
                "explicit Workbench run `{run_id}` returned `{}` receipt where `{expected_name}` was required",
                reference.stage
            );
        }
        let supplied_path = PathBuf::from(&reference.path);
        let path = if supplied_path.is_absolute() {
            supplied_path
        } else {
            root.join(supplied_path)
        };
        let canonical_path = path.canonicalize().with_context(|| {
            format!(
                "resolve explicit Workbench {expected_name} receipt `{}`",
                path.display()
            )
        })?;
        if !canonical_path.starts_with(&canonical_root) {
            bail!("explicit Workbench receipt path escaped Arda root");
        }
        let receipt: HermesExecutionReceipt =
            serde_json::from_slice(&std::fs::read(&canonical_path).with_context(|| {
                format!(
                    "read explicit Workbench receipt `{}`",
                    canonical_path.display()
                )
            })?)
            .with_context(|| {
                format!(
                    "decode explicit Workbench receipt `{}`",
                    canonical_path.display()
                )
            })?;
        if receipt.schema_version != "arda.execution-receipt.v3"
            || receipt.run_id != run_id
            || receipt.node_id != expected_name
            || receipt.receipt_digest != reference.digest
            || receipt.project_contract_digest != project_contract_digest
            || receipt.status != HermesReceiptStatus::Succeeded
            || !receipt.has_valid_digest()?
        {
            bail!("explicit Workbench {expected_name} receipt failed canonical validation");
        }
        let provider = receipt
            .usage
            .provider
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                anyhow!("explicit Workbench {expected_name} receipt omitted provider")
            })?;
        let model = receipt
            .usage
            .model
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow!("explicit Workbench {expected_name} receipt omitted model"))?;
        let recorded_at_ms = i64::try_from(receipt.recorded_at_unix_ms)
            .context("explicit Workbench receipt timestamp exceeds ObjectiveStore bounds")?;
        let run_path = canonical_path
            .strip_prefix(&canonical_root)
            .context("project explicit Workbench receipt path")?
            .to_string_lossy()
            .to_string();
        let digest = receipt.receipt_digest;
        projected.push(StageReceipt {
            contract: "arda.hermes_execution_receipt.v4".into(),
            stage,
            digest: digest.clone(),
            predecessor_digest: predecessor,
            run_path,
            provider,
            model,
            started_at_ms: recorded_at_ms,
            completed_at_ms: recorded_at_ms,
            verdict: match stage {
                ReceiptStage::Review => "approved",
                ReceiptStage::Close => "closed",
                _ => "succeeded",
            }
            .into(),
        });
        predecessor = Some(digest);
    }
    if outcome.root_receipt_digest.as_deref() != predecessor.as_deref() {
        bail!("explicit Workbench run `{run_id}` root receipt did not match close receipt");
    }
    Ok(projected)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::{CostMeasurement, NormalizedHermesUsage};
    use crate::objectives::LeafExecutionSpec;
    use serde_json::json;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct RecordingWorkbench {
        outcome: ExplicitExecutionOutcome,
        item: Arc<Mutex<Option<ExplicitWorkbenchWorkItem>>>,
    }

    impl ExplicitWorkbenchExecution for RecordingWorkbench {
        fn execute_explicit<'a>(
            &'a self,
            item: &'a ExplicitWorkbenchWorkItem,
        ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
            let outcome = self.outcome.clone();
            let recorded = Arc::clone(&self.item);
            Box::pin(async move {
                *recorded.lock().unwrap() = Some(item.clone());
                Ok(outcome)
            })
        }
    }

    #[tokio::test]
    async fn claimed_leaf_maps_to_explicit_workbench_and_exact_receipts() {
        let root = tempfile::tempdir().unwrap();
        let workspace = root.path().join("project");
        std::fs::create_dir_all(&workspace).unwrap();
        let run_id = "objective-objective-1-leaf-leaf-1-attempt-1";
        let project_digest = format!("sha256:{}", "a".repeat(64));
        let receipt_root = root
            .path()
            .join("data/runs")
            .join(run_id)
            .join("execution-receipts");
        std::fs::create_dir_all(&receipt_root).unwrap();
        let mut predecessor = None;
        let mut references = Vec::new();
        for (index, stage) in ["execute", "verify", "review", "close"]
            .into_iter()
            .enumerate()
        {
            let mut receipt = HermesExecutionReceipt {
                schema_version: "arda.execution-receipt.v3".into(),
                receipt_digest: String::new(),
                authority_binding_digest: format!("sha256:{}", "b".repeat(64)),
                run_id: run_id.into(),
                node_id: stage.into(),
                idempotency_key: format!("{run_id}-{stage}"),
                status: HermesReceiptStatus::Succeeded,
                summary: format!("{stage} completed"),
                tool_evidence: vec![],
                test_evidence: vec![],
                artifacts: vec![],
                usage: NormalizedHermesUsage {
                    provider: Some(format!("provider-{stage}")),
                    model: Some(format!("model-{stage}")),
                    api_calls: 1,
                    input_tokens: 1,
                    output_tokens: 1,
                    total_tokens: 2,
                    estimated_cost_usd: 0.0,
                    cost_measurement: CostMeasurement::Observed,
                    completed: true,
                    failed: false,
                },
                adapter: "hermes".into(),
                adapter_version: "1".into(),
                project_contract_digest: project_digest.clone(),
                parent_receipts: predecessor.iter().cloned().collect(),
                context_capsule_id: None,
                context_capsule_digest: None,
                context_use_receipt_ref: None,
                context_handoff: None,
                recorded_at_unix_ms: 100 + index as u128,
            };
            receipt.receipt_digest = receipt.computed_digest().unwrap();
            predecessor = Some(receipt.receipt_digest.clone());
            let path = receipt_root.join(format!("{stage}.json"));
            std::fs::write(&path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
            references.push(arda_aule::prometheus::autopilot::ExplicitReceiptReference {
                stage: stage.into(),
                digest: receipt.receipt_digest,
                path: path.display().to_string(),
            });
        }
        let recorded = Arc::new(Mutex::new(None));
        let executor = WorkbenchLeafExecution::with_adapter(
            root.path(),
            RecordingWorkbench {
                outcome: ExplicitExecutionOutcome {
                    run_id: run_id.into(),
                    status: "succeeded".into(),
                    root_receipt_digest: predecessor,
                    receipts: references,
                },
                item: Arc::clone(&recorded),
            },
        );
        let claim = ClaimedLeaf {
            objective_id: "objective-1".into(),
            leaf_id: "leaf-1".into(),
            project_id: Some("project-1".into()),
            workspace_root: workspace.display().to_string(),
            authority: "read_only".into(),
            stage: crate::objectives::LeafStage::Execute,
            attempt: 1,
            lease_owner: "runtime".into(),
            lease_expires_ms: 1_000,
            current_receipt_digest: None,
            project_contract_digest: Some(project_digest.clone()),
            execution: Some(LeafExecutionSpec {
                objective: "inspect the exact project".into(),
                execution_prompt: "execute exact project".into(),
                verification_prompt: "verify exact project".into(),
                review_prompt: "review exact evidence".into(),
                approval_envelope: json!({
                    "approval": {
                        "schema_version": "arda.orome.task_approval.v1",
                        "approval_id": "approval-1",
                        "ledger_writes": ["data/arda/objectives.sqlite3", "data/runs"]
                    }
                }),
                objective_plan_receipt: format!("sha256:{}", "c".repeat(64)),
            }),
            dependency_receipts: vec![StageReceipt {
                contract: "arda.hermes_execution_receipt.v4".into(),
                stage: ReceiptStage::Close,
                digest: format!("sha256:{}", "d".repeat(64)),
                predecessor_digest: Some(format!("sha256:{}", "e".repeat(64))),
                run_path: "data/runs/dependency/execution-receipts/close.json".into(),
                provider: "provider-dependency".into(),
                model: "model-dependency".into(),
                started_at_ms: 80,
                completed_at_ms: 90,
                verdict: "succeeded".into(),
            }],
        };

        let result = executor.execute(claim).await.unwrap();

        assert_eq!(result.receipts.len(), 4);
        assert_eq!(result.receipts[3].stage, ReceiptStage::Close);
        assert_eq!(
            result.receipts[3].predecessor_digest,
            Some(result.receipts[2].digest.clone())
        );
        let item = recorded.lock().unwrap().clone().unwrap();
        assert_eq!(item.project_id, "project-1");
        assert_eq!(item.project_contract_digest, project_digest);
        assert_eq!(
            item.objective_plan_receipt,
            format!("sha256:{}", "c".repeat(64))
        );
        assert_eq!(item.dependency_receipts.len(), 1);
        assert_eq!(item.dependency_receipts[0].stage, "close");
        assert_eq!(
            item.dependency_receipts[0].path,
            "data/runs/dependency/execution-receipts/close.json"
        );
        assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    }
}
