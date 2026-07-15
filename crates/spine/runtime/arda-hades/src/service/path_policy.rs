use arda_core::error::ardaError;
use once_cell::sync::Lazy;
use regex::RegexSet;
use std::path::{Path, PathBuf};

// Pre-compiled skip patterns — compiled once, used per-file in the hot sweep loop.
// This replaces 40+ inline `contains()` / `ends_with()` calls with a single regex set match.
static SKIP_PATTERNS: Lazy<Option<RegexSet>> = Lazy::new(|| {
    RegexSet::new([
        // build / VCS dirs
        "(^|/)target/",
        "(^|/)\\.git/",
        // core state (protected)
        "(^|/)core/state/",
        "core/state/world\\.json$",
        "core/state/autonomy_runtime\\.json$",
        // generated env files
        "\\.env\\.generated$",
        "runtime\\.generated\\.env$",
        "(^|/)\\.generated/",
        // directory index
        "(?i)INDEX\\.jsonl$",
        // tmp
        "(^|/)tmp/",
        // metrics (low-value, handled separately but also skipped)
        "(^|/)core/metrics/history/",
        "(^|/)core/metrics/by_crate/",
        "core/metrics/audit_latest\\.json$",
        "core/metrics/manifest\\.json$",
        // agent data dirs (never sweep own data)
        "(^|/)data/hades/",
        "(^|/)data/mnemosyne/",
        "(^|/)data/prometheus/",
        // core protected subtrees (case-insensitive)
        "(?i)(^|/)core/realm/",
        "(?i)(^|/)core/projects/",
        "(?i)(^|/)core/clients/",
        "(?i)(^|/)core/queue/",
        "(?i)(^|/)core/projects/plans/",
        "(?i)core/projects/_registry\\.toml$",
        // backup files
        "(?i)\\.bak\\.",
    ])
    .ok()
});

// File extensions that are always skipped (binary / archive — faster than regex).
const SKIP_EXTENSIONS: &[&str] = &["png", "ico", "icns", "zip", "tar", "gz"];

pub(super) fn default_root() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_HADES_HOME") {
        return PathBuf::from(custom);
    }
    PathBuf::from("data/hades")
}

pub(super) fn default_world_state_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_WORLD_STATE_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/state/world.json")
}

pub(super) fn default_destructive_policy_path() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_HADES_DESTRUCTIVE_POLICY_PATH") {
        return PathBuf::from(custom);
    }
    PathBuf::from("core/state/destructive_quorum.json")
}

pub(super) fn default_watch_paths() -> Vec<PathBuf> {
    if let Ok(custom) = std::env::var("ARDA_HADES_WATCH_PATHS") {
        let parsed = custom
            .split(':')
            .flat_map(|seg| seg.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if !parsed.is_empty() {
            return parsed;
        }
    }
    vec![
        PathBuf::from("core"),
        PathBuf::from("docs"),
        PathBuf::from("config"),
    ]
}

pub(super) fn sweep_interval_hours() -> i64 {
    std::env::var("ARDA_HADES_SWEEP_INTERVAL_HOURS")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.clamp(1, 48))
        .unwrap_or(8)
}

pub(super) fn scheduler_snapshot() -> serde_json::Value {
    let morning_hour_utc = std::env::var("ARDA_HADES_MORNING_HOUR_UTC")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .map(|v| v.min(23))
        .unwrap_or(14);
    let nightly_hour_utc = std::env::var("ARDA_HADES_NIGHTLY_HOUR_UTC")
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .map(|v| v.min(23))
        .unwrap_or(4);
    serde_json::json!({
        "sweep_interval_hours": sweep_interval_hours(),
        "morning_hour_utc": morning_hour_utc,
        "nightly_hour_utc": nightly_hour_utc,
        "watch_paths": default_watch_paths(),
    })
}

pub(super) fn should_skip_watch_file(path: &Path) -> bool {
    let path_s = path.to_string_lossy();

    // Fast path: check binary/archive extensions first (no allocation, no regex).
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if SKIP_EXTENSIONS.contains(&ext) {
            return true;
        }
    }

    // Single regex-set match replaces 40+ contains/ends_with checks.
    SKIP_PATTERNS
        .as_ref()
        .is_some_and(|patterns| patterns.is_match(&path_s))
}

pub(super) fn is_low_value_warden_repair_target(path: &Path) -> bool {
    let path_s = path.to_string_lossy().to_ascii_lowercase();
    path_s.contains("/core/metrics/history/")
        || path_s.contains("core/metrics/history/")
        || path_s.contains("/core/metrics/by_crate/")
        || path_s.contains("core/metrics/by_crate/")
        || path_s.ends_with("core/metrics/audit_latest.json")
        || path_s.ends_with("core/metrics/manifest.json")
        || path_s.ends_with("/index.jsonl")
        || path_s.ends_with("index.jsonl")
        || path_s.ends_with("governance/signals_history.jsonl")
}

pub(super) fn low_value_warden_repair_class(path: &Path) -> &'static str {
    let path_s = path.to_string_lossy().to_ascii_lowercase();
    if path_s.contains("/core/metrics/history/") || path_s.contains("core/metrics/history/") {
        "metrics_history"
    } else if path_s.contains("/core/metrics/by_crate/")
        || path_s.contains("core/metrics/by_crate/")
    {
        "metrics_projection"
    } else if path_s.ends_with("/index.jsonl") || path_s.ends_with("index.jsonl") {
        "generated_index"
    } else if path_s.ends_with("governance/signals_history.jsonl") {
        "governance_signals_history"
    } else if path_s.ends_with("core/metrics/audit_latest.json")
        || path_s.ends_with("core/metrics/manifest.json")
    {
        "metrics_manifest"
    } else {
        "low_value_generated"
    }
}

pub(super) fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}
