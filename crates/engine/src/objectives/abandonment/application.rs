//! Concrete offline application. No public receipt or cessation callback API.
use super::*;
use serde_json::Value;
use std::path::Path;

#[cfg(test)]
#[path = "public_os_tests.rs"]
mod public_os_tests;
#[cfg(test)]
#[path = "application_tests.rs"]
mod tests;

// Share the existing historical serializer and pinned offline/cessation code.
// Their CLI/startup entry points are never called by this private module.
#[allow(dead_code)]
#[path = "../../bin/keeper_abandonment/mod.rs"]
mod keeper_abandonment;
#[allow(dead_code)]
#[path = "../../bin/keeper_managed/mod.rs"]
mod keeper_managed;
#[allow(dead_code)]
#[path = "../../bin/keeper_reconcile/mod.rs"]
mod keeper_reconcile;
#[allow(dead_code)]
#[path = "../../bin/keeper_storage/mod.rs"]
mod keeper_storage;

impl super::super::store::ObjectiveMaintenance {
    /// Concrete keeper-first application. Supplied files must exactly match
    /// durable authenticated intent; they cannot create or expand authority.
    /// Never supplies cleanup acknowledgements or changes historical attempts.
    pub fn apply_abandonment(
        &self,
        event: &str,
        operator: &str,
        durable: &Path,
        runtime: &Path,
        owner: &str,
        provenance: &Path,
        baseline: &Path,
    ) -> Result<Vec<Value>> {
        keeper_abandonment::engine_stopped()?;
        let baseline: AbandonmentManifest = serde_json::from_slice(&std::fs::read(baseline)?)?;
        baseline.validate()?;
        let provenance = std::fs::read(provenance)?;
        anyhow::ensure!(
            keeper_managed::digest(&provenance) == baseline.mutation_provenance_digest,
            "abandonment provenance does not match baseline"
        );
        // self owns exclusive Engine lifetime exclusion before keeper exclusion.
        let mut offline = keeper_storage::offline::open(durable, runtime, owner, true)?;
        let mut db = self.maintenance_connection()?;
        let mut tx = db.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let was_existing: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM retained_snapshot_operator_abandonments WHERE event_id=?1)", [event], |r| r.get(0))?;
        let records = apply_in(
            &mut tx,
            event,
            operator,
            &baseline,
            || chrono::Utc::now().timestamp_millis(),
            |auth, first| {
                keeper_evidence(
                    &mut offline.db,
                    auth,
                    owner,
                    runtime,
                    &provenance,
                    first,
                    |binding| {
                        keeper_abandonment::engine_stopped()?;
                        keeper_managed::stopped(binding)
                    },
                )
            },
        )?;
        // Recheck concrete service and worker cessation immediately before commit.
        keeper_abandonment::engine_stopped()?;
        for target in &baseline.targets {
            let (_, binding, digest) =
                keeper_reconcile::abandonment_record(&offline.db, owner, &target.run_id, runtime)?;
            anyhow::ensure!(
                digest == target.keeper_record_digest,
                "keeper evidence changed before Engine commit"
            );
            keeper_managed::stopped(&binding)?;
        }
        // A first commit must still be fresh after all OS queries. Historical
        // replay uses the saved time, not renewed authority.
        if !was_existing {
            let auth: AbandonmentAuthorization =
                serde_json::from_value(records[0]["authorization"].clone())?;
            let now = chrono::Utc::now().timestamp_millis();
            anyhow::ensure!(
                now >= auth.authorized_at_ms && now < auth.expires_at_ms,
                "abandonment authority expired before Engine commit; keeper remains deny-only"
            );
        }
        tx.commit()?;
        for record in &records {
            let leaf = record["target"]["leaf_id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("disposition leaf absent"))?;
            let raw: String = db.query_row(
                "SELECT record_json FROM retained_snapshot_operator_abandonments WHERE leaf_id=?1",
                [leaf],
                |r| r.get(0),
            )?;
            anyhow::ensure!(
                serde_json::from_str::<Value>(&raw)? == *record,
                "Engine disposition readback mismatch"
            );
        }
        Ok(records)
    }
}

// Private backend; test-only callers may inject cessation failures, but the
// public operation always supplies the concrete managed/service checks.
fn keeper_evidence(
    db: &mut rusqlite::Connection,
    auth: &AbandonmentAuthorization,
    owner: &str,
    runtime: &Path,
    provenance: &[u8],
    first: bool,
    mut stopped: impl FnMut(&keeper_managed::Binding) -> Result<()>,
) -> Result<Vec<Value>> {
    if first {
        keeper_abandonment::apply(db, auth, owner, runtime, provenance, &mut stopped)?;
    }
    let mut saved = Vec::new();
    for target in &auth.manifest.targets {
        anyhow::ensure!(
            target.keeper_owner == owner,
            "abandonment keeper owner mismatch"
        );
        let raw: String = db.query_row(
            "SELECT receipt_json FROM snapshot_operator_abandonments WHERE run=?1",
            [&target.run_id],
            |r| r.get(0),
        )?;
        let receipt: Value = serde_json::from_str(&raw)?;
        let (current, binding, digest) =
            keeper_reconcile::abandonment_record(db, owner, &target.run_id, runtime)?;
        anyhow::ensure!(
            digest == target.keeper_record_digest && receipt["keeper_record"] == current,
            "abandonment durable keeper evidence changed"
        );
        if first {
            stopped(&binding)?;
        }
        saved.push(receipt);
    }
    receipts::validate_keeper_receipt_set(auth, &saved)?;
    Ok(saved)
}

// Private orchestration seam for deterministic failure tests. Only the concrete
// maintenance entry point may supply the real keeper reader/application.
pub(super) fn apply_in(
    tx: &mut rusqlite::Transaction<'_>,
    event: &str,
    operator: &str,
    baseline: &AbandonmentManifest,
    mut clock: impl FnMut() -> i64,
    mut keeper: impl FnMut(&AbandonmentAuthorization, bool) -> Result<Vec<Value>>,
) -> Result<Vec<Value>> {
    baseline.validate()?;
    let raw: String = tx.query_row(
        "SELECT record_json FROM operator_abandonment_authorizations WHERE event_id=?1",
        [event],
        |r| r.get(0),
    )?;
    let auth: AbandonmentAuthorization = serde_json::from_str(&raw)?;
    let payload: String = tx.query_row(
        "SELECT payload_digest FROM gateway_event_bindings WHERE event_id=?1",
        [event],
        |r| r.get(0),
    )?;
    anyhow::ensure!(
        auth.event_id == event
            && auth.operator_id == operator
            && auth.payload_digest == payload
            && auth.manifest == *baseline,
        "abandonment durable authority or exact baseline mismatch"
    );
    let existing: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM retained_snapshot_operator_abandonments WHERE event_id=?1)",
        [event],
        |r| r.get(0),
    )?;
    guards::reject_in(tx, None, None, None)?;
    for target in &auth.manifest.targets {
        let conflict: bool = tx.query_row(
            "SELECT EXISTS(SELECT 1 FROM retained_snapshot_operator_abandonments WHERE (leaf_id=?1 OR run_id=?2) AND event_id!=?3)",
            rusqlite::params![target.leaf_id, target.run_id, event], |r| r.get(0),
        )?;
        anyhow::ensure!(
            !conflict,
            "Engine disposition target belongs to another event"
        );
    }
    if existing {
        let mut statement = tx.prepare(
            "SELECT record_json FROM retained_snapshot_operator_abandonments WHERE event_id=?1",
        )?;
        let jsons = statement
            .query_map([event], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let saved_receipts = jsons
            .iter()
            .map(|json| Ok(serde_json::from_str::<Value>(json)?["keeper_receipt"].clone()))
            .collect::<Result<Vec<_>>>()?;
        let (_, saved) = disposition::checked_records(tx, &auth, &saved_receipts, 0)?;
        anyhow::ensure!(saved, "Engine disposition replay set absent");
    }
    let check_first = |now| -> Result<()> {
        anyhow::ensure!(
            now >= auth.authorized_at_ms && now < auth.expires_at_ms,
            "abandonment first application authority expired or not yet valid"
        );
        for target in &auth.manifest.targets {
            let current = engine_record(tx, target)?;
            anyhow::ensure!(
                current[8].as_str() == Some(operator)
                    && record_digest(&current)? == target.engine_record_digest,
                "abandonment Engine reservation changed"
            );
        }
        Ok(())
    };
    if !existing {
        check_first(clock())?;
    }
    // Keeper commits first; a subsequent failure intentionally leaves deny-only
    // tombstones and held Engine reservations, never fabricated release evidence.
    let receipts = keeper(&auth, !existing)?;
    if !existing {
        check_first(clock())?;
    }
    if existing {
        let (records, saved) = disposition::checked_records(tx, &auth, &receipts, 0)?;
        anyhow::ensure!(saved, "abandonment replay set absent");
        return Ok(records);
    }
    disposition::persist(tx, &auth, &receipts, clock())
}
