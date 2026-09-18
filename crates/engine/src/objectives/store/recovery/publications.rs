//! Committed terminal effects. A receipt file is evidence, never this authority.
use super::*;
use crate::runs::RunStore;
use arda_core::run_graph::NodeId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
mod completion;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecoveryPublication {
    pub key: String,
    pub kind: String,
    pub node_id: NodeId,
    pub payload: serde_json::Value,
}

impl RecoveryPublication {
    fn validate(&self) -> Result<()> {
        anyhow::ensure!(!self.key.trim().is_empty(), "publication key is empty");
        anyhow::ensure!(
            matches!(
                self.kind.as_str(),
                "provider-finalization" | "close" | "completion"
            ),
            "unknown recovery publication kind"
        );
        Ok(())
    }
}

fn digest(value: &impl Serialize) -> Result<String> {
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(serde_json::to_vec(value)?)
    ))
}

impl ObjectiveStore {
    /// The caller holds the canonical journal guard through the surrounding
    /// SQLite commit. This inserts intent only: no receipt or memory writes.
    pub(crate) fn insert_recovery_publication_in(
        &self,
        tx: &Transaction<'_>,
        grant: &RecoveryGrant,
        lease: &super::super::super::snapshot_protocol::Lease,
        publication: &RecoveryPublication,
    ) -> Result<()> {
        publication.validate()?;
        let encoded = serde_json::to_string(&publication.payload)?;
        let payload_digest = digest(publication)?;
        let grant_digest = digest(grant)?;
        let existing: Option<(String, String, i64, String, i64)> = tx.query_row(
            "SELECT payload_digest, grant_digest, lease_generation, lease_owner, lease_expires_ms
             FROM recovery_publications WHERE authenticated_event_id=?1 AND publication_key=?2",
            params![grant.authenticated_event_id, publication.key],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional()?;
        if let Some(existing) = existing {
            anyhow::ensure!(
                existing
                    == (
                        payload_digest,
                        grant_digest,
                        lease.generation,
                        lease.owner.clone(),
                        lease.expires_ms
                    ),
                "conflicting recovery publication replay"
            );
            return Ok(());
        }
        tx.execute(
            "INSERT INTO recovery_publications(authenticated_event_id, publication_key, kind, node_id,
             payload_json, payload_digest, grant_digest, lease_generation, lease_owner, lease_expires_ms, authorized_at_ms)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![grant.authenticated_event_id, publication.key, publication.kind,
                publication.node_id.as_str(), encoded, payload_digest, grant_digest,
                lease.generation, lease.owner, lease.expires_ms, Utc::now().timestamp_millis()],
        )?;
        Ok(())
    }

    /// Apply only previously committed bytes, without extending execution time.
    /// Cancellation/supersession still wins over unapplied terminal success.
    pub(crate) fn reconcile_recovery_publications(
        &self,
        root: &Path,
        operator_id: &str,
        event_id: &str,
        apply: impl Fn(&RunStore, &RecoveryPublication) -> Result<()>,
    ) -> Result<()> {
        self.reconcile_recovery_publications_clock(root, operator_id, event_id, apply, || {
            Ok(Utc::now().timestamp_millis())
        })
    }

    pub(crate) fn reconcile_recovery_publications_clock(
        &self,
        root: &Path,
        operator_id: &str,
        event_id: &str,
        apply: impl Fn(&RunStore, &RecoveryPublication) -> Result<()>,
        mut clock: impl FnMut() -> Result<i64>,
    ) -> Result<()> {
        loop {
            let saved = read_admission(&self.connection()?, operator_id, event_id)?
                .context("recovery admission intent is missing")?;
            // No effect authority is needed for a terminal empty outbox. The
            // writer-fenced selector below still rechecks before any callback.
            let pending: bool = self.connection()?.query_row(
                "SELECT EXISTS(SELECT 1 FROM recovery_publications p
                 WHERE p.authenticated_event_id=?1 AND p.applied_at_ms IS NULL
                 AND NOT EXISTS(SELECT 1 FROM recovery_completion_suppressions s
                     WHERE s.authenticated_event_id=p.authenticated_event_id
                       AND s.publication_key=p.publication_key))",
                [event_id],
                |row| row.get(0),
            )?;
            if !pending {
                return Ok(());
            }
            let guards = self.with_recovery_control_fence_inner(operator_id, &saved, false, |tx| {
            let current = read_admission(tx, operator_id, event_id)?
                .context("recovery admission intent disappeared")?;
            anyhow::ensure!(current == saved, "recovery admission changed during reconciliation");
            check_gateway_binding(tx, &current)?;
            let grant = &current;
            let mut statement = tx.prepare(
                "SELECT publication_key,kind,node_id,payload_json,payload_digest,grant_digest,
                 lease_generation,lease_owner,lease_expires_ms FROM recovery_publications
                 WHERE authenticated_event_id=?1 AND applied_at_ms IS NULL
                 AND NOT EXISTS(SELECT 1 FROM recovery_completion_suppressions s
                     WHERE s.authenticated_event_id=recovery_publications.authenticated_event_id
                       AND s.publication_key=recovery_publications.publication_key)
                 ORDER BY rowid LIMIT 1",
            )?;
            let rows = statement.query_map([event_id], |row| Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                row.get::<_, String>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?, row.get::<_, String>(7)?, row.get::<_, i64>(8)?,
            )))?.collect::<rusqlite::Result<Vec<_>>>()?;
            drop(statement);
            let mut guards = Vec::new();
            for (key, kind, node, payload, payload_digest, grant_digest, generation, owner, expiry) in rows {
                let publication = RecoveryPublication {
                    key, kind, node_id: NodeId::new(node)?, payload: serde_json::from_str(&payload)?,
                };
                publication.validate()?;
                anyhow::ensure!(digest(&publication)? == payload_digest && digest(grant)? == grant_digest,
                    "committed recovery publication binding changed");
                let current: bool = tx.query_row(
                    "SELECT EXISTS(SELECT 1 FROM leaves l JOIN retained_workspace_snapshots s ON s.leaf_id=l.id
                     WHERE l.id=?1 AND l.execution_run_id=?2 AND l.attempt=?3 AND s.committed_generation=?3
                       AND l.lease_owner=?4 AND l.lease_expires_ms=?5)",
                    params![grant.bindings.leaf_id, grant.bindings.run_id.as_str(), generation, owner, expiry],
                    |row| row.get(0),
                )?;
                anyhow::ensure!(current, "committed recovery publication lease superseded");
                let guarded = RunStore::open(root, grant.bindings.run_id.clone())?
                    .lock_recovery_reconciliation(grant, &publication.node_id)?;
                apply(&guarded, &publication)?;
                tx.execute("UPDATE recovery_publications SET applied_at_ms=?3
                    WHERE authenticated_event_id=?1 AND publication_key=?2 AND applied_at_ms IS NULL
                    AND NOT EXISTS(SELECT 1 FROM recovery_completion_suppressions s
                        WHERE s.authenticated_event_id=recovery_publications.authenticated_event_id
                          AND s.publication_key=recovery_publications.publication_key)",
                    params![event_id, publication.key, Utc::now().timestamp_millis()])?;
                // Keep the journal fence through acknowledgement commit.
                guards.push(guarded);
            }
            Ok(guards)
        }, &mut clock)?;
            if guards.is_empty() {
                return Ok(());
            }
            drop(guards);
        }
    }
}
