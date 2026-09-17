//! Cross-store stop ordering for recovery. This is a control fence, not an
//! authenticated admission or a replacement for canonical evidence checks.
use super::{ObjectiveStore, StageReceipt};
use crate::runs::RecoveryGrant;
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
mod admission;
mod material;
mod publications;
pub use material::RecoveryMaterial;
pub(crate) use publications::RecoveryPublication;

impl ObjectiveStore {
    /// Recognize an exact already-published chain and retry only terminal keeper
    /// release. No active recovery window/lease is required for cleanup, and no
    /// execution, receipt writes or objective scheduling can occur here.
    /// Caller must authenticate the owner and verify historical canonical files
    /// in the callback; a saved grant alone does not authenticate transport.
    pub fn reconcile_completed_recovery(
        &self,
        operator_id: &str,
        event_id: &str,
        receipts: &[StageReceipt],
        validate: impl FnOnce(&Transaction<'_>, &RecoveryGrant, &[StageReceipt]) -> Result<()>,
    ) -> Result<bool> {
        use super::super::model::ReceiptStage;
        anyhow::ensure!(
            receipts.iter().map(|r| r.stage).eq([
                ReceiptStage::Execute,
                ReceiptStage::Verify,
                ReceiptStage::Review,
                ReceiptStage::Close,
            ]),
            "completed recovery requires its full ordered receipt chain"
        );
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let grant = read_admission(&transaction, operator_id, event_id)?
            .context("recovery admission intent is missing")?;
        anyhow::ensure!(
            receipts[0].digest == grant.bindings.execute_receipt_digest,
            "completed recovery execution receipt changed"
        );
        let complete: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM leaves l JOIN objectives o ON o.id=l.objective_id
             JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
             JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt
             WHERE l.id=?1 AND l.objective_id=?2 AND o.operator_id=?3 AND l.execution_run_id=?4
               AND l.stage='complete' AND l.authority='read_only'
               AND l.lease_owner IS NULL AND l.lease_expires_ms IS NULL
               AND l.current_receipt_digest=?5 AND s.committed_generation=l.attempt
               AND i.recovery_event_id=?6)",
            params![
                grant.bindings.leaf_id,
                grant.bindings.objective_id,
                operator_id,
                grant.bindings.run_id.as_str(),
                receipts[3].digest,
                event_id
            ],
            |row| row.get(0),
        )?;
        if !complete {
            return Ok(false);
        }
        for receipt in receipts {
            super::validate_receipt(receipt)?;
            let exact: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM stage_receipts
                 WHERE leaf_id=?1 AND stage=?2 AND contract=?3 AND digest=?4
                   AND predecessor_digest IS ?5 AND run_path=?6 AND provider=?7 AND model=?8
                   AND started_at_ms=?9 AND completed_at_ms=?10 AND verdict=?11
                   AND context_outcome_receipt_id IS ?12 AND context_outcome_receipt_digest IS ?13
                   AND binding_digest IS ?14)",
                params![
                    grant.bindings.leaf_id,
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
                    receipt.binding_digest
                ],
                |row| row.get(0),
            )?;
            anyhow::ensure!(
                exact,
                "completed recovery receipt chain differs from stored evidence"
            );
        }
        validate(&transaction, &grant, receipts)?;
        transaction.commit()?;
        self.reconcile_snapshot_commits_for_leaf(&grant.bindings.leaf_id)?;
        Ok(true)
    }

    /// Project the complete canonical chain under the same stop/lease fence.
    /// The trusted callback must validate canonical files, graph lineage,
    /// context outcomes and policy; caller-supplied digests are not proof.
    pub fn record_retained_recovery_receipts(
        &self,
        operator_id: &str,
        gateway_event_id: &str,
        expected: &super::super::snapshot_protocol::Lease,
        receipts: &[StageReceipt],
        validate: impl FnOnce(
            &Transaction<'_>,
            &RecoveryGrant,
            &super::super::snapshots::RetainedExecution,
            &[StageReceipt],
        ) -> Result<()>,
    ) -> Result<()> {
        use super::super::model::ReceiptStage;
        anyhow::ensure!(
            receipts.iter().map(|receipt| receipt.stage).eq([
                ReceiptStage::Execute,
                ReceiptStage::Verify,
                ReceiptStage::Review,
                ReceiptStage::Close,
            ]),
            "recovery projection requires the complete ordered receipt chain"
        );
        self.with_retained_recovery(
            operator_id,
            gateway_event_id,
            expected,
            |tx, grant, retained| {
                anyhow::ensure!(
                    receipts[0].digest == grant.bindings.execute_receipt_digest,
                    "recovery cannot replace the successful execution receipt"
                );
                validate(tx, grant, retained, receipts)?;
                for receipt in receipts {
                    let now = chrono::Utc::now().timestamp_millis();
                    anyhow::ensure!(
                        grant.is_active(u64::try_from(now)?),
                        "recovery window expired"
                    );
                    anyhow::ensure!(expected.expires_ms > now, "recovery lease expired");
                    Self::record_stage_receipt_in(
                        tx,
                        &grant.bindings.leaf_id,
                        &expected.owner,
                        expected.generation,
                        receipt,
                        now,
                    )?;
                }
                // Expiry while validating/writing rolls the entire chain back.
                let now = chrono::Utc::now().timestamp_millis();
                anyhow::ensure!(
                    grant.is_active(u64::try_from(now)?),
                    "recovery window expired"
                );
                anyhow::ensure!(expected.expires_ms > now, "recovery lease expired");
                Ok(())
            },
        )?;
        self.notify_change();
        Ok(())
    }

    /// Order the canonical stop/lease fence before atomic journal admission.
    /// The callback must validate captured evidence/current policy, and only an
    /// Appended outcome may dispatch. Reauthorize immediately before launch.
    pub fn append_retained_recovery_start(
        &self,
        operator_id: &str,
        event_id: &str,
        expected: &super::super::snapshot_protocol::Lease,
        run_store: &crate::runs::RunStore,
        draft: crate::runs::RunEventDraft,
        validate: impl FnOnce(
            &Transaction<'_>,
            &RecoveryGrant,
            &super::super::RetainedExecution,
        ) -> Result<()>,
    ) -> Result<crate::runs::AppendOutcome> {
        if run_store.run_id().as_str() != expected.run_id {
            bail!("recovery journal does not match retained run");
        }
        self.with_retained_recovery(operator_id, event_id, expected, |tx, grant, retained| {
            validate(tx, grant, retained)?;
            Ok(run_store.append_recovery_start_before(
                grant,
                draft,
                u64::try_from(expected.expires_ms)?,
            )?)
        })
    }

    /// Resolve recovery's exact acknowledged lease under the stop fence. The
    /// caller must validate canonical evidence/current policy in the callback;
    /// ordinary retained_execution intentionally continues to reject Paused.
    /// Never acquire this fence while already holding a RunStore journal lock.
    pub fn with_retained_recovery<T>(
        &self,
        operator_id: &str,
        event_id: &str,
        expected: &super::super::snapshot_protocol::Lease,
        operation: impl FnOnce(
            &Transaction<'_>,
            &RecoveryGrant,
            &super::super::RetainedExecution,
        ) -> Result<T>,
    ) -> Result<T> {
        self.with_retained_recovery_clock(operator_id, event_id, expected, operation, || {
            Ok(i64::try_from(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_millis(),
            )?)
        })
    }

    fn with_retained_recovery_clock<T>(
        &self,
        operator_id: &str,
        event_id: &str,
        expected: &super::super::snapshot_protocol::Lease,
        operation: impl FnOnce(
            &Transaction<'_>,
            &RecoveryGrant,
            &super::super::RetainedExecution,
        ) -> Result<T>,
        mut clock: impl FnMut() -> Result<i64>,
    ) -> Result<T> {
        self.with_recorded_recovery_admission(operator_id, event_id, |tx, grant| {
            if expected.run_id != grant.bindings.run_id.as_str()
                || expected.expires_ms > i64::try_from(grant.expires_at_unix_ms)? {
                bail!("recovery lease is outside the saved grant");
            }
            let now = clock()?;
            let encoded: Option<String> = tx.query_row(
                "SELECT s.capability_json FROM leaves l
                 JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
                 JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt
                 JOIN resident_context_bindings c ON c.run_id=l.execution_run_id
                 WHERE l.id=?1 AND l.execution_run_id=?2 AND l.attempt=?3
                   AND s.committed_generation=l.attempt
                   AND l.lease_owner=?4 AND i.lease_owner=l.lease_owner
                   AND l.lease_expires_ms=?5 AND i.lease_expires_ms=l.lease_expires_ms
                   AND l.lease_expires_ms>?6 AND i.recovery_event_id=?7
                   AND l.context_bound=1 AND c.deleted_by_operator_ms IS NULL
                   AND NOT EXISTS(SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id=l.id)",
                params![grant.bindings.leaf_id, expected.run_id, expected.generation, expected.owner, expected.expires_ms, now, event_id],
                |row| row.get(0),
            ).optional()?;
            let encoded = encoded.context("recovery retained lease is missing, fenced, expired or released")?;
            let retained = super::super::RetainedExecution {
                snapshot: serde_json::from_str(&encoded).context("decode retained recovery snapshot")?,
                lease: expected.clone(),
            };
            let result = operation(tx, grant, &retained)?;
            let now = clock()?;
            anyhow::ensure!(now < expected.expires_ms, "recovery lease expired during validation");
            anyhow::ensure!(grant.is_active(u64::try_from(now)?), "recovery grant expired during validation");
            Ok(result)
        })
    }

    /// Claim only the saved recovery leaf, without invoking ordinary scheduling.
    /// The trusted caller must revalidate captured evidence and current policy;
    /// this method does not authenticate transport or manufacture run authority.
    pub fn claim_retained_recovery(
        &self,
        operator_id: &str,
        event_id: &str,
        lease_owner: &str,
        lease_duration_ms: i64,
        validate: impl Fn(&Transaction<'_>, &RecoveryGrant) -> Result<()>,
    ) -> Result<super::ClaimedLeaf> {
        self.claim_retained_recovery_phased(
            operator_id,
            event_id,
            lease_owner,
            lease_duration_ms,
            &validate,
            &validate,
        )
    }

    /// Canonical validation with a narrowly scoped pending-acknowledgement phase.
    /// Transport authentication remains the ingress caller's responsibility.
    pub fn claim_validated_retained_recovery(
        &self,
        root: &std::path::Path,
        operator_id: &str,
        event_id: &str,
        lease_owner: &str,
        lease_duration_ms: i64,
    ) -> Result<super::ClaimedLeaf> {
        self.claim_retained_recovery_phased(
            operator_id,
            event_id,
            lease_owner,
            lease_duration_ms,
            |tx, grant| self.validate_recovery_before_reconciliation(root, tx, grant),
            |tx, grant| {
                self.validate_retained_recovery_material(root, tx, grant)
                    .map(|_| ())
            },
        )
    }

    fn claim_retained_recovery_phased(
        &self,
        operator_id: &str,
        event_id: &str,
        lease_owner: &str,
        lease_duration_ms: i64,
        validate_before_reconciliation: impl Fn(&Transaction<'_>, &RecoveryGrant) -> Result<()>,
        validate: impl Fn(&Transaction<'_>, &RecoveryGrant) -> Result<()>,
    ) -> Result<super::ClaimedLeaf> {
        if lease_owner.trim().is_empty() || lease_duration_ms <= 0 {
            bail!("recovery lease owner and positive duration are required");
        }
        let leaf_id =
            self.with_recorded_recovery_admission(operator_id, event_id, |tx, grant| {
                validate_before_reconciliation(tx, grant)?;
                Ok(grant.bindings.leaf_id.clone())
            })?;
        self.reconcile_snapshot_commits_for_leaf(&leaf_id)?;
        let claim = self.with_recorded_recovery_admission(operator_id, event_id, |tx, grant| {
            validate(tx, grant)?;
            let now = i64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis())?;
            let expiry = now.checked_add(lease_duration_ms).context("recovery lease overflow")?
                .min(i64::try_from(grant.expires_at_unix_ms)?);
            if expiry <= now { bail!("recovery lease expired before claim"); }
            super::super::snapshots::check_policy(tx, self.snapshot_admission.is_some())?;
            if self.snapshot_admission.is_none() { bail!("recovery keeper is unavailable"); }
            let eligible: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM leaves l
                  JOIN retained_workspace_snapshots s ON s.leaf_id=l.id AND s.run_id=l.execution_run_id
                  JOIN lease_workspace_identities w ON w.leaf_id=l.id
                  JOIN resident_context_bindings c ON c.run_id=l.execution_run_id
                  WHERE l.id=?1 AND l.execution_run_id=?2 AND l.attempt>0 AND l.context_bound=1
                    AND l.execution_json IS NOT NULL AND l.project_id IS NOT NULL
                    AND c.deleted_by_operator_ms IS NULL
                    AND NOT EXISTS(SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id=l.id)
                    AND NOT EXISTS(SELECT 1 FROM leaves busy WHERE busy.id!=l.id AND busy.lease_owner IS NOT NULL AND busy.lease_expires_ms>?3)
                    AND NOT EXISTS(SELECT 1 FROM leaf_dependencies d JOIN leaves parent ON parent.id=d.dependency_leaf_id WHERE d.leaf_id=l.id AND parent.stage!='complete')
                    AND s.committed_generation=l.attempt)",
                params![leaf_id,grant.bindings.run_id.as_str(),now], |row| row.get(0),
            )?;
            if !eligible { bail!("exact retained recovery leaf is not claimable"); }
            let active: bool = tx.query_row("SELECT lease_owner IS NOT NULL AND lease_expires_ms>?2 FROM leaves WHERE id=?1", params![leaf_id,now], |row| row.get(0))?;
            if active {
                let replay: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM leaves l JOIN retained_snapshot_lease_intents i ON i.leaf_id=l.id AND i.generation=l.attempt WHERE l.id=?1 AND l.lease_owner=?2 AND i.lease_owner=l.lease_owner AND i.lease_expires_ms=l.lease_expires_ms AND i.recovery_event_id=?3)", params![leaf_id,lease_owner,event_id], |row| row.get(0))?;
                if !replay { bail!("recovery leaf already has a different live lease"); }
                return Self::decode_claim(tx, &leaf_id);
            }
            let previous: i64 = tx.query_row("SELECT attempt FROM leaves WHERE id=?1", [&leaf_id], |row| row.get(0))?;
            let generation = previous.checked_add(1).context("recovery lease generation exhausted")?;
            tx.execute("UPDATE leaves SET attempt=?1,lease_owner=?2,lease_expires_ms=?3,updated_at_ms=?4 WHERE id=?5",
                params![generation,lease_owner,expiry,now,leaf_id])?;
            tx.execute("INSERT INTO retained_snapshot_lease_intents (leaf_id,generation,lease_owner,lease_expires_ms,recovery_event_id) VALUES (?1,?2,?3,?4,?5)",
                params![leaf_id,generation,lease_owner,expiry,event_id])?;
            Self::decode_claim(tx, &leaf_id)
        })?;
        self.reconcile_snapshot_commits_for_leaf(&leaf_id)?;
        self.with_recorded_recovery_admission(operator_id, event_id, |tx, grant| {
            validate(tx, grant)?;
            let current = Self::decode_claim(tx, &leaf_id)?;
            let acknowledged: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM retained_workspace_snapshots s JOIN retained_snapshot_lease_intents i ON i.leaf_id=s.leaf_id AND i.generation=s.committed_generation WHERE s.leaf_id=?1 AND i.generation=?2 AND i.lease_owner=?3 AND i.lease_expires_ms=?4 AND i.recovery_event_id=?5)",
                params![leaf_id,claim.attempt,claim.lease_owner,claim.lease_expires_ms,event_id], |row| row.get(0),
            )?;
            let now = i64::try_from(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis())?;
            if !acknowledged || current.attempt != claim.attempt || current.lease_owner != claim.lease_owner
                || current.lease_expires_ms != claim.lease_expires_ms || current.lease_expires_ms <= now {
                bail!("recovery lease acknowledgement changed or expired");
            }
            Ok(current)
        })
    }

    /// Save a server-derived admission before journal activation. The callback
    /// must check canonical evidence and policy under this transaction. Neither
    /// a grant nor a gateway binding alone authenticates the operator.
    pub fn record_recovery_admission_intent(
        &self,
        operator_id: &str,
        candidate: &RecoveryGrant,
        validate: impl FnOnce(&Transaction<'_>, &RecoveryGrant) -> Result<()>,
    ) -> Result<RecoveryGrant> {
        self.with_recovery_control_fence(operator_id, candidate, |transaction| {
            if let Some(saved) = read_admission(transaction, operator_id, &candidate.authenticated_event_id)? {
                if saved.authenticated_payload_digest != candidate.authenticated_payload_digest
                    || saved.bindings != candidate.bindings {
                    bail!("recovery admission replay changed immutable bindings");
                }
                validate(transaction, &saved)?;
                return Ok(saved);
            }
            check_gateway_binding(transaction, candidate)?;
            validate(transaction, candidate)?;
            transaction.execute(
                "INSERT INTO recovery_admissions (authenticated_event_id,operator_id,objective_id,leaf_id,run_id,grant_json) VALUES (?1,?2,?3,?4,?5,?6)",
                params![candidate.authenticated_event_id,operator_id,candidate.bindings.objective_id,candidate.bindings.leaf_id,candidate.bindings.run_id.as_str(),serde_json::to_string(candidate)?],
            )?;
            Ok(candidate.clone())
        })
    }

    /// Resolve the saved intent and hold the stop fence through a journal or
    /// claim mutation. Callback must check current evidence, lease and policy.
    /// A transport handler must never substitute a request-supplied grant.
    pub fn with_recorded_recovery_admission<T>(
        &self,
        operator_id: &str,
        event_id: &str,
        operation: impl FnOnce(&Transaction<'_>, &RecoveryGrant) -> Result<T>,
    ) -> Result<T> {
        let saved = read_admission(&self.connection()?, operator_id, event_id)?
            .context("recovery admission intent is missing")?;
        self.with_recovery_control_fence(operator_id, &saved, |transaction| {
            let current = read_admission(transaction, operator_id, event_id)?
                .context("recovery admission intent disappeared")?;
            if current != saved {
                bail!("recovery admission changed while acquiring control fence");
            }
            operation(transaction, &current)
        })
    }

    /// Run an already-authorized recovery mutation while later Pause/Cancel
    /// controls are excluded. The caller must validate the saved admission,
    /// canonical evidence, lease and current policy inside this transaction.
    /// Lock order is transport ledger -> ObjectiveStore -> RunStore journal.
    /// Never call an API that reopens this ObjectiveStore from the callback.
    /// A journal write cannot be undone by SQLite rollback: retries must retain
    /// its original immutable identity and must never redispatch a spent start.
    pub fn with_recovery_control_fence<T>(
        &self,
        operator_id: &str,
        grant: &RecoveryGrant,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        self.with_recovery_control_fence_inner(operator_id, grant, true, operation)
    }

    fn with_recovery_control_fence_inner<T>(
        &self,
        operator_id: &str,
        grant: &RecoveryGrant,
        require_active_window: bool,
        operation: impl FnOnce(&Transaction<'_>) -> Result<T>,
    ) -> Result<T> {
        grant.validate().map_err(anyhow::Error::msg)?;
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let valid: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM objectives o JOIN leaves l ON l.objective_id=o.id
             WHERE o.id=?1 AND o.operator_id=?2 AND o.state='paused'
               AND o.revision=?3 AND o.approved_revision=o.revision AND o.stop_generation=?4
               AND l.id=?5 AND l.execution_run_id=?6 AND l.authority='read_only'
               AND l.stage IN ('execute','verify','review','close'))",
            params![
                grant.bindings.objective_id,
                operator_id,
                grant.bindings.objective_revision,
                i64::try_from(grant.bindings.stop_generation)?,
                grant.bindings.leaf_id,
                grant.bindings.run_id.as_str()
            ],
            |row| row.get(0),
        )?;
        if !valid {
            bail!("recovery control was superseded or canonical lineage changed");
        }
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .context("recovery clock unavailable")?
                .as_millis(),
        )?;
        if require_active_window && !grant.is_active(now) {
            bail!("recovery window is not active");
        }
        let result = operation(&transaction)?;
        transaction.commit()?;
        Ok(result)
    }
}

fn check_gateway_binding(connection: &Connection, grant: &RecoveryGrant) -> Result<()> {
    let payload: Option<String> = connection
        .query_row(
            "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
            [&grant.authenticated_event_id],
            |row| row.get(0),
        )
        .optional()?;
    // Ingress stores bare hex; the recovery journal uses an explicit label.
    if payload.as_deref() != grant.authenticated_payload_digest.strip_prefix("sha256:") {
        bail!("recovery admission lacks the exact authenticated payload binding");
    }
    Ok(())
}

fn read_admission(
    connection: &Connection,
    operator_id: &str,
    event_id: &str,
) -> Result<Option<RecoveryGrant>> {
    let row: Option<(String,String,String,String,String)> = connection.query_row(
        "SELECT operator_id,objective_id,leaf_id,run_id,grant_json FROM recovery_admissions WHERE authenticated_event_id=?1",
        [event_id], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?,row.get(3)?,row.get(4)?)),
    ).optional()?;
    let Some((owner, objective, leaf, run, json)) = row else {
        return Ok(None);
    };
    let grant: RecoveryGrant =
        serde_json::from_str(&json).context("invalid recovery admission intent")?;
    grant.validate().map_err(anyhow::Error::msg)?;
    if owner != operator_id
        || grant.authenticated_event_id != event_id
        || grant.bindings.objective_id != objective
        || grant.bindings.leaf_id != leaf
        || grant.bindings.run_id.as_str() != run
    {
        bail!("recovery admission intent envelope conflicts with its grant");
    }
    check_gateway_binding(connection, &grant)?;
    Ok(Some(grant))
}
