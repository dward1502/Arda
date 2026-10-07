use super::CharonService;
use arda_core::error::{ArdaError, Result};
use fs2::FileExt;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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

    pub(super) fn manwe_eval_receipts_path(&self) -> PathBuf {
        self.root.join("model_eval_receipts.jsonl")
    }
}

pub(crate) fn runtime_build_cache_state_path() -> PathBuf {
    std::env::var("ARDA_RUNTIME_BUILD_CACHE_STATE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| super::paths::arda_root().join("core/state/runtime_build_cache.json"))
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
                "annunimas-cli".to_string(),
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

/// Sample recent valid records using bounded I/O. `complete` means the requested
/// count was found or the entire file was sampled; false must not imply zero
/// historical failures. A partial first line is never parsed as a record.
pub(super) fn read_recent_jsonl_bounded(
    path: &Path,
    limit: usize,
    max_bytes: usize,
) -> (Vec<serde_json::Value>, bool) {
    if limit == 0 {
        return (Vec::new(), true);
    }
    fs::File::open(path)
        .and_then(|mut file| read_jsonl_tail(&mut file, limit, max_bytes))
        .unwrap_or_else(|_| (Vec::new(), false))
}

fn read_jsonl_tail(
    file: &mut (impl Read + Seek),
    limit: usize,
    max_bytes: usize,
) -> std::io::Result<(Vec<serde_json::Value>, bool)> {
    let end = file.seek(SeekFrom::End(0))?;
    let start = end.saturating_sub(max_bytes as u64);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.take(end - start).read_to_end(&mut bytes)?;
    let complete_line_start = if start == 0 {
        0
    } else {
        bytes
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |p| p + 1)
    };
    let mut values = bytes[complete_line_start..]
        .split(|b| *b == b'\n')
        .rev()
        .filter_map(|line| serde_json::from_slice(line).ok())
        .take(limit)
        .collect::<Vec<serde_json::Value>>();
    let complete = values.len() == limit || start == 0;
    values.reverse();
    Ok((values, complete))
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
    let arda_home = std::env::var_os("ARDA_HOME");
    crate::config::adaptive_state_dir(arda_home.as_deref().map(Path::new))
}

pub(crate) fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

#[cfg(test)]
mod tail_tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn tail_retains_order_and_skips_malformed_records() {
        let mut input = Cursor::new(b"{\"n\":1}\ninvalid\n\n{\"n\":2}\r\n{\"n\":3}");
        let (records, complete) = read_jsonl_tail(&mut input, 2, 1024).unwrap();
        assert_eq!(records, vec![json!({"n":2}), json!({"n":3})]);
        assert!(complete);
    }

    #[test]
    fn cutoff_never_invents_a_record_from_a_partial_line() {
        let mut input = Cursor::new(b"invalid prefix {\"n\":1}\n{\"n\":2}\n");
        let (records, complete) = read_jsonl_tail(&mut input, 48, 16).unwrap();
        assert_eq!(records, vec![json!({"n":2})]);
        assert!(!complete);
    }

    #[test]
    fn history_size_does_not_increase_sample_io() {
        struct Metered {
            inner: Cursor<Vec<u8>>,
            bytes: usize,
        }
        impl Read for Metered {
            fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
                let n = self.inner.read(b)?;
                self.bytes += n;
                Ok(n)
            }
        }
        impl Seek for Metered {
            fn seek(&mut self, p: SeekFrom) -> std::io::Result<u64> {
                self.inner.seek(p)
            }
        }
        let mut data = vec![b'x'; 1024 * 1024];
        data.push(b'\n');
        for n in 0..80 {
            data.extend_from_slice(format!("{{\"n\":{n}}}\n").as_bytes());
        }
        let mut input = Metered {
            inner: Cursor::new(data),
            bytes: 0,
        };
        let (records, complete) = read_jsonl_tail(&mut input, 48, 1024).unwrap();
        assert_eq!(input.bytes, 1024);
        assert_eq!(records.first(), Some(&json!({"n":32})));
        assert_eq!(records.last(), Some(&json!({"n":79})));
        assert!(complete);
    }

    #[test]
    fn empty_missing_and_zero_samples_report_coverage() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent");
        assert_eq!(read_recent_jsonl_bounded(&path, 48, 1024), (vec![], false));
        assert_eq!(read_recent_jsonl_bounded(&path, 0, 1024), (vec![], true));
        fs::write(&path, "").unwrap();
        assert_eq!(read_recent_jsonl_bounded(&path, 48, 1024), (vec![], true));
    }
}
