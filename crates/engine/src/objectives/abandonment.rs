//! Operator abandonment is new authority over explicitly named damaged records,
//! not reconstruction of original paired admission or a Release acknowledgement.
mod application;
mod disposition;
mod reservations;

// Transaction-local reservation exclusion only; never execution/release authority.
pub(super) fn verified_reservations(
    tx: &rusqlite::Transaction<'_>,
) -> Result<std::collections::BTreeSet<(String, String)>> {
    reservations::verified_in(tx)
        .map_err(|error| error.context("abandonment reservation verification failed"))
}
pub(crate) mod guards;
pub mod receipts;
use super::ObjectiveStore;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbandonmentTarget {
    pub objective_id: String,
    pub leaf_id: String,
    pub run_id: String,
    pub keeper_owner: String,
    pub engine_record_digest: String,
    pub keeper_record_digest: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbandonmentManifest {
    pub version: u32,
    pub reason: String,
    pub mutation_provenance_digest: String,
    pub original_paired_lineage_unrecoverable: bool,
    pub relinquish_paused_resumability: bool,
    pub artifacts_retained: bool,
    pub worker_cleanup_ack: bool,
    pub cleanup_verified: bool,
    pub targets: Vec<AbandonmentTarget>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbandonmentAuthorization {
    pub event_id: String,
    pub payload_digest: String,
    pub operator_id: String,
    pub authorized_at_ms: i64,
    pub expires_at_ms: i64,
    pub manifest: AbandonmentManifest,
}
impl super::store::ObjectiveMaintenance {
    /// Execute deny-only offline maintenance while the Engine writer is fenced.
    /// This does NOT authorize reservation retirement or certify cessation.
    pub fn with_fenced_abandonment_authorization<T>(
        &self,
        event: &str,
        operator: &str,
        now: i64,
        apply: impl FnOnce(&AbandonmentAuthorization) -> Result<T>,
    ) -> Result<T> {
        let mut db = self.maintenance_connection()?;
        let tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let raw: String = tx.query_row(
            "SELECT record_json FROM operator_abandonment_authorizations WHERE event_id=?1",
            [event],
            |r| r.get(0),
        )?;
        let auth: AbandonmentAuthorization = serde_json::from_str(&raw)?;
        auth.manifest.validate()?;
        let bound: String = tx.query_row(
            "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
            [event],
            |r| r.get(0),
        )?;
        if auth.event_id != event
            || auth.operator_id != operator
            || auth.payload_digest != bound
            || auth.authorized_at_ms > now
            || now >= auth.expires_at_ms
        {
            bail!("abandonment application authority is mismatched or expired");
        }
        for target in &auth.manifest.targets {
            let record = engine_record(&tx, target)?;
            if record[8].as_str() != Some(operator)
                || record_digest(&record)? != target.engine_record_digest
            {
                bail!("abandonment Engine record changed");
            }
        }
        let result = apply(&auth)?;
        tx.commit()?;
        Ok(result)
    }
}
impl ObjectiveStore {
    pub(crate) fn has_abandonment_authorization(&self, event: &str) -> Result<bool> {
        Ok(self.connection()?.query_row(
            "SELECT EXISTS(SELECT 1 FROM operator_abandonment_authorizations WHERE event_id=?1)",
            [event],
            |r| r.get(0),
        )?)
    }

    /// Only an authenticated ingress handler may issue new authorization.
    /// The caller must first pass the configured capability/operator/freshness
    /// checks. An event ID or a bound payload alone is never authentication.
    pub(crate) fn authorize_abandonment(
        &self,
        event: &str,
        payload: &str,
        operator: &str,
        manifest: &AbandonmentManifest,
        now: i64,
    ) -> Result<AbandonmentAuthorization> {
        use rusqlite::{params, OptionalExtension, TransactionBehavior};
        manifest.validate()?;
        if event.trim().is_empty()
            || operator.trim().is_empty()
            || !digest_valid(payload)
            || now < 0
        {
            bail!("invalid abandonment event binding");
        }
        let mut db = self.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let bound: Option<String> = tx
            .query_row(
                "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
                [event],
                |r| r.get(0),
            )
            .optional()?;
        if bound.as_deref() != Some(payload) {
            bail!("abandonment event payload is not bound");
        }
        let previous: Option<String> = tx
            .query_row(
                "SELECT record_json FROM operator_abandonment_authorizations WHERE event_id=?1",
                [event],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(previous) = previous {
            let saved: AbandonmentAuthorization = serde_json::from_str(&previous)?;
            if saved.event_id != event
                || saved.payload_digest != payload
                || saved.operator_id != operator
                || saved.manifest != *manifest
            {
                bail!("abandonment authorization replay changed scope");
            }
            return Ok(saved);
        }
        for target in &manifest.targets {
            let record = engine_record(&tx, target)?;
            if record[8].as_str() != Some(operator) {
                bail!("abandonment objective operator mismatch");
            }
            // Intake records intent only. Expired lease metadata is preserved;
            // it is NOT cessation evidence. Offline application must separately
            // prove worker cessation under exclusion and revalidate this digest.
            if !matches!(record[7].as_str(), Some("paused" | "cancelled"))
                || record[6].as_i64().is_some_and(|expiry| expiry > now)
                || (!record[5].is_null() && record[6].is_null())
            {
                bail!("abandonment requires paused/cancelled objectives without a current lease");
            }
            if record_digest(&record)? != target.engine_record_digest {
                bail!("abandonment Engine record changed");
            }
        }
        let saved = AbandonmentAuthorization {
            event_id: event.into(),
            payload_digest: payload.into(),
            operator_id: operator.into(),
            authorized_at_ms: now,
            expires_at_ms: now
                .checked_add(24 * 60 * 60 * 1000)
                .ok_or_else(|| anyhow::anyhow!("authorization expiry overflow"))?,
            manifest: manifest.clone(),
        };
        tx.execute(
            "INSERT INTO operator_abandonment_authorizations(event_id,record_json) VALUES(?1,?2)",
            params![event, serde_json::to_string(&saved)?],
        )?;
        tx.commit()?;
        Ok(saved)
    }
}
fn digest_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl AbandonmentManifest {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.reason.trim().is_empty()
            || self.reason.len() > 4096
            || !digest_valid(&self.mutation_provenance_digest)
            || !self.original_paired_lineage_unrecoverable
            || !self.relinquish_paused_resumability
            || !self.artifacts_retained
            || self.worker_cleanup_ack
            || self.cleanup_verified
            || self.targets.len() != 5
        {
            bail!("invalid bounded abandonment preservation scope");
        }
        let mut runs = std::collections::BTreeSet::new();
        let mut leaves = std::collections::BTreeSet::new();
        for t in &self.targets {
            if t.objective_id.trim().is_empty()
                || t.leaf_id.trim().is_empty()
                || t.run_id.trim().is_empty()
                || t.keeper_owner.trim().is_empty()
                || !digest_valid(&t.engine_record_digest)
                || !digest_valid(&t.keeper_record_digest)
                || !runs.insert(&t.run_id)
                || !leaves.insert(&t.leaf_id)
            {
                bail!("invalid or duplicate abandonment target");
            }
        }
        Ok(())
    }
}
fn engine_record(
    db: &rusqlite::Connection,
    target: &AbandonmentTarget,
) -> Result<serde_json::Value> {
    use rusqlite::params;
    let raw: String=db.query_row("SELECT json_array(l.objective_id,l.id,l.execution_run_id,l.attempt,l.stage,l.lease_owner,l.lease_expires_ms,o.state,o.operator_id,s.run_id,s.capability_json,s.committed_generation,i.identity_json,l.authority) FROM leaves l JOIN objectives o ON o.id=l.objective_id JOIN retained_workspace_snapshots s ON s.leaf_id=l.id JOIN lease_workspace_identities i ON i.leaf_id=l.id WHERE l.id=?1 AND l.objective_id=?2 AND l.execution_run_id=?3 AND s.run_id=?3", params![target.leaf_id,target.objective_id,target.run_id], |r|r.get(0))?;
    let mut record: serde_json::Value = serde_json::from_str(&raw)?;
    let mut reservation = serde_json::Map::new();
    reservation.insert("fingerprint_version".into(), serde_json::json!(2));
    // Include complete named rows, including absence of releases/proofs. Added
    // schema columns deliberately change the fingerprint rather than silently
    // retaining authority over a different reservation representation.
    for (name, sql, key) in [
        ("leaf", "SELECT * FROM leaves WHERE id=?1 ORDER BY id", &target.leaf_id),
        ("objective", "SELECT * FROM objectives WHERE id=?1 ORDER BY id", &target.objective_id),
        ("snapshot", "SELECT * FROM retained_workspace_snapshots WHERE leaf_id=?1 ORDER BY leaf_id", &target.leaf_id),
        ("workspace_identity", "SELECT * FROM lease_workspace_identities WHERE leaf_id=?1 ORDER BY leaf_id", &target.leaf_id),
        ("lease_intents", "SELECT * FROM retained_snapshot_lease_intents WHERE leaf_id=?1 ORDER BY generation", &target.leaf_id),
        ("releases", "SELECT * FROM retained_snapshot_releases WHERE leaf_id=?1 ORDER BY leaf_id", &target.leaf_id),
        ("terminal_revocations", "SELECT * FROM retained_snapshot_terminal_revocations WHERE leaf_id=?1 ORDER BY leaf_id", &target.leaf_id),
    ] {
        reservation.insert(name.into(), named_rows(db, sql, key)?);
    }
    record
        .as_array_mut()
        .expect("json_array is array")
        .push(reservation.into());
    Ok(record)
}
fn named_rows(db: &rusqlite::Connection, sql: &str, key: &str) -> Result<serde_json::Value> {
    use rusqlite::types::ValueRef;
    let mut statement = db.prepare(sql)?;
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|s| (*s).into())
        .collect();
    let mut rows = statement.query([key])?;
    let mut result = Vec::new();
    while let Some(row) = rows.next()? {
        let mut fields = serde_json::Map::new();
        for (index, name) in names.iter().enumerate() {
            let value = match row.get_ref(index)? {
                ValueRef::Null => serde_json::Value::Null,
                ValueRef::Integer(value) => value.into(),
                ValueRef::Text(value) => std::str::from_utf8(value)?.into(),
                // These authority schemas contain text and integers. Fail closed
                // on corruption/unreviewed encodings, never normalize them.
                _ => bail!("unexpected abandonment fingerprint SQL type"),
            };
            fields.insert(name.clone(), value);
        }
        result.push(serde_json::Value::Object(fields));
    }
    Ok(result.into())
}
fn record_digest(record: &serde_json::Value) -> Result<String> {
    use sha2::{Digest, Sha256};
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(record)?)))
}
#[cfg(test)]
pub(crate) mod tests;
