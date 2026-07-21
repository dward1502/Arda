#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Outcome observer — tails queue.jsonl for records that have transitioned
//! to a terminal state since the last cycle, and feeds learning + registry.

use super::delegation::AgentRegistry;
use super::task_queue::QueueRecord;
use arda_core::learning::{LearningState, OutcomeStats};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservedCursor {
    #[serde(default)]
    pub byte_offset: u64,
    #[serde(default)]
    pub seen_terminal_ids: BTreeSet<String>,
}

pub struct OutcomeObserver {
    cursor_path: PathBuf,
}

impl OutcomeObserver {
    pub fn new(cursor_path: impl AsRef<Path>) -> Self {
        Self {
            cursor_path: cursor_path.as_ref().to_path_buf(),
        }
    }

    pub fn load(&self) -> ObservedCursor {
        std::fs::read_to_string(&self.cursor_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, c: &ObservedCursor) -> std::io::Result<()> {
        if let Some(p) = self.cursor_path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let data = serde_json::to_vec_pretty(c).map_err(std::io::Error::other)?;
        std::fs::write(&self.cursor_path, data)
    }

    /// Tail queue, observe terminal records, return number of new outcomes ingested.
    pub fn ingest(
        &self,
        queue_path: impl AsRef<Path>,
        registry: &mut AgentRegistry,
        learning: &mut LearningState,
    ) -> usize {
        let mut cursor = self.load();
        let records = match load_records_since(queue_path.as_ref(), &mut cursor) {
            Ok(records) => records,
            Err(_) => return 0,
        };
        let mut new = 0usize;
        for r in records {
            let status = r.status.as_deref().unwrap_or("");
            let terminal = matches!(status, "completed" | "done" | "failed" | "error");
            if !terminal {
                continue;
            }
            if cursor.seen_terminal_ids.contains(&r.id) {
                continue;
            }
            cursor.seen_terminal_ids.insert(r.id.clone());
            let success = matches!(status, "completed" | "done");
            let agent = r.owner.clone().unwrap_or_else(|| "unknown".into());
            let task_type = task_type_of(&r);
            let dur = duration_secs(&r);
            let joules = r
                .extra
                .get("joule_cost_estimate")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            learning.observe(&agent, &task_type, success, dur, joules);
            registry.record_completed(&agent, success);
            new += 1;
        }
        let _ = self.save(&cursor);
        new
    }
}

fn load_records_since(
    queue_path: &Path,
    cursor: &mut ObservedCursor,
) -> std::io::Result<Vec<QueueRecord>> {
    let mut file = File::open(queue_path)?;
    let len = file.metadata()?.len();
    if cursor.byte_offset > len {
        cursor.byte_offset = 0;
    }
    file.seek(SeekFrom::Start(cursor.byte_offset))?;

    let mut reader = BufReader::new(file);
    let mut records = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            break;
        }
        cursor.byte_offset = reader.stream_position()?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(record) = serde_json::from_str::<QueueRecord>(trimmed) {
            records.push(record);
        }
    }
    Ok(records)
}

fn task_type_of(r: &QueueRecord) -> String {
    r.extra
        .get("task_type")
        .and_then(|v| v.as_str())
        .unwrap_or("ops")
        .to_string()
}

fn duration_secs(r: &QueueRecord) -> f64 {
    let start = r
        .started_at_utc
        .as_deref()
        .or(r.queued_at_utc.as_deref())
        .and_then(parse_utc);
    let end = r.completed_at_utc.as_deref().and_then(parse_utc);
    match (start, end) {
        (Some(s), Some(e)) => (e - s).num_milliseconds() as f64 / 1000.0,
        _ => 0.0,
    }
}

fn parse_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::super::delegation::AgentCapabilities;
    use super::*;
    use std::io::Write;
    #[test]
    fn ingests_only_new_terminals() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        std::fs::write(&q, r#"{"id":"x","status":"completed","owner":"warden","task_type":"monitor","queued_at_utc":"2026-01-01T00:00:00Z","completed_at_utc":"2026-01-01T00:00:30Z"}
{"id":"y","status":"failed","owner":"warden","task_type":"monitor"}
{"id":"z","status":"pending","owner":"warden"}
"#).unwrap();
        let mut reg = AgentRegistry::new();
        reg.register(AgentCapabilities {
            agent_id: "warden".into(),
            task_types: vec!["monitor".into()],
            max_concurrent: 4,
            current_load: 2,
            success_rate: 0.5,
        });
        let mut learn = LearningState::default();
        let obs = OutcomeObserver::new(dir.path().join("cursor.json"));
        let n1 = obs.ingest(&q, &mut reg, &mut learn);
        assert_eq!(n1, 2);
        let n2 = obs.ingest(&q, &mut reg, &mut learn);
        assert_eq!(n2, 0, "second pass should ingest nothing");
        let cursor = obs.load();
        assert_eq!(cursor.byte_offset, std::fs::metadata(&q).unwrap().len());
    }

    #[test]
    fn tails_only_appended_queue_records() {
        let dir = tempfile::tempdir().unwrap();
        let q = dir.path().join("queue.jsonl");
        std::fs::write(
            &q,
            r#"{"id":"x","status":"completed","owner":"warden","task_type":"monitor"}
"#,
        )
        .unwrap();
        let mut reg = AgentRegistry::new();
        reg.register(AgentCapabilities {
            agent_id: "warden".into(),
            task_types: vec!["monitor".into()],
            max_concurrent: 4,
            current_load: 0,
            success_rate: 0.5,
        });
        let mut learn = LearningState::default();
        let obs = OutcomeObserver::new(dir.path().join("cursor.json"));
        assert_eq!(obs.ingest(&q, &mut reg, &mut learn), 1);

        std::fs::OpenOptions::new()
            .append(true)
            .open(&q)
            .unwrap()
            .write_all(
                br#"{"id":"y","status":"completed","owner":"warden","task_type":"monitor"}
"#,
            )
            .unwrap();

        assert_eq!(obs.ingest(&q, &mut reg, &mut learn), 1);
        assert_eq!(learn.stats["warden::monitor"].attempts, 2);
    }
}
