use annunimas_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ThoughtLedger {
    root: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtRecord {
    pub thought_id: String,
    pub thought_type: String,
    pub trigger: String,
    pub content: String,
    pub ts: String,
}

impl ThoughtLedger {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        fs::create_dir_all(root.as_ref())?;
        Ok(Self {
            root: root.as_ref().to_path_buf(),
        })
    }

    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_machine_thought_path();
        match Self::new(&primary) {
            Ok(v) => Ok(v),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(PathBuf::from("data").join("minds").join("machine"))
            }
        }
    }

    pub fn append(
        &self,
        thought_type: &str,
        trigger: &str,
        content: &str,
    ) -> Result<ThoughtRecord> {
        let today = Utc::now().format("%Y-%m-%d").to_string();
        let day_dir = self.root.join(today);
        fs::create_dir_all(&day_dir)?;

        let thought_id = format!("tht_{}", &Uuid::new_v4().simple().to_string()[..8]);
        let file_path = day_dir.join(format!("{thought_id}.jsonl"));
        let ts = Utc::now().to_rfc3339();

        let header = serde_json::json!({
            "sigil":"ANKH",
            "created_at_utc": ts,
            "thought_id": thought_id,
            "authored_by":"prometheus",
            "version":"0.1.0"
        });
        let entry = ThoughtRecord {
            thought_id: thought_id.clone(),
            thought_type: thought_type.to_string(),
            trigger: trigger.to_string(),
            content: content.to_string(),
            ts: Utc::now().to_rfc3339(),
        };
        let body = serde_json::json!({
            "type": entry.thought_type,
            "ts": entry.ts,
            "trigger": entry.trigger,
            "content": entry.content
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)?;
        writeln!(file, "{}", serde_json::to_string(&header)?)?;
        writeln!(file, "{}", serde_json::to_string(&body)?)?;

        Ok(entry)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn count_today(&self) -> Result<usize> {
        let day_dir = self.root.join(Utc::now().format("%Y-%m-%d").to_string());
        if !day_dir.exists() {
            return Ok(0);
        }
        let count = std::fs::read_dir(day_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|v| v.to_str()) == Some("jsonl"))
            .count();
        Ok(count)
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let day_dir = self.root.join(Utc::now().format("%Y-%m-%d").to_string());
        if !day_dir.exists() {
            return Ok(Vec::new());
        }

        let mut files = std::fs::read_dir(day_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|v| v.to_str()) == Some("jsonl"))
            .collect::<Vec<_>>();
        files.sort();

        let mut entries = Vec::new();
        for file in files {
            let content = std::fs::read_to_string(file)?;
            // Line 2 is the thought body.
            if let Some(line) = content.lines().nth(1) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                    entries.push(value);
                }
            }
        }

        if entries.len() > limit {
            let start = entries.len().saturating_sub(limit);
            Ok(entries.split_off(start))
        } else {
            Ok(entries)
        }
    }
}

fn default_machine_thought_path() -> PathBuf {
    if let Ok(path) = std::env::var("ANNUNIMAS_PROMETHEUS_MINDS") {
        return PathBuf::from(path);
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".citadel")
            .join("minds")
            .join("machine");
    }
    PathBuf::from(".citadel/minds/machine")
}

fn is_permission_error(err: &annunimas_core::error::AnnunimasError) -> bool {
    matches!(
        err,
        annunimas_core::error::AnnunimasError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

#[cfg(test)]
mod tests {
    use super::ThoughtLedger;
    use tempfile::tempdir;

    #[test]
    fn writes_machine_thought_file() {
        let dir = tempdir().expect("tempdir");
        let ledger = ThoughtLedger::new(dir.path()).expect("ledger");
        let thought = ledger
            .append("audit", "test", "delegated to athena")
            .expect("append");

        let day_dir = ledger
            .root()
            .join(chrono::Utc::now().format("%Y-%m-%d").to_string());
        let file_path = day_dir.join(format!("{}.jsonl", thought.thought_id));
        let content = std::fs::read_to_string(file_path).expect("content");
        assert!(content.contains("\"type\":\"audit\""));
    }
}
