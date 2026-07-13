// sigil: REPAIR
use crate::triad::{triad_validate, GateOutcome, TriadConfig, TriadResult};
use annunimas_core::task::Task;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

fn annunimas_root() -> PathBuf {
    if let Ok(path) = std::env::var("ANNUNIMAS_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
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

    append_machine_log(&event)?;
    append_human_log(&event)?;
    Ok(event)
}

fn append_machine_log(event: &BaconLiteEvent) -> std::io::Result<()> {
    let path = machine_log_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(event)?)?;
    Ok(())
}

fn append_human_log(event: &BaconLiteEvent) -> std::io::Result<()> {
    let path = human_log_path();
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

fn machine_log_path() -> PathBuf {
    std::env::var("ANNUNIMAS_BACON_LITE_LOG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| annunimas_root().join("data/governance/bacon_lite.jsonl"))
}

fn human_log_path() -> PathBuf {
    std::env::var("ANNUNIMAS_BACON_LITE_HUMAN_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| annunimas_root().join("human/library/governance/bacon_lite.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use annunimas_core::Task;

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
        std::env::set_var("ANNUNIMAS_BACON_LITE_LOG_PATH", &machine);
        std::env::set_var("ANNUNIMAS_BACON_LITE_HUMAN_PATH", &human);

        let task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        let event = record_bacon_lite(
            "athena",
            "ingest",
            &task,
            serde_json::json!({"source":"example"}),
        )
        .expect("recorded");

        let machine_text = std::fs::read_to_string(&machine).expect("machine log");
        let human_text = std::fs::read_to_string(&human).expect("human log");
        assert!(machine_text.contains(&event.task_id));
        assert!(human_text.contains("Bacon-Lite Validation Log"));

        let _ = std::fs::remove_file(machine);
        let _ = std::fs::remove_file(human);
        std::env::remove_var("ANNUNIMAS_BACON_LITE_LOG_PATH");
        std::env::remove_var("ANNUNIMAS_BACON_LITE_HUMAN_PATH");
    }
}
