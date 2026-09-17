//! Process-local bridge from canonical Engine recovery authority to Workbench.
//! This is not ingress authentication and never authorizes a replacement capture.
use super::*;
use crate::objectives::snapshot_protocol::Lease;
use crate::objectives::{ObjectiveStore, RecoveryMaterial};
use crate::runs::RecoveryGrant;
use arda_aule::prometheus::autopilot::workbench_executor::ExplicitRecoveryWindow;
mod provider;

pub struct RecoveryAuthorization {
    root: PathBuf,
    store: ObjectiveStore,
    operator_id: String,
    event_id: String,
    lease: Lease,
}

impl RecoveryAuthorization {
    /// The caller supplies an authenticated operator and a server-owned claim.
    /// Construction grants no permission: every use resolves the saved admission,
    /// stop fence, exact acknowledged lease and canonical material afresh.
    pub fn new(
        root: PathBuf,
        store: ObjectiveStore,
        operator_id: String,
        event_id: String,
        claim: &ClaimedLeaf,
    ) -> Result<Self> {
        let run_id = claim
            .execution_run_id
            .clone()
            .context("recovery claim has no retained run")?;
        Ok(Self {
            root,
            store,
            operator_id,
            event_id,
            lease: Lease {
                run_id,
                generation: claim.attempt,
                owner: claim.lease_owner.clone(),
                expires_ms: claim.lease_expires_ms,
            },
        })
    }

    /// Resolve the original item without assembling a new context or approving,
    /// claiming, scheduling or dispatching any work.
    pub fn canonical_item(&self) -> Result<ExplicitWorkbenchWorkItem> {
        self.checked(None).map(|(item, _, _)| item)
    }

    fn checked(
        &self,
        expected: Option<&ExplicitWorkbenchWorkItem>,
    ) -> Result<(ExplicitWorkbenchWorkItem, Lease, ExplicitRecoveryWindow)> {
        self.store.with_retained_recovery(
            &self.operator_id,
            &self.event_id,
            &self.lease,
            |tx, grant, retained| {
                let material = self
                    .store
                    .validate_retained_recovery_material(&self.root, tx, grant)?;
                anyhow::ensure!(
                    material.snapshot == retained.snapshot,
                    "recovery material differs from the fenced snapshot"
                );
                let item = canonical_work_item(grant, &material);
                if let Some(expected) = expected {
                    anyhow::ensure!(
                        serde_json::to_value(expected)? == serde_json::to_value(&item)?,
                        "recovery authorization is for another work item"
                    );
                }
                Ok((
                    item,
                    retained.lease.clone(),
                    ExplicitRecoveryWindow {
                        event_id: self.event_id.clone(),
                        activated_at_unix_ms: grant.activated_at_unix_ms,
                        expires_at_unix_ms: grant.expires_at_unix_ms,
                    },
                ))
            },
        )
    }
}

impl ExplicitWorkspaceAuthorization for RecoveryAuthorization {
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
        self.checked(Some(item)).map(|_| ())
    }

    fn provider_lease(&self, item: &ExplicitWorkbenchWorkItem) -> Result<serde_json::Value> {
        Ok(serde_json::to_value(self.checked(Some(item))?.1)?)
    }

    fn recovery_window(
        &self,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Option<ExplicitRecoveryWindow>> {
        Ok(Some(self.checked(Some(item))?.2))
    }
}

/// Construct the same item as ordinary resident execution from validated inputs.
/// The admission-only research binding is not a Workbench mutation-envelope field.
fn canonical_work_item(
    grant: &RecoveryGrant,
    material: &RecoveryMaterial,
) -> ExplicitWorkbenchWorkItem {
    let execution = &material.execution;
    let mut envelope = execution.approval_envelope.clone();
    if let Some(object) = envelope.as_object_mut() {
        object.remove("research_brief_id");
    }
    ExplicitWorkbenchWorkItem {
        read_only: true,
        objective_id: grant.bindings.objective_id.clone(),
        leaf_id: grant.bindings.leaf_id.clone(),
        run_id: grant.bindings.run_id.as_str().to_owned(),
        objective: execution.objective.clone(),
        execution_prompt: execution.execution_prompt.clone(),
        verification_prompt: execution.verification_prompt.clone(),
        review_prompt: execution.review_prompt.clone(),
        project_id: material.project_id.clone(),
        project_contract_digest: grant.bindings.project_contract_digest.clone(),
        workspace_root: PathBuf::from(&material.workspace_root),
        approval_envelope: envelope,
        objective_plan_receipt: execution.objective_plan_receipt.clone(),
        dependency_receipts: material
            .dependency_receipts
            .iter()
            .map(|receipt| ExplicitReceiptReference {
                stage: receipt.stage.as_str().to_owned(),
                digest: receipt.digest.clone(),
                path: receipt.run_path.clone(),
            })
            .collect(),
        context_assembly: Some(material.context.clone()),
    }
}
