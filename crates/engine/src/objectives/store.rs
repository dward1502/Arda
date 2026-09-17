use super::migrations;
use super::model::{
    ClaimedLeaf, ControlAction, LeafExecutionSpec, LeafRecord, LeafStage, NewObjective,
    ObjectiveRecord, ObjectiveState, ReceiptStage, ScheduleSpec, StageReceipt,
};
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{
    params, Connection, OpenFlags, OptionalExtension, Transaction, TransactionBehavior,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;
use tokio::sync::watch;

mod admissions;
mod authority;
mod recovery;
pub use recovery::RecoveryMaterial;

#[derive(Clone)]
pub struct ObjectiveStore {
    path: PathBuf,
    authority: authority::Authority,
    changes: Arc<watch::Sender<()>>,
    pub(super) snapshot_admission: Option<Arc<dyn super::snapshots::SnapshotAdmission>>,
}

impl std::fmt::Debug for ObjectiveStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ObjectiveStore")
            .field("path", &self.path)
            .field("snapshot_admission", &self.snapshot_admission.is_some())
            .finish_non_exhaustive()
    }
}

// Store instances opened by HTTP ingress and the resident loop share a wakeup.
// SQLite remains authoritative; cross-process writes use bounded fallback polls.
type StoreWakeups = Mutex<HashMap<PathBuf, Weak<watch::Sender<()>>>>;
static STORE_WAKEUPS: OnceLock<StoreWakeups> = OnceLock::new();

const MAX_LEAF_ATTEMPTS: i64 = MAX_OBJECTIVE_ATTEMPTS;
pub const MAX_OBJECTIVE_ATTEMPTS: i64 = 5;

impl ObjectiveStore {
    pub fn with_snapshot_admission(
        mut self,
        keeper: Arc<dyn super::snapshots::SnapshotAdmission>,
    ) -> Self {
        self.snapshot_admission = Some(keeper);
        self
    }

    pub fn execution_workspace_identity(&self, run_id: &str) -> Result<Option<String>> {
        let identity: Option<Option<String>> = self.connection()?.query_row(
            "SELECT i.identity_json FROM leaves l LEFT JOIN lease_workspace_identities i ON i.leaf_id = l.id WHERE l.execution_run_id = ?1",
            [run_id], |row| row.get(0),
        ).optional()?;
        match identity {
            Some(None) => bail!("resident execution workspace identity is unknown"),
            Some(Some(identity)) => Ok(Some(identity)),
            None => Ok(None),
        }
    }

    /// Explicit offline provisioning or legacy adoption. Runtime callers must
    /// use `open_existing`; loss of a provisioned database is never reset here.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::provision(path.as_ref(), false)
    }

    /// Provision a new database only; never adopt or reset an existing one.
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self> {
        Self::provision(path.as_ref(), true)
    }

    fn provision(path: &Path, new_only: bool) -> Result<Self> {
        let path = path.to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create ObjectiveStore directory {}", parent.display()))?;
        }
        let path = authority::normalize(&path)?;
        authority::provision(&path, new_only)?;
        Self::open_existing(path)
    }

    /// Reopen provisioned authority without creating a database or marker.
    pub fn open_existing(path: impl AsRef<Path>) -> Result<Self> {
        let path = authority::normalize(path.as_ref())?;
        let authority = authority::Authority::load(&path)?;
        let mut store = Self {
            path,
            authority,
            changes: Arc::new(watch::channel(()).0),
            snapshot_admission: None,
        };
        let connection = store.connection()?;
        migrations::apply(&connection)?;
        store.path = std::fs::canonicalize(&store.path).context("resolve ObjectiveStore path")?;
        let mut wakeups = STORE_WAKEUPS
            .get_or_init(Default::default)
            .lock()
            .map_err(|_| anyhow!("objective wake registry poisoned"))?;
        wakeups.retain(|_, sender| sender.strong_count() > 0);
        if let Some(sender) = wakeups.get(&store.path).and_then(Weak::upgrade) {
            store.changes = sender;
        } else {
            wakeups.insert(store.path.clone(), Arc::downgrade(&store.changes));
        }
        Ok(store)
    }

    pub(crate) fn subscribe_changes(&self) -> watch::Receiver<()> {
        self.changes.subscribe()
    }

    /// Bind every gateway command family before side effects, including retries
    /// after a crash between command application and transport-ledger commit.
    pub fn bind_gateway_event(&self, event_id: &str, payload_digest: &str) -> Result<()> {
        if event_id.is_empty() || payload_digest.is_empty() {
            bail!("gateway event identity and payload digest are required");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
                [event_id],
                |row| row.get(0),
            )
            .optional()?;
        match existing {
            Some(existing) if existing != payload_digest => {
                bail!("gateway event replay changed payload or command family")
            }
            Some(_) => (),
            None => {
                transaction.execute(
                    "INSERT INTO gateway_event_bindings(event_id,payload_digest) VALUES(?1,?2)",
                    params![event_id, payload_digest],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    /// Unfinished prior attempts in schedulable objectives, including live
    /// leases and unbound legacy attempts. An empty claim batch cannot prove
    /// these have been reconciled. This is observation, not admission authority.
    pub(crate) fn pending_recovery(&self) -> Result<u64> {
        self.connection()?
            .query_row(
                "SELECT COUNT(*) FROM leaves l JOIN objectives o ON o.id = l.objective_id
             WHERE o.state IN ('approved', 'running') AND l.attempt > 0
               AND l.stage NOT IN ('complete', 'cancelled', 'failed')",
                [],
                |row| row.get(0),
            )
            .context("count unresolved resident attempts")
    }

    fn notify_change(&self) {
        self.changes.send_replace(());
    }

    pub(crate) fn resident_context(
        &self,
        run_id: &str,
        request_digest: &str,
    ) -> Result<Option<arda_vaire::ContextAssembly>> {
        let row: Option<(String, String, Option<i64>)> = self
            .connection()?
            .query_row(
                "SELECT request_digest, assembly_json, deleted_by_operator_ms
             FROM resident_context_bindings WHERE run_id = ?1",
                [run_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .optional()?;
        row.map(|(digest, json, deleted_at)| {
            if digest != request_digest {
                bail!("resident run binding conflicts with claimed authority");
            }
            if deleted_at.is_some() {
                bail!("resident context deleted by operator; reconciliation required");
            }
            Ok(serde_json::from_str(&json)?)
        })
        .transpose()
    }

    /// Records a deny-reuse marker only; does not erase recovery evidence.
    /// Test-only marker fixture; production deletion goes through apply_control.
    #[cfg(test)]
    pub(crate) fn resident_context_deleted_by_operator(&self, run_id: &str) -> Result<()> {
        let now_ms = i64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("get current time")?
                .as_millis(),
        )
        .context("operator marker timestamp overflow")?;
        let changed = self.connection()?.execute(
            "UPDATE resident_context_bindings
             SET deleted_by_operator_ms = COALESCE(deleted_by_operator_ms, ?1) WHERE run_id = ?2",
            params![now_ms, run_id],
        )?;
        if changed != 1 {
            bail!("cannot mark missing resident context binding");
        }
        Ok(())
    }

    pub(crate) fn can_prepare_resident_context(&self, claim: &ClaimedLeaf) -> Result<bool> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves WHERE id = ?1 AND execution_run_id = ?2
             AND lease_owner = ?3 AND attempt = ?4 AND context_bound = 0)",
            params![
                claim.leaf_id,
                claim.execution_run_id,
                claim.lease_owner,
                claim.attempt
            ],
            |row| row.get(0),
        )?)
    }

    pub(crate) fn bind_resident_context(
        &self,
        claim: &ClaimedLeaf,
        request_digest: &str,
        assembly: &arda_vaire::ContextAssembly,
    ) -> Result<arda_vaire::ContextAssembly> {
        let run_id = claim
            .execution_run_id
            .as_deref()
            .context("missing execution identity")?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let changed = transaction.execute(
            "UPDATE leaves SET context_bound = 1 WHERE id = ?1 AND execution_run_id = ?2
             AND lease_owner = ?3 AND attempt = ?4 AND context_bound = 0",
            params![claim.leaf_id, run_id, claim.lease_owner, claim.attempt],
        )?;
        if changed != 1 {
            bail!("context preparation no longer owns an unbound claim; reconciliation required");
        }
        transaction.execute(
            "INSERT INTO resident_context_bindings(run_id, request_digest, assembly_json, deleted_by_operator_ms)
             VALUES (?1, ?2, ?3, NULL) ON CONFLICT(run_id) DO NOTHING",
            params![run_id, request_digest, serde_json::to_string(assembly)?],
        )?;
        transaction.commit()?;
        self.resident_context(run_id, request_digest)?
            .context("resident context binding disappeared")
    }

    pub fn create_authenticated_objective(
        &self,
        objective: NewObjective,
        now_ms: i64,
    ) -> Result<ObjectiveRecord> {
        validate_objective(&objective)?;
        let payload_digest = digest_json(&objective)?;
        let admission_json =
            serde_json::to_string(&objective).context("serialize objective admission")?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin objective creation")?;

        // Both command kinds share the authenticated ingress namespace. Check
        // under the write transaction so concurrent commands cannot claim it.
        if transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM controls WHERE idempotency_key = ?1)",
            [&objective.idempotency_key],
            |row| row.get::<_, bool>(0),
        )? {
            bail!(
                "objective idempotency conflict for {}",
                objective.idempotency_key
            );
        }

        if let Some((existing_id, existing_digest)) = transaction
            .query_row(
                "SELECT id, payload_digest FROM objectives WHERE ingress_key = ?1",
                [&objective.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .context("look up objective ingress key")?
        {
            if existing_id != objective.id || existing_digest != payload_digest {
                bail!(
                    "objective idempotency conflict for {}",
                    objective.idempotency_key
                );
            }
            let record = objective_in(&transaction, &existing_id)?
                .ok_or_else(|| anyhow!("idempotent objective disappeared"))?;
            transaction.commit().context("commit objective replay")?;
            return Ok(record);
        }

        transaction
            .execute(
                "INSERT INTO objectives
                 (id, source_id, ingress_key, payload_digest, operator_id, text, priority,
                  revision, approved_revision, state, created_at_ms, updated_at_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, NULL, ?8, ?9, ?9)",
                params![
                    objective.id,
                    objective.source_id,
                    objective.idempotency_key,
                    payload_digest,
                    objective.operator_id,
                    objective.text,
                    objective.priority,
                    ObjectiveState::PendingApproval.as_str(),
                    now_ms,
                ],
            )
            .context("insert objective")?;

        transaction
            .execute(
                "INSERT INTO objective_admissions (objective_id, input_json) VALUES (?1, ?2)",
                params![objective.id, admission_json],
            )
            .context("persist original objective admission")?;

        for (ordinal, project) in objective.projects.iter().enumerate() {
            transaction
                .execute(
                    "INSERT INTO objective_projects
                     (objective_id, ordinal, project_id, contract_digest)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        objective.id,
                        ordinal as i64,
                        project.project_id,
                        project.contract_digest,
                    ],
                )
                .with_context(|| format!("insert project authority {}", project.project_id))?;
        }

        for leaf in &objective.leaves {
            let execution_json = leaf
                .execution
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .context("serialize leaf execution payload")?;
            transaction
                .execute(
                    "INSERT INTO leaves
                     (id, objective_id, project_id, workspace_root, authority, execution_json,
                      stage, updated_at_ms)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                    params![
                        leaf.id,
                        objective.id,
                        leaf.project_id,
                        leaf.workspace_root,
                        leaf.authority,
                        execution_json,
                        LeafStage::Execute.as_str(),
                        now_ms,
                    ],
                )
                .with_context(|| format!("insert objective leaf {}", leaf.id))?;
        }
        for leaf in &objective.leaves {
            for dependency in &leaf.dependencies {
                transaction
                    .execute(
                        "INSERT INTO leaf_dependencies (leaf_id, dependency_leaf_id)
                         VALUES (?1, ?2)",
                        params![leaf.id, dependency],
                    )
                    .with_context(|| format!("insert dependency for {}", leaf.id))?;
            }
        }

        let record = objective_in(&transaction, &objective.id)?
            .ok_or_else(|| anyhow!("created objective disappeared"))?;
        transaction.commit().context("commit objective creation")?;
        self.notify_change();
        Ok(record)
    }

    pub fn objective(&self, objective_id: &str) -> Result<Option<ObjectiveRecord>> {
        let connection = self.connection()?;
        objective_in(&connection, objective_id)
    }

    pub fn list_objectives(&self) -> Result<Vec<ObjectiveRecord>> {
        let connection = self.connection()?;
        let ids = {
            let mut statement = connection.prepare(
                "SELECT id FROM objectives ORDER BY priority DESC, updated_at_ms DESC, id",
            )?;
            let ids = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            ids
        };
        ids.into_iter()
            .map(|id| {
                objective_in(&connection, &id)?
                    .ok_or_else(|| anyhow::anyhow!("objective {id} disappeared during listing"))
            })
            .collect()
    }

    pub fn list_leaves(&self, objective_id: &str) -> Result<Vec<LeafRecord>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, objective_id, project_id, workspace_root, authority, stage, attempt,
                    lease_owner, lease_expires_ms, current_receipt_digest,
                    (SELECT contract_digest FROM objective_projects p
                     WHERE p.objective_id = leaves.objective_id
                       AND p.project_id = leaves.project_id),
                    execution_json
             FROM leaves WHERE objective_id = ?1 ORDER BY id",
        )?;
        let rows = statement.query_map([objective_id], leaf_from_row)?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("list objective leaves")
    }

    pub fn leaf(&self, leaf_id: &str) -> Result<Option<LeafRecord>> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT id, objective_id, project_id, workspace_root, authority, stage, attempt,
                        lease_owner, lease_expires_ms, current_receipt_digest,
                        (SELECT contract_digest FROM objective_projects p
                         WHERE p.objective_id = leaves.objective_id
                           AND p.project_id = leaves.project_id),
                        execution_json
                 FROM leaves WHERE id = ?1",
                [leaf_id],
                leaf_from_row,
            )
            .optional()
            .context("read objective leaf")
    }

    pub fn apply_control(
        &self,
        objective_id: &str,
        action: ControlAction,
        idempotency_key: &str,
        operator_id: &str,
        now_ms: i64,
    ) -> Result<ObjectiveRecord> {
        let action_json = serde_json::to_string(&action).context("serialize objective control")?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin objective control")?;

        if transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM objectives WHERE ingress_key = ?1)",
            [idempotency_key],
            |row| row.get::<_, bool>(0),
        )? {
            bail!("control idempotency conflict for {idempotency_key}");
        }

        if let Some((stored_objective, stored_operator, stored_action)) = transaction
            .query_row(
                "SELECT objective_id, operator_id, action_json FROM controls
                 WHERE idempotency_key = ?1",
                [idempotency_key],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()?
        {
            if stored_objective != objective_id
                || stored_operator != operator_id
                || stored_action != action_json
            {
                bail!("control idempotency conflict for {idempotency_key}");
            }
            let record = objective_in(&transaction, objective_id)?
                .ok_or_else(|| anyhow!("controlled objective disappeared"))?;
            transaction.commit()?;
            return Ok(record);
        }

        let current = objective_in(&transaction, objective_id)?
            .ok_or_else(|| anyhow!("objective {objective_id} does not exist"))?;
        if current.operator_id != operator_id {
            bail!("operator authority does not match objective owner");
        }

        match &action {
            ControlAction::DeleteRecoveryContext { run_id } => {
                let terminal = matches!(
                    current.state,
                    ObjectiveState::Completed | ObjectiveState::Cancelled | ObjectiveState::Failed
                );
                let (all_closed, live_lease): (bool, bool) = transaction.query_row(
                    "SELECT NOT EXISTS (
                        SELECT 1 FROM leaves l WHERE l.objective_id = ?1
                        AND (l.stage != 'complete' OR NOT EXISTS (
                            SELECT 1 FROM stage_receipts r WHERE r.leaf_id = l.id
                            AND r.stage = 'close' AND r.digest = l.current_receipt_digest))
                     ), EXISTS (SELECT 1 FROM leaves WHERE objective_id = ?1 AND lease_expires_ms > ?2)",
                    params![objective_id, now_ms], |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                if !arda_vaire::service::retention::recovery_snapshot_deletion_allowed(
                    terminal, all_closed, live_lease,
                ) {
                    bail!("unfinished recovery evidence is protected from deletion");
                }
                let target_count: i64 = transaction.query_row(
                    "SELECT COUNT(*) FROM leaves WHERE objective_id = ?1 AND execution_run_id = ?2
                     AND context_bound = 1",
                    params![objective_id, run_id],
                    |row| row.get(0),
                )?;
                if target_count != 1 {
                    bail!("recovery deletion requires one exact bound objective run");
                }
                let changed = transaction.execute(
                    "UPDATE resident_context_bindings SET assembly_json = '',
                     deleted_by_operator_ms = COALESCE(deleted_by_operator_ms, ?1) WHERE run_id = ?2",
                    params![now_ms, run_id],
                )?;
                if changed != 1 {
                    bail!("recovery context binding is missing");
                }
            }
            ControlAction::Approve { revision } => {
                if *revision != current.revision {
                    bail!(
                        "approval revision {} does not match current revision {}",
                        revision,
                        current.revision
                    );
                }
                if current.state != ObjectiveState::PendingApproval {
                    bail!("objective is not pending approval");
                }
                transaction.execute(
                    "UPDATE objectives SET state = ?1, approved_revision = revision,
                     updated_at_ms = ?2 WHERE id = ?3",
                    params![ObjectiveState::Approved.as_str(), now_ms, objective_id],
                )?;
            }
            ControlAction::Reject => {
                if current.state != ObjectiveState::PendingApproval {
                    bail!("only a pending objective can be rejected");
                }
                transaction.execute(
                    "UPDATE objectives SET state = ?1, updated_at_ms = ?2 WHERE id = ?3",
                    params![ObjectiveState::Cancelled.as_str(), now_ms, objective_id],
                )?;
            }
            ControlAction::Pause => {
                if matches!(
                    current.state,
                    ObjectiveState::Completed | ObjectiveState::Cancelled | ObjectiveState::Failed
                ) {
                    bail!("terminal objective cannot be paused");
                }
                transaction.execute(
                    "UPDATE objectives SET state = ?1, updated_at_ms = ?2 WHERE id = ?3",
                    params![ObjectiveState::Paused.as_str(), now_ms, objective_id],
                )?;
            }
            ControlAction::Resume => {
                if current.state != ObjectiveState::Paused {
                    bail!("only a paused objective can be resumed");
                }
                let state = if current.revision == approved_revision(&transaction, objective_id)? {
                    ObjectiveState::Approved
                } else {
                    ObjectiveState::PendingApproval
                };
                transaction.execute(
                    "UPDATE objectives SET state = ?1, updated_at_ms = ?2 WHERE id = ?3",
                    params![state.as_str(), now_ms, objective_id],
                )?;
            }
            ControlAction::Cancel => {
                if current.state == ObjectiveState::Completed {
                    bail!("completed objective cannot be cancelled");
                }
                transaction.execute(
                    "UPDATE objectives SET state = ?1, updated_at_ms = ?2 WHERE id = ?3",
                    params![ObjectiveState::Cancelled.as_str(), now_ms, objective_id],
                )?;
                transaction.execute(
                    "UPDATE leaves SET stage = ?1, lease_owner = NULL, lease_expires_ms = NULL,
                     updated_at_ms = ?2 WHERE objective_id = ?3 AND stage != ?4",
                    params![
                        LeafStage::Cancelled.as_str(),
                        now_ms,
                        objective_id,
                        LeafStage::Complete.as_str()
                    ],
                )?;
            }
            ControlAction::Reprioritize { priority } => {
                if matches!(
                    current.state,
                    ObjectiveState::Completed | ObjectiveState::Cancelled | ObjectiveState::Failed
                ) {
                    bail!("terminal objective cannot be reprioritized");
                }
                transaction.execute(
                    "UPDATE objectives SET priority = ?1, updated_at_ms = ?2 WHERE id = ?3",
                    params![priority, now_ms, objective_id],
                )?;
            }
            ControlAction::Revise { text } => {
                if text.trim().is_empty() {
                    bail!("objective revision text must not be empty");
                }
                if matches!(
                    current.state,
                    ObjectiveState::Completed | ObjectiveState::Cancelled | ObjectiveState::Failed
                ) {
                    bail!("terminal objective cannot be revised");
                }
                let execution_started = transaction.query_row(
                    "SELECT EXISTS(
                        SELECT 1 FROM leaves
                        WHERE objective_id = ?1
                          AND (attempt > 0 OR current_receipt_digest IS NOT NULL OR stage != ?2)
                    )",
                    params![objective_id, LeafStage::Execute.as_str()],
                    |row| row.get::<_, bool>(0),
                )?;
                if execution_started {
                    bail!(
                        "objective cannot be revised after execution started; cancel it and create a new objective"
                    );
                }
                let has_execution_plan = transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM leaves WHERE objective_id = ?1 AND execution_json IS NOT NULL)",
                    params![objective_id],
                    |row| row.get::<_, bool>(0),
                )?;
                if has_execution_plan {
                    bail!("objective has a persisted execution plan; cancel it and create a new objective with a newly reviewed plan instead of revising text alone");
                }
                transaction.execute(
                    "UPDATE objectives SET text = ?1, revision = revision + 1,
                     approved_revision = NULL, state = ?2, updated_at_ms = ?3 WHERE id = ?4",
                    params![
                        text,
                        ObjectiveState::PendingApproval.as_str(),
                        now_ms,
                        objective_id
                    ],
                )?;
            }
        }

        // Accepted new stops invalidate recovery even if the state was already
        // Paused. Exact replay returned above; overflow rolls back all changes.
        if matches!(action, ControlAction::Pause | ControlAction::Cancel) {
            let changed = transaction.execute(
                "UPDATE objectives SET stop_generation = stop_generation + 1
                 WHERE id = ?1 AND stop_generation < 9223372036854775807",
                [objective_id],
            )?;
            if changed != 1 {
                bail!("objective stop generation exhausted");
            }
        }
        transaction.execute(
            "INSERT INTO controls
             (idempotency_key, objective_id, operator_id, action_json, created_at_ms, stop_generation)
             SELECT ?1, ?2, ?3, ?4, ?5, stop_generation FROM objectives WHERE id = ?2",
            params![
                idempotency_key,
                objective_id,
                operator_id,
                action_json,
                now_ms
            ],
        )?;
        let updated = objective_in(&transaction, objective_id)?
            .ok_or_else(|| anyhow!("controlled objective disappeared"))?;
        transaction.commit().context("commit objective control")?;
        self.notify_change();
        Ok(updated)
    }

    pub fn claim_runnable(
        &self,
        lease_owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
        capacity: usize,
    ) -> Result<Vec<ClaimedLeaf>> {
        self.claim_leaves(lease_owner, now_ms, lease_duration_ms, capacity, false)
    }

    pub(crate) fn claim_reconciliation(
        &self,
        lease_owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
        capacity: usize,
    ) -> Result<Vec<ClaimedLeaf>> {
        self.claim_leaves(lease_owner, now_ms, lease_duration_ms, capacity, true)
    }

    pub(crate) fn fail_reconciliation(&self, claim: &ClaimedLeaf, now_ms: i64) -> Result<()> {
        self.connection()?.execute(
            "UPDATE objectives SET state = 'failed', updated_at_ms = ?1
             WHERE id = ?2 AND state IN ('approved', 'running')
             AND EXISTS (SELECT 1 FROM leaves WHERE id = ?3 AND lease_owner = ?4
                         AND attempt = ?5 AND execution_run_id = ?6
                         AND lease_expires_ms > ?1)",
            params![
                now_ms,
                claim.objective_id,
                claim.leaf_id,
                claim.lease_owner,
                claim.attempt,
                claim.execution_run_id
            ],
        )?;
        Ok(())
    }

    fn claim_leaves(
        &self,
        lease_owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
        capacity: usize,
        reconciliation_only: bool,
    ) -> Result<Vec<ClaimedLeaf>> {
        self.reconcile_snapshot_commits()?;
        let claimed = self.claim_leaves_with_topology(
            lease_owner,
            now_ms,
            lease_duration_ms,
            capacity,
            reconciliation_only,
            read_workspace_topology,
        )?;
        // Persist before external commit. Lost acknowledgements leave an intent
        // for exact-capability reconciliation, never an executable claim return.
        self.reconcile_snapshot_commits()?;
        Ok(claimed)
    }

    fn claim_leaves_with_topology(
        &self,
        lease_owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
        capacity: usize,
        reconciliation_only: bool,
        mut read_topology: impl FnMut() -> Result<Vec<u8>>,
    ) -> Result<Vec<ClaimedLeaf>> {
        if lease_owner.trim().is_empty() {
            bail!("lease owner must not be empty");
        }
        if lease_duration_ms <= 0 {
            bail!("lease duration must be positive");
        }
        if capacity == 0 {
            return Ok(Vec::new());
        }

        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin objective claim")?;
        super::snapshots::check_policy(&transaction, self.snapshot_admission.is_some())?;
        super::scheduling::consume_due(&transaction, now_ms)?;
        // Retry exhaustion belongs to the resident claim transaction, not store
        // opening (operator projections open this store too). Preserve live leases.
        transaction.execute(
            "UPDATE objectives SET state = 'failed', updated_at_ms = ?1
             WHERE state IN ('approved', 'running')
               AND EXISTS (
                   SELECT 1 FROM leaves l WHERE l.objective_id = objectives.id
                     AND l.stage NOT IN ('complete', 'cancelled', 'failed')
                     AND l.attempt >= ?2
                     AND (COALESCE(l.context_bound, 0) != 1 OR l.execution_run_id IS NULL)
                     AND (l.lease_owner IS NULL OR l.lease_expires_ms <= ?1)
               )",
            params![now_ms, MAX_LEAF_ATTEMPTS],
        )?;
        // An empty eligible recovery batch is not proof that recovery is done:
        // a previous worker may still own a live receipt-only recovery lease.
        // Check under the admission transaction so fresh callers cannot race it.
        if !reconciliation_only {
            let pending: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM leaves l JOIN objectives o ON o.id = l.objective_id
                 WHERE o.state IN ('approved', 'running')
                   AND l.stage NOT IN ('complete', 'cancelled', 'failed')
                   AND l.attempt >= ?1 AND l.context_bound = 1
                   AND l.execution_run_id IS NOT NULL)",
                [MAX_LEAF_ATTEMPTS],
                |row| row.get(0),
            )?;
            if pending {
                transaction.commit()?;
                return Ok(Vec::new());
            }
        }
        let candidate_limit = capacity.saturating_mul(8).saturating_add(32) as i64;

        // Only first-admission readers may overlap. Recovery and any unknown or
        // write-capable authority remain exclusive, including paused live leases.
        let exclusive_live: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves l
             WHERE l.lease_expires_ms > ?1
               AND l.stage IN ('execute','verify','review','close')
               AND (l.authority != 'read_only' OR l.attempt != 1))",
            [now_ms],
            |row| row.get(0),
        )?;
        if exclusive_live {
            transaction.commit()?;
            return Ok(Vec::new());
        }
        let live_any: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves WHERE lease_expires_ms > ?1
             AND stage IN ('execute','verify','review','close'))",
            [now_ms],
            |row| row.get(0),
        )?;

        // Resolve live leases inside the same immediate transaction as admission.
        // Stored contract spelling is preserved; aliases are not separate capacity.
        // Retained readers are the narrow exception: never resolve their old
        // pathname as captured authority. Across calls readers may alias, but
        // cannot write; this exception is NOT proof of project independence.
        // Writers/recovery remain exclusive even when these roots are omitted.
        let topology = read_topology()?;
        let mut occupied_roots = {
            let mut statement = transaction.prepare(
                "SELECT l.workspace_root, i.identity_json FROM leaves l
                 LEFT JOIN lease_workspace_identities i ON i.leaf_id = l.id
                 WHERE l.lease_expires_ms > ?1
                 AND l.stage IN ('execute', 'verify', 'review', 'close')
                 AND NOT (l.authority = 'read_only' AND EXISTS (
                    SELECT 1 FROM retained_workspace_snapshots s
                    WHERE s.leaf_id = l.id AND s.run_id = l.execution_run_id
                    AND NOT EXISTS (SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id=l.id)))",
            )?;
            let roots = statement
                .query_map([now_ms], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            roots
                .iter()
                .map(|(root, saved)| {
                    let physical = physical_workspace_root(root)?;
                    let identity = workspace_identity(&physical, &topology)?;
                    if saved.as_deref() != Some(identity.as_str()) {
                        bail!("live workspace identity changed or is unknown; admission blocked");
                    }
                    Ok(physical)
                })
                .collect::<Result<Vec<_>>>()?
        };
        let expires_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or_else(|| anyhow!("lease expiry overflow"))?;
        let mut claimed = Vec::new();
        // Keyset pages bound temporary memory without letting blocked aliases at
        // the head of the queue permanently hide eligible independent work.
        let mut cursor: Option<(String, i64, i64, String)> = None;
        'pages: while claimed.len() < capacity {
            let candidates = {
                let mut statement = transaction.prepare(
                    "SELECT l.id, o.priority, o.created_at_ms, o.id
                 FROM leaves l
                 JOIN objectives o ON o.id = l.objective_id
                 WHERE o.state IN (?1, ?2)
                   AND (l.attempt > 0 OR NOT EXISTS (
                       SELECT 1 FROM leaves interrupted
                       JOIN objectives recovering ON recovering.id = interrupted.objective_id
                       WHERE recovering.state IN (?1, ?2)
                         AND interrupted.attempt > 0 AND interrupted.context_bound = 1
                         AND (COALESCE(interrupted.lease_expires_ms, 0) <= ?7 OR NOT EXISTS (
                             SELECT 1 FROM retained_workspace_snapshots active_reader
                             WHERE active_reader.leaf_id = interrupted.id
                               AND active_reader.run_id = interrupted.execution_run_id
                               AND interrupted.authority = 'read_only' AND interrupted.attempt = 1
                               AND NOT EXISTS (SELECT 1 FROM retained_snapshot_releases released
                                               WHERE released.leaf_id = interrupted.id)))
                         AND interrupted.stage IN (?3, ?4, ?5, ?6)))
                   AND (l.attempt > 0 OR NOT EXISTS
                        (SELECT 1 FROM schedules s WHERE s.objective_id = o.id)
                        OR EXISTS (SELECT 1 FROM schedules s JOIN schedule_wakes w ON w.schedule_id = s.id
                                   WHERE s.objective_id = o.id))
                   AND ((?15 = 0 AND l.attempt < ?10)
                     OR (?15 = 1 AND l.attempt >= ?10 AND l.context_bound = 1
                         AND l.execution_run_id IS NOT NULL))
                   AND l.stage IN (?3, ?4, ?5, ?6)
                   AND (l.lease_owner IS NULL OR l.lease_expires_ms <= ?7)
                   AND NOT EXISTS (
                       SELECT 1 FROM leaf_dependencies d
                       JOIN leaves prerequisite ON prerequisite.id = d.dependency_leaf_id
                       WHERE d.leaf_id = l.id AND prerequisite.stage != ?8
                   )
                   AND NOT EXISTS (
                       SELECT 1 FROM leaves active
                       WHERE active.id != l.id
                         AND active.workspace_root = l.workspace_root
                         AND active.lease_expires_ms > ?7
                         AND active.stage IN (?3, ?4, ?5, ?6)
                   )
                 AND (?11 IS NULL OR o.priority < ?11
                      OR (o.priority = ?11 AND (o.created_at_ms, o.id, l.id) > (?12, ?13, ?14)))
                 ORDER BY o.priority DESC, o.created_at_ms, o.id, l.id
                 LIMIT ?9",
                )?;
                let rows = statement.query_map(
                    params![
                        ObjectiveState::Approved.as_str(),
                        ObjectiveState::Running.as_str(),
                        LeafStage::Execute.as_str(),
                        LeafStage::Verify.as_str(),
                        LeafStage::Review.as_str(),
                        LeafStage::Close.as_str(),
                        now_ms,
                        LeafStage::Complete.as_str(),
                        candidate_limit,
                        MAX_LEAF_ATTEMPTS,
                        cursor.as_ref().map(|c| c.1),
                        cursor.as_ref().map(|c| c.2),
                        cursor.as_ref().map(|c| c.3.as_str()),
                        cursor.as_ref().map(|c| c.0.as_str()),
                        reconciliation_only,
                    ],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
            if candidates.is_empty() {
                break;
            }
            cursor = candidates.last().cloned();
            for (leaf_id, _, _, _) in candidates {
                if claimed.len() == capacity {
                    break;
                }
                let (workspace, attempt, authority): (String, u32, String) = transaction
                    .query_row(
                        "SELECT workspace_root, attempt, authority FROM leaves WHERE id = ?1",
                        [&leaf_id],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                    )?;
                let fresh_read_only =
                    !reconciliation_only && attempt == 0 && authority == "read_only";
                if !fresh_read_only && (live_any || !claimed.is_empty()) {
                    continue;
                }
                let saved: Option<String> = transaction
                    .query_row(
                        "SELECT identity_json FROM lease_workspace_identities WHERE leaf_id = ?1",
                        [&leaf_id],
                        |row| row.get(0),
                    )
                    .optional()?;
                let retained: Option<bool> = transaction.query_row(
                    "SELECT COALESCE(s.run_id = l.execution_run_id, 0)
                       AND NOT EXISTS (SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id = l.id)
                     FROM retained_workspace_snapshots s JOIN leaves l ON l.id = s.leaf_id
                     WHERE l.id = ?1",
                    [&leaf_id], |row| row.get(0),
                ).optional()?;
                if retained == Some(false) || (retained.is_some() && attempt == 0) {
                    bail!("retained recovery binding invalid; reconciliation required");
                }
                let recovering_retained = retained == Some(true);
                let (physical_root, identity) = if recovering_retained {
                    if !occupied_roots.is_empty() {
                        continue;
                    }
                    (
                        PathBuf::from(&workspace),
                        saved
                            .clone()
                            .context("retained recovery identity missing")?,
                    )
                } else {
                    let held: bool = transaction.query_row(
                        "SELECT EXISTS(SELECT 1 FROM retained_workspace_snapshots s
                         JOIN leaves holder ON holder.id = s.leaf_id
                         WHERE NOT EXISTS (SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id = s.leaf_id)
                         AND (?1 = 0 OR holder.authority != 'read_only'))",
                        [fresh_read_only], |row| row.get(0),
                    )?;
                    if held {
                        continue;
                    }
                    let physical = physical_workspace_root(&workspace)?;
                    let identity =
                        if self.snapshot_admission.is_some() && saved.is_none() && attempt == 0 {
                            retained_workspace_identity(&physical, &topology)?
                        } else {
                            // Never upgrade historical v2 admissions from current paths.
                            workspace_identity(&physical, &topology)?
                        };
                    if (attempt > 0 && saved.is_none())
                        || saved.as_ref().is_some_and(|saved| saved != &identity)
                    {
                        bail!(
                            "recovery workspace identity changed or is unknown; admission blocked"
                        );
                    }
                    (physical, identity)
                };
                let mut overlaps = false;
                for root in &occupied_roots {
                    if workspace_roots_overlap(&physical_root, root, &topology)? {
                        overlaps = true;
                        break;
                    }
                }
                if overlaps {
                    continue;
                }
                let changed = transaction.execute(
                    "UPDATE leaves AS target
                 SET lease_owner = ?1, lease_expires_ms = ?2,
                     execution_run_id = CASE WHEN attempt = 0 THEN
                         'objective-' || objective_id || '-leaf-' || id || '-attempt-1'
                         ELSE execution_run_id END,
                     context_bound = CASE WHEN attempt = 0 THEN 0 ELSE context_bound END,
                     attempt = attempt + 1,
                     updated_at_ms = ?3
                 WHERE id = ?4
                   AND ((?10 = 0 AND attempt < ?9)
                     OR (?10 = 1 AND attempt >= ?9 AND context_bound = 1
                         AND execution_run_id IS NOT NULL))
                   AND (lease_owner IS NULL OR lease_expires_ms <= ?3)
                   AND NOT EXISTS (
                       SELECT 1 FROM leaves active
                       WHERE active.id != target.id
                         AND active.workspace_root = target.workspace_root
                         AND active.lease_expires_ms > ?3
                         AND active.stage IN (?5, ?6, ?7, ?8)
                   )",
                    params![
                        lease_owner,
                        expires_ms,
                        now_ms,
                        leaf_id,
                        LeafStage::Execute.as_str(),
                        LeafStage::Verify.as_str(),
                        LeafStage::Review.as_str(),
                        LeafStage::Close.as_str(),
                        MAX_LEAF_ATTEMPTS,
                        reconciliation_only,
                    ],
                )?;
                if changed == 0 {
                    continue;
                }
                transaction.execute(
                    "INSERT INTO lease_workspace_identities (leaf_id, identity_json) VALUES (?1, ?2)
                     ON CONFLICT(leaf_id) DO NOTHING",
                    params![leaf_id, identity],
                )?;
                super::snapshots::prepare(
                    &transaction,
                    self.snapshot_admission.as_deref(),
                    &leaf_id,
                    &physical_root,
                    &identity,
                    attempt == 0,
                )?;
                transaction.execute(
                    "INSERT INTO retained_snapshot_lease_intents
                     (leaf_id, generation, lease_owner, lease_expires_ms)
                     SELECT id, attempt, lease_owner, lease_expires_ms FROM leaves
                     WHERE id = ?1 AND EXISTS (SELECT 1 FROM retained_workspace_snapshots WHERE leaf_id = ?1)",
                    [&leaf_id],
                )?;
                occupied_roots.push(physical_root);
                let claim = Self::decode_claim(&transaction, &leaf_id)?;
                transaction.execute(
                    "UPDATE objectives SET state = ?1, updated_at_ms = ?2
                 WHERE id = ?3 AND state = ?4",
                    params![
                        ObjectiveState::Running.as_str(),
                        now_ms,
                        claim.objective_id,
                        ObjectiveState::Approved.as_str()
                    ],
                )?;
                claimed.push(claim);
                if !fresh_read_only {
                    break 'pages;
                }
            }
        }
        if read_topology()? != topology {
            bail!("mount topology changed during admission; transaction rolled back");
        }
        transaction.commit().context("commit objective claims")?;
        Ok(claimed)
    }

    // Decode only; callers own claim authorization and lease mutation. Keep
    // receipt projection identical for ordinary and exact-leaf recovery claims.
    pub(super) fn decode_claim(
        transaction: &rusqlite::Transaction<'_>,
        leaf_id: &str,
    ) -> Result<ClaimedLeaf> {
        let mut claim = transaction.query_row(
            "SELECT objective_id, id, project_id, workspace_root, authority, stage, attempt,
                        current_receipt_digest,
                        (SELECT contract_digest FROM objective_projects p
                         WHERE p.objective_id = leaves.objective_id
                           AND p.project_id = leaves.project_id),
                        execution_json, execution_run_id, lease_owner, lease_expires_ms
                 FROM leaves WHERE id = ?1",
            [&leaf_id],
            |row| {
                let stage = parse_leaf_stage(row.get::<_, String>(5)?)?;
                Ok(ClaimedLeaf {
                    objective_id: row.get(0)?,
                    leaf_id: row.get(1)?,
                    project_id: row.get(2)?,
                    workspace_root: row.get(3)?,
                    authority: row.get(4)?,
                    stage,
                    attempt: row.get(6)?,
                    lease_owner: row.get(11)?,
                    lease_expires_ms: row.get(12)?,
                    current_receipt_digest: row.get(7)?,
                    project_contract_digest: row.get(8)?,
                    execution: parse_execution_spec(row.get(9)?)?,
                    execution_run_id: row.get(10)?,
                    dependency_receipts: Vec::new(),
                })
            },
        )?;
        claim.dependency_receipts = Self::read_dependency_receipts(transaction, leaf_id)?;
        Ok(claim)
    }

    fn read_dependency_receipts(
        transaction: &Transaction<'_>,
        leaf_id: &str,
    ) -> Result<Vec<StageReceipt>> {
        let mut statement = transaction.prepare(
            "SELECT r.contract, r.digest, r.predecessor_digest, r.run_path, r.provider,
                            r.model, r.started_at_ms, r.completed_at_ms, r.verdict,
                            r.context_outcome_receipt_id, r.context_outcome_receipt_digest,
                            r.binding_digest
                     FROM leaf_dependencies d
                     JOIN stage_receipts r ON r.leaf_id = d.dependency_leaf_id
                     WHERE d.leaf_id = ?1 AND r.stage = ?2
                     ORDER BY d.dependency_leaf_id",
        )?;
        let rows = statement.query_map(params![leaf_id, ReceiptStage::Close.as_str()], |row| {
            Ok(StageReceipt {
                contract: row.get(0)?,
                stage: ReceiptStage::Close,
                digest: row.get(1)?,
                predecessor_digest: row.get(2)?,
                run_path: row.get(3)?,
                provider: row.get(4)?,
                model: row.get(5)?,
                started_at_ms: row.get(6)?,
                completed_at_ms: row.get(7)?,
                verdict: row.get(8)?,
                context_outcome_receipt_id: row.get(9)?,
                context_outcome_receipt_digest: row.get(10)?,
                binding_digest: row.get(11)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn record_stage_receipt(
        &self,
        leaf_id: &str,
        lease_owner: &str,
        attempt: i64,
        receipt: StageReceipt,
        now_ms: i64,
    ) -> Result<()> {
        validate_receipt(&receipt)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin receipt recording")?;

        let changed = Self::record_stage_receipt_in(
            &transaction,
            leaf_id,
            lease_owner,
            attempt,
            &receipt,
            now_ms,
        )?;
        transaction.commit().context("commit stage receipt")?;
        if changed {
            self.notify_change();
        }
        Ok(())
    }

    fn record_stage_receipt_in(
        transaction: &rusqlite::Transaction<'_>,
        leaf_id: &str,
        lease_owner: &str,
        attempt: i64,
        receipt: &StageReceipt,
        now_ms: i64,
    ) -> Result<bool> {
        validate_receipt(receipt)?;

        if let Some((digest, predecessor, outcome_id, outcome_digest, binding_digest)) = transaction
            .query_row(
                "SELECT digest, predecessor_digest, context_outcome_receipt_id,
                        context_outcome_receipt_digest, binding_digest FROM stage_receipts
                 WHERE leaf_id = ?1 AND stage = ?2",
                params![leaf_id, receipt.stage.as_str()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                    ))
                },
            )
            .optional()?
        {
            if digest == receipt.digest
                && predecessor == receipt.predecessor_digest
                && outcome_id == receipt.context_outcome_receipt_id
                && outcome_digest == receipt.context_outcome_receipt_digest
                && binding_digest == receipt.binding_digest
            {
                return Ok(false);
            }
            bail!("receipt idempotency conflict for {leaf_id}");
        }

        let leaf = transaction
            .query_row(
                "SELECT stage, lease_owner, lease_expires_ms, current_receipt_digest, attempt
                 FROM leaves WHERE id = ?1",
                [leaf_id],
                |row| {
                    Ok((
                        parse_leaf_stage(row.get::<_, String>(0)?)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("leaf {leaf_id} does not exist"))?;
        if leaf.0 != receipt.stage.leaf_stage() {
            bail!("receipt stage does not match current leaf stage");
        }
        if leaf.1.as_deref() != Some(lease_owner)
            || leaf.2.is_none_or(|expiry| expiry <= now_ms)
            || leaf.4 != attempt
        {
            bail!("receipt writer does not hold the active leaf lease");
        }
        if receipt.predecessor_digest != leaf.3 {
            bail!("receipt predecessor does not match current receipt lineage");
        }

        transaction.execute(
            "INSERT INTO stage_receipts
             (leaf_id, stage, contract, digest, predecessor_digest, run_path, provider, model,
              started_at_ms, completed_at_ms, verdict, context_outcome_receipt_id,
              context_outcome_receipt_digest, binding_digest, recorded_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            params![
                leaf_id,
                receipt.stage.as_str(),
                receipt.contract,
                receipt.digest,
                receipt.predecessor_digest,
                receipt.run_path,
                receipt.provider,
                receipt.model,
                receipt.started_at_ms,
                receipt.completed_at_ms,
                receipt.verdict,
                receipt.context_outcome_receipt_id,
                receipt.context_outcome_receipt_digest,
                receipt.binding_digest,
                now_ms,
            ],
        )?;
        let next_stage = receipt.stage.next_leaf_stage();
        let release = next_stage == LeafStage::Complete;
        transaction.execute(
            "UPDATE leaves SET stage = ?1, current_receipt_digest = ?2,
             lease_owner = CASE WHEN ?3 THEN NULL ELSE lease_owner END,
             lease_expires_ms = CASE WHEN ?3 THEN NULL ELSE lease_expires_ms END,
             updated_at_ms = ?4 WHERE id = ?5",
            params![
                next_stage.as_str(),
                receipt.digest,
                release,
                now_ms,
                leaf_id
            ],
        )?;
        Ok(true)
    }

    pub fn close_objective(
        &self,
        objective_id: &str,
        root_receipt_digest: &str,
        now_ms: i64,
    ) -> Result<ObjectiveRecord> {
        if root_receipt_digest.trim().is_empty() {
            bail!("root receipt digest must not be empty");
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current = objective_in(&transaction, objective_id)?
            .ok_or_else(|| anyhow!("objective {objective_id} does not exist"))?;
        if current.state == ObjectiveState::Completed {
            if current.terminal_receipt_digest.as_deref() == Some(root_receipt_digest) {
                transaction.commit()?;
                return Ok(current);
            }
            bail!("objective already closed with a different receipt");
        }
        let incomplete: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM leaves WHERE objective_id = ?1 AND stage != ?2",
            params![objective_id, LeafStage::Complete.as_str()],
            |row| row.get(0),
        )?;
        if incomplete != 0 {
            bail!("objective leaves are not complete");
        }
        transaction.execute(
            "UPDATE objectives SET state = ?1, terminal_receipt_digest = ?2,
             updated_at_ms = ?3 WHERE id = ?4",
            params![
                ObjectiveState::Completed.as_str(),
                root_receipt_digest,
                now_ms,
                objective_id
            ],
        )?;
        let closed = objective_in(&transaction, objective_id)?
            .ok_or_else(|| anyhow!("closed objective disappeared"))?;
        transaction.commit()?;
        Ok(closed)
    }

    pub fn complete_objective_if_ready(
        &self,
        objective_id: &str,
        root_receipt_digest: &str,
        now_ms: i64,
    ) -> Result<bool> {
        let connection = self.connection()?;
        let incomplete: i64 = connection.query_row(
            "SELECT COUNT(*) FROM leaves WHERE objective_id = ?1 AND stage != ?2",
            params![objective_id, LeafStage::Complete.as_str()],
            |row| row.get(0),
        )?;
        if incomplete != 0 {
            return Ok(false);
        }
        drop(connection);
        self.close_objective(objective_id, root_receipt_digest, now_ms)?;
        Ok(true)
    }

    pub fn put_schedule(&self, schedule: ScheduleSpec, now_ms: i64) -> Result<ScheduleSpec> {
        validate_schedule(&schedule)?;
        let payload_digest = digest_json(&schedule)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if objective_in(&transaction, &schedule.objective_id)?.is_none() {
            bail!("scheduled objective does not exist");
        }
        if let Some((id, digest)) = transaction
            .query_row(
                "SELECT id, payload_digest FROM schedules WHERE idempotency_key = ?1",
                [&schedule.idempotency_key],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
        {
            if id != schedule.id || digest != payload_digest {
                bail!(
                    "schedule idempotency conflict for {}",
                    schedule.idempotency_key
                );
            }
            let stored = schedule_in(&transaction, &id)?
                .ok_or_else(|| anyhow!("idempotent schedule disappeared"))?;
            transaction.commit()?;
            return Ok(stored);
        }
        transaction.execute(
            "INSERT INTO schedules
             (id, objective_id, next_wake_ms, recurrence, idempotency_key, payload_digest,
              created_at_ms, updated_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
            params![
                schedule.id,
                schedule.objective_id,
                schedule.next_wake_ms,
                schedule.recurrence,
                schedule.idempotency_key,
                payload_digest,
                now_ms,
            ],
        )?;
        transaction.commit()?;
        self.notify_change();
        Ok(schedule)
    }

    pub fn schedule(&self, schedule_id: &str) -> Result<Option<ScheduleSpec>> {
        let connection = self.connection()?;
        schedule_in(&connection, schedule_id)
    }

    /// Persisted schedule failures, including historical and paused objectives.
    pub fn quarantined_schedule_count(&self) -> Result<u64> {
        self.connection()?
            .query_row("SELECT COUNT(*) FROM schedule_errors", [], |row| row.get(0))
            .context("read schedule quarantine count")
    }

    /// Earliest actionable timer; paused/terminal objectives do not wake workers.
    pub fn next_wake_ms(&self, now_ms: i64) -> Result<Option<i64>> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT MIN(wake_ms) FROM (
                SELECT s.next_wake_ms AS wake_ms FROM schedules s
                JOIN objectives o ON o.id = s.objective_id
                WHERE o.state IN ('approved', 'running')
                  AND NOT EXISTS (SELECT 1 FROM schedule_errors e WHERE e.schedule_id = s.id)
                  AND (s.recurrence IS NOT NULL OR NOT EXISTS
                       (SELECT 1 FROM schedule_wakes w WHERE w.schedule_id = s.id))
                UNION ALL
                SELECT l.lease_expires_ms AS wake_ms FROM leaves l
                JOIN objectives o ON o.id = l.objective_id
                WHERE o.state IN ('approved', 'running')
                  AND l.stage IN ('execute', 'verify', 'review', 'close')
                  AND l.lease_expires_ms > ?1
            )",
                [now_ms],
                |row| row.get(0),
            )
            .context("read next objective wake")
    }

    pub fn due_schedules(&self, now_ms: i64, limit: usize) -> Result<Vec<ScheduleSpec>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT s.id, s.objective_id, s.next_wake_ms, s.recurrence, s.idempotency_key
             FROM schedules s JOIN objectives o ON o.id = s.objective_id
             WHERE s.next_wake_ms <= ?1 AND o.state NOT IN (?2, ?3, ?4, ?5)
               AND NOT EXISTS (SELECT 1 FROM schedule_errors e WHERE e.schedule_id = s.id)
               AND (s.recurrence IS NOT NULL OR NOT EXISTS
                    (SELECT 1 FROM schedule_wakes w WHERE w.schedule_id = s.id))
             ORDER BY s.next_wake_ms, s.id LIMIT ?6",
        )?;
        let rows = statement.query_map(
            params![
                now_ms,
                ObjectiveState::Paused.as_str(),
                ObjectiveState::Completed.as_str(),
                ObjectiveState::Cancelled.as_str(),
                ObjectiveState::Failed.as_str(),
                limit as i64,
            ],
            schedule_from_row,
        )?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("list due objective schedules")
    }

    pub(super) fn connection(&self) -> Result<Connection> {
        if authority::Authority::load(&self.path)? != self.authority {
            bail!("ObjectiveStore authority binding changed");
        }
        let connection = Connection::open_with_flags(
            &self.path,
            OpenFlags::default() & !OpenFlags::SQLITE_OPEN_CREATE,
        )
        .with_context(|| format!("open ObjectiveStore {}", self.path.display()))?;
        self.authority.check_path(&self.path)?;
        self.authority.check_connection(&connection)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        Ok(connection)
    }
}

fn validate_objective(objective: &NewObjective) -> Result<()> {
    for (name, value) in [
        ("objective id", objective.id.as_str()),
        ("source id", objective.source_id.as_str()),
        ("idempotency key", objective.idempotency_key.as_str()),
        ("operator id", objective.operator_id.as_str()),
        ("objective text", objective.text.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{name} must not be empty");
        }
    }
    if objective.leaves.is_empty() {
        bail!("objective must contain at least one leaf");
    }
    let mut projects = HashSet::new();
    for project in &objective.projects {
        if project.project_id.trim().is_empty() || project.contract_digest.trim().is_empty() {
            bail!("project authority fields must not be empty");
        }
        if !projects.insert(project.project_id.as_str()) {
            bail!("duplicate project authority {}", project.project_id);
        }
    }
    let mut leaves = HashSet::new();
    for leaf in &objective.leaves {
        if leaf.id.trim().is_empty()
            || leaf.workspace_root.trim().is_empty()
            || leaf.authority.trim().is_empty()
        {
            bail!("leaf identity, workspace, and authority must not be empty");
        }
        if !leaves.insert(leaf.id.as_str()) {
            bail!("duplicate leaf id {}", leaf.id);
        }
        if leaf
            .project_id
            .as_ref()
            .is_some_and(|project_id| !projects.contains(project_id.as_str()))
        {
            bail!("leaf {} references an undeclared project", leaf.id);
        }
    }
    for leaf in &objective.leaves {
        let mut dependencies = HashSet::new();
        for dependency in &leaf.dependencies {
            if dependency == &leaf.id || !leaves.contains(dependency.as_str()) {
                bail!("leaf {} has an invalid dependency {}", leaf.id, dependency);
            }
            if !dependencies.insert(dependency.as_str()) {
                bail!("leaf {} has duplicate dependency {}", leaf.id, dependency);
            }
        }
    }
    let mut unresolved = objective
        .leaves
        .iter()
        .map(|leaf| (leaf.id.as_str(), leaf.dependencies.len()))
        .collect::<HashMap<_, _>>();
    let mut ready = unresolved
        .iter()
        .filter_map(|(leaf_id, count)| (*count == 0).then_some(*leaf_id))
        .collect::<Vec<_>>();
    let mut visited = 0;
    while let Some(completed) = ready.pop() {
        if unresolved.remove(completed).is_none() {
            continue;
        }
        visited += 1;
        for leaf in &objective.leaves {
            if leaf
                .dependencies
                .iter()
                .any(|dependency| dependency == completed)
            {
                if let Some(count) = unresolved.get_mut(leaf.id.as_str()) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push(leaf.id.as_str());
                    }
                }
            }
        }
    }
    if visited != objective.leaves.len() {
        bail!("objective leaf dependencies contain a cycle");
    }
    Ok(())
}

// Resolve existing ancestors too: a not-yet-created workspace below a symlink
// must not evade exclusion. Never collapse `..` before resolving symlinks.
fn physical_workspace_root(root: &str) -> Result<PathBuf> {
    // Workbench resolves legacy relative roots against the daemon working
    // directory as well. Preserve that compatibility, not a database-parent guess.
    let mut path = std::path::absolute(root).context("resolve workspace base directory")?;
    let mut missing = Vec::new();
    loop {
        match path.canonicalize() {
            Ok(mut resolved) => {
                for component in missing.iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // A dangling link is not an absent ordinary directory.
                if std::fs::symlink_metadata(&path).is_ok() {
                    return Err(error).context("resolve workspace symlink");
                }
                let Some(Component::Normal(name)) = path.components().next_back() else {
                    bail!("cannot resolve workspace root safely: {root}");
                };
                missing.push(name.to_os_string());
                if !path.pop() {
                    bail!("cannot resolve workspace root: {root}");
                }
            }
            Err(error) => return Err(error).context("resolve physical workspace root"),
        }
    }
}

// Persist stable filesystem identity, not mutable timestamps. Missing descendants
// retain the nearest existing ancestor as an explicit conservative anchor.
fn workspace_identity(root: &Path, topology: &[u8]) -> Result<String> {
    let mut anchor = root;
    let metadata = loop {
        match std::fs::metadata(anchor) {
            Ok(metadata) => break metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                anchor = anchor
                    .parent()
                    .context("workspace has no existing ancestor")?;
            }
            Err(error) => return Err(error).context("read workspace identity"),
        }
    };
    if !metadata.is_dir() {
        bail!("workspace identity anchor is not a directory");
    }
    require_workspace_access(anchor)?;
    #[cfg(unix)]
    let filesystem_id = {
        use std::os::unix::fs::MetadataExt;
        Some((metadata.dev(), metadata.ino()))
    };
    #[cfg(not(unix))]
    let filesystem_id: Option<(u64, u64)> = None;
    encode_workspace_identity(root, anchor, filesystem_id, topology)
        .context("encode workspace identity")
}

fn retained_workspace_identity(root: &Path, topology: &[u8]) -> Result<String> {
    #[cfg(target_os = "linux")]
    {
        require_workspace_access(root)?;
        let physical = super::physical_tree::Physical::pin(root, topology)?;
        let witness = physical.witness()?;
        physical.revalidate_coordinate()?;
        if topology != read_workspace_topology()? {
            bail!("retained admission topology changed during pinning");
        }
        let object = &witness.mounts[0].object;
        Ok(serde_json::to_string(&(
            3,
            root,
            root,
            Some((object.device, object.inode)),
            witness.digest()?,
        ))?)
    }
    #[cfg(not(target_os = "linux"))]
    {
        workspace_identity(root, topology)
    }
}

fn read_workspace_topology() -> Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    return std::fs::read("/proc/self/mountinfo").context("read admission mount topology");
    #[cfg(not(target_os = "linux"))]
    Ok(Vec::new())
}

// Versioned, conservative namespace-wide baseline. Never upgrade legacy tuples
// from current mounts: an absent admission baseline must fail comparison closed.
pub(crate) fn encode_workspace_identity(
    root: &Path,
    anchor: &Path,
    filesystem_id: Option<(u64, u64)>,
    topology: &[u8],
) -> serde_json::Result<String> {
    let fingerprint = format!("{:x}", Sha256::digest(topology));
    serde_json::to_string(&(2, root, anchor, filesystem_id, fingerprint))
}

// Missing roots are permitted only as ancestor-based reservations. Actual
// execution uses this helper on the exact root; neither path creates directories
// or grants permissions. Read-only admission does not require write access.
pub(super) fn require_workspace_access(root: &Path) -> Result<()> {
    if !std::fs::metadata(root)
        .context("read workspace directory")?
        .is_dir()
    {
        bail!("workspace is not a directory");
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(root.as_os_str().as_bytes())
            .context("workspace path contains a NUL byte")?;
        // SAFETY: path is a live NUL-terminated string. AT_EACCESS checks the
        // effective credentials used by the daemon, including directory search.
        if unsafe {
            libc::faccessat(
                libc::AT_FDCWD,
                path.as_ptr(),
                libc::R_OK | libc::X_OK,
                libc::AT_EACCESS,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error())
                .context("workspace requires read and search access");
        }
    }
    std::fs::read_dir(root).context("open workspace directory")?;
    Ok(())
}

// Canonical paths do not collapse bind mounts. Compare relative subtrees at
// shared physical ancestors as well, without treating independent siblings as
// overlapping merely because they share a filesystem or ancestor directory.
/// Reject owner state exposed through any physical workspace mount alias.
#[cfg(target_os = "linux")]
pub fn validate_snapshot_owner_paths(workspace: &Path, state: &[&Path]) -> Result<()> {
    let topology = std::fs::read("/proc/self/mountinfo")?;
    for path in state {
        if workspace_roots_overlap(workspace, path, &topology)? {
            bail!("snapshot owner state overlaps execution workspace");
        }
    }
    if std::fs::read("/proc/self/mountinfo")? != topology {
        bail!("mount topology changed during execution-grant overlap validation");
    }
    Ok(())
}

fn workspace_roots_overlap(left: &Path, right: &Path, _topology: &[u8]) -> Result<bool> {
    let mut left_roots = vec![left.to_path_buf()];
    let mut right_roots = vec![right.to_path_buf()];
    #[cfg(target_os = "linux")]
    {
        // A workspace can contain a mounted subtree shared with an otherwise
        // disjoint workspace. Inspect this process's namespace, not host-wide
        // assumptions. Failure to enumerate mounts must not authorize work.
        let mounts = workspace_mountpoints(_topology)?;
        let backing_roots = |root: &Path| -> Result<Vec<(Vec<u8>, PathBuf)>> {
            let (device, backing, point) = mounts
                .iter()
                .filter(|(_, _, point)| root.starts_with(point))
                .max_by_key(|(_, _, point)| point.components().count())
                .context("workspace has no containing mount")?;
            if !backing.is_absolute() {
                bail!("workspace containing mount has an opaque source root");
            }
            let mut roots = vec![(device.clone(), backing.join(root.strip_prefix(point)?))];
            for (device, backing, point) in &mounts {
                if point.starts_with(root) && point != root {
                    if !backing.is_absolute() {
                        bail!("workspace nested mount has an opaque source root");
                    }
                    roots.push((device.clone(), backing.clone()));
                }
            }
            Ok(roots)
        };
        let left_backing = backing_roots(left)?;
        let right_backing = backing_roots(right)?;
        // A bind of a child subtree loses its source parent in pathname ancestry.
        // Mountinfo retains the filesystem-relative source root for that case.
        if left_backing.iter().any(|(device, root)| {
            right_backing.iter().any(|(other_device, other)| {
                device == other_device && (root.starts_with(other) || other.starts_with(root))
            })
        }) {
            return Ok(true);
        }
        for (_, _, mount) in mounts {
            if mount.starts_with(left) && mount != left {
                left_roots.push(mount.clone());
            }
            if mount.starts_with(right) && mount != right {
                right_roots.push(mount);
            }
        }
    }
    for left in &left_roots {
        for right in &right_roots {
            if workspace_subtrees_overlap(left, right)? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(target_os = "linux")]
fn workspace_mountpoints(mountinfo: &[u8]) -> Result<Vec<(Vec<u8>, PathBuf, PathBuf)>> {
    use std::os::unix::ffi::OsStringExt;
    let mut mounts = Vec::new();
    for line in mountinfo
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let fields = line.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() < 5 {
            bail!("mount topology omitted mountpoint");
        }
        let mut paths = Vec::new();
        for encoded in [fields[3], fields[4]] {
            let mut decoded = Vec::new();
            let mut index = 0;
            while index < encoded.len() {
                if encoded[index] == b'\\' {
                    let escape = encoded
                        .get(index..index + 4)
                        .context("truncated mountpoint escape")?;
                    decoded.push(match escape {
                        b"\\040" => b' ',
                        b"\\011" => b'\t',
                        b"\\012" => b'\n',
                        b"\\134" => b'\\',
                        _ => bail!("invalid mountpoint escape"),
                    });
                    index += 4;
                } else {
                    decoded.push(encoded[index]);
                    index += 1;
                }
            }
            let mount = PathBuf::from(std::ffi::OsString::from_vec(decoded));
            if paths.len() == 1 && !mount.is_absolute() {
                bail!("mount topology contains a relative mountpoint");
            }
            paths.push(mount);
        }
        mounts.push((fields[2].to_vec(), paths.remove(0), paths.remove(0)));
    }
    if mounts.is_empty() {
        bail!("workspace mount topology is empty");
    }
    Ok(mounts)
}

fn workspace_subtrees_overlap(left: &Path, right: &Path) -> Result<bool> {
    if left.starts_with(right) || right.starts_with(left) {
        return Ok(true);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let ancestors = |root: &Path| -> Result<Vec<(u64, u64, PathBuf)>> {
            let mut entries = Vec::new();
            for ancestor in root.ancestors() {
                match std::fs::metadata(ancestor) {
                    Ok(metadata) => {
                        if !metadata.is_dir() && !(ancestor == root && metadata.is_file()) {
                            bail!("workspace ancestor is not a directory");
                        }
                        entries.push((
                            metadata.dev(),
                            metadata.ino(),
                            root.strip_prefix(ancestor)?.to_path_buf(),
                        ));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error).context("read workspace overlap identity"),
                }
            }
            Ok(entries)
        };
        let left = ancestors(left)?;
        let right = ancestors(right)?;
        for (device, inode, suffix) in &left {
            if right
                .iter()
                .any(|(other_device, other_inode, other_suffix)| {
                    device == other_device
                        && inode == other_inode
                        && (suffix.starts_with(other_suffix) || other_suffix.starts_with(suffix))
                })
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn validate_sha256(value: &str, name: &str) -> Result<()> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        bail!("{name} must use the sha256:<lowercase-hex> form");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        bail!("{name} must use the sha256:<lowercase-hex> form");
    }
    Ok(())
}

fn validate_receipt(receipt: &StageReceipt) -> Result<()> {
    if receipt.contract != "arda.hermes_execution_receipt.v4" {
        bail!("receipt contract must be arda.hermes_execution_receipt.v4");
    }
    for (name, value) in [
        ("receipt digest", receipt.digest.as_str()),
        ("run path", receipt.run_path.as_str()),
        ("provider", receipt.provider.as_str()),
        ("model", receipt.model.as_str()),
        ("verdict", receipt.verdict.as_str()),
    ] {
        if value.trim().is_empty() {
            bail!("{name} must not be empty");
        }
    }
    validate_sha256(&receipt.digest, "receipt digest")?;
    if let Some(predecessor) = receipt.predecessor_digest.as_deref() {
        validate_sha256(predecessor, "receipt predecessor digest")?;
    }
    let run_path = Path::new(&receipt.run_path);
    if run_path.is_absolute()
        || run_path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        bail!("receipt run path must be a safe repository-relative path");
    }
    if receipt.completed_at_ms < receipt.started_at_ms {
        bail!("receipt completion precedes its start");
    }
    match (
        receipt.context_outcome_receipt_id.as_deref(),
        receipt.context_outcome_receipt_digest.as_deref(),
    ) {
        (Some(id), Some(digest)) if !id.trim().is_empty() && digest.starts_with("sha256:") => {}
        (None, None) => {}
        _ => bail!("context outcome receipt binding must include a non-empty id and digest"),
    }
    if let Some(binding_digest) = receipt.binding_digest.as_deref() {
        if binding_digest != receipt.computed_binding_digest()? {
            bail!("receipt binding digest is invalid");
        }
    } else if receipt.context_outcome_receipt_id.is_some() {
        bail!("context outcome receipt requires a binding digest");
    }
    Ok(())
}

fn validate_schedule(schedule: &ScheduleSpec) -> Result<()> {
    if let Some(recurrence) = &schedule.recurrence {
        let interval = super::scheduling::recurrence_ms(recurrence)?;
        schedule
            .next_wake_ms
            .checked_add(interval)
            .ok_or_else(|| anyhow!("next schedule wake overflow"))?;
    }
    if schedule.id.trim().is_empty()
        || schedule.objective_id.trim().is_empty()
        || schedule.idempotency_key.trim().is_empty()
    {
        bail!("schedule id, objective, and idempotency key must not be empty");
    }
    Ok(())
}

fn digest_json(value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(value).context("serialize digest payload")?;
    let digest = Sha256::digest(bytes);
    Ok(format!("sha256:{digest:x}"))
}

fn approved_revision(transaction: &Transaction<'_>, objective_id: &str) -> Result<i64> {
    Ok(transaction
        .query_row(
            "SELECT approved_revision FROM objectives WHERE id = ?1",
            [objective_id],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .unwrap_or_default())
}

fn objective_in(connection: &Connection, objective_id: &str) -> Result<Option<ObjectiveRecord>> {
    let base = connection
        .query_row(
            "SELECT id, source_id, operator_id, text, priority, revision, state,
                    terminal_receipt_digest, created_at_ms, updated_at_ms
             FROM objectives WHERE id = ?1",
            [objective_id],
            |row| {
                let state = parse_objective_state(row.get::<_, String>(6)?)?;
                Ok(ObjectiveRecord {
                    id: row.get(0)?,
                    source_id: row.get(1)?,
                    operator_id: row.get(2)?,
                    text: row.get(3)?,
                    priority: row.get(4)?,
                    revision: row.get(5)?,
                    state,
                    project_ids: Vec::new(),
                    terminal_receipt_digest: row.get(7)?,
                    created_at_ms: row.get(8)?,
                    updated_at_ms: row.get(9)?,
                })
            },
        )
        .optional()?;
    let Some(mut record) = base else {
        return Ok(None);
    };
    let mut statement = connection.prepare(
        "SELECT project_id FROM objective_projects WHERE objective_id = ?1 ORDER BY ordinal",
    )?;
    let rows = statement.query_map([objective_id], |row| row.get::<_, String>(0))?;
    record.project_ids = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Some(record))
}

fn schedule_in(connection: &Connection, schedule_id: &str) -> Result<Option<ScheduleSpec>> {
    connection
        .query_row(
            "SELECT id, objective_id, next_wake_ms, recurrence, idempotency_key
             FROM schedules WHERE id = ?1",
            [schedule_id],
            schedule_from_row,
        )
        .optional()
        .context("read objective schedule")
}

fn leaf_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<LeafRecord> {
    Ok(LeafRecord {
        id: row.get(0)?,
        objective_id: row.get(1)?,
        project_id: row.get(2)?,
        workspace_root: row.get(3)?,
        authority: row.get(4)?,
        stage: parse_leaf_stage(row.get::<_, String>(5)?)?,
        attempt: row.get(6)?,
        lease_owner: row.get(7)?,
        lease_expires_ms: row.get(8)?,
        current_receipt_digest: row.get(9)?,
        project_contract_digest: row.get(10)?,
        execution: parse_execution_spec(row.get(11)?)?,
    })
}

fn schedule_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ScheduleSpec> {
    Ok(ScheduleSpec {
        id: row.get(0)?,
        objective_id: row.get(1)?,
        next_wake_ms: row.get(2)?,
        recurrence: row.get(3)?,
        idempotency_key: row.get(4)?,
    })
}

fn parse_objective_state(value: String) -> rusqlite::Result<ObjectiveState> {
    ObjectiveState::parse(&value).ok_or_else(|| invalid_enum("objective state", &value))
}

fn parse_leaf_stage(value: String) -> rusqlite::Result<LeafStage> {
    LeafStage::parse(&value).ok_or_else(|| invalid_enum("leaf stage", &value))
}

fn parse_execution_spec(value: Option<String>) -> rusqlite::Result<Option<LeafExecutionSpec>> {
    value
        .map(|value| {
            serde_json::from_str(&value).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })
        .transpose()
}

fn invalid_enum(kind: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        format!("invalid {kind} {value}").into(),
    )
}

#[cfg(test)]
mod resident_marker_tests {
    use super::*;

    #[test]
    fn snapshot_policy_activation_is_checked_inside_claim_transaction() {
        let dir = tempfile::tempdir().unwrap();
        let store = ObjectiveStore::open(dir.path().join("objectives.sqlite3")).unwrap();
        store
            .create_authenticated_objective(
                NewObjective {
                    id: "policy".into(),
                    source_id: "policy".into(),
                    idempotency_key: "policy".into(),
                    operator_id: "owner".into(),
                    text: "policy race".into(),
                    priority: 0,
                    projects: vec![],
                    leaves: vec![crate::objectives::NewLeaf {
                        id: "policy-leaf".into(),
                        project_id: None,
                        workspace_root: dir.path().display().to_string(),
                        authority: "read_only".into(),
                        dependencies: vec![],
                        execution: None,
                    }],
                },
                1,
            )
            .unwrap();
        store
            .apply_control(
                "policy",
                ControlAction::Approve { revision: 1 },
                "approve",
                "owner",
                2,
            )
            .unwrap();
        // Force the exact interleaving: precheck passed, another writer activates
        // policy, then the original unconfigured handle enters admission.
        store.reconcile_snapshot_commits().unwrap();
        store
            .connection()
            .unwrap()
            .execute("INSERT INTO retained_snapshot_policy VALUES (1)", [])
            .unwrap();
        let result =
            store.claim_leaves_with_topology("worker", 3, 100, 1, false, read_workspace_topology);
        assert!(result.is_err(), "admission ignored newly activated policy");
        assert_eq!(store.leaf("policy-leaf").unwrap().unwrap().attempt, 0);
    }

    #[test]
    fn admission_topology_drift_rolls_back_claim_and_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        store
            .create_authenticated_objective(
                NewObjective {
                    id: "drift".into(),
                    source_id: "drift".into(),
                    idempotency_key: "drift".into(),
                    operator_id: "owner".into(),
                    text: "topology race".into(),
                    priority: 0,
                    projects: vec![],
                    leaves: vec![crate::objectives::NewLeaf {
                        id: "drift-leaf".into(),
                        project_id: None,
                        workspace_root: dir.path().display().to_string(),
                        authority: "read_only".into(),
                        dependencies: vec![],
                        execution: None,
                    }],
                },
                1,
            )
            .unwrap();
        store
            .apply_control(
                "drift",
                ControlAction::Approve { revision: 1 },
                "approve",
                "owner",
                2,
            )
            .unwrap();
        let baseline = read_workspace_topology().unwrap();
        let mut reads = 0;
        let result = store.claim_leaves_with_topology("worker", 3, 100, 1, false, || {
            reads += 1;
            let mut snapshot = baseline.clone();
            if reads == 2 {
                snapshot.push(b'\n');
            }
            Ok(snapshot)
        });
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("topology changed during admission"));
        assert_eq!(reads, 2);
        drop(store);
        let reopened = ObjectiveStore::open(&path).unwrap();
        let leaf = reopened.leaf("drift-leaf").unwrap().unwrap();
        assert_eq!(leaf.attempt, 0);
        let count: i64 = reopened
            .connection()
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM lease_workspace_identities",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert_eq!(
            reopened.claim_runnable("worker", 4, 100, 1).unwrap().len(),
            1
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn mountpoint_parser_preserves_bytes_and_rejects_unknown_topology() {
        use std::os::unix::ffi::OsStrExt;
        let mounts = workspace_mountpoints(
            b"1 0 0:1 / /space\\040tab\\011line\\012slash\\134\xff rw - tmpfs tmpfs rw\n",
        )
        .unwrap();
        assert_eq!(
            mounts[0].2.as_os_str().as_bytes(),
            b"/space tab\tline\nslash\\\xff"
        );
        for invalid in [
            b"".as_slice(),
            b"1 0",
            b"1 0 0:1 / relative rw",
            b"1 0 0:1 / /truncated\\0 rw",
            b"1 0 0:1 / /unknown\\123 rw",
        ] {
            assert!(workspace_mountpoints(invalid).is_err());
        }
    }

    #[test]
    fn committed_schedule_wakes_all_observers_but_replay_and_rejection_do_not() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        let writer = ObjectiveStore::open(&path).unwrap();
        writer
            .create_authenticated_objective(
                NewObjective {
                    id: "wake".into(),
                    source_id: "wake-source".into(),
                    idempotency_key: "wake-ingress".into(),
                    operator_id: "owner".into(),
                    text: "wake test".into(),
                    priority: 0,
                    projects: vec![],
                    leaves: vec![crate::objectives::NewLeaf {
                        id: "wake-leaf".into(),
                        project_id: None,
                        workspace_root: dir.path().display().to_string(),
                        authority: "read_only".into(),
                        dependencies: vec![],
                        execution: None,
                    }],
                },
                1,
            )
            .unwrap();
        let mut first = store.subscribe_changes();
        let mut second = store.subscribe_changes();
        let other = ObjectiveStore::open(dir.path().join("other.sqlite3")).unwrap();
        let unrelated = other.subscribe_changes();
        let schedule = ScheduleSpec {
            id: "wake-schedule".into(),
            objective_id: "wake".into(),
            next_wake_ms: 1000,
            recurrence: None,
            idempotency_key: "wake-schedule".into(),
        };
        writer.put_schedule(schedule.clone(), 2).unwrap();
        assert!(first.has_changed().unwrap());
        assert!(second.has_changed().unwrap());
        assert!(!unrelated.has_changed().unwrap());
        first.borrow_and_update();
        second.borrow_and_update();
        writer.put_schedule(schedule.clone(), 3).unwrap();
        let mut changed = schedule;
        changed.next_wake_ms = 2000;
        assert!(writer.put_schedule(changed, 4).is_err());
        assert!(!first.has_changed().unwrap());
        assert!(!second.has_changed().unwrap());
    }

    #[test]
    fn operator_marker_targets_one_run_preserves_evidence_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("objectives.sqlite3");
        let store = ObjectiveStore::open(&path).unwrap();
        store.connection().unwrap().execute_batch(
            "INSERT INTO resident_context_bindings VALUES ('run-1', 'digest-1', 'original-1', NULL);
             INSERT INTO resident_context_bindings VALUES ('run-2', 'digest-2', 'original-2', NULL);",
        ).unwrap();
        store.resident_context_deleted_by_operator("run-1").unwrap();
        let read = || {
            let connection = store.connection().unwrap();
            let mut query = connection
                .prepare(
                    "SELECT run_id, request_digest, assembly_json, deleted_by_operator_ms
                 FROM resident_context_bindings ORDER BY run_id",
                )
                .unwrap();
            query
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, Option<i64>>(3)?,
                    ))
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap()
        };
        let marked = read();
        assert_eq!(
            (&marked[0].0, &marked[0].1, &marked[0].2),
            (&"run-1".into(), &"digest-1".into(), &"original-1".into())
        );
        assert!(marked[0].3.is_some_and(|timestamp| timestamp > 0));
        assert_eq!(
            marked[1],
            ("run-2".into(), "digest-2".into(), "original-2".into(), None)
        );
        std::thread::sleep(Duration::from_millis(5));
        store.resident_context_deleted_by_operator("run-1").unwrap();
        assert_eq!(read(), marked);
        assert!(store
            .resident_context_deleted_by_operator("unknown")
            .is_err());
        assert_eq!(read(), marked);
        let reopened = ObjectiveStore::open(&path).unwrap();
        let error = reopened.resident_context("run-1", "digest-1").unwrap_err();
        assert!(
            error.to_string().contains("deleted by operator"),
            "{error:#}"
        );
    }
}
