//! Canonical material resolution under the caller's objective-store transaction.
use super::*;
use crate::objectives::RetainedSnapshot;
use arda_core::run_graph::RunGraph;
use arda_vaire::{ContextAssembly, MnemosyneService};
use sha2::{Digest, Sha256};
#[cfg(test)]
mod tests;

pub struct RecoveryMaterial {
    pub graph: RunGraph,
    pub snapshot: RetainedSnapshot,
    pub context: ContextAssembly,
    pub execution: super::super::super::LeafExecutionSpec,
    pub project_id: String,
    pub workspace_root: String,
    pub dependency_receipts: Vec<StageReceipt>,
}

impl ObjectiveStore {
    /// Resolve canonical run, capability, original context and current policy.
    /// This does not authenticate ingress, activate a grant, claim a lease, or
    /// validate a caller-supplied derived stage task. Call inside the recovery
    /// control fence; never use these materials as a standalone dispatch permit.
    pub fn validate_retained_recovery_material(
        &self,
        root: &std::path::Path,
        tx: &Transaction<'_>,
        grant: &RecoveryGrant,
    ) -> Result<RecoveryMaterial> {
        self.validate_recovery_material_phase(root, tx, grant, false)
    }

    /// No material or dispatch permit escapes the pre-reconciliation phase.
    pub(super) fn validate_recovery_before_reconciliation(
        &self,
        root: &std::path::Path,
        tx: &Transaction<'_>,
        grant: &RecoveryGrant,
    ) -> Result<()> {
        self.validate_recovery_material_phase(root, tx, grant, true)
            .map(|_| ())
    }

    fn validate_recovery_material_phase(
        &self,
        root: &std::path::Path,
        tx: &Transaction<'_>,
        grant: &RecoveryGrant,
        pending_ack: bool,
    ) -> Result<RecoveryMaterial> {
        super::super::super::snapshots::check_policy(tx, self.snapshot_admission.is_some())?;
        anyhow::ensure!(
            self.snapshot_admission.is_some(),
            "recovery keeper unavailable"
        );
        let b = &grant.bindings;
        let (context_json, execution_json, capability_json, project_id, workspace_root, request_digest): (String, String, String, String, String, String) = tx.query_row(
            "SELECT c.assembly_json,l.execution_json,s.capability_json,l.project_id,l.workspace_root,c.request_digest FROM leaves l
             JOIN resident_context_bindings c ON c.run_id=l.execution_run_id
             JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
             JOIN objective_projects p ON p.objective_id=l.objective_id AND p.project_id=l.project_id
             WHERE l.id=?1 AND l.objective_id=?2 AND l.execution_run_id=?3
               AND p.contract_digest=?4 AND l.authority='read_only' AND l.context_bound=1
               AND (s.committed_generation=l.attempt OR
                 (?5 AND s.committed_generation>0 AND s.committed_generation=l.attempt-1
                  AND EXISTS(SELECT 1 FROM retained_snapshot_lease_intents i
                    WHERE i.leaf_id=l.id AND i.generation=l.attempt
                      AND i.lease_owner=l.lease_owner AND i.lease_expires_ms=l.lease_expires_ms
                      AND i.recovery_event_id=?6)))
               AND c.deleted_by_operator_ms IS NULL
               AND NOT EXISTS(SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id=l.id)",
            params![b.leaf_id, b.objective_id, b.run_id.as_str(), b.project_contract_digest, pending_ack, grant.authenticated_event_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )?;
        let snapshot: RetainedSnapshot = serde_json::from_str(&capability_json)?;
        let context: ContextAssembly = serde_json::from_str(&context_json)?;
        let execution: super::super::super::LeafExecutionSpec =
            serde_json::from_str(&execution_json)?;
        let dependency_receipts = Self::read_dependency_receipts(tx, &b.leaf_id)?;
        let expected_request_digest = crate::objectives::request_binding::ResidentRequestBinding {
            objective_id: &b.objective_id,
            leaf_id: &b.leaf_id,
            project_id: &project_id,
            project_contract_digest: &b.project_contract_digest,
            workspace_root: &workspace_root,
            authority: "read_only",
            execution: &execution,
            dependencies: &dependency_receipts,
        }
        .digest()?;
        anyhow::ensure!(
            request_digest == expected_request_digest,
            "original resident request changed"
        );
        let memory = MnemosyneService::new(root.join("data/vaire"))?
            .with_contract_memory_root(root.join("core/state/memory"));
        validate_material_binding(&memory, grant, &snapshot, &context, &project_id)?;
        let graph = crate::runs::RunStore::open(root, b.run_id.clone())?
            .validate_recovery_run_evidence(b)?;
        // File reads can take time. Do not return an already-expired authority
        // snapshot merely because it was live before those reads.
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_millis(),
        )?;
        anyhow::ensure!(
            grant.is_active(now),
            "recovery window expired during material validation"
        );
        Ok(RecoveryMaterial {
            graph,
            snapshot,
            context,
            execution,
            project_id,
            workspace_root,
            dependency_receipts,
        })
    }
}

fn validate_material_binding(
    memory: &MnemosyneService,
    grant: &RecoveryGrant,
    snapshot: &RetainedSnapshot,
    assembly: &ContextAssembly,
    project_id: &str,
) -> Result<()> {
    grant.validate().map_err(anyhow::Error::msg)?;
    let b = &grant.bindings;
    let snapshot_digest = format!("sha256:{:x}", Sha256::digest(serde_json::to_vec(snapshot)?));
    let lineage = &assembly.capsule.context.lineage;
    anyhow::ensure!(
        snapshot_digest == b.retained_authority_digest
            && assembly.capsule.capsule_digest == b.context_capsule_digest
            && assembly.use_receipt.receipt_digest == b.context_use_receipt_digest
            && assembly.use_receipt.run_id.as_deref() == Some(b.run_id.as_str())
            && lineage.objective_id.as_str() == b.objective_id
            && lineage.task_id.as_deref() == Some(b.leaf_id.as_str())
            && lineage.run_id.as_ref() == Some(&b.run_id)
            && lineage.project_id == project_id.parse().ok(),
        "canonical recovery capability or original context binding changed"
    );
    memory.validate_context_assembly_for_recovery(
        assembly,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
        u128::from(grant.activated_at_unix_ms),
        u128::from(grant.expires_at_unix_ms),
    )?;
    Ok(())
}
