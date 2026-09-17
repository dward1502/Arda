//! Execution-time overlay, not an authentication or start-admission mechanism.
use super::{HermesAdapter, HermesAdapterError, HermesNodeTask};
use crate::runs::RecoveryGrant;
use arda_core::run_graph::{AuthorityClass, NodeKind};
use arda_vaire::MnemosyneService;

pub(super) struct RecoveryExecution {
    grant: RecoveryGrant,
    task_digest: String,
    lease_expires_ms: u64,
    memory: MnemosyneService,
    #[cfg(target_os = "linux")]
    pub(super) dispatch_gate: Option<std::sync::Arc<dyn super::RecoveryDispatchGate>>,
}

impl HermesAdapter {
    /// Install process-local authority for every retained operation. Chat start
    /// admission and auxiliary reauthorization belong to this trusted gate;
    /// the transport owns sent/error cleanup. Requires the immutable overlay.
    #[cfg(target_os = "linux")]
    pub fn with_recovery_dispatch_gate(
        mut self,
        gate: std::sync::Arc<dyn super::RecoveryDispatchGate>,
    ) -> Result<Self, HermesAdapterError> {
        let recovery = self
            .recovery
            .as_mut()
            .ok_or_else(|| invalid("recovery overlay required"))?;
        if recovery.dispatch_gate.is_some() {
            return Err(invalid("recovery dispatch gate is immutable"));
        }
        recovery.dispatch_gate = Some(gate);
        Ok(self)
    }

    /// The trusted caller must resolve saved admission, validate original
    /// evidence and deterministic stage derivation, atomically consume a start,
    /// and reauthorize the stop/lease fence immediately before dispatch.
    /// This overlay does not grant any of those permissions.
    #[cfg(target_os = "linux")]
    pub fn with_recovery_window(
        mut self,
        task: &HermesNodeTask,
        grant: RecoveryGrant,
        memory: MnemosyneService,
    ) -> Result<Self, HermesAdapterError> {
        grant.validate().map_err(invalid)?;
        if self.recovery.is_some() {
            return Err(invalid("recovery overlay is immutable"));
        }
        let binding = self
            .retained
            .as_ref()
            .ok_or_else(|| invalid("recovery requires retained execution"))?;
        let expected_kind = if task.node.id == grant.bindings.verify_node_id {
            NodeKind::Verify
        } else if task.node.id == grant.bindings.review_node_id {
            NodeKind::Review
        } else {
            return Err(invalid("recovery cannot execute this node"));
        };
        let lease_expires_ms = u64::try_from(binding.lease.expires_ms)
            .map_err(|_| invalid("invalid recovery lease expiry"))?;
        if task.run_id != grant.bindings.run_id
            || binding.lease.run_id != task.run_id.as_str()
            || task.project_contract_digest != grant.bindings.project_contract_digest
            || task.node.kind != expected_kind
            || !matches!(
                (task.node.kind, task.node.authority),
                (NodeKind::Verify, AuthorityClass::Verify)
                    | (NodeKind::Review, AuthorityClass::ReadOnly)
            )
            || lease_expires_ms > grant.expires_at_unix_ms
            || !task.checks.is_empty()
            || !task.check_commands.is_empty()
            || !task.node.worker.as_ref().is_some_and(|worker| {
                worker.allowed_toolsets.len() == 1 && worker.allowed_toolsets.contains("file")
            })
        {
            return Err(invalid(
                "recovery task or lease differs from read-only scope",
            ));
        }
        self.recovery = Some(RecoveryExecution {
            dispatch_gate: None,
            grant,
            task_digest: task.authority_binding_digest()?,
            lease_expires_ms,
            memory,
        });
        self.preflight(task)?;
        Ok(self)
    }

    pub(super) fn recovery_deadline(
        &self,
        task: &HermesNodeTask,
    ) -> Result<Option<u128>, HermesAdapterError> {
        let Some(recovery) = &self.recovery else {
            return Ok(None);
        };
        let now = super::unix_time_ms()?;
        if !u64::try_from(now)
            .is_ok_and(|now| recovery.grant.is_active(now) && now < recovery.lease_expires_ms)
            || recovery.task_digest != task.authority_binding_digest()?
        {
            return Err(invalid("recovery window expired or task changed"));
        }
        let assembly = task
            .context_assembly
            .as_ref()
            .ok_or_else(|| invalid("recovery requires original bound context"))?;
        recovery
            .memory
            .validate_context_assembly_for_recovery(
                assembly,
                now,
                recovery.grant.activated_at_unix_ms.into(),
                recovery.grant.expires_at_unix_ms.into(),
            )
            .map_err(|_| invalid("recovery context failed current canonical validation"))?;
        Ok(Some(recovery.lease_expires_ms.into()))
    }
}

fn invalid(message: &str) -> HermesAdapterError {
    HermesAdapterError::InvalidTask(message.into())
}
