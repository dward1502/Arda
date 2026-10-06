use anyhow::{Context, Result};
use rusqlite::Connection;

pub(crate) fn apply(connection: &Connection) -> Result<()> {
    // Foreign-key enforcement must be enabled outside the transaction. Hold the
    // writer lock across all schema inspection and DDL, including legacy checks.
    connection.pragma_update(None, "foreign_keys", true)?;
    let transaction =
        rusqlite::Transaction::new_unchecked(connection, rusqlite::TransactionBehavior::Immediate)
            .context("begin objective schema migration")?;
    apply_locked(&transaction)?;
    transaction
        .commit()
        .context("commit objective schema migration")
}

fn apply_locked(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS gateway_event_bindings (
                event_id TEXT PRIMARY KEY,
                payload_digest TEXT NOT NULL
            );

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

            CREATE TABLE IF NOT EXISTS objective_admissions (
                objective_id TEXT PRIMARY KEY REFERENCES objectives(id) ON DELETE CASCADE,
                input_json TEXT NOT NULL
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

            CREATE TABLE IF NOT EXISTS lease_workspace_identities (
                leaf_id TEXT PRIMARY KEY REFERENCES leaves(id) ON DELETE CASCADE,
                identity_json TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS retained_workspace_snapshots (
                leaf_id TEXT PRIMARY KEY REFERENCES leaves(id),
                run_id TEXT NOT NULL UNIQUE,
                capability_json TEXT NOT NULL,
                committed_generation INTEGER NOT NULL CHECK (committed_generation >= 0)
            );

            CREATE TABLE IF NOT EXISTS retained_snapshot_policy (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1)
            );

            CREATE TABLE IF NOT EXISTS retained_snapshot_lease_intents (
                leaf_id TEXT NOT NULL REFERENCES retained_workspace_snapshots(leaf_id),
                generation INTEGER NOT NULL CHECK (generation > 0),
                lease_owner TEXT NOT NULL,
                lease_expires_ms INTEGER NOT NULL,
                PRIMARY KEY (leaf_id, generation)
            );

            CREATE TABLE IF NOT EXISTS retained_snapshot_releases (
                leaf_id TEXT PRIMARY KEY REFERENCES retained_workspace_snapshots(leaf_id)
            );

            CREATE TABLE IF NOT EXISTS operator_abandonment_authorizations (
            event_id TEXT PRIMARY KEY NOT NULL,
            record_json TEXT NOT NULL
        );
        CREATE TRIGGER IF NOT EXISTS abandonment_auth_no_update BEFORE UPDATE ON operator_abandonment_authorizations BEGIN SELECT RAISE(ABORT,'immutable abandonment authorization'); END;
        CREATE TRIGGER IF NOT EXISTS abandonment_auth_no_delete BEFORE DELETE ON operator_abandonment_authorizations BEGIN SELECT RAISE(ABORT,'immutable abandonment authorization'); END;
        CREATE TRIGGER IF NOT EXISTS abandonment_auth_no_replace BEFORE INSERT ON operator_abandonment_authorizations WHEN EXISTS(SELECT 1 FROM operator_abandonment_authorizations WHERE event_id=NEW.event_id) BEGIN SELECT RAISE(ABORT,'immutable abandonment authorization'); END;
        CREATE TABLE IF NOT EXISTS retained_snapshot_operator_abandonments (
            leaf_id TEXT PRIMARY KEY NOT NULL REFERENCES retained_workspace_snapshots(leaf_id),
            run_id TEXT NOT NULL UNIQUE,
            objective_id TEXT NOT NULL REFERENCES objectives(id),
            event_id TEXT NOT NULL REFERENCES operator_abandonment_authorizations(event_id),
            record_json TEXT NOT NULL
        );
        CREATE TRIGGER IF NOT EXISTS engine_abandonment_no_update BEFORE UPDATE ON retained_snapshot_operator_abandonments BEGIN SELECT RAISE(ABORT,'immutable Engine abandonment'); END;
        CREATE TRIGGER IF NOT EXISTS engine_abandonment_no_delete BEFORE DELETE ON retained_snapshot_operator_abandonments BEGIN SELECT RAISE(ABORT,'immutable Engine abandonment'); END;
        CREATE TRIGGER IF NOT EXISTS engine_abandonment_no_replace BEFORE INSERT ON retained_snapshot_operator_abandonments WHEN EXISTS(SELECT 1 FROM retained_snapshot_operator_abandonments WHERE leaf_id=NEW.leaf_id OR run_id=NEW.run_id) BEGIN SELECT RAISE(ABORT,'immutable Engine abandonment'); END;
        CREATE TABLE IF NOT EXISTS retained_snapshot_terminal_revocations (
                leaf_id TEXT PRIMARY KEY REFERENCES retained_workspace_snapshots(leaf_id),
                objective_id TEXT NOT NULL REFERENCES objectives(id),
                run_id TEXT NOT NULL,
                workspace TEXT NOT NULL,
                identity_json TEXT NOT NULL,
                disposition TEXT NOT NULL CHECK(disposition = 'terminal_revocation'),
                proof_json TEXT NOT NULL
            );
            CREATE TRIGGER IF NOT EXISTS immutable_terminal_revocations_update
            BEFORE UPDATE ON retained_snapshot_terminal_revocations
            BEGIN SELECT RAISE(ABORT, 'terminal revocation proof is immutable'); END;
            CREATE TRIGGER IF NOT EXISTS immutable_terminal_revocations_delete
            BEFORE DELETE ON retained_snapshot_terminal_revocations
            BEGIN SELECT RAISE(ABORT, 'terminal revocation proof is immutable'); END;

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

            CREATE TABLE IF NOT EXISTS schedule_wakes (
                schedule_id TEXT PRIMARY KEY REFERENCES schedules(id) ON DELETE CASCADE,
                last_wake_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS schedule_errors (
                schedule_id TEXT PRIMARY KEY REFERENCES schedules(id) ON DELETE CASCADE,
                error TEXT NOT NULL,
                observed_at_ms INTEGER NOT NULL
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
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS resident_context_bindings (
        run_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL, assembly_json TEXT NOT NULL,
        deleted_by_operator_ms INTEGER
    );",
    )?;
    let has_operator_marker = {
        let mut statement = connection.prepare("PRAGMA table_info(resident_context_bindings)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        columns
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "deleted_by_operator_ms")
    };
    if !has_operator_marker {
        connection.execute(
            "ALTER TABLE resident_context_bindings ADD COLUMN deleted_by_operator_ms INTEGER",
            [],
        )?;
    }
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
    let has_run_id = {
        let mut statement = connection.prepare("PRAGMA table_info(leaves)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        columns
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "execution_run_id")
    };
    if !has_run_id {
        connection.execute("ALTER TABLE leaves ADD COLUMN execution_run_id TEXT", [])?;
    }
    let has_context_bound = {
        let mut statement = connection.prepare("PRAGMA table_info(leaves)")?;
        let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
        columns
            .collect::<rusqlite::Result<Vec<_>>>()?
            .iter()
            .any(|column| column == "context_bound")
    };
    if !has_context_bound {
        // NULL means historical/unknown, never proof of no previous dispatch.
        connection.execute(
            "ALTER TABLE leaves ADD COLUMN context_bound INTEGER CHECK (context_bound IN (0, 1))",
            [],
        )?;
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
    for (table, sql) in [
        ("objectives", "ALTER TABLE objectives ADD COLUMN stop_generation INTEGER NOT NULL DEFAULT 0 CHECK(typeof(stop_generation) = 'integer' AND stop_generation >= 0)"),
        ("controls", "ALTER TABLE controls ADD COLUMN stop_generation INTEGER CHECK(stop_generation IS NULL OR (typeof(stop_generation) = 'integer' AND stop_generation >= 0))"),
    ] {
        let exists = {
            let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            columns.collect::<rusqlite::Result<Vec<_>>>()?.iter().any(|column| column == "stop_generation")
        };
        if !exists {
            connection.execute(sql, [])?;
        }
    }
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS recovery_admissions (
            authenticated_event_id TEXT PRIMARY KEY REFERENCES gateway_event_bindings(event_id),
            operator_id TEXT NOT NULL,
            objective_id TEXT NOT NULL REFERENCES objectives(id),
            leaf_id TEXT NOT NULL REFERENCES leaves(id),
            run_id TEXT NOT NULL UNIQUE,
            grant_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS recovery_publications (
            authenticated_event_id TEXT NOT NULL REFERENCES recovery_admissions(authenticated_event_id),
            publication_key TEXT NOT NULL,
            kind TEXT NOT NULL CHECK(kind IN ('provider-finalization','close','completion')),
            node_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            payload_digest TEXT NOT NULL,
            grant_digest TEXT NOT NULL,
            lease_generation INTEGER NOT NULL,
            lease_owner TEXT NOT NULL,
            lease_expires_ms INTEGER NOT NULL,
            authorized_at_ms INTEGER NOT NULL,
            applied_at_ms INTEGER,
            PRIMARY KEY(authenticated_event_id, publication_key)
        );
        CREATE TABLE IF NOT EXISTS recovery_completion_suppressions (
            authenticated_event_id TEXT PRIMARY KEY,
            publication_key TEXT NOT NULL CHECK(publication_key='completion'),
            identity_digest TEXT NOT NULL,
            reason TEXT NOT NULL CHECK(reason IN ('later-control','run-cancelled')),
            suppressed_at_ms INTEGER NOT NULL,
            FOREIGN KEY(authenticated_event_id,publication_key)
                REFERENCES recovery_publications(authenticated_event_id,publication_key)
        );
        CREATE TRIGGER IF NOT EXISTS recovery_suppression_unapplied
        BEFORE INSERT ON recovery_completion_suppressions BEGIN
            SELECT CASE WHEN NOT EXISTS (SELECT 1 FROM recovery_publications
                WHERE authenticated_event_id=NEW.authenticated_event_id
                AND publication_key='completion' AND kind='completion' AND applied_at_ms IS NULL)
                THEN RAISE(ABORT,'only pending completion may be suppressed') END;
        END;
        CREATE TRIGGER IF NOT EXISTS recovery_suppression_immutable_update
        BEFORE UPDATE ON recovery_completion_suppressions BEGIN
            SELECT RAISE(ABORT,'completion suppression is immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS recovery_suppression_immutable_delete
        BEFORE DELETE ON recovery_completion_suppressions BEGIN
            SELECT RAISE(ABORT,'completion suppression is immutable');
        END;
        CREATE TRIGGER IF NOT EXISTS recovery_suppression_no_ack
        BEFORE UPDATE OF applied_at_ms ON recovery_publications
        WHEN NEW.applied_at_ms IS NOT NULL AND EXISTS
            (SELECT 1 FROM recovery_completion_suppressions
             WHERE authenticated_event_id=NEW.authenticated_event_id AND publication_key=NEW.publication_key)
        BEGIN SELECT RAISE(ABORT,'suppressed completion cannot be acknowledged'); END;",
    )?;
    let has_recovery_marker: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('retained_snapshot_lease_intents') WHERE name='recovery_event_id')",
        [], |row| row.get(0),
    )?;
    if !has_recovery_marker {
        connection.execute("ALTER TABLE retained_snapshot_lease_intents ADD COLUMN recovery_event_id TEXT REFERENCES recovery_admissions(authenticated_event_id)", [])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_schema_upgrade_rolls_back_earlier_column_additions() {
        let db = Connection::open_in_memory().unwrap();
        apply(&db).unwrap();
        db.execute_batch(
            "ALTER TABLE objectives DROP COLUMN stop_generation;
            DROP TABLE controls;
            CREATE VIEW controls AS SELECT '' AS idempotency_key;",
        )
        .unwrap();
        assert!(apply(&db).is_err());
        let count: i64 = db
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('objectives') WHERE name='stop_generation'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 0);
        assert!(db.is_autocommit());
    }

    #[test]
    fn concurrent_legacy_openers_serialize_schema_checks() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("legacy.sqlite3");
        let db = Connection::open(&path).unwrap();
        apply(&db).unwrap();
        db.execute_batch(
            "ALTER TABLE controls DROP COLUMN stop_generation;
            ALTER TABLE objectives DROP COLUMN stop_generation;",
        )
        .unwrap();
        drop(db);
        let ready = std::sync::Arc::new(std::sync::Barrier::new(8));
        let workers: Vec<_> = (0..8).map(|_| {
            let (path, ready) = (path.clone(), ready.clone());
            std::thread::spawn(move || {
                let db = Connection::open(path).unwrap();
                db.busy_timeout(std::time::Duration::from_secs(10)).unwrap();
                ready.wait();
                apply(&db).unwrap();
                apply(&db).unwrap();
                let enabled: i64 = db.query_row("PRAGMA foreign_keys", [], |row| row.get(0)).unwrap();
                assert_eq!(enabled, 1);
                for table in ["objectives", "controls"] {
                    let count: i64 = db.query_row(&format!("SELECT COUNT(*) FROM pragma_table_info('{table}') WHERE name='stop_generation'"), [], |row| row.get(0)).unwrap();
                    assert_eq!(count, 1);
                }
            })
        }).collect();
        for worker in workers {
            worker.join().unwrap();
        }
    }

    #[test]
    fn existing_resident_context_bindings_gain_operator_marker_without_data_loss() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE resident_context_bindings (
                run_id TEXT PRIMARY KEY, request_digest TEXT NOT NULL, assembly_json TEXT NOT NULL
             );
             INSERT INTO resident_context_bindings VALUES ('run-1', 'digest-1', 'original-bytes');",
            )
            .unwrap();
        apply(&connection).unwrap();
        apply(&connection).unwrap();
        let binding: (String, String, Option<i64>) = connection
            .query_row(
                "SELECT request_digest, assembly_json, deleted_by_operator_ms
             FROM resident_context_bindings WHERE run_id = 'run-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(binding, ("digest-1".into(), "original-bytes".into(), None));
    }

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
