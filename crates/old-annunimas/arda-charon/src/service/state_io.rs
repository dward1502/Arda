use super::CharonService;
use arda_core::error::{ArdaError, Result};
use fs2::FileExt;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

impl CharonService {
    pub(super) fn package_runtime_signals_path(&self) -> PathBuf {
        self.root.join("package_runtime_signals.json")
    }

    pub(super) fn lane_fitness_path(&self) -> PathBuf {
        self.root.join("lane_fitness.json")
    }

    pub(super) fn provider_runtime_state_path(&self) -> PathBuf {
        self.root.join("provider_runtime_state.json")
    }

    pub(super) fn provider_capability_receipts_path(&self) -> PathBuf {
        self.provider_capability_receipts_path.clone()
    }

    pub(super) fn charon_eval_receipts_path(&self) -> PathBuf {
        self.root.join("model_eval_receipts.jsonl")
    }
}

pub(crate) fn runtime_build_cache_state_path() -> PathBuf {
    std::env::var("ARDA_RUNTIME_BUILD_CACHE_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            super::paths::arda_root().join("core/state/runtime_build_cache.json")
        })
}

pub(crate) fn runtime_build_cache_command_program() -> String {
    std::env::var("ARDA_RUNTIME_BUILD_COMPACTOR").unwrap_or_else(|_| "cargo".to_string())
}

pub(crate) fn runtime_build_cache_command_args() -> Vec<String> {
    std::env::var("ARDA_RUNTIME_BUILD_COMPACTOR_ARGS")
        .map(|value| {
            value
                .split_whitespace()
                .map(|item| item.to_string())
                .collect()
        })
        .unwrap_or_else(|_| {
            vec![
                "run".to_string(),
                "-p".to_string(),
                "arda-cli".to_string(),
                "--".to_string(),
                "control".to_string(),
                "prune-runtime-build-cache".to_string(),
            ]
        })
}

pub(crate) fn runtime_build_cache_autorun_enabled() -> bool {
    std::env::var("ARDA_RUNTIME_BUILD_CACHE_AUTORUN")
        .map(|value| value != "0")
        .unwrap_or(false)
}

pub(crate) fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<serde_json::Value> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let mut values = Vec::new();
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            values.push(value);
            if values.len() >= limit {
                break;
            }
        }
    }
    values.reverse();
    values
}

pub(crate) fn count_malformed_jsonl(path: &Path) -> usize {
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

pub(crate) fn touch(path: &Path) -> Result<()> {
    OpenOptions::new().create(true).append(true).open(path)?;
    Ok(())
}

pub(crate) fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let line = serde_json::to_string(value)?;
    let write_result = (|| -> Result<()> {
        writeln!(file, "{line}")?;
        file.sync_data()?;
        Ok(())
    })();
    let unlock_result = file.unlock().map_err(ArdaError::Ledger);
    write_result?;
    unlock_result?;
    Ok(())
}

pub(crate) fn default_root() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_CHARON_HOME") {
        return PathBuf::from(custom);
    }
    super::paths::arda_root().join("data/charon")
}

pub(crate) fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}
