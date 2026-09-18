use super::RecoveredRun;
use arda_core::council_run::CouncilRun;
use arda_core::run_graph::{
    CapabilityCompositionReceipt, CompositionTrigger, NodeId, NodeState, RunGraph, RunGraphError,
    RunId,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RunEventKind {
    Planned {
        project_id: String,
        approval_id: String,
    },
    NodeTransition {
        state: NodeState,
    },
    Cancelled {
        reason: String,
    },
    EvidenceLinked {
        evidence_id: String,
        evidence_path: String,
        authority: String,
    },
    CapabilityCompositionSelected {
        composition_digest: String,
        trigger: CompositionTrigger,
        selected_capability_count: usize,
    },
    GovernanceEvaluated {
        action_digest: String,
        verdict: String,
        approval_required: bool,
        transition: NodeState,
    },
    ResultProjected,
    RecoveryActivated {
        grant: Box<super::RecoveryGrant>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunEvent {
    pub schema_version: String,
    pub sequence: u64,
    pub run_id: RunId,
    pub node_id: NodeId,
    pub idempotency_key: String,
    pub kind: RunEventKind,
    pub receipt_digest: Option<String>,
    pub recorded_at_unix_ms: u128,
}

impl RunEvent {
    pub const SCHEMA_VERSION: &'static str = "arda.run-event.v1";
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunEventDraft {
    pub node_id: NodeId,
    pub idempotency_key: String,
    pub kind: RunEventKind,
    pub receipt_digest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppendOutcome {
    Appended { sequence: u64 },
    AlreadyApplied { sequence: u64 },
}

#[derive(Debug, Clone)]
pub struct RunStore {
    run_id: RunId,
    root: PathBuf,
    directory: PathBuf,
    mutation_lock: Option<std::sync::Arc<std::fs::File>>,
    mutation_node: Option<NodeId>,
    mutation_deadline: Option<u64>,
    provider_free_close: bool,
}

impl RunStore {
    pub fn open(root: impl AsRef<Path>, run_id: RunId) -> Result<Self, RunStoreError> {
        let root = root.as_ref().to_path_buf();
        let directory = root.join("data/runs").join(run_id.as_str());
        fs::create_dir_all(&directory).map_err(|source| RunStoreError::Io {
            path: directory.clone(),
            source,
        })?;
        Ok(Self {
            run_id,
            root,
            directory,
            mutation_lock: None,
            mutation_node: None,
            mutation_deadline: None,
            provider_free_close: false,
        })
    }

    pub fn run_id(&self) -> &RunId {
        &self.run_id
    }

    /// Owned exclusive journal guard for a synchronous, exact-node recovery
    /// mutation. Clones retain the same lock rather than reacquiring flock.
    /// Call only inside the canonical objective/lease transaction.
    pub(crate) fn lock_recovery_mutation(
        &self,
        grant: &super::RecoveryGrant,
        node: &NodeId,
        lease_deadline: u64,
    ) -> Result<Self, RunStoreError> {
        self.lock_recovery_projection(
            grant,
            node,
            Some(lease_deadline.min(grant.expires_at_unix_ms)),
        )
    }

    /// Only the committed-outbox reconciler may request this guard. It grants
    /// no provider starts and creates no new execution window.
    pub(crate) fn lock_recovery_reconciliation(
        &self,
        grant: &super::RecoveryGrant,
        node: &NodeId,
    ) -> Result<Self, RunStoreError> {
        self.lock_recovery_projection(grant, node, None)
    }

    /// Inspect terminal cleanup history without granting any mutation capability.
    /// Caller holds the objective writer fence; callback may commit bookkeeping.
    pub(crate) fn with_recovery_cleanup_history<T>(
        &self,
        grant: &super::RecoveryGrant,
        inspect: impl FnOnce(bool) -> anyhow::Result<T>,
    ) -> anyhow::Result<T> {
        use anyhow::Context;
        grant.validate().map_err(anyhow::Error::msg)?;
        anyhow::ensure!(self.run_id == grant.bindings.run_id, "cleanup run changed");
        let _lock = self.journal_lock(true)?;
        let recovered = self.recover_locked()?;
        let activations: Vec<_> = recovered
            .events
            .iter()
            .filter_map(|event| match &event.kind {
                RunEventKind::RecoveryActivated { grant } => Some(grant.as_ref()),
                _ => None,
            })
            .collect();
        anyhow::ensure!(activations == vec![grant], "cleanup activation changed");
        let graph = recovered
            .checkpoint
            .as_ref()
            .context("cleanup canonical checkpoint missing")?;
        let receipt: crate::adapters::HermesExecutionReceipt = serde_json::from_value(
            self.read_execution_receipt(&NodeId::new("execute")?)?
                .context("cleanup Execute receipt missing")?,
        )?;
        super::recovery_evidence::validate_historical_lineage(
            graph,
            &recovered.events,
            &receipt,
            &grant.bindings,
        )?;
        let cancelled = recovered.events.iter().any(|event| {
            matches!(
                event.kind,
                RunEventKind::Cancelled { .. }
                    | RunEventKind::NodeTransition {
                        state: NodeState::Cancelled
                    }
            )
        });
        inspect(cancelled)
    }

    fn lock_recovery_projection(
        &self,
        grant: &super::RecoveryGrant,
        node: &NodeId,
        deadline: Option<u64>,
    ) -> Result<Self, RunStoreError> {
        let lock = self.journal_lock(true)?;
        let mut guarded = self.clone();
        guarded.mutation_lock = Some(lock);
        guarded.mutation_node = Some(node.clone());
        guarded.mutation_deadline = deadline;
        guarded.check_mutation_clock()?;
        let graph = guarded
            .validate_recovery_run_evidence(&grant.bindings)
            .map_err(|error| RunStoreError::InvalidRecoveryGrant(error.to_string()))?;
        let events = guarded.recover_locked()?.events;
        if !events.iter().any(|event| {
            matches!(&event.kind,
            RunEventKind::RecoveryActivated { grant: saved } if saved.as_ref() == grant)
        }) || events
            .iter()
            .any(|event| matches!(event.kind, RunEventKind::Cancelled { .. }))
            || ![
                &grant.bindings.verify_node_id,
                &grant.bindings.review_node_id,
                &grant.bindings.close_node_id,
            ]
            .contains(&node)
        {
            return Err(RunStoreError::InvalidRecoveryGrant(
                "recovery terminal authority changed or run cancelled".into(),
            ));
        }
        if node == &grant.bindings.close_node_id {
            let close = graph.nodes.iter().find(|candidate| &candidate.id == node);
            let review = graph
                .nodes
                .iter()
                .find(|candidate| candidate.id == grant.bindings.review_node_id);
            if !graph.nodes.iter().any(|candidate| {
                candidate.id == grant.bindings.verify_node_id
                    && candidate.state == NodeState::Succeeded
                    && candidate.output_digest.is_some()
            }) || !close.is_some_and(|close| {
                close.worker.is_none()
                    && review
                        .and_then(|review| review.output_digest.as_ref())
                        .is_some_and(|digest| close.parent_receipts == [digest.clone()])
            }) || !review.is_some_and(|review| {
                review.state == NodeState::Succeeded && review.output_digest.is_some()
            }) {
                return Err(RunStoreError::InvalidRecoveryGrant(
                    "provider-free Close requires completed Review".into(),
                ));
            }
            guarded.provider_free_close = true;
        }
        Ok(guarded)
    }

    fn check_mutation_clock(&self) -> Result<(), RunStoreError> {
        if self.mutation_deadline.is_some_and(|deadline| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(true, |now| now.as_millis() >= u128::from(deadline))
        }) {
            return Err(RunStoreError::InvalidRecoveryGrant(
                "recovery terminal deadline expired".into(),
            ));
        }
        Ok(())
    }

    pub fn events_path(&self) -> PathBuf {
        self.directory.join("events.jsonl")
    }

    pub fn checkpoint_path(&self) -> PathBuf {
        self.directory.join("checkpoint.json")
    }

    pub fn result_path(&self) -> PathBuf {
        self.directory.join("result.json")
    }

    pub fn execution_receipt_path(&self, node_id: &NodeId) -> PathBuf {
        self.directory
            .join("execution-receipts")
            .join(format!("{}.json", node_id.as_str()))
    }

    pub fn composition_receipt_path(&self) -> PathBuf {
        self.directory.join("capability-composition.json")
    }

    pub fn governance_receipts_path(&self) -> PathBuf {
        self.directory.join("governance-receipts.jsonl")
    }

    pub fn resource_ledger_path(&self) -> PathBuf {
        self.root.join("data/resource-ledger/events.jsonl")
    }

    pub fn council_run_path(&self) -> PathBuf {
        self.directory.join("council-run.json")
    }

    pub fn composition_receipt_archive_path(&self, receipt_digest: &str) -> PathBuf {
        self.directory
            .join("capability-composition-receipts")
            .join(format!("{receipt_digest}.json"))
    }

    pub fn append(&self, draft: RunEventDraft) -> Result<AppendOutcome, RunStoreError> {
        if matches!(draft.kind, RunEventKind::RecoveryActivated { .. })
            || draft.idempotency_key == "bounded-recovery-window"
        {
            return Err(RunStoreError::IdempotencyConflict {
                key: draft.idempotency_key,
            });
        }
        let _lock = self.journal_lock(true)?;
        if matches!(
            draft.kind,
            RunEventKind::NodeTransition {
                state: NodeState::Running
            }
        ) && !(self.provider_free_close && self.mutation_node.as_ref() == Some(&draft.node_id))
            && self
                .recover_locked()?
                .events
                .iter()
                .any(|event| matches!(event.kind, RunEventKind::RecoveryActivated { .. }))
        {
            return Err(RunStoreError::IdempotencyConflict {
                key: draft.idempotency_key,
            });
        }
        self.append_locked(draft)
    }

    fn journal_lock(
        &self,
        exclusive: bool,
    ) -> Result<std::sync::Arc<std::fs::File>, RunStoreError> {
        if let Some(lock) = &self.mutation_lock {
            return Ok(lock.clone());
        }
        let path = self.directory.join("journal.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|source| RunStoreError::Io {
                path: path.clone(),
                source,
            })?;
        if exclusive {
            fs2::FileExt::lock_exclusive(&file)
        } else {
            fs2::FileExt::lock_shared(&file)
        }
        .map_err(|source| RunStoreError::Io { path, source })?;
        Ok(std::sync::Arc::new(file))
    }

    /// Journal one immutable recovery window per run. Authentication and binding
    /// validation must occur in the canonical operator admission path first.
    /// Exact replay returns the ORIGINAL window, including after expiration.
    pub fn activate_recovery(
        &self,
        grant: super::RecoveryGrant,
    ) -> Result<super::RecoveryGrant, RunStoreError> {
        let conflict = || RunStoreError::IdempotencyConflict {
            key: "bounded-recovery-window".into(),
        };
        grant.validate().map_err(|_| conflict())?;
        if grant.bindings.run_id != self.run_id {
            return Err(conflict());
        }
        let _lock = self.journal_lock(true)?;
        let recovered = self.recover_locked()?;
        if let Some(prior) = recovered.events.iter().find_map(|event| {
            if let RunEventKind::RecoveryActivated { grant } = &event.kind {
                Some(grant.as_ref())
            } else {
                None
            }
        }) {
            if prior.authenticated_event_id != grant.authenticated_event_id
                || prior.authenticated_payload_digest != grant.authenticated_payload_digest
                || prior.bindings != grant.bindings
            {
                return Err(conflict());
            }
            return Ok(prior.clone());
        }
        self.append_locked(RunEventDraft {
            node_id: grant.bindings.verify_node_id.clone(),
            idempotency_key: "bounded-recovery-window".into(),
            kind: RunEventKind::RecoveryActivated {
                grant: Box::new(grant.clone()),
            },
            receipt_digest: Some(grant.authenticated_payload_digest.clone()),
        })?;
        Ok(grant)
    }

    /// Count and consume a recovery provider start in the same journal critical
    /// section. Callers must already have checked current canonical authority.
    /// Only `Appended` permits dispatch: an exact replay MUST NOT launch again.
    pub fn append_recovery_start(
        &self,
        expected_grant: &super::RecoveryGrant,
        draft: RunEventDraft,
    ) -> Result<AppendOutcome, RunStoreError> {
        self.append_recovery_start_before(expected_grant, draft, expected_grant.expires_at_unix_ms)
    }

    /// Additionally cap admission by a lease deadline, sampled after the
    /// journal lock is acquired. Replay is a receipt, never dispatch permission.
    pub fn append_recovery_start_before(
        &self,
        expected_grant: &super::RecoveryGrant,
        draft: RunEventDraft,
        not_after_unix_ms: u64,
    ) -> Result<AppendOutcome, RunStoreError> {
        self.append_recovery_start_with_clock(expected_grant, draft, || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
                .filter(|now| *now < not_after_unix_ms)
        })
    }

    pub(super) fn append_recovery_start_with_clock(
        &self,
        expected_grant: &super::RecoveryGrant,
        draft: RunEventDraft,
        clock: impl FnOnce() -> Option<u64>,
    ) -> Result<AppendOutcome, RunStoreError> {
        self.recovery_start_then_with_clock(expected_grant, draft, clock, || ())
            .map(|(outcome, _)| outcome)
    }

    /// Keep journal cancellation serialized through a synchronous, one-shot
    /// delimiter send. Caller must hold the objective/lease fence; the callback
    /// must not await or reenter this store. A failed send never refunds a start.
    pub fn with_recovery_start_before<T>(
        &self,
        grant: &super::RecoveryGrant,
        draft: RunEventDraft,
        not_after_unix_ms: u64,
        send: impl FnOnce() -> T,
    ) -> Result<(AppendOutcome, Option<T>), RunStoreError> {
        let key = draft.idempotency_key.clone();
        let fresh = || {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|duration| u64::try_from(duration.as_millis()).ok())
                .filter(|now| *now < not_after_unix_ms && grant.is_active(*now))
        };
        let (outcome, result) = self.recovery_start_then_with_clock(grant, draft, fresh, || {
            // fsync may outlast the lease/grant; refusal still spends the start.
            fresh().ok_or(RunStoreError::IdempotencyConflict { key })?;
            Ok::<T, RunStoreError>(send())
        })?;
        Ok((outcome, result.transpose()?))
    }

    /// Reauthorize an auxiliary operation under the same journal fence as its
    /// already-launched Chat. This never appends or consumes a provider start.
    /// The caller owns the objective/lease fence and must not reenter this store.
    pub fn with_recovery_auxiliary_before<T>(
        &self,
        grant: &super::RecoveryGrant,
        start: &RunEventDraft,
        not_after_unix_ms: u64,
        send: impl FnOnce() -> T,
    ) -> Result<T, RunStoreError> {
        let conflict = || RunStoreError::IdempotencyConflict {
            key: start.idempotency_key.clone(),
        };
        let _lock = self.journal_lock(true)?;
        let recovered = self.recover_locked()?;
        let activation_matches = recovered.events.iter().any(|event| {
            matches!(&event.kind, RunEventKind::RecoveryActivated { grant: saved }
                if saved.as_ref() == grant)
        });
        let start_matches = recovered.events.iter().any(|event| {
            event.idempotency_key == start.idempotency_key
                && event.node_id == start.node_id
                && event.kind == start.kind
                && event.receipt_digest == start.receipt_digest
        });
        let cancelled = recovered.events.iter().any(|event| {
            matches!(
                event.kind,
                RunEventKind::Cancelled { .. }
                    | RunEventKind::NodeTransition {
                        state: NodeState::Cancelled
                    }
            )
        });
        let latest = recovered.events.iter().rev().find_map(|event| {
            if event.node_id == start.node_id {
                if let RunEventKind::NodeTransition { state } = event.kind {
                    return Some((state, event.idempotency_key.as_str()));
                }
            }
            None
        });
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok());
        if !activation_matches
            || !start_matches
            || cancelled
            || !matches!(
                start.kind,
                RunEventKind::NodeTransition {
                    state: NodeState::Running
                }
            )
            || latest != Some((NodeState::Running, start.idempotency_key.as_str()))
            || !grant
                .bindings
                .provider_start_ceilings
                .contains_key(start.node_id.as_str())
            || !now.is_some_and(|now| now < not_after_unix_ms && grant.is_active(now))
        {
            return Err(conflict());
        }
        Ok(send())
    }

    pub(super) fn recovery_start_then_with_clock<T>(
        &self,
        expected_grant: &super::RecoveryGrant,
        draft: RunEventDraft,
        clock: impl FnOnce() -> Option<u64>,
        send: impl FnOnce() -> T,
    ) -> Result<(AppendOutcome, Option<T>), RunStoreError> {
        let conflict = || RunStoreError::IdempotencyConflict {
            key: draft.idempotency_key.clone(),
        };
        if !matches!(
            draft.kind,
            RunEventKind::NodeTransition {
                state: NodeState::Running
            }
        ) {
            return Err(conflict());
        }
        let _lock = self.journal_lock(true)?;
        let recovered = self.recover_locked()?;
        let grant = recovered
            .events
            .iter()
            .find_map(|event| match &event.kind {
                RunEventKind::RecoveryActivated { grant } => Some(grant.as_ref()),
                _ => None,
            })
            .ok_or_else(conflict)?;
        if grant != expected_grant {
            return Err(conflict());
        }
        if let Some(event) = recovered
            .events
            .iter()
            .find(|event| event.idempotency_key == draft.idempotency_key)
        {
            if event.node_id != draft.node_id
                || event.kind != draft.kind
                || event.receipt_digest != draft.receipt_digest
            {
                return Err(conflict());
            }
            return Ok((
                AppendOutcome::AlreadyApplied {
                    sequence: event.sequence,
                },
                None,
            ));
        }
        if recovered.events.iter().any(|event| {
            matches!(
                event.kind,
                RunEventKind::Cancelled { .. }
                    | RunEventKind::NodeTransition {
                        state: NodeState::Cancelled
                    }
            )
        }) {
            return Err(conflict());
        }
        let starts = recovered
            .events
            .iter()
            .filter(|event| {
                event.node_id == draft.node_id
                    && matches!(
                        event.kind,
                        RunEventKind::NodeTransition {
                            state: NodeState::Running
                        }
                    )
            })
            .count();
        let next = u64::try_from(starts)
            .ok()
            .and_then(|starts| starts.checked_add(1))
            .ok_or_else(conflict)?;
        let latest_state = recovered.events.iter().rev().find_map(|event| {
            if event.node_id != draft.node_id {
                return None;
            }
            match event.kind {
                RunEventKind::NodeTransition { state } => Some(state),
                _ => None,
            }
        });
        let already_succeeded = recovered.events.iter().any(|event| {
            event.node_id == draft.node_id
                && matches!(
                    event.kind,
                    RunEventKind::NodeTransition {
                        state: NodeState::Succeeded
                    }
                )
        });
        if already_succeeded
            || latest_state != Some(NodeState::Ready)
            || !clock().is_some_and(|now| grant.permits_start(&draft.node_id, next, now))
        {
            return Err(conflict());
        }
        let outcome = self.append_locked(draft)?;
        let result = matches!(outcome, AppendOutcome::Appended { .. }).then(send);
        Ok((outcome, result))
    }

    fn append_locked(&self, draft: RunEventDraft) -> Result<AppendOutcome, RunStoreError> {
        self.check_mutation_clock()?;
        if self
            .mutation_node
            .as_ref()
            .is_some_and(|node| node != &draft.node_id)
        {
            return Err(RunStoreError::InvalidRecoveryGrant(
                "terminal mutation changed node".into(),
            ));
        }
        if draft.idempotency_key.trim().is_empty() {
            return Err(RunStoreError::EmptyIdempotencyKey);
        }
        let recovered = self.recover_locked()?;
        if let Some(existing) = recovered
            .events
            .iter()
            .find(|event| event.idempotency_key == draft.idempotency_key)
        {
            if existing.node_id != draft.node_id
                || existing.kind != draft.kind
                || existing.receipt_digest != draft.receipt_digest
            {
                return Err(RunStoreError::IdempotencyConflict {
                    key: draft.idempotency_key,
                });
            }
            return Ok(AppendOutcome::AlreadyApplied {
                sequence: existing.sequence,
            });
        }

        let sequence = recovered.events.len() as u64 + 1;
        let event = RunEvent {
            schema_version: RunEvent::SCHEMA_VERSION.to_string(),
            sequence,
            run_id: self.run_id.clone(),
            node_id: draft.node_id,
            idempotency_key: draft.idempotency_key,
            kind: draft.kind,
            receipt_digest: draft.receipt_digest,
            recorded_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis(),
        };
        let mut bytes = serde_json::to_vec(&event).map_err(RunStoreError::Serialize)?;
        bytes.push(b'\n');
        let path = self.events_path();
        let mut journal = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|source| RunStoreError::Io {
                path: path.clone(),
                source,
            })?;
        journal
            .write_all(&bytes)
            .and_then(|_| journal.sync_all())
            .map_err(|source| RunStoreError::Io { path, source })?;
        Ok(AppendOutcome::Appended { sequence })
    }

    pub fn recover(&self) -> Result<RecoveredRun, RunStoreError> {
        let _lock = self.journal_lock(false)?;
        self.recover_locked()
    }

    fn recover_locked(&self) -> Result<RecoveredRun, RunStoreError> {
        let path = self.events_path();
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(RunStoreError::Io {
                    path: path.clone(),
                    source,
                });
            }
        };
        if !raw.is_empty() && !raw.ends_with('\n') {
            return Err(RunStoreError::CorruptJournal {
                line: raw.lines().count(),
                message: "journal tail is not newline-terminated".to_string(),
            });
        }

        let mut events = Vec::new();
        let mut recovery_seen = false;
        for (index, line) in raw.lines().enumerate() {
            let line_number = index + 1;
            let event: RunEvent =
                serde_json::from_str(line).map_err(|error| RunStoreError::CorruptJournal {
                    line: line_number,
                    message: error.to_string(),
                })?;
            let expected = line_number as u64;
            if event.sequence != expected {
                return Err(RunStoreError::SequenceGap {
                    expected,
                    actual: event.sequence,
                });
            }
            if event.schema_version != RunEvent::SCHEMA_VERSION {
                return Err(RunStoreError::UnsupportedEventVersion(event.schema_version));
            }
            if event.run_id != self.run_id {
                return Err(RunStoreError::RunIdMismatch {
                    expected: self.run_id.clone(),
                    actual: event.run_id,
                });
            }
            if let RunEventKind::RecoveryActivated { grant } = &event.kind {
                if recovery_seen
                    || grant.validate().is_err()
                    || grant.bindings.run_id != self.run_id
                    || event.node_id != grant.bindings.verify_node_id
                    || event.idempotency_key != "bounded-recovery-window"
                    || event.receipt_digest.as_ref() != Some(&grant.authenticated_payload_digest)
                {
                    return Err(RunStoreError::CorruptJournal {
                        line: line_number,
                        message: "invalid or duplicate recovery activation".into(),
                    });
                }
                recovery_seen = true;
            } else if event.idempotency_key == "bounded-recovery-window" {
                return Err(RunStoreError::CorruptJournal {
                    line: line_number,
                    message: "reserved recovery activation key".into(),
                });
            }
            events.push(event);
        }

        let checkpoint_path = self.checkpoint_path();
        let checkpoint = match fs::read_to_string(&checkpoint_path) {
            Ok(raw) => Some(serde_json::from_str::<RunGraph>(&raw).map_err(|error| {
                RunStoreError::CorruptCheckpoint {
                    path: checkpoint_path,
                    message: error.to_string(),
                }
            })?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => {
                return Err(RunStoreError::Io {
                    path: checkpoint_path,
                    source,
                });
            }
        };
        Ok(RecoveredRun::from_parts(events, checkpoint))
    }

    pub fn write_checkpoint(&self, graph: &RunGraph) -> Result<(), RunStoreError> {
        self.check_mutation_clock()?;
        graph.validate().map_err(RunStoreError::Graph)?;
        let bytes = serde_json::to_vec_pretty(graph).map_err(RunStoreError::Serialize)?;
        atomic_write(&self.checkpoint_path(), &bytes)
    }

    pub fn write_result(&self, result: &serde_json::Value) -> Result<(), RunStoreError> {
        let bytes = serde_json::to_vec_pretty(result).map_err(RunStoreError::Serialize)?;
        atomic_write(&self.result_path(), &bytes)
    }

    pub fn read_result(&self) -> Result<Option<serde_json::Value>, RunStoreError> {
        let path = self.result_path();
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(RunStoreError::Serialize),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(RunStoreError::Io { path, source }),
        }
    }

    pub(crate) fn write_recovery_outcome_evidence(
        &self,
        node: &NodeId,
        payload: &Value,
    ) -> anyhow::Result<()> {
        let key = payload["start_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("outcome evidence start missing"))?;
        let _guard = self.journal_lock(true)?;
        let path = self.recovery_outcome_path(node, key);
        if path.exists() {
            let existing: Value = serde_json::from_slice(&fs::read(&path)?)?;
            anyhow::ensure!(
                existing == *payload,
                "conflicting immutable outcome evidence"
            );
            return Ok(());
        }
        fs::create_dir_all(path.parent().expect("evidence has parent"))?;
        atomic_write(&path, &serde_json::to_vec_pretty(payload)?)?;
        Ok(())
    }

    pub(crate) fn read_recovery_outcome_evidence(
        &self,
        node: &NodeId,
        start_key: &str,
    ) -> anyhow::Result<Option<Value>> {
        let _guard = self.journal_lock(false)?;
        let path = self.recovery_outcome_path(node, start_key);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&fs::read(path)?)?))
    }

    fn recovery_outcome_path(&self, node: &NodeId, key: &str) -> PathBuf {
        let digest = <sha2::Sha256 as sha2::Digest>::digest(key.as_bytes());
        self.directory
            .join("recovery-outcomes")
            .join(node.as_str())
            .join(format!("{digest:x}.json"))
    }

    pub fn write_execution_receipt(
        &self,
        node_id: &NodeId,
        receipt: &serde_json::Value,
    ) -> Result<(), RunStoreError> {
        let path = self.execution_receipt_path(node_id);
        self.check_mutation_clock()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RunStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let bytes = serde_json::to_vec_pretty(receipt).map_err(RunStoreError::Serialize)?;
        atomic_write(&path, &bytes)
    }

    pub fn read_execution_receipt(
        &self,
        node_id: &NodeId,
    ) -> Result<Option<serde_json::Value>, RunStoreError> {
        let path = self.execution_receipt_path(node_id);
        match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str(&raw)
                .map(Some)
                .map_err(RunStoreError::Serialize),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(RunStoreError::Io { path, source }),
        }
    }

    pub fn write_council_run(
        &self,
        council: &CouncilRun,
        graph: &RunGraph,
    ) -> Result<String, RunStoreError> {
        council
            .validate(graph)
            .map_err(|error| RunStoreError::InvalidCouncilRun(error.to_string()))?;
        if council.run_id != self.run_id.as_str() {
            return Err(RunStoreError::CouncilRunIdMismatch {
                expected: self.run_id.as_str().to_string(),
                actual: council.run_id.clone(),
            });
        }
        let digest = council
            .stable_digest()
            .map_err(|error| RunStoreError::InvalidCouncilRun(error.to_string()))?;
        let bytes = serde_json::to_vec_pretty(council).map_err(RunStoreError::Serialize)?;
        atomic_write(&self.council_run_path(), &bytes)?;
        Ok(digest)
    }

    pub fn read_council_run(&self) -> Result<Option<CouncilRun>, RunStoreError> {
        let path = self.council_run_path();
        let council = match fs::read_to_string(&path) {
            Ok(raw) => {
                serde_json::from_str::<CouncilRun>(&raw).map_err(RunStoreError::Serialize)?
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(RunStoreError::Io { path, source }),
        };
        if council.run_id != self.run_id.as_str() {
            return Err(RunStoreError::CouncilRunIdMismatch {
                expected: self.run_id.as_str().to_string(),
                actual: council.run_id,
            });
        }
        Ok(Some(council))
    }

    pub fn write_composition_receipt(
        &self,
        receipt: &CapabilityCompositionReceipt,
    ) -> Result<String, RunStoreError> {
        if receipt.schema_version != CapabilityCompositionReceipt::SCHEMA_VERSION {
            return Err(RunStoreError::UnsupportedCompositionReceiptVersion(
                receipt.schema_version.clone(),
            ));
        }
        if receipt.run_id != self.run_id.as_str() {
            return Err(RunStoreError::CompositionReceiptRunIdMismatch {
                expected: self.run_id.as_str().to_string(),
                actual: receipt.run_id.clone(),
            });
        }
        let digest = receipt
            .digest()
            .map_err(|error| RunStoreError::InvalidCompositionReceipt(error.to_string()))?;
        let bytes = serde_json::to_vec_pretty(receipt).map_err(RunStoreError::Serialize)?;
        let archive_path = self.composition_receipt_archive_path(&digest);
        if let Some(parent) = archive_path.parent() {
            fs::create_dir_all(parent).map_err(|source| RunStoreError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        atomic_write(&archive_path, &bytes)?;
        atomic_write(&self.composition_receipt_path(), &bytes)?;
        Ok(digest)
    }

    pub fn read_composition_receipt(
        &self,
    ) -> Result<Option<CapabilityCompositionReceipt>, RunStoreError> {
        let path = self.composition_receipt_path();
        let receipt = match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<CapabilityCompositionReceipt>(&raw)
                .map_err(RunStoreError::Serialize)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(RunStoreError::Io { path, source }),
        };
        if receipt.schema_version != CapabilityCompositionReceipt::SCHEMA_VERSION {
            return Err(RunStoreError::UnsupportedCompositionReceiptVersion(
                receipt.schema_version,
            ));
        }
        if receipt.run_id != self.run_id.as_str() {
            return Err(RunStoreError::CompositionReceiptRunIdMismatch {
                expected: self.run_id.as_str().to_string(),
                actual: receipt.run_id,
            });
        }
        Ok(Some(receipt))
    }
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), RunStoreError> {
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|source| RunStoreError::Io {
                path: temporary.clone(),
                source,
            })?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|source| RunStoreError::Io {
                path: temporary.clone(),
                source,
            })?;
        drop(file);
        fs::rename(&temporary, path).map_err(|source| RunStoreError::Io {
            path: path.to_path_buf(),
            source,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[derive(Debug, thiserror::Error)]
pub enum RunStoreError {
    #[error("invalid recovery terminal mutation: {0}")]
    InvalidRecoveryGrant(String),
    #[error("run store I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize run data: {0}")]
    Serialize(serde_json::Error),
    #[error("corrupt run journal at line {line}: {message}")]
    CorruptJournal { line: usize, message: String },
    #[error("run journal sequence gap: expected {expected}, got {actual}")]
    SequenceGap { expected: u64, actual: u64 },
    #[error("unsupported run event version: {0}")]
    UnsupportedEventVersion(String),
    #[error("journal run id mismatch: expected {expected:?}, got {actual:?}")]
    RunIdMismatch { expected: RunId, actual: RunId },
    #[error("corrupt checkpoint at {path}: {message}")]
    CorruptCheckpoint { path: PathBuf, message: String },
    #[error("idempotency key cannot be empty")]
    EmptyIdempotencyKey,
    #[error("idempotency key {key:?} was reused for a different durable mutation")]
    IdempotencyConflict { key: String },
    #[error("run graph validation failed: {0}")]
    Graph(#[source] RunGraphError),
    #[error("unsupported capability composition receipt version: {0}")]
    UnsupportedCompositionReceiptVersion(String),
    #[error("capability composition receipt run id mismatch: expected {expected}, got {actual}")]
    CompositionReceiptRunIdMismatch { expected: String, actual: String },
    #[error("invalid capability composition receipt: {0}")]
    InvalidCompositionReceipt(String),
    #[error("invalid council run: {0}")]
    InvalidCouncilRun(String),
    #[error("council run id mismatch: expected {expected}, got {actual}")]
    CouncilRunIdMismatch { expected: String, actual: String },
}
