use crate::error::Result;
use chrono::Utc;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

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
