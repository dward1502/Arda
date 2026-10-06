//! Durable admission intents for independently owned retained mount snapshots.
//! Keeper operations must be bounded; they run under the admission writer lock.
use anyhow::{bail, Context, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Private transport material. Deliberately not Debug or part of ClaimedLeaf:
/// capabilities must not enter prompts, receipts, or public projections.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedSnapshot {
    pub endpoint: String,
    pub capability: String,
    pub manifest_digest: String,
}

/// Private execution authority read from one durable store snapshot. Not a
/// receipt or public projection; the worker independently fences this lease.
#[derive(Clone)]
pub struct RetainedExecution {
    pub snapshot: RetainedSnapshot,
    pub lease: super::snapshot_protocol::Lease,
}

/// Independent keeper boundary, not a daemon-owned subprocess factory.
/// Prepare must be idempotent for admission_id and must never recreate a lost
/// retained tree. The owner must retain uncommitted preparations for explicit
/// reconciliation, including preparations whose SQLite transaction rolls back.
pub trait SnapshotAdmission: Send + Sync {
    fn prepare(
        &self,
        admission_id: &str,
        workspace: &Path,
        workspace_identity: &str,
    ) -> Result<RetainedSnapshot>;

    /// Idempotently admit/rebind exactly this capability to a monotonically
    /// increasing generation. Reject older generations and changed same-generation
    /// payloads. Success must mean any previous execution has stopped and joined.
    /// Missing keeper/tree is an error, never an instruction to prepare again.
    fn commit(
        &self,
        snapshot: &RetainedSnapshot,
        run_id: &str,
        generation: i64,
        lease_owner: &str,
        lease_expires_ms: i64,
    ) -> Result<()>;

    /// Idempotent terminal revocation: prohibit all future execution and prove
    /// process/tree teardown before success. A retry after lost acknowledgement
    /// requires the independent owner's durable release receipt; an absent socket
    /// alone is not proof of release. Never prepare a replacement on this path.
    fn release(&self, snapshot: &RetainedSnapshot, run_id: &str) -> Result<()>;

    /// Read-only proof query, never a replacement capability or cleanup ACK.
    fn terminal_revocation(
        &self,
        _run: &str,
        _workspace: &str,
        _identity: &str,
    ) -> Result<super::terminal_revocation::TerminalRevocationProof> {
        bail!("terminal revocation query unsupported")
    }
}

pub(super) fn check_policy(transaction: &Transaction<'_>, configured: bool) -> Result<()> {
    if configured {
        transaction.execute(
            "INSERT OR IGNORE INTO retained_snapshot_policy VALUES (1)",
            [],
        )?;
    } else {
        let required: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM retained_snapshot_policy)",
            [],
            |row| row.get(0),
        )?;
        if required {
            bail!("retained snapshot keeper is not configured; reconciliation required");
        }
    }
    Ok(())
}

pub(super) fn prepare(
    transaction: &Transaction<'_>,
    keeper: Option<&dyn SnapshotAdmission>,
    leaf_id: &str,
    workspace: &Path,
    identity: &str,
    first_attempt: bool,
) -> Result<()> {
    super::abandonment::guards::reject_in(transaction, None, Some(leaf_id), None)?;
    let saved: Option<String> = transaction
        .query_row(
            "SELECT capability_json FROM retained_workspace_snapshots WHERE leaf_id = ?1",
            [leaf_id],
            |row| row.get(0),
        )
        .optional()?;
    if saved.is_some() {
        if keeper.is_none() {
            bail!("retained snapshot keeper is not configured; reconciliation required");
        }
        // Never replace a saved capability, even if the keeper has disappeared.
        return Ok(());
    }
    let Some(keeper) = keeper else {
        return Ok(());
    };
    if !first_attempt {
        bail!("started execution has no retained snapshot; reconciliation required");
    }
    let run_id: String = transaction.query_row(
        "SELECT execution_run_id FROM leaves WHERE id = ?1",
        [leaf_id],
        |row| row.get(0),
    )?;
    let snapshot = keeper.prepare(&run_id, workspace, identity)?;
    if snapshot.endpoint.is_empty()
        || snapshot.capability.is_empty()
        || snapshot.manifest_digest.len() != 64
        || !snapshot
            .manifest_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        bail!("invalid prepared snapshot capability");
    }
    transaction.execute(
        "INSERT INTO retained_workspace_snapshots
         (leaf_id, run_id, capability_json, committed_generation)
         VALUES (?1, ?2, ?3, 0)",
        params![leaf_id, run_id, serde_json::to_string(&snapshot)?],
    )?;
    Ok(())
}

impl super::store::ObjectiveStore {
    /// Historical routing only: this never grants a live execution lease.
    pub fn has_retained_snapshot(&self, run_id: &str) -> Result<bool> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM retained_workspace_snapshots WHERE run_id = ?1)",
            [run_id],
            |row| row.get(0),
        )?)
    }

    /// Resolve only an acknowledged, current, live lease. This is read-only and
    /// does not require an admission client on the Harness's reopened store.
    /// None means the legacy/nonresident path, never a missing required snapshot.
    /// The lookup is internally fenced, but the returned value is not a permit
    /// for later effects; those must recheck authority under their writer fence.
    pub fn retained_execution(
        &self,
        run_id: &str,
        now_ms: i64,
    ) -> Result<Option<RetainedExecution>> {
        self.with_unabandoned_run(run_id, |connection| {
        let saved = connection.query_row(
            "SELECT s.capability_json, l.attempt, i.lease_owner, i.lease_expires_ms,
                    COALESCE(s.run_id = l.execution_run_id
                        AND s.committed_generation = l.attempt
                        AND i.lease_owner = l.lease_owner
                        AND i.lease_expires_ms = l.lease_expires_ms
                        AND l.lease_expires_ms > ?2
                        AND l.stage NOT IN ('complete', 'cancelled', 'failed')
                        AND o.state IN ('approved', 'running')
                        AND NOT EXISTS (SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id = l.id), 0),
                    EXISTS(SELECT 1 FROM retained_snapshot_policy)
             FROM leaves l JOIN objectives o ON o.id = l.objective_id
             LEFT JOIN retained_workspace_snapshots s ON s.leaf_id = l.id
             LEFT JOIN retained_snapshot_lease_intents i ON i.leaf_id = l.id AND i.generation = l.attempt
             WHERE l.execution_run_id = ?1",
            params![run_id, now_ms],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?,
                row.get::<_, Option<String>>(2)?, row.get::<_, Option<i64>>(3)?,
                row.get::<_, bool>(4)?, row.get::<_, bool>(5)?)),
        ).optional()?;
        let Some((encoded, generation, owner, expires, valid, required)) = saved else {
            return Ok(None);
        };
        let Some(encoded) = encoded else {
            if required {
                bail!("resident retained snapshot missing; reconciliation required");
            }
            return Ok(None);
        };
        if !valid {
            bail!("resident retained execution is unacknowledged, fenced, expired or terminal");
        }
        Ok(Some(RetainedExecution {
            snapshot: serde_json::from_str(&encoded).context("decode retained execution")?,
            lease: super::snapshot_protocol::Lease {
                run_id: run_id.to_owned(),
                generation,
                owner: owner.context("retained lease owner missing")?,
                expires_ms: expires.context("retained lease expiry missing")?,
            },
        }))
        })
    }

    /// Reconcile SQLite-committed admission intents after an acknowledgement loss
    /// or daemon crash. This never prepares a snapshot or dispatches a provider.
    /// The writer lock prevents an older acknowledgement overwriting a new claim.
    pub fn reconcile_snapshot_commits(&self) -> Result<()> {
        self.reconcile_snapshot_commits_scoped(None)
    }

    /// Reconcile only an existing retained leaf. This does not claim work,
    /// authorize recovery, or change the objective's paused state.
    pub fn reconcile_snapshot_commits_for_leaf(&self, leaf_id: &str) -> Result<()> {
        if leaf_id.trim().is_empty() {
            anyhow::bail!("retained reconciliation requires an exact leaf");
        }
        self.reconcile_snapshot_commits_scoped(Some(leaf_id))
    }

    fn reconcile_snapshot_commits_scoped(&self, leaf_id: Option<&str>) -> Result<()> {
        let mut connection = self.connection()?;
        let transaction =
            connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        check_policy(&transaction, self.snapshot_admission.is_some())?;
        if let Some(leaf_id) = leaf_id {
            let exists: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM retained_workspace_snapshots WHERE leaf_id = ?1)",
                [leaf_id],
                |row| row.get(0),
            )?;
            if !exists {
                anyhow::bail!("retained reconciliation leaf is missing");
            }
        }
        let rows = {
            let mut statement = transaction.prepare(
                "SELECT s.leaf_id, s.run_id, s.capability_json, l.attempt,
                        i.lease_owner, i.lease_expires_ms,
                        (l.stage IN ('complete', 'cancelled', 'failed')
                         OR o.state IN ('completed', 'cancelled', 'failed')) AS terminal
                 FROM retained_workspace_snapshots s JOIN leaves l ON l.id = s.leaf_id
                 JOIN objectives o ON o.id = l.objective_id
                 LEFT JOIN retained_snapshot_lease_intents i ON i.leaf_id = l.id AND i.generation = l.attempt
                 WHERE NOT EXISTS (SELECT 1 FROM retained_snapshot_releases r WHERE r.leaf_id = s.leaf_id)
                   AND (s.committed_generation < l.attempt OR terminal)
                   AND (?1 IS NULL OR s.leaf_id = ?1)
                 ORDER BY s.leaf_id",
            )?;
            let rows = statement.query_map([leaf_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, Option<i64>>(5)?,
                    row.get::<_, bool>(6)?,
                ))
            })?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
        };
        // Validate every persisted retirement and current fingerprint before
        // excluding any member. Exact-leaf requests remain permanently denied.
        let retired = super::abandonment::verified_reservations(&transaction)?;
        let rows: Vec<_> = rows
            .into_iter()
            .filter(|(leaf, run, ..)| !retired.contains(&(leaf.clone(), run.clone())))
            .collect();
        // Preflight the entire batch before any irreversible keeper call. A
        // denial on a later row cannot undo earlier external acknowledgements.
        super::abandonment::guards::reject_in(&transaction, None, leaf_id, None)?;
        for (leaf, run, ..) in &rows {
            super::abandonment::guards::reject_in(&transaction, None, Some(leaf), Some(run))?;
        }
        for (leaf, run, encoded, generation, owner, expires, terminal) in rows {
            let keeper = self
                .snapshot_admission
                .as_deref()
                .context("retained snapshot keeper is not configured; reconciliation required")?;
            let snapshot =
                serde_json::from_str(&encoded).context("decode durable retained snapshot")?;
            if terminal {
                // Terminal authority supersedes an unacknowledged admission.
                // Do not reconstruct or recommit the cleared mutable leaf lease.
                if let Err(error) = keeper.release(&snapshot, &run) {
                    if error.downcast_ref::<super::keeper_client::KeeperFailure>()
                        != Some(&super::keeper_client::KeeperFailure::MissingRevokedAuthority)
                    {
                        return Err(error);
                    }
                    super::terminal_revocation::retain(&transaction, keeper, &leaf, &run)?;
                }
                transaction.execute(
                    "INSERT INTO retained_snapshot_releases VALUES (?1)",
                    [&leaf],
                )?;
                continue;
            }
            keeper.commit(
                &snapshot,
                &run,
                generation,
                owner
                    .as_deref()
                    .context("snapshot admission lease owner missing")?,
                expires.context("snapshot admission lease expiry missing")?,
            )?;
            transaction.execute(
                "UPDATE retained_workspace_snapshots SET committed_generation = ?1 WHERE leaf_id = ?2",
                params![generation, leaf],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}
