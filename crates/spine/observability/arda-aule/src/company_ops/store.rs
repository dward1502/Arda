use super::CompanyOpsEvent;
use fs2::FileExt;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    Appended,
    Duplicate,
}

#[derive(Debug)]
pub struct CompanyOpsStore {
    pub events_path: PathBuf,
}

impl CompanyOpsStore {
    pub fn new(root: &Path) -> Self {
        Self {
            events_path: root.join("data/business/events.jsonl"),
        }
    }

    pub fn append(&self, event: &CompanyOpsEvent) -> Result<AppendOutcome, CompanyOpsStoreError> {
        if event.idempotency_key.trim().is_empty() {
            return Err(CompanyOpsStoreError::EmptyIdempotencyKey);
        }
        let parent = self.events_path.parent().expect("event path has parent");
        std::fs::create_dir_all(parent)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.events_path)?;
        file.lock_exclusive()?;
        file.seek(SeekFrom::Start(0))?;
        for (line, value) in BufReader::new(&file).lines().enumerate() {
            let value = value?;
            if value.trim().is_empty() {
                continue;
            }
            let existing: CompanyOpsEvent =
                serde_json::from_str(&value).map_err(|source| CompanyOpsStoreError::Parse {
                    line: line + 1,
                    source,
                })?;
            if existing.event_id == event.event_id
                || existing.idempotency_key == event.idempotency_key
            {
                FileExt::unlock(&file)?;
                return Ok(AppendOutcome::Duplicate);
            }
        }
        writeln!(file, "{}", serde_json::to_string(event)?)?;
        file.sync_all()?;
        FileExt::unlock(&file)?;
        Ok(AppendOutcome::Appended)
    }

    pub fn load(&self) -> Result<Vec<CompanyOpsEvent>, CompanyOpsStoreError> {
        let file = match std::fs::File::open(&self.events_path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(error) => return Err(error.into()),
        };
        file.lock_shared()?;
        let mut events: Vec<CompanyOpsEvent> = vec![];
        for (line, value) in BufReader::new(&file).lines().enumerate() {
            let value = value?;
            if value.trim().is_empty() {
                continue;
            }
            events.push(serde_json::from_str(&value).map_err(|source| {
                CompanyOpsStoreError::Parse {
                    line: line + 1,
                    source,
                }
            })?);
        }
        FileExt::unlock(&file)?;
        events.sort_by_key(|event| (event.occurred_at, event.event_id));
        Ok(events)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CompanyOpsStoreError {
    #[error("company operations idempotency key cannot be empty")]
    EmptyIdempotencyKey,
    #[error("company operations I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("company operations serialization failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid company operations event at line {line}: {source}")]
    Parse {
        line: usize,
        #[source]
        source: serde_json::Error,
    },
}
