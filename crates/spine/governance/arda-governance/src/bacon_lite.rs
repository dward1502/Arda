// sigil: REPAIR
use crate::paths::GovernancePaths;
use crate::triad::{triad_validate, GateOutcome, TriadConfig, TriadResult};
use arda_core::task::Task;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Explicit destinations for Bacon-Lite machine and operator evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaconLiteLogPaths {
    pub machine: PathBuf,
    pub human: PathBuf,
}

impl BaconLiteLogPaths {
    pub fn from_base_dir(base_dir: impl Into<PathBuf>) -> Self {
        let paths = GovernancePaths::new(base_dir);
        Self {
            machine: paths.bacon_lite_machine_log(),
            human: paths.bacon_lite_human_log(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaconLiteResult {
    pub passed: bool,
    pub confidence: f64,
    pub mode: String,
    pub rationale: String,
    pub triad: TriadResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaconLiteEvent {
    pub ts_utc: String,
    pub crate_name: String,
    pub action: String,
    pub task_id: String,
    pub task_type: String,
    pub description: String,
    pub passed: bool,
    pub confidence: f64,
    pub rationale: String,
    pub triad_passed: bool,
    pub aurelius_score: f64,
    pub bacon_score: f64,
    pub sun_tzu_score: f64,
    pub context: Value,
}

pub fn bacon_lite_validate(task: &Task) -> BaconLiteResult {
    let triad = triad_validate(
        task,
        Some(&TriadConfig {
            strict: false,
            required_passes: Some(1),
        }),
    );

    let bacon_ok = triad.bacon != GateOutcome::Fail;
    let support_ok = triad.aurelius != GateOutcome::Fail || triad.sun_tzu != GateOutcome::Fail;
    let passed = bacon_ok && support_ok;
    let confidence =
        ((triad.bacon_score * 0.6) + (triad.aurelius_score * 0.2) + (triad.sun_tzu_score * 0.2))
            .clamp(0.0, 1.0);
    let rationale = if passed {
        "bacon-lite pass: evidence gate plus one support gate".to_string()
    } else {
        "bacon-lite fail: insufficient evidence/support gates".to_string()
    };

    BaconLiteResult {
        passed,
        confidence,
        mode: "bacon_lite".to_string(),
        rationale,
        triad,
    }
}

pub fn record_bacon_lite(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
) -> std::io::Result<BaconLiteEvent> {
    let paths = default_log_paths()?;
    record_bacon_lite_to(crate_name, action, task, context, &paths)
}

/// Validate and write Bacon-Lite evidence to caller-selected destinations.
pub fn record_bacon_lite_to(
    crate_name: &str,
    action: &str,
    task: &Task,
    context: Value,
    paths: &BaconLiteLogPaths,
) -> std::io::Result<BaconLiteEvent> {
    let result = bacon_lite_validate(task);
    let event = BaconLiteEvent {
        ts_utc: Utc::now().to_rfc3339(),
        crate_name: crate_name.to_string(),
        action: action.to_string(),
        task_id: task.id.to_string(),
        task_type: task.task_type.clone(),
        description: task.description.clone(),
        passed: result.passed,
        confidence: result.confidence,
        rationale: result.rationale,
        triad_passed: result.triad.passed,
        aurelius_score: result.triad.aurelius_score,
        bacon_score: result.triad.bacon_score,
        sun_tzu_score: result.triad.sun_tzu_score,
        context,
    };

    append_machine_log(&event, &paths.machine)?;
    append_human_log(&event, &paths.human)?;
    Ok(event)
}

fn append_machine_log(event: &BaconLiteEvent, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

fn append_human_log(event: &BaconLiteEvent, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let is_new = !path.exists();
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    if is_new {
        writeln!(file, "# Bacon-Lite Validation Log")?;
        writeln!(file)?;
    }
    writeln!(
        file,
        "- {} | crate=`{}` action=`{}` passed=`{}` confidence=`{:.3}` task_type=`{}`",
        event.ts_utc,
        event.crate_name,
        event.action,
        event.passed,
        event.confidence,
        event.task_type
    )?;
    Ok(())
}

fn default_log_paths() -> std::io::Result<BaconLiteLogPaths> {
    let base = std::env::var_os("ARDA_ROOT")
        .map(PathBuf::from)
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;
    let mut paths = BaconLiteLogPaths::from_base_dir(base);
    if let Some(path) = std::env::var_os("ARDA_BACON_LITE_LOG_PATH") {
        paths.machine = PathBuf::from(path);
    }
    if let Some(path) = std::env::var_os("ARDA_BACON_LITE_HUMAN_PATH") {
        paths.human = PathBuf::from(path);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_core::Task;

    fn temp_log_path(prefix: &str, ext: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!("{prefix}-{nanos}.{ext}"))
    }

    #[test]
    fn bacon_lite_validate_returns_confidence_and_triads() {
        let task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        let result = bacon_lite_validate(&task);
        assert!(result.confidence >= 0.0);
        assert_eq!(result.mode, "bacon_lite");
    }

    #[test]
    fn record_bacon_lite_writes_machine_and_human_logs() {
        let machine = temp_log_path("bacon-lite-machine", "jsonl");
        let human = temp_log_path("bacon-lite-human", "md");
        let paths = BaconLiteLogPaths {
            machine: machine.clone(),
            human: human.clone(),
        };

        let task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        let event = record_bacon_lite_to(
            "athena",
            "ingest",
            &task,
            serde_json::json!({"source":"example"}),
            &paths,
        )
        .expect("recorded");

        let machine_text = std::fs::read_to_string(&machine).expect("machine log");
        let human_text = std::fs::read_to_string(&human).expect("human log");
        assert!(machine_text.contains(&event.task_id));
        assert!(human_text.contains("Bacon-Lite Validation Log"));

        let _ = std::fs::remove_file(machine);
        let _ = std::fs::remove_file(human);
    }
}
