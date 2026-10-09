//! Canonical resident receipt projection, independent of provider dispatch.
use super::*;
#[cfg(test)]
mod tests;

/// Rejection is evidence for a correction, never a successful stage projection.
pub(crate) fn rejected_chain(
    root: &Path,
    item: &ExplicitWorkbenchWorkItem,
) -> Result<Vec<HermesExecutionReceipt>> {
    use crate::runs::{RunEventKind, RunStore};
    use arda_core::run_graph::{NodeState, RunId};
    let recovered = RunStore::open(root, RunId::new(&item.run_id)?)?.recover()?;
    let graph = recovered.checkpoint.context("critic checkpoint missing")?;
    anyhow::ensure!(graph.run_id.as_str() == item.run_id, "critic run mismatch");
    anyhow::ensure!(
        graph.objective_id.as_str() == item.leaf_id
            && graph.provenance.project_contract_digest == item.project_contract_digest,
        "critic graph lineage mismatch"
    );
    anyhow::ensure!(
        !recovered.events.iter().any(|e| matches!(
            e.kind,
            RunEventKind::Cancelled { .. }
                | RunEventKind::NodeTransition {
                    state: NodeState::Cancelled
                }
        )),
        "cancelled run cannot authorize critic correction"
    );
    anyhow::ensure!(
        !graph.nodes.iter().any(|n| n.state == NodeState::Running),
        "critic run still has running workers"
    );
    let mut identities = std::collections::HashSet::new();
    let mut receipts = Vec::new();
    let mut references = Vec::new();
    for stage in ["execute", "verify", "review"] {
        let node = graph
            .nodes
            .iter()
            .find(|n| n.id.as_str() == stage)
            .context("critic chain node missing")?;
        let worker =
            serde_json::to_value(node.worker.as_ref().context("workerless critic chain")?)?;
        let identity = worker["worker_id"]
            .as_str()
            .filter(|s| !s.trim().is_empty())
            .context("critic worker identity missing")?;
        anyhow::ensure!(
            identities.insert(identity.to_owned()),
            "critic identities are not independent"
        );
        anyhow::ensure!(
            worker["route_id"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty()),
            "critic route missing"
        );
        let expected = if stage == "review" {
            NodeState::Failed
        } else {
            NodeState::Succeeded
        };
        anyhow::ensure!(node.state == expected, "critic chain state mismatch");
        let path = root
            .join("data/runs")
            .join(&item.run_id)
            .join("execution-receipts")
            .join(format!("{stage}.json"));
        anyhow::ensure!(
            std::fs::metadata(&path)?.len() <= 1_048_576,
            "critic receipt exceeds budget"
        );
        let receipt: HermesExecutionReceipt = serde_json::from_slice(&std::fs::read(&path)?)?;
        anyhow::ensure!(
            receipt.adapter == "hermes-workbench" && !receipt.adapter_version.trim().is_empty(),
            "critic chain lacks provider adapter provenance"
        );
        anyhow::ensure!(
            if expected == NodeState::Failed {
                // Failed transitions retain their digest in the journal, not
                // in the successful-output slot. The terminal event below is
                // the authoritative binding for the rejecting receipt.
                node.output_digest.is_none()
            } else {
                node.output_digest.as_deref() == Some(receipt.receipt_digest.as_str())
            },
            "critic output binding mismatch"
        );
        let started = recovered
            .events
            .iter()
            .find(|e| {
                e.node_id == node.id
                    && matches!(
                        e.kind,
                        RunEventKind::NodeTransition {
                            state: NodeState::Running
                        }
                    )
            })
            .context("critic worker never started")?;
        anyhow::ensure!(
            recovered.events.iter().any(|e| e.node_id == node.id
                && e.sequence > started.sequence
                && matches!(e.kind, RunEventKind::NodeTransition {state} if state == expected)
                && e.receipt_digest.as_deref() == Some(receipt.receipt_digest.as_str())),
            "critic terminal event missing"
        );
        anyhow::ensure!(
            !receipt.tool_evidence.is_empty(),
            "critic chain lacks real tool evidence"
        );
        references.push(ExplicitReceiptReference {
            stage: stage.into(),
            digest: receipt.receipt_digest.clone(),
            path: path.to_string_lossy().into_owned(),
        });
        receipts.push(receipt);
    }
    let review = &receipts[2];
    anyhow::ensure!(
        receipts[0].parent_receipts.as_slice()
            == [item.approval_envelope["approval"]["approval_id"]
                .as_str()
                .context("critic approval missing")?],
        "critic chain approval mismatch"
    );
    anyhow::ensure!(
        review.summary.lines().next() == Some("VERDICT: BLOCK")
            && review
                .summary
                .lines()
                .skip(1)
                .any(|line| !line.trim().is_empty()),
        "critic did not name a blocking defect"
    );
    anyhow::ensure!(
        !receipts[0].artifacts.is_empty(),
        "critic correction requires rejected artifact identity"
    );
    anyhow::ensure!(
        graph
            .nodes
            .iter()
            .find(|n| n.id.as_str() == "close")
            .is_some_and(|n| n.state != NodeState::Succeeded && n.output_digest.is_none()),
        "critic run already closed"
    );
    project_chain(
        root,
        &item.run_id,
        &item.project_contract_digest,
        item,
        &ExplicitExecutionOutcome {
            run_id: item.run_id.clone(),
            status: "failed".into(),
            root_receipt_digest: None,
            receipts: references,
        },
        true,
    )?;
    Ok(receipts)
}

pub(super) fn project_receipts(
    root: &Path,
    run_id: &str,
    project_contract_digest: &str,
    item: &ExplicitWorkbenchWorkItem,
    outcome: &ExplicitExecutionOutcome,
) -> Result<Vec<StageReceipt>> {
    project_chain(root, run_id, project_contract_digest, item, outcome, false)
}

fn project_chain(
    root: &Path,
    run_id: &str,
    project_contract_digest: &str,
    item: &ExplicitWorkbenchWorkItem,
    outcome: &ExplicitExecutionOutcome,
    rejected: bool,
) -> Result<Vec<StageReceipt>> {
    let expected_stages = [
        ("execute", ReceiptStage::Execute),
        ("verify", ReceiptStage::Verify),
        ("review", ReceiptStage::Review),
        ("close", ReceiptStage::Close),
    ];
    let expected_stages = &expected_stages[..if rejected { 3 } else { 4 }];
    if outcome.receipts.len() != expected_stages.len() {
        bail!(
            "explicit Workbench run `{run_id}` returned {} receipts instead of four",
            outcome.receipts.len()
        );
    }
    let canonical_root = root.canonicalize().context("canonicalize Arda root")?;
    let memory = MnemosyneService::new(root.join("data/vaire"))?
        .with_contract_memory_root(root.join("core/state/memory"));
    let mut predecessor = None;
    let mut projected = Vec::with_capacity(expected_stages.len());
    for (reference, (expected_name, stage)) in
        outcome.receipts.iter().zip(expected_stages.iter().copied())
    {
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
        if !receipt.valid_route_evidence()
            || receipt.run_id != run_id
            || receipt.node_id != expected_name
            || receipt.receipt_digest != reference.digest
            || receipt.project_contract_digest != project_contract_digest
            || receipt.status
                != if rejected && stage == ReceiptStage::Review {
                    HermesReceiptStatus::Failed
                } else {
                    HermesReceiptStatus::Succeeded
                }
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
        let context = item
            .stage_context(root, expected_name, receipt.parent_receipts.clone())?
            .context("resident stage context missing")?;
        let expected_context_ref = context.use_receipt.receipt_ref();
        // Historical evidence requires its original durable binding, not renewed
        // execution permission. Do not recreate missing receipts during recovery.
        if memory
            .context_use_receipt(&context.use_receipt.receipt_id)?
            .as_ref()
            != Some(&context.use_receipt)
        {
            bail!("explicit Workbench {expected_name} durable stage context is missing or conflicts; reconciliation required");
        }
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
                ReceiptStage::Review if rejected => "rejected",
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
    if !rejected && outcome.root_receipt_digest.as_deref() != predecessor.as_deref() {
        bail!("explicit Workbench run `{run_id}` root receipt did not match close receipt");
    }
    Ok(projected)
}
