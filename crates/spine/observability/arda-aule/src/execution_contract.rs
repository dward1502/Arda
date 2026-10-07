//! Explicit execution boundary shared by Engine admission and the adapter.
//!
//! Work items are data, not authority. Runtime and workspace permissions enter
//! through separate process-local callbacks; receipt-only reconciliation must
//! not dispatch a provider. This module performs no HTTP or filesystem I/O.
//! It can be compiled without Aule's `full-cli` or telemetry features.

use anyhow::{bail, Result};
use arda_vaire::ContextAssembly;
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplicitWorkbenchWorkItem {
    /// Narrow the execution graph to file-only inspection. Never grants authority.
    #[serde(default)]
    pub read_only: bool,
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

/// Implementors retain the runtime's shared maintenance-exclusion lease.
/// This lease grants no workspace permission or retained execution authority.
pub trait ExplicitRuntimeLease: Send + Sync {}

/// Admission must validate the exact root and canonical objective/leaf/run binding
/// and refuse permanently abandoned history before returning an owned lease.
pub trait ExplicitRuntimeAdmission: Send + Sync {
    fn admit(
        &self,
        root: &Path,
        item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Box<dyn ExplicitRuntimeLease>>;
}

pub trait ExplicitWorkspaceAuthorization: Send + Sync {
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()>;
    /// Opaque Engine-owned lease, sent only at provider dispatch. It is not part
    /// of the serialized work item and Aule does not interpret its schema.
    fn provider_lease(&self, _item: &ExplicitWorkbenchWorkItem) -> Result<Value> {
        anyhow::bail!("retained provider dispatch requires an Engine lease")
    }
    /// Engine-owned saved admission; never deserialize this from a work item.
    fn recovery_window(
        &self,
        _item: &ExplicitWorkbenchWorkItem,
    ) -> Result<Option<ExplicitRecoveryWindow>> {
        Ok(None)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExplicitRecoveryWindow {
    pub event_id: String,
    pub activated_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
}

impl<F> ExplicitWorkspaceAuthorization for F
where
    F: Fn(&ExplicitWorkbenchWorkItem) -> Result<()> + Send + Sync,
{
    fn authorize(&self, item: &ExplicitWorkbenchWorkItem) -> Result<()> {
        self(item)
    }
}

pub trait ExplicitWorkbenchExecution: Send + Sync {
    fn inspect_retry_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<Option<ExplicitExecutionOutcome>>> {
        self.reconcile_explicit(item, owner)
    }
    fn execute_explicit_authorized<'a>(
        &'a self,
        _item: &'a ExplicitWorkbenchWorkItem,
        _owner: &'a dyn ExplicitRuntimeAdmission,
        _authority: &'a dyn ExplicitWorkspaceAuthorization,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>> {
        Box::pin(
            async move { bail!("executor does not implement retained workspace authorization") },
        )
    }
    fn reconcile_explicit<'a>(
        &'a self,
        _item: &'a ExplicitWorkbenchWorkItem,
        _owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<Option<ExplicitExecutionOutcome>>> {
        Box::pin(async { Ok(None) })
    }

    fn execute_explicit<'a>(
        &'a self,
        item: &'a ExplicitWorkbenchWorkItem,
        owner: &'a dyn ExplicitRuntimeAdmission,
    ) -> BoxFuture<'a, Result<ExplicitExecutionOutcome>>;
}
