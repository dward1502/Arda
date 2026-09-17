//! Concrete final-delimiter authority for a server-constructed recovery task.
use super::RecoveryAuthorization;
use crate::adapters::{HermesAdapter, HermesAdapterError, HermesNodeTask, RecoveryDispatchGate};
use crate::objectives::{runtime_operation::RuntimeOperation, RetainedExecution};
use crate::runs::{AppendOutcome, RunEventDraft, RunStore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub struct RecoveryProviderDispatch {
    authority: Arc<RecoveryAuthorization>,
    request: serde_json::Value,
    chat: serde_json::Value,
    node: arda_core::run_graph::RunNode,
    start: RunEventDraft,
    sent: AtomicBool,
    result: Mutex<Option<(String, Vec<String>)>>,
}

impl RecoveryProviderDispatch {
    /// The authenticated Harness must construct the task (including instructions)
    /// with its ordinary canonical builder. Neither a task nor this gate is an
    /// ingress credential. Every delimiter resolves saved authority afresh.
    pub fn new(
        authority: Arc<RecoveryAuthorization>,
        request: serde_json::Value,
        task: &HermesNodeTask,
        adapter: &HermesAdapter,
        start: RunEventDraft,
    ) -> anyhow::Result<Self> {
        authority.validate_provider_request(task.node.id.as_str(), &request)?;
        anyhow::ensure!(
            task.run_id.as_str() == authority.lease.run_id
                && task.node.id == start.node_id
                && request.get("objective").and_then(|v| v.as_str())
                    == Some(task.objective.as_str())
                && request.get("context_assembly")
                    == Some(&serde_json::to_value(&task.context_assembly)?)
                && task.checks.is_empty()
                && task.check_commands.is_empty(),
            "recovery dispatch task differs from canonical request"
        );
        authority.store.with_retained_recovery(
            &authority.operator_id,
            &authority.event_id,
            &authority.lease,
            |tx, grant, _| {
                let material = authority.store.validate_retained_recovery_material(
                    &authority.root,
                    tx,
                    grant,
                )?;
                anyhow::ensure!(
                    task.project_contract_digest
                        == material.graph.provenance.project_contract_digest,
                    "recovery dispatch task changed captured project contract"
                );
                let mut expected = material
                    .graph
                    .nodes
                    .into_iter()
                    .find(|node| node.id == task.node.id)
                    .ok_or_else(|| anyhow::anyhow!("recovery node missing"))?;
                expected.state = arda_core::run_graph::NodeState::Ready;
                anyhow::ensure!(
                    serde_json::to_value(expected)? == serde_json::to_value(&task.node)?,
                    "recovery dispatch task changed captured node authority"
                );
                Ok(())
            },
        )?;
        let chat = adapter.retained_chat_operation(task)?;
        anyhow::ensure!(
            matches!(&chat, RuntimeOperation::Chat { workspace_writable: false, toolsets, .. }
            if toolsets.as_slice() == ["file"]),
            "recovery dispatch requires file-only read authority"
        );
        Ok(Self {
            authority,
            request,
            chat: serde_json::to_value(chat)?,
            node: task.node.clone(),
            start,
            sent: AtomicBool::new(false),
            result: Mutex::new(None),
        })
    }
}

fn refused() -> HermesAdapterError {
    HermesAdapterError::InvalidTask("recovery dispatch authority refused operation".into())
}

impl RecoveryDispatchGate for RecoveryProviderDispatch {
    fn bind_result(
        &self,
        session_id: &str,
        artifact_paths: &[String],
    ) -> Result<(), HermesAdapterError> {
        if !self.sent.load(Ordering::Acquire) {
            return Err(refused());
        }
        RuntimeOperation::Export {
            session_id: session_id.to_owned(),
        }
        .validate()
        .map_err(|_| refused())?;
        if !artifact_paths.is_empty() {
            RuntimeOperation::VerifyArtifacts {
                paths: artifact_paths.to_vec(),
            }
            .validate()
            .map_err(|_| refused())?;
        }
        let mut result = self.result.lock().map_err(|_| refused())?;
        if result.is_some() {
            return Err(refused());
        }
        *result = Some((session_id.to_owned(), artifact_paths.to_vec()));
        Ok(())
    }

    fn dispatch(
        &self,
        binding: &RetainedExecution,
        operation: &RuntimeOperation,
        delimiter: Box<dyn FnOnce() -> Result<(), HermesAdapterError> + '_>,
    ) -> Result<(), HermesAdapterError> {
        operation.validate().map_err(|_| refused())?;
        match operation {
            RuntimeOperation::Chat { .. } => {
                if serde_json::to_value(operation).map_err(|_| refused())? != self.chat {
                    return Err(refused());
                }
                let journal = RunStore::open(
                    &self.authority.root,
                    arda_core::run_graph::RunId::new(self.authority.lease.run_id.clone())
                        .map_err(|_| refused())?,
                )
                .map_err(|_| refused())?;
                let outcome = self
                    .authority
                    .launch_provider_request_checked(
                        &self.request,
                        binding,
                        &journal,
                        self.start.clone(),
                        Some(&self.node),
                        || {
                            delimiter().map_err(anyhow::Error::from)?;
                            self.sent.store(true, Ordering::Release);
                            Ok(())
                        },
                    )
                    .map_err(|_| refused())?;
                if !matches!(outcome, AppendOutcome::Appended { .. }) {
                    return Err(refused());
                }
                Ok(())
            }
            RuntimeOperation::Export { session_id } => {
                let result = self.result.lock().map_err(|_| refused())?;
                if !result
                    .as_ref()
                    .is_some_and(|(expected, _)| expected == session_id)
                {
                    return Err(refused());
                }
                drop(result);
                self.authority
                    .dispatch_provider_auxiliary(
                        &self.request,
                        binding,
                        &self.start,
                        &self.node,
                        || delimiter().map_err(anyhow::Error::from),
                    )
                    .map_err(|_| refused())
            }
            RuntimeOperation::VerifyArtifacts { paths } => {
                let result = self.result.lock().map_err(|_| refused())?;
                if !result
                    .as_ref()
                    .is_some_and(|(_, expected)| expected == paths)
                {
                    return Err(refused());
                }
                drop(result);
                self.authority
                    .dispatch_provider_auxiliary(
                        &self.request,
                        binding,
                        &self.start,
                        &self.node,
                        || delimiter().map_err(anyhow::Error::from),
                    )
                    .map_err(|_| refused())
            }
            RuntimeOperation::Probe {} => Err(refused()),
        }
    }
}
