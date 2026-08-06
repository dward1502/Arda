//! Append-only JSONL store for personal-ops events.

use std::{
    io::{BufRead, BufReader, Seek, SeekFrom, Write},
    os::{fd::AsRawFd, unix::fs::PermissionsExt},
    path::{Path, PathBuf},
};

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
        self.append_once(envelope).map(|_| ())
    }

    /// Append an event exactly once by durable event ID. The exclusive file
    /// lock makes concurrent retry checks and writes one atomic operation.
    /// Returns `true` when appended and `false` for a replayed event ID.
    pub fn append_once(
        &self,
        envelope: &PersonalOpsEnvelope<PersonalOpsRecord>,
    ) -> std::io::Result<bool> {
        let parent = self.events_path.parent().expect("events path has a parent");
        std::fs::create_dir_all(parent)?;
        std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(&self.events_path)?;
        std::fs::set_permissions(&self.events_path, std::fs::Permissions::from_mode(0o600))?;
        let _lock = FileLock::exclusive(&file)?;

        file.seek(SeekFrom::Start(0))?;
        for line in BufReader::new(&file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let existing: PersonalOpsEnvelope<PersonalOpsRecord> = serde_json::from_str(&line)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            if existing.record.event_id() == envelope.record.event_id() {
                return Ok(false);
            }
        }

        let line = serde_json::to_string(envelope).expect("envelope is serializable");
        writeln!(file, "{}", line)?;
        file.sync_all()?;
        Ok(true)
    }

    /// Load all events in order from the log file. Missing file yields empty
    /// vec; malformed lines are surfaced as `LoadError`.
    pub fn load_all(&self) -> Result<Vec<PersonalOpsEnvelope<PersonalOpsRecord>>, LoadError> {
        let file = match std::fs::File::open(&self.events_path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(LoadError::Io(error)),
        };
        let _lock = FileLock::shared(&file).map_err(LoadError::Io)?;

        let mut events = Vec::new();
        for (line_no, line) in BufReader::new(&file).lines().enumerate() {
            let line = line.map_err(LoadError::Io)?;
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

struct FileLock {
    fd: std::os::fd::RawFd,
}

impl FileLock {
    fn exclusive(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_EX)
    }

    fn shared(file: &std::fs::File) -> std::io::Result<Self> {
        Self::acquire(file, libc::LOCK_SH)
    }

    fn acquire(file: &std::fs::File, operation: libc::c_int) -> std::io::Result<Self> {
        let fd = file.as_raw_fd();
        // SAFETY: `fd` belongs to the live file declared before the guard.
        if unsafe { libc::flock(fd, operation) } == -1 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self { fd })
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        // SAFETY: the file is declared before the guard and drops after it.
        let _ = unsafe { libc::flock(self.fd, libc::LOCK_UN) };
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
