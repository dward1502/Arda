use anyhow::{Context, Result};
use rusqlite::Connection;

pub(crate) fn apply(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS objectives (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL UNIQUE,
                ingress_key TEXT NOT NULL UNIQUE,
                payload_digest TEXT NOT NULL,
                operator_id TEXT NOT NULL,
                text TEXT NOT NULL,
                priority INTEGER NOT NULL,
                revision INTEGER NOT NULL,
                approved_revision INTEGER,
                state TEXT NOT NULL,
                terminal_receipt_digest TEXT,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS objective_projects (
                objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE CASCADE,
                ordinal INTEGER NOT NULL,
                project_id TEXT NOT NULL,
                contract_digest TEXT NOT NULL,
                PRIMARY KEY (objective_id, project_id),
                UNIQUE (objective_id, ordinal)
            );

            CREATE TABLE IF NOT EXISTS leaves (
                id TEXT PRIMARY KEY,
                objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE CASCADE,
                project_id TEXT,
                workspace_root TEXT NOT NULL,
                authority TEXT NOT NULL,
                execution_json TEXT,
                stage TEXT NOT NULL,
                attempt INTEGER NOT NULL DEFAULT 0,
                lease_owner TEXT,
                lease_expires_ms INTEGER,
                current_receipt_digest TEXT,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS leaf_dependencies (
                leaf_id TEXT NOT NULL REFERENCES leaves(id) ON DELETE CASCADE,
                dependency_leaf_id TEXT NOT NULL REFERENCES leaves(id) ON DELETE CASCADE,
                PRIMARY KEY (leaf_id, dependency_leaf_id)
            );

            CREATE TABLE IF NOT EXISTS controls (
                idempotency_key TEXT PRIMARY KEY,
                objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE CASCADE,
                operator_id TEXT NOT NULL,
                action_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schedules (
                id TEXT PRIMARY KEY,
                objective_id TEXT NOT NULL REFERENCES objectives(id) ON DELETE CASCADE,
                next_wake_ms INTEGER NOT NULL,
                recurrence TEXT,
                idempotency_key TEXT NOT NULL UNIQUE,
                payload_digest TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS stage_receipts (
                leaf_id TEXT NOT NULL REFERENCES leaves(id) ON DELETE CASCADE,
                stage TEXT NOT NULL,
                contract TEXT NOT NULL,
                digest TEXT NOT NULL UNIQUE,
                predecessor_digest TEXT,
                run_path TEXT NOT NULL,
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                started_at_ms INTEGER NOT NULL,
                completed_at_ms INTEGER NOT NULL,
                verdict TEXT NOT NULL,
                context_outcome_receipt_id TEXT,
                context_outcome_receipt_digest TEXT,
                binding_digest TEXT,
                recorded_at_ms INTEGER NOT NULL,
                PRIMARY KEY (leaf_id, stage)
            );

            CREATE INDEX IF NOT EXISTS objectives_state_priority_idx
                ON objectives(state, priority DESC, created_at_ms, id);
            CREATE INDEX IF NOT EXISTS leaves_claim_idx
                ON leaves(stage, lease_expires_ms, workspace_root, objective_id);
            CREATE INDEX IF NOT EXISTS leaf_dependencies_dependency_idx
                ON leaf_dependencies(dependency_leaf_id, leaf_id);
            CREATE INDEX IF NOT EXISTS schedules_due_idx
                ON schedules(next_wake_ms, id);
            "#,
        )
        .context("apply ObjectiveStore schema")?;
    let has_execution_json = {
        let mut statement = connection.prepare("PRAGMA table_info(leaves)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        columns
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "execution_json")
    };
    if !has_execution_json {
        connection
            .execute("ALTER TABLE leaves ADD COLUMN execution_json TEXT", [])
            .context("add leaf execution payload column")?;
    }
    for (column, sql) in [
        (
            "context_outcome_receipt_id",
            "ALTER TABLE stage_receipts ADD COLUMN context_outcome_receipt_id TEXT",
        ),
        (
            "context_outcome_receipt_digest",
            "ALTER TABLE stage_receipts ADD COLUMN context_outcome_receipt_digest TEXT",
        ),
        (
            "binding_digest",
            "ALTER TABLE stage_receipts ADD COLUMN binding_digest TEXT",
        ),
    ] {
        let exists = {
            let mut statement = connection.prepare("PRAGMA table_info(stage_receipts)")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns
                .collect::<rusqlite::Result<Vec<_>>>()?
                .iter()
                .any(|candidate| candidate == column)
        };
        if !exists {
            connection.execute(sql, [])?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partially_migrated_stage_receipts_gain_binding_digest() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE stage_receipts (
                    context_outcome_receipt_id TEXT,
                    context_outcome_receipt_digest TEXT
                );",
            )
            .unwrap();

        apply(&connection).unwrap();

        let mut statement = connection
            .prepare("PRAGMA table_info(stage_receipts)")
            .unwrap();
        let columns = statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        assert!(columns.iter().any(|column| column == "binding_digest"));
    }
}
