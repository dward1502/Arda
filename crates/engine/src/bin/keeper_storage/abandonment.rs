//! Deny-only tombstones. Existence is sufficient to refuse execution, never to
//! release an Engine reservation. Only the offline authenticated maintenance
//! transaction may insert production records; schema creation grants nothing.
use super::*;

pub(super) fn migrate(db: &Connection) -> Result<()> {
    db.execute_batch(
        "BEGIN IMMEDIATE;
         CREATE TABLE IF NOT EXISTS snapshot_operator_abandonments(
             run TEXT PRIMARY KEY NOT NULL CHECK(length(trim(run)) > 0),
             receipt_json TEXT NOT NULL
         );
         CREATE TRIGGER IF NOT EXISTS snapshot_abandonment_no_update
         BEFORE UPDATE ON snapshot_operator_abandonments
         BEGIN SELECT RAISE(ABORT, 'operator abandonment is immutable'); END;
         CREATE TRIGGER IF NOT EXISTS snapshot_abandonment_no_delete
         BEFORE DELETE ON snapshot_operator_abandonments
         BEGIN SELECT RAISE(ABORT, 'operator abandonment is immutable'); END;
         CREATE TRIGGER IF NOT EXISTS snapshot_abandonment_no_replace
         BEFORE INSERT ON snapshot_operator_abandonments
         WHEN EXISTS(SELECT 1 FROM snapshot_operator_abandonments WHERE run=NEW.run)
         BEGIN SELECT RAISE(ABORT, 'operator abandonment already exists'); END;
         COMMIT;",
    )?;
    Ok(())
}
