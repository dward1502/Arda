use crate::error::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Non-blocking transport contract for governance ledger events.
///
/// The event type remains generic so this lowest-layer crate owns transport
/// semantics without owning a particular governance receipt schema.
pub trait GovernanceLedgerSink<E>: Send + Sync {
    fn try_enqueue(&self, event: E) -> std::result::Result<(), LedgerEnqueueError>;
}

/// Shared contract for durable ledgers whose only mutation is appending a new
/// entry. Implementations must never rewrite or truncate prior records.
pub trait AppendOnlyLedger<E>: Send + Sync {
    type Error;

    fn append_entry(&self, entry: &E) -> std::result::Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerEnqueueError {
    Saturated,
    Closed,
}

impl std::fmt::Display for LedgerEnqueueError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Saturated => formatter.write_str("governance ledger queue is saturated"),
            Self::Closed => formatter.write_str("governance ledger writer is closed"),
        }
    }
}

impl std::error::Error for LedgerEnqueueError {}

pub struct Ledger {
    path: PathBuf,
}

impl Ledger {
    pub fn new(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref();
        fs::create_dir_all(dir)?;
        let filename = format!("ledger_{}.jsonl", Utc::now().format("%Y-%m-%d"));
        Ok(Self {
            path: dir.join(filename),
        })
    }

    pub fn append<T: Serialize>(&self, entry: &T) -> Result<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;

        let mut json = serde_json::to_value(entry)?;
        if let serde_json::Value::Object(map) = &mut json {
            map.insert(
                "soterion".to_string(),
                serde_json::json!({
                    "sigil": "𓆣",  // Energy / Ledger
                    "realm": "ledger",
                    "timestamp": Utc::now().to_rfc3339()
                }),
            );
        }

        let line = serde_json::to_string(&json)?;
        writeln!(file, "{}", line)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl<E: Serialize + Send + Sync> AppendOnlyLedger<E> for Ledger {
    type Error = crate::error::ArdaError;

    fn append_entry(&self, entry: &E) -> std::result::Result<(), Self::Error> {
        self.append(entry)
    }
}
