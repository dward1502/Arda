use arda_core::error::{ArdaError, Result};
use fs2::FileExt;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub(super) fn background_signal_limit() -> usize {
    std::env::var("Arda_BACKGROUND_SIGNAL_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(4)
}

pub(super) fn hades_sweep_limit() -> usize {
    std::env::var("Arda_HADES_SWEEP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

pub(super) fn hades_removal_step_limit() -> usize {
    std::env::var("Arda_HADES_REMOVAL_STEP_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

pub(super) fn touch(path: &Path) -> Result<()> {
    OpenOptions::new().create(true).append(true).open(path)?;
    Ok(())
}

pub(super) fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let mut line = serde_json::to_string(value)?;
    line.push('\n');
    let write_result = file.write_all(line.as_bytes());
    let unlock_result = file.unlock().map_err(ArdaError::Ledger);
    write_result?;
    unlock_result?;
    Ok(())
}

/// Batch-append multiple JSONL records in a single lock+write+fsync cycle.
/// Much faster than calling `append_jsonl` in a loop for each record.
pub(super) fn append_jsonl_batch<T: Serialize>(path: &Path, values: &[T]) -> Result<()> {
    if values.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let write_result = (|| -> Result<()> {
        for value in values {
            let mut line = serde_json::to_string(value)?;
            line.push('\n');
            file.write_all(line.as_bytes())?;
        }
        file.sync_data()?;
        Ok(())
    })();
    let unlock_result = file.unlock().map_err(ArdaError::Ledger);
    write_result?;
    unlock_result?;
    Ok(())
}

pub(super) fn read_recent_jsonl(path: &Path, limit: usize) -> Result<Vec<serde_json::Value>> {
    let content = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            out.push(value);
            if out.len() >= limit {
                break;
            }
        }
    }
    out.reverse();
    Ok(out)
}

pub(super) fn count_malformed_jsonl(path: &Path) -> usize {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_err())
        .count()
}
