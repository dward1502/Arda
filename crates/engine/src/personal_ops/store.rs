//! Append-only JSONL store for personal-ops events.

use std::path::{Path, PathBuf};

use arda_core::personal_ops::{PersonalOpsEnvelope, PersonalOpsRecord};

/// Append-only JSONL store for personal-ops events.
#[derive(Debug)]
pub struct PersonalOpsLogStore {
    pub events_path: PathBuf,
}

impl PersonalOpsLogStore {
    pub fn new(root: &Path) -> Self {
        Self {
            events_path: root.join("data/personal/events.jsonl"),
        }
    }

    /// Append a single envelope to the event log.
    pub fn append(&self, envelope: &PersonalOpsEnvelope<PersonalOpsRecord>) -> std::io::Result<()> {
        let parent = self.events_path.parent().expect("events path has a parent");
        std::fs::create_dir_all(parent)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.events_path)?;
        let line = serde_json::to_string(envelope).expect("envelope is serializable");
        use std::io::Write;
        writeln!(file, "{}", line)?;
        file.sync_all()?;
        Ok(())
    }

    /// Load all events in order from the log file. Missing file yields empty
    /// vec; malformed lines are surfaced as `LoadError`.
    pub fn load_all(&self) -> Result<Vec<PersonalOpsEnvelope<PersonalOpsRecord>>, LoadError> {
        let raw = match std::fs::read_to_string(&self.events_path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(LoadError::Io(error)),
        };

        let mut events = Vec::new();
        for (line_no, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let envelope: PersonalOpsEnvelope<PersonalOpsRecord> = serde_json::from_str(line)
                .map_err(|error| LoadError::Parse {
                    line: line_no + 1,
                    error,
                })?;
            events.push(envelope);
        }
        Ok(events)
    }
}

/// Error type for loading personal-ops event logs.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("io error reading personal-ops log: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse personal-ops event at line {line}: {error}")]
    Parse {
        line: usize,
        #[source]
        error: serde_json::Error,
    },
}
