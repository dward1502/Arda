use arda_core::error::Result;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub(crate) fn prometheus_home() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_PROMETHEUS_HOME") {
        return PathBuf::from(path);
    }
    arda_root().join("data/prometheus")
}

pub(crate) fn append_jsonl(path: &Path, value: &serde_json::Value) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(value)?;
    writeln!(file, "{line}")?;
    Ok(())
}

pub(crate) fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<serde_json::Value> {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut values = Vec::new();
    for line in content.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            values.push(value);
            if values.len() >= limit.max(1) {
                break;
            }
        }
    }
    values.reverse();
    values
}

pub(crate) fn sha256_file_if_exists(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok("missing".to_string());
    }
    let bytes = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

pub(crate) fn queue_contains_task(path: &Path, task_id: &str) -> Result<bool> {
    let content = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(err.into()),
    };
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("id").and_then(|v| v.as_str()) == Some(task_id) {
            return Ok(true);
        }
    }
    Ok(false)
}
