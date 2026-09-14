//! Read-only current objective summaries. Never load legacy logs, execution
//! prompts or recovery capsules, and never create/migrate state during a read.
use super::ObjectiveState;
use anyhow::{bail, Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::time::Duration;

pub(crate) const OBJECTIVE_STORE_PATH: &str = "data/arda/objectives.sqlite3";

pub(crate) struct AgendaObjective {
    pub id: String,
    pub operator_id: String,
    pub text: String,
    pub priority: i64,
    pub state: ObjectiveState,
    pub project_id: Option<String>,
    pub next_wake_ms: Option<i64>,
    /// Exact persisted (leaf, run) bindings, without execution payloads.
    pub runs: Vec<(String, String)>,
}

pub(crate) fn read_agenda(root: &Path) -> Result<Option<Vec<AgendaObjective>>> {
    let path = root.join(OBJECTIVE_STORE_PATH);
    match std::fs::metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        other => {
            other.context("inspect objective store")?;
        }
    }
    let connection = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("open current objective authority read-only")?;
    connection.busy_timeout(Duration::from_secs(2))?;
    // One read transaction, with no ObjectiveStore::open migration side effects.
    connection.execute_batch("BEGIN DEFERRED")?;
    let has_schedule_runtime: bool = connection.query_row(
        "SELECT COUNT(*) = 2 FROM sqlite_master WHERE type = 'table' AND name IN ('schedule_wakes', 'schedule_errors')",
        [], |row| row.get(0),
    )?;
    // Pre-cutover schemas cannot prove wake consumption/quarantine. Do not
    // invent a next wake or migrate their tables from a projection reader.
    let next_wake = if has_schedule_runtime {
        "CASE WHEN o.state IN ('approved', 'running') THEN
         (SELECT MIN(s.next_wake_ms) FROM schedules s WHERE s.objective_id = o.id
          AND NOT EXISTS (SELECT 1 FROM schedule_wakes w WHERE w.schedule_id = s.id AND w.last_wake_ms >= s.next_wake_ms)
          AND NOT EXISTS (SELECT 1 FROM schedule_errors e WHERE e.schedule_id = s.id)) END"
    } else {
        "NULL"
    };
    let mut statement = connection.prepare(&format!(
        "SELECT o.id, o.operator_id, o.text, o.priority, o.state,
         (SELECT CASE WHEN COUNT(*) = 1 THEN MIN(project_id) END
          FROM objective_projects WHERE objective_id = o.id), {next_wake}
         FROM objectives o WHERE o.state NOT IN ('completed', 'cancelled', 'failed')
         ORDER BY o.priority DESC, o.updated_at_ms DESC, o.id"
    ))?;
    let has_run_identity: bool = connection.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('leaves') WHERE name = 'execution_run_id'",
        [],
        |row| row.get(0),
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<i64>>(6)?,
        ))
    })?;
    let mut agenda = Vec::new();
    for row in rows {
        let (id, operator_id, text, priority, state, project_id, next_wake_ms) = row?;
        let Some(state) = ObjectiveState::parse(&state) else {
            bail!("unknown objective state");
        };
        let runs = if has_run_identity {
            let mut leaves = connection.prepare(
                "SELECT id, execution_run_id FROM leaves WHERE objective_id = ?1
                 AND stage NOT IN ('complete', 'cancelled', 'failed') AND execution_run_id IS NOT NULL
                 ORDER BY id",
            )?;
            let bindings = leaves
                .query_map([&id], |row| Ok((row.get(0)?, row.get(1)?)))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            bindings
        } else {
            Vec::new()
        };
        agenda.push(AgendaObjective {
            id,
            operator_id,
            text,
            priority,
            state,
            project_id,
            next_wake_ms,
            runs,
        });
    }
    Ok(Some(agenda))
}
