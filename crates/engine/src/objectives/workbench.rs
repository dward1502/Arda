use super::model::{ClaimedLeaf, ReceiptStage, StageReceipt};
use super::runtime::{LeafExecution, LeafExecutionResult};
use crate::adapters::{HermesExecutionReceipt, HermesReceiptStatus};
use anyhow::{anyhow, bail, Context, Result};
use arda_aule::prometheus::autopilot::{
    ExplicitExecutionOutcome, ExplicitReceiptReference, ExplicitWorkbenchWorkItem,
    WorkbenchExecutionAdapter,
};
use arda_core::capability_composition::{
    CompositionAuthorityClass, DataClass, EgressTarget, RoleKind,
};
use arda_core::contract::{MemoryKind, MemoryRecord};
use arda_core::run_graph::{ObjectiveId, RunId};
use arda_vaire::service::scope_policy::{ConsumerContext, MemoryDomain};
use arda_vaire::{
    ContextAssembly, ContextConsumer, ContextDisposition, ContextLineage, ContextObjective,
    ContextOutcomeInput, ContextOutcomeReceipt, ContextReturnContract, MnemosyneService,
    OrganismContext,
};
use chrono::Utc;
use futures::future::BoxFuture;
use serde_json::json;
use sha2::{Digest, Sha256};
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
            let (memory, context_assembly) =
                assemble_resident_context(&root, &claim, &run_id, execution, project_id)?;
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
                context_assembly: Some(context_assembly.clone()),
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
            let mut receipts = project_receipts(
                &root,
                &run_id,
                project_contract_digest,
                &context_assembly,
                &outcome,
            )?;
            let context_outcome = record_resident_context_outcome(
                &root,
                &memory,
                &context_assembly,
                &claim,
                &run_id,
                &receipts,
            )?;
            for receipt in &mut receipts {
                receipt.context_outcome_receipt_id = Some(context_outcome.receipt_id.clone());
                receipt.context_outcome_receipt_digest =
                    Some(context_outcome.receipt_digest.clone());
                receipt.binding_digest = Some(receipt.computed_binding_digest()?);
            }
            Ok(LeafExecutionResult { receipts })
        })
    }
}

fn assemble_resident_context(
    root: &Path,
    claim: &ClaimedLeaf,
    run_id: &str,
    execution: &super::model::LeafExecutionSpec,
    project_id: &str,
) -> Result<(MnemosyneService, ContextAssembly)> {
    let service = MnemosyneService::new(root.join("data/vaire"))?
        .with_contract_memory_root(root.join("core/state/memory"));
    let consumer_id = format!("arda.resident-objective:{run_id}");
    let mut consumer = ConsumerContext::new(&consumer_id, vec![MemoryDomain::System]);
    consumer.purpose = Some(execution.objective.clone());
    consumer.operator_authorized = true;
    let memory_refs = service
        .recall_governed_memories(Some(&consumer))?
        .into_iter()
        .filter(|record| {
            record
                .extensions
                .get("resident_objective_outcome")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
                && record
                    .extensions
                    .get("objective_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(claim.objective_id.as_str())
                && record
                    .extensions
                    .get("project_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(project_id)
                && record
                    .extensions
                    .get("project_contract_digest")
                    .and_then(serde_json::Value::as_str)
                    == claim.project_contract_digest.as_deref()
        })
        .take(8)
        .map(|record| record.id)
        .collect::<Vec<_>>();
    let now = Utc::now().timestamp_millis().max(0) as u128;
    let context = OrganismContext {
        schema_version: OrganismContext::SCHEMA_VERSION.into(),
        organism_id: "arda:resident".into(),
        generated_at_unix_ms: now,
        expires_at_unix_ms: now.saturating_add(3_600_000),
        consumer: ContextConsumer {
            consumer_id: consumer_id.clone(),
            role: RoleKind::Worker,
            authority_ceiling: CompositionAuthorityClass::ExecuteWithApproval,
            operator_authorized: true,
            memory_domains: vec![MemoryDomain::System],
            data_classes: vec![DataClass::Internal],
            permitted_egress: vec![EgressTarget::LocalDevice],
            compute_node_refs: Vec::new(),
            agent_ref: Some("aule:resident-workbench".into()),
        },
        lineage: ContextLineage {
            objective_id: ObjectiveId::new(claim.objective_id.clone())?,
            project_id: project_id.parse().ok(),
            run_id: Some(RunId::new(run_id.to_owned())?),
            task_id: Some(claim.leaf_id.clone()),
            session_ref: None,
            parent_receipts: claim
                .dependency_receipts
                .iter()
                .map(|receipt| receipt.digest.clone())
                .collect(),
        },
        objective: ContextObjective {
            requested_outcome: execution.objective.clone(),
            acceptance_conditions: vec![execution.verification_prompt.clone()],
            required_capabilities: vec!["resident_objective_execution".into()],
            forbidden_capabilities: vec!["legacy_queue_authority".into()],
        },
        evidence_refs: claim
            .dependency_receipts
            .iter()
            .map(|receipt| receipt.run_path.clone())
            .collect(),
        memory_refs,
        unresolved_failures: Vec::new(),
        return_contract: ContextReturnContract {
            schema_version: "arda.context-return.v1".into(),
            required_receipt_types: vec![
                "arda.execution-receipt.v3".into(),
                "arda.context-outcome-receipt.v1".into(),
            ],
            max_output_bytes: 65_536,
        },
    };
    let assembly = service.assemble_organism_context(context, &consumer, now)?;
    Ok((service, assembly))
}

fn record_resident_context_outcome(
    root: &Path,
    service: &MnemosyneService,
    assembly: &ContextAssembly,
    claim: &ClaimedLeaf,
    run_id: &str,
    receipts: &[StageReceipt],
) -> Result<ContextOutcomeReceipt> {
    let evidence_refs = receipts
        .iter()
        .map(|receipt| receipt.run_path.clone())
        .collect::<Vec<_>>();
    let influenced = assembly.use_receipt.memory_refs.clone();
    let context_outcome = service.record_context_outcome(
        &assembly.use_receipt,
        ContextOutcomeInput {
            consumer_id: assembly.use_receipt.consumer_id.clone(),
            disposition: ContextDisposition::Used,
            influenced_memory_refs: influenced.clone(),
            evidence_refs,
            rationale: if influenced.is_empty() {
                "The resident worker used a governed empty-baseline capsule and produced canonical terminal evidence."
                    .into()
            } else {
                format!(
                    "The resident worker used {} governed prior outcome(s) while producing canonical terminal evidence.",
                    influenced.len()
                )
            },
            recorded_at_unix_ms: assembly.use_receipt.recorded_at_unix_ms,
        },
    )?;

    let memory_id = format!("mem_resident_{:x}", Sha256::digest(run_id.as_bytes()));
    let mut consumer = ConsumerContext::new(
        assembly.use_receipt.consumer_id.clone(),
        vec![MemoryDomain::System],
    );
    consumer.purpose = Some(assembly.capsule.context.objective.requested_outcome.clone());
    consumer.operator_authorized = true;
    if service
        .recall_governed_memories(Some(&consumer))?
        .iter()
        .any(|record| record.id == memory_id)
    {
        return Ok(context_outcome);
    }
    let execute_receipt = receipts
        .iter()
        .find(|receipt| receipt.stage == ReceiptStage::Execute)
        .ok_or_else(|| anyhow!("resident memory promotion omitted execute receipt"))?;
    let execute_canonical: HermesExecutionReceipt = serde_json::from_slice(
        &std::fs::read(root.join(&execute_receipt.run_path)).with_context(|| {
            format!(
                "read resident memory approval receipt `{}`",
                execute_receipt.run_path
            )
        })?,
    )?;
    let approval_reference = execute_canonical
        .parent_receipts
        .first()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("resident memory promotion omitted operator approval receipt"))?
        .clone();
    let review_receipt = receipts
        .iter()
        .find(|receipt| receipt.stage == ReceiptStage::Review)
        .ok_or_else(|| anyhow!("resident memory promotion omitted review receipt"))?;
    let review_canonical: HermesExecutionReceipt = serde_json::from_slice(
        &std::fs::read(root.join(&review_receipt.run_path)).with_context(|| {
            format!(
                "read resident memory evaluation receipt `{}`",
                review_receipt.run_path
            )
        })?,
    )?;
    if review_canonical.summary.lines().next() != Some("VERDICT: APPROVE") {
        bail!("resident memory promotion requires an approved independent review receipt");
    }
    let outcome_summaries = receipts
        .iter()
        .map(|receipt| -> Result<String> {
            let path = root.join(&receipt.run_path);
            let canonical: HermesExecutionReceipt =
                serde_json::from_slice(&std::fs::read(&path).with_context(|| {
                    format!("read resident outcome receipt `{}`", path.display())
                })?)
                .with_context(|| format!("decode resident outcome receipt `{}`", path.display()))?;
            Ok(format!("{}={}", receipt.stage.as_str(), canonical.summary))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut memory = MemoryRecord::new(
        &memory_id,
        MemoryKind::Episodic,
        "vaire-resident-objective",
        format!(
            "Resident objective {} leaf {} completed run {}. Outcomes: {}. Canonical stage receipts: {}.",
            claim.objective_id,
            claim.leaf_id,
            run_id,
            outcome_summaries.join("; "),
            receipts
                .iter()
                .map(|receipt| format!("{}={}", receipt.stage.as_str(), receipt.digest))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    );
    memory
        .extensions
        .insert("memory_domain".into(), json!("system"));
    memory
        .extensions
        .insert("resident_objective_outcome".into(), json!(true));
    memory
        .extensions
        .insert("objective_id".into(), json!(claim.objective_id));
    memory
        .extensions
        .insert("project_id".into(), json!(claim.project_id));
    memory.extensions.insert(
        "project_contract_digest".into(),
        json!(claim.project_contract_digest),
    );
    memory.extensions.insert("run_id".into(), json!(run_id));
    memory.extensions.insert(
        "knowledge_approval_reference".into(),
        json!(approval_reference),
    );
    memory.extensions.insert(
        "knowledge_evaluation_reference".into(),
        json!(review_canonical.receipt_digest),
    );
    memory.extensions.insert(
        "context_use_receipt_id".into(),
        json!(assembly.use_receipt.receipt_id),
    );
    service.write_governed_memory(memory, Some(&consumer))?;
    Ok(context_outcome)
}

fn project_receipts(
    root: &Path,
    run_id: &str,
    project_contract_digest: &str,
    context: &ContextAssembly,
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
        match predecessor.as_deref() {
            Some(expected_parent) if receipt.parent_receipts.as_slice() != [expected_parent] => {
                bail!(
                    "explicit Workbench {expected_name} receipt did not bind the canonical predecessor"
                );
            }
            None if receipt.parent_receipts.len() != 1 => {
                bail!(
                    "explicit Workbench execute receipt did not bind exactly one approval receipt"
                );
            }
            _ => {}
        }
        let expected_context_ref = context.use_receipt.receipt_ref();
        if receipt.context_capsule_id.as_deref() != Some(context.capsule.capsule_id.as_str())
            || receipt.context_capsule_digest.as_deref()
                != Some(context.capsule.capsule_digest.as_str())
            || receipt.context_use_receipt_ref.as_deref() != Some(expected_context_ref.as_str())
        {
            bail!(
                "explicit Workbench {expected_name} receipt was not bound to the resident Vairë context-use receipt"
            );
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
            context_outcome_receipt_id: None,
            context_outcome_receipt_digest: None,
            binding_digest: None,
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
            let mut outcome = self.outcome.clone();
            let recorded = Arc::clone(&self.item);
            Box::pin(async move {
                let context = item.context_assembly.as_ref().unwrap();
                let mut predecessor = Some(format!("sha256:{}", "f".repeat(64)));
                for reference in &mut outcome.receipts {
                    let mut receipt: HermesExecutionReceipt =
                        serde_json::from_slice(&std::fs::read(&reference.path).unwrap()).unwrap();
                    receipt.parent_receipts = predecessor.iter().cloned().collect();
                    receipt.context_capsule_id = Some(context.capsule.capsule_id.clone());
                    receipt.context_capsule_digest = Some(context.capsule.capsule_digest.clone());
                    receipt.context_use_receipt_ref = Some(context.use_receipt.receipt_ref());
                    receipt.receipt_digest = receipt.computed_digest().unwrap();
                    reference.digest = receipt.receipt_digest.clone();
                    predecessor = Some(receipt.receipt_digest.clone());
                    std::fs::write(
                        &reference.path,
                        serde_json::to_vec_pretty(&receipt).unwrap(),
                    )
                    .unwrap();
                }
                outcome.root_receipt_digest = predecessor;
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
        let mut predecessor = Some(format!("sha256:{}", "f".repeat(64)));
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
                summary: if stage == "review" {
                    "VERDICT: APPROVE\nreview completed".into()
                } else {
                    format!("{stage} completed")
                },
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
                context_outcome_receipt_id: Some("outcome-prior".into()),
                context_outcome_receipt_digest: Some(format!("sha256:{}", "8".repeat(64))),
                binding_digest: None,
            }],
        };

        let followup_claim = claim.clone();
        let followup_execution = claim.execution.clone().unwrap();
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
        let assembly = item.context_assembly.expect("resident Vairë context");
        assert!(assembly.use_receipt.has_valid_digest().unwrap());
        assert_eq!(assembly.use_receipt.objective_id, "objective-1");
        assert_eq!(assembly.use_receipt.run_id.as_deref(), Some(run_id));
        let service = MnemosyneService::new(root.path().join("data/vaire"))
            .unwrap()
            .with_contract_memory_root(root.path().join("core/state/memory"));
        let outcomes = service.context_outcome_receipts().unwrap();
        assert_eq!(outcomes.len(), 1);
        assert!(outcomes[0].has_valid_digest().unwrap());
        assert!(result.receipts.iter().all(|receipt| {
            receipt.context_outcome_receipt_id.as_deref() == Some(outcomes[0].receipt_id.as_str())
                && receipt.context_outcome_receipt_digest.as_deref()
                    == Some(outcomes[0].receipt_digest.as_str())
                && receipt.binding_digest.as_deref()
                    == Some(receipt.computed_binding_digest().unwrap().as_str())
        }));
        let (_, followup) = assemble_resident_context(
            root.path(),
            &followup_claim,
            "objective-objective-1-leaf-leaf-1-attempt-2",
            &followup_execution,
            "project-1",
        )
        .unwrap();
        assert_eq!(followup.use_receipt.memory_refs.len(), 1);
        assert_eq!(followup.capsule.memories.len(), 1);
        assert!(followup.capsule.memories[0].content.contains("objective-1"));
        assert!(followup.capsule.memories[0]
            .content
            .contains("close=close completed"));
        let mut sibling_claim = followup_claim;
        sibling_claim.leaf_id = "leaf-2".into();
        sibling_claim.project_id = Some("project-2".into());
        sibling_claim.project_contract_digest = Some(format!("sha256:{}", "9".repeat(64)));
        let (_, sibling) = assemble_resident_context(
            root.path(),
            &sibling_claim,
            "objective-objective-1-leaf-leaf-2-attempt-1",
            &followup_execution,
            "project-2",
        )
        .unwrap();
        assert!(sibling.use_receipt.memory_refs.is_empty());
        assert!(sibling.capsule.memories.is_empty());
        assert!(!root.path().join("core/projects/tasks/queue.jsonl").exists());
    }
}
