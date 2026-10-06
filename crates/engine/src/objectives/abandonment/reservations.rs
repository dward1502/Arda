//! Transaction-local complete-set integrity for reservation consumers.
use super::*;
use std::collections::BTreeSet;

pub(super) fn verified_in(tx: &rusqlite::Transaction<'_>) -> Result<BTreeSet<(String, String)>> {
    // Denial validation checks canonical leaf/objective/retained-run associations
    // and relational/JSON agreement, even for otherwise unrelated candidates.
    guards::reject_in(tx, None, None, None)?;
    let events = {
        let mut statement = tx.prepare("SELECT DISTINCT event_id FROM retained_snapshot_operator_abandonments ORDER BY event_id")?;
        let rows = statement.query_map([], |row| row.get::<_, String>(0))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut verified = BTreeSet::new();
    for event in events {
        let raw: String = tx.query_row(
            "SELECT record_json FROM operator_abandonment_authorizations WHERE event_id=?1",
            [&event],
            |r| r.get(0),
        )?;
        let auth: AbandonmentAuthorization = serde_json::from_str(&raw)?;
        anyhow::ensure!(
            auth.event_id == event,
            "abandonment authorization event mismatch"
        );
        let receipts = {
            let mut statement = tx.prepare("SELECT record_json FROM retained_snapshot_operator_abandonments WHERE event_id=?1 ORDER BY run_id")?;
            let rows = statement.query_map([&event], |r| r.get::<_, String>(0))?;
            let mut receipts = Vec::new();
            for row in rows {
                let record: serde_json::Value = serde_json::from_str(&row?)?;
                receipts.push(record["keeper_receipt"].clone());
            }
            receipts
        };
        // Reuse the complete immutable replay contract without issuing writes.
        // Expiry does not erase a valid past commit, nor permit a new one here.
        let (_, existing) = disposition::checked_records(tx, &auth, &receipts, 0)?;
        anyhow::ensure!(existing, "abandonment disposition set absent");
        for target in &auth.manifest.targets {
            let current = engine_record(tx, target)?;
            anyhow::ensure!(
                record_digest(&current)? == target.engine_record_digest,
                "abandonment current reservation fingerprint changed"
            );
            anyhow::ensure!(
                verified.insert((target.leaf_id.clone(), target.run_id.clone())),
                "abandonment duplicate reservation membership"
            );
        }
    }
    Ok(verified)
}
