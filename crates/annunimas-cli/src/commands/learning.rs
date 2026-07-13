use anyhow::Context;
use chrono::Utc;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::BufRead;
use std::io::Write;
use std::path::PathBuf;

const LEARNING_STATE_PATH: &str = "core/state/learning_loop_v1.json";
const PROPOSALS_PATH: &str = "data/prometheus/learning_task_proposals.jsonl";
const LIFECYCLE_PATH: &str = "core/state/learning_loop_lifecycle.jsonl";

#[derive(Debug, clap::Subcommand)]
pub enum LearningCommands {
    /// Show autonomous learning loop status from core/state/learning_loop_v1.json
    Status,
    /// Stream learning task proposals from data/prometheus/learning_task_proposals.jsonl
    Proposals,
    /// Inspect the latest learning lifecycle packet from core/state/learning_loop_lifecycle.jsonl
    Lifecycle,
    /// Set or mutate the current learning lifecycle packet phase and confidence
    #[command(name = "set-lifecycle")]
    SetLifecycle {
        /// New lifecycle phase name
        phase: String,
        /// Optional confidence bucket for the phase transition
        confidence: Option<String>,
    },
    /// Mark the latest learning lifecycle packet as approved
    ApproveLifecycle {
        /// Optional lifecycle packet ID
        packet_id: Option<String>,
    },
}

pub fn handle(command: LearningCommands) -> anyhow::Result<()> {
    match command {
        LearningCommands::Status => handle_learning_status(),
        LearningCommands::Proposals => handle_learning_proposals(),
        LearningCommands::Lifecycle => handle_learning_lifecycle(),
        LearningCommands::SetLifecycle { phase, confidence } => {
            handle_set_lifecycle(phase, confidence)
        }
        LearningCommands::ApproveLifecycle { packet_id } => handle_approve_lifecycle(packet_id),
    }
}

#[derive(Serialize)]
struct LearningStatusView {
    current_phase: String,
    completed_phases: Vec<String>,
    pending_phase: Option<String>,
    proposals_pending: usize,
    proposals_adopted: usize,
    source: String,
    updated_at: String,
}

#[derive(Serialize)]
struct LearningProposalsView {
    count: usize,
    proposals: Vec<Value>,
    source: String,
}

#[derive(Serialize)]
struct LearningLifecycleView {
    packet_id: Option<String>,
    phase: String,
    confidence: Option<String>,
    activated_at: Option<String>,
    last_update_at: Option<String>,
    source: String,
}

#[derive(Serialize)]
struct LearningLifecycleSetView {
    packet_id: Option<String>,
    phase: String,
    confidence: Option<String>,
    activated_at: Option<String>,
    updated_at: Option<String>,
    source: String,
}

#[derive(Serialize)]
struct LearningLifecycleApprovalView {
    packet_id: Option<String>,
    approval_id: Option<String>,
    approved: bool,
    source: String,
}

fn learning_state_path() -> PathBuf {
    PathBuf::from(LEARNING_STATE_PATH)
}

fn proposals_path() -> PathBuf {
    PathBuf::from(PROPOSALS_PATH)
}

fn lifecycle_path() -> PathBuf {
    PathBuf::from(LIFECYCLE_PATH)
}

fn parse_learning_state(path: &PathBuf) -> anyhow::Result<Value> {
    let payload =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let state: Value = serde_json::from_str(&payload)
        .with_context(|| format!("invalid learning state JSON in {}", path.display()))?;
    Ok(state)
}

fn load_proposal_lines(path: &PathBuf) -> anyhow::Result<Vec<String>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file =
        fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(&line)
            .with_context(|| format!("invalid proposal JSON in {}", path.display()))?;
        if value.is_array() || value.is_null() {
            continue;
        }
        lines.push(line);
    }
    Ok(lines)
}

fn handle_learning_status() -> anyhow::Result<()> {
    let path = learning_state_path();

    if !path.exists() {
        anyhow::bail!("missing learning state file: {}", path.display());
    }

    let state = parse_learning_state(&path)?;
    let current_phase = state
        .get("current_phase")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let completed_phases = state
        .get("completed_phases")
        .and_then(|v| v.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|entry| entry.as_str().map(|v| v.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let pending_phase = state
        .get("pending_phase")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string());

    let proposals_path = proposals_path();
    let proposals_pending = if proposals_path.exists() {
        std::io::BufReader::new(fs::File::open(&proposals_path)?)
            .lines()
            .flatten()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("status")
                            .and_then(|v| v.as_str().map(|s| s.to_owned()))
                    })
                    .map(|status| status == "pending")
            })
            .count()
    } else {
        0
    };

    let proposals_adopted = if proposals_path.exists() {
        std::io::BufReader::new(fs::File::open(&proposals_path)?)
            .lines()
            .flatten()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| {
                serde_json::from_str::<Value>(&line)
                    .ok()
                    .and_then(|value| {
                        value
                            .get("status")
                            .and_then(|v| v.as_str().map(|s| s.to_owned()))
                    })
                    .map(|status| status == "adopted")
            })
            .count()
    } else {
        0
    };

    let view = LearningStatusView {
        current_phase: current_phase.clone(),
        completed_phases: completed_phases.clone(),
        pending_phase: pending_phase.clone(),
        proposals_pending,
        proposals_adopted,
        source: path.display().to_string(),
        updated_at: Utc::now().to_rfc3339(),
    };

    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

fn handle_learning_proposals() -> anyhow::Result<()> {
    let path = proposals_path();

    if !path.exists() {
        anyhow::bail!("missing learning proposals file: {}", path.display());
    }

    let mut proposals = Vec::new();
    for line in std::io::BufReader::new(fs::File::open(&path)?).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str(&line) {
            Ok(serde_json::Value::Null) | Ok(serde_json::Value::Array(_)) => continue,
            Ok(value) => proposals.push(value),
            Err(err) => eprintln!("skipping proposals line: {err}"),
        }
    }

    let view = LearningProposalsView {
        count: proposals.len(),
        proposals,
        source: path.display().to_string(),
    };

    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

fn handle_learning_lifecycle() -> anyhow::Result<()> {
    let path = lifecycle_path();

    if !path.exists() {
        anyhow::bail!("missing learning lifecycle file: {}", path.display());
    }

    let file = fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    let mut packets: Vec<Value> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str(&line) {
            Ok(Value::Array(items)) if items.is_empty() => continue,
            Ok(value) => packets.push(value),
            Err(err) => eprintln!("sk lifecycle line: {err}"),
        }
    }

    if packets.is_empty() {
        anyhow::bail!("learning lifecycle file has no valid packets");
    }

    let current = packets.last().expect("packets is non-empty");
    let source = path.display().to_string();

    let view = LearningLifecycleView {
        packet_id: current
            .get("packet_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_owned()),
        phase: current
            .get("phase")
            .and_then(|v| v.as_str())
            .map(|v| v.to_owned())
            .unwrap_or_default(),
        confidence: current
            .get("confidence")
            .and_then(|v| v.as_str())
            .map(|v| v.to_owned()),
        activated_at: current
            .get("activated_at")
            .and_then(|v| v.as_str())
            .map(|v| v.to_owned()),
        last_update_at: current
            .get("updated_at")
            .and_then(|v| v.as_str())
            .map(|v| v.to_owned()),
        source,
    };

    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

fn handle_set_lifecycle(phase: String, confidence: Option<String>) -> anyhow::Result<()> {
    let path = lifecycle_path();

    if !path.exists() {
        std::fs::create_dir_all(path.parent().expect("lifecycle path has parent"))?;
        std::fs::write(&path, "[]\n")?;
    }

    let file = fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    let mut packets: Vec<Value> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str(&line) {
            Ok(Value::Array(items)) if items.is_empty() => continue,
            Ok(value) => packets.push(value),
            Err(err) => eprintln!("sk lifecycle line: {err}"),
        }
    }

    let packet_id = format!(
        "{}-{}",
        phase.trim().to_lowercase().replace(' ', "-"),
        Utc::now().timestamp()
    );
    let now = Utc::now().to_rfc3339();

    let mut record = serde_json::Map::new();
    record.insert("packet_id".into(), Value::String(packet_id.clone()));
    record.insert("phase".into(), Value::String(phase.clone()));
    if let Some(value) = confidence.as_ref() {
        record.insert("confidence".into(), Value::String(value.clone()));
    }
    record.insert("activated_at".into(), Value::String(now.clone()));
    record.insert("updated_at".into(), Value::String(now));
    record.insert("meta".into(), Value::Object(serde_json::Map::new()));

    packets.push(Value::Object(record));

    let backing = std::fs::File::create(&path)?;
    let mut writer = std::io::BufWriter::new(backing);

    for packet in packets {
        writeln!(writer, "{}", serde_json::to_string(&packet)?)?;
    }
    writer.flush()?;

    let view = LearningLifecycleSetView {
        packet_id: Some(packet_id),
        phase,
        confidence,
        activated_at: Some(Utc::now().to_rfc3339()),
        updated_at: Some(Utc::now().to_rfc3339()),
        source: path.display().to_string(),
    };

    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}

fn handle_approve_lifecycle(packet_id: Option<String>) -> anyhow::Result<()> {
    let path = lifecycle_path();

    if !path.exists() {
        std::fs::create_dir_all(path.parent().expect("lifecycle path has parent"))?;
        std::fs::write(&path, "[]\n")?;
    }

    let file = fs::File::open(&path)?;
    let reader = std::io::BufReader::new(file);
    let mut packets: Vec<Value> = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }

        match serde_json::from_str(&line) {
            Ok(Value::Array(items)) if items.is_empty() => continue,
            Ok(value) => packets.push(value),
            Err(err) => eprintln!("sk lifecycle line: {err}"),
        }
    }

    if packets.is_empty() {
        anyhow::bail!("learning lifecycle packet log is empty");
    }

    let approval_id = format!("approval-{}", Utc::now().timestamp());

    let target = packets.last_mut().expect("packets is non-empty");

    if let Some(map) = target.as_object_mut() {
        map.insert("approved".into(), Value::Bool(true));
        map.insert("approval_id".into(), Value::String(approval_id.clone()));
    } else {
        anyhow::bail!("invalid lifecycle packet: expected object");
    }

    if let Some(provided) = packet_id {
        if target.get("packet_id").is_none() {
            if let Some(map) = target.as_object_mut() {
                map.insert("packet_id".into(), Value::String(provided));
            }
        }
    }

    let packet_id_value = target
        .get("packet_id")
        .and_then(|v| v.as_str())
        .map(|v| v.to_owned());

    let backing = std::fs::File::create(&path)?;
    let mut writer = std::io::BufWriter::new(backing);

    for packet in packets {
        writeln!(writer, "{}", serde_json::to_string(&packet)?)?;
    }
    writer.flush()?;

    let view = LearningLifecycleApprovalView {
        packet_id: packet_id_value,
        approval_id: Some(approval_id),
        approved: true,
        source: path.display().to_string(),
    };

    println!("{}", serde_json::to_string_pretty(&view)?);
    Ok(())
}
