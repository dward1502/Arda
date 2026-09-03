use super::migrations;
use super::model::{
    ClaimedLeaf, ControlAction, LeafExecutionSpec, LeafRecord, LeafStage, NewObjective,
    ObjectiveRecord, ObjectiveState, ReceiptStage, ScheduleSpec, StageReceipt,
};
use anyhow::{anyhow, bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ObjectiveStore {
    path: PathBuf,
}

impl ObjectiveStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create ObjectiveStore directory {}", parent.display()))?;
        }
        let store = Self { path };
        let connection = store.connection()?;
        migrations::apply(&connection)?;
        Ok(store)
    }

    pub fn create_authenticated_objective(
        &self,
        objective: NewObjective,
        now_ms: i64,
    ) -> Result<ObjectiveRecord> {
        validate_objective(&objective)?;
        let payload_digest = digest_json(&objective)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin objective creation")?;

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

        transaction.execute(
            "INSERT INTO controls
             (idempotency_key, objective_id, operator_id, action_json, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)",
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
        Ok(updated)
    }

    pub fn claim_runnable(
        &self,
        lease_owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
        capacity: usize,
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
        let candidate_limit = capacity.saturating_mul(8).saturating_add(32) as i64;
        let candidates = {
            let mut statement = transaction.prepare(
                "SELECT l.id
                 FROM leaves l
                 JOIN objectives o ON o.id = l.objective_id
                 WHERE o.state IN (?1, ?2)
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
                ],
                |row| row.get::<_, String>(0),
            )?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };

        let expires_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or_else(|| anyhow!("lease expiry overflow"))?;
        let mut claimed = Vec::new();
        for leaf_id in candidates {
            if claimed.len() == capacity {
                break;
            }
            let changed = transaction.execute(
                "UPDATE leaves AS target
                 SET lease_owner = ?1, lease_expires_ms = ?2, attempt = attempt + 1,
                     updated_at_ms = ?3
                 WHERE id = ?4
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
                ],
            )?;
            if changed == 0 {
                continue;
            }
            let mut claim = transaction.query_row(
                "SELECT objective_id, id, project_id, workspace_root, authority, stage, attempt,
                        current_receipt_digest,
                        (SELECT contract_digest FROM objective_projects p
                         WHERE p.objective_id = leaves.objective_id
                           AND p.project_id = leaves.project_id),
                        execution_json
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
                        lease_owner: lease_owner.to_owned(),
                        lease_expires_ms: expires_ms,
                        current_receipt_digest: row.get(7)?,
                        project_contract_digest: row.get(8)?,
                        execution: parse_execution_spec(row.get(9)?)?,
                        dependency_receipts: Vec::new(),
                    })
                },
            )?;
            claim.dependency_receipts = {
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
                let rows =
                    statement.query_map(params![leaf_id, ReceiptStage::Close.as_str()], |row| {
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
                rows.collect::<rusqlite::Result<Vec<_>>>()?
            };
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
        }
        transaction.commit().context("commit objective claims")?;
        Ok(claimed)
    }

    pub fn record_stage_receipt(
        &self,
        leaf_id: &str,
        lease_owner: &str,
        receipt: StageReceipt,
        now_ms: i64,
    ) -> Result<()> {
        validate_receipt(&receipt)?;
        let mut connection = self.connection()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .context("begin receipt recording")?;

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
                transaction.commit()?;
                return Ok(());
            }
            bail!("receipt idempotency conflict for {leaf_id}");
        }

        let leaf = transaction
            .query_row(
                "SELECT stage, lease_owner, lease_expires_ms, current_receipt_digest
                 FROM leaves WHERE id = ?1",
                [leaf_id],
                |row| {
                    Ok((
                        parse_leaf_stage(row.get::<_, String>(0)?)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, Option<i64>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("leaf {leaf_id} does not exist"))?;
        if leaf.0 != receipt.stage.leaf_stage() {
            bail!("receipt stage does not match current leaf stage");
        }
        if leaf.1.as_deref() != Some(lease_owner) || leaf.2.is_none_or(|expiry| expiry < now_ms) {
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
        transaction.commit().context("commit stage receipt")?;
        Ok(())
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
        Ok(schedule)
    }

    pub fn schedule(&self, schedule_id: &str) -> Result<Option<ScheduleSpec>> {
        let connection = self.connection()?;
        schedule_in(&connection, schedule_id)
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

    fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)
            .with_context(|| format!("open ObjectiveStore {}", self.path.display()))?;
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
        for dependency in &leaf.dependencies {
            if dependency == &leaf.id || !leaves.contains(dependency.as_str()) {
                bail!("leaf {} has an invalid dependency {}", leaf.id, dependency);
            }
        }
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
