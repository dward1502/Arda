#![warn(rust_2018_idioms)]
#![recursion_limit = "256"]

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "arda-cli", about = "Arda observability CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    TelemetrySchema,
    Receipt { id: String },
    GovernancePolicy { policy_id: String },
    GovernanceReceipt { receipt_id: String },
    ServiceGraph,
    ToolManifest { agent_id: String },
    RuntimeReceipt { run_id: String },
    EvalRun { task_id: String },
    LearningDelta { run_id: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::TelemetrySchema => {
            let registry = read_registry()?;
            let track = track_by_id(&registry, "arda-ecosystem-standard-track-1-observability")
                .unwrap_or(registry);
            println!("{}", serde_json::to_string_pretty(&track)?);
        }
        Commands::Receipt { id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["receipt_id", "run_id", "policy_id", "label"], &id)
                .unwrap_or_else(|| not_found(&id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernancePolicy { policy_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["policy_id", "receipt_id", "label"], &policy_id)
                .unwrap_or_else(|| not_found(&policy_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernanceReceipt { receipt_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["receipt_id", "policy_id", "label"], &receipt_id)
                .unwrap_or_else(|| not_found(&receipt_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::ServiceGraph => {
            let registry = read_registry()?;
            let track = track_by_id(
                &registry,
                "arda-ecosystem-standard-track-3-agent-runtime-tooling",
            )
            .unwrap_or(registry);
            println!("{}", serde_json::to_string_pretty(&track)?);
        }
        Commands::ToolManifest { agent_id } => {
            let record = resolve_tool_manifest(&agent_id);
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Commands::RuntimeReceipt { run_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["receipt_id", "run_id", "label"], &run_id)
                .unwrap_or_else(|| not_found(&run_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::EvalRun { task_id } => {
            let record = load_eval_run_record(&task_id);
            println!("{}", serde_json::to_string_pretty(&record)?);
        }
        Commands::LearningDelta { run_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(&registry, &["run_id", "receipt_id", "label"], &run_id)
                .unwrap_or_else(|| not_found(&run_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
    }
    Ok(())
}

fn read_registry() -> Result<Value> {
    let path = candidate_paths()
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("missing registry"))?;
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn find_receipt(registry: &Value, id: &str) -> Option<Value> {
    union_find_receipt(registry, &["receipt_id", "run_id", "policy_id"], id)
}

fn union_find_receipt(
    registry: &Value,
    keys: &[&str],
    id: &str,
) -> Option<Value> {
    let tracks = registry.get("tracks")?.as_array()?;
    for track in tracks {
        let stores = track.get("receipt_stores")?.as_array()?;
        for store in stores {
            let base = store.as_str()?.trim_end_matches('/');
            let path = PathBuf::from(base);
            if !path.exists() {
                continue;
            }
            let candidates = if path.is_dir() {
                std::fs::read_dir(&path)
                    .ok()?
                    .filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .collect::<Vec<_>>()
            } else {
                vec![path]
            };
            for candidate in candidates {
                if candidate.extension().map(|e| e == "jsonl").unwrap_or(false) {
                    if let Ok(raw) = std::fs::read_to_string(&candidate) {
                        for line in raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                            if let Ok(value) = serde_json::from_str::<Value>(line) {
                                if id_matches_any(&value, id, keys) {
                                    return Some(value);
                                }
                            }
                        }
                    }
                    continue;
                }
                if let Ok(raw) = std::fs::read_to_string(&candidate) {
                    if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                        let arr = data
                            .as_array()
                            .or_else(|| data.get("receipts").and_then(|v| v.as_array()))
                            .or_else(|| data.get("recent_receipts").and_then(|v| v.as_array()))
                            .cloned()
                            .unwrap_or_default();
                        for rec in arr {
                            if id_matches_any(&rec, id, keys) {
                                return Some(rec);
                            }
                        }
                    }
                }
            }
        }
    }

    if let Some(value) = find_receipt_in_known_backing_stores(id, keys) {
        return Some(value);
    }
    None
}

fn find_receipt_in_known_backing_stores(id: &str, keys: &[&str]) -> Option<Value> {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let candidates = [
        root.join("core/state/runtime_admission_receipts.json"),
        root.join("data/prometheus/runtime_admission_shed_receipts.jsonl"),
    ];

    for candidate in candidates {
        if !candidate.exists() {
            continue;
        }
        if candidate.extension().map(|e| e == "jsonl").unwrap_or(false) {
            if let Ok(raw) = std::fs::read_to_string(&candidate) {
                for line in raw.lines().map(|l| l.trim()).filter(|l| !l.is_empty()) {
                    if let Ok(value) = serde_json::from_str::<Value>(line) {
                        if id_matches_any(&value, id, keys) {
                            return Some(value);
                        }
                    }
                }
            }
            continue;
        }
        if let Ok(raw) = std::fs::read_to_string(&candidate) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                let arr = data
                    .as_array()
                    .or_else(|| data.get("receipts").and_then(|v| v.as_array()))
                    .or_else(|| data.get("recent_receipts").and_then(|v| v.as_array()))
                    .cloned()
                    .unwrap_or_default();
                for rec in arr {
                    if id_matches_any(&rec, id, keys) {
                        return Some(rec);
                    }
                }
            }
        }
    }

    None
}

fn load_eval_run_record(task_id: &str) -> Value {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let candidate_paths = [
        root.join("core/state/queue_summary.json"),
        root.join("core/state/project_task_executor.json"),
        root.join("core/state/queue_active.json"),
    ];

    if let Some(path) = candidate_paths.iter().find(|p| p.exists()) {
        if let Ok(raw) = std::fs::read_to_string(path) {
            if let Ok(data) = serde_json::from_str::<Value>(&raw) {
                if let Some(entry) = find_task_id(&data, task_id) {
                    return json!({
                        "contract": "arda.eval.run.v1",
                        "task_id": task_id,
                        "status": "found",
                        "source": path.to_string_lossy().to_string(),
                        "record": entry,
                    });
                }
            }
        }
    }

    json!({
        "contract": "arda.eval.run.v1",
        "task_id": task_id,
        "status": "queued",
        "note": "no persistent eval record found yet",
    })
}

fn find_task_id(data: &Value, task_id: &str) -> Option<Value> {
    find_task_id_by_keys(data, &["task_id", "id"], task_id)
}

fn find_task_id_by_keys(data: &Value, keys: &[&str], task_id: &str) -> Option<Value> {
    if let Some(map) = data.as_object() {
        for key in keys {
            if map.get(*key).and_then(|v| v.as_str()) == Some(task_id) {
                return Some(data.clone());
            }
        }
        for (_, child) in map {
            if let Some(found) = find_task_id_by_keys(child, keys, task_id) {
                return Some(found);
            }
        }
    } else if let Some(arr) = data.as_array() {
        for item in arr {
            if let Some(found) = find_task_id_by_keys(item, keys, task_id) {
                return Some(found);
            }
        }
    }
    None
}

fn resolve_tool_manifest(agent_id: &str) -> Value {
    let root = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let personalities_path = root.join("core/state/agent_personalities.json");
    let framework_path = root.join("core/state/agent_framework_alignment.json");

    let personalities = read_json_file(&personalities_path);
    let framework = read_json_file(&framework_path);

    let mut record = json!({
        "contract": "arda.tool.manifest.v1",
        "agent_id": agent_id,
        "status": "not_found",
    });

    if let Some(profile) = personalities
        .as_object()
        .and_then(|obj| obj.get(agent_id))
    {
        record = json!({
            "contract": "arda.tool.manifest.v1",
            "agent_id": agent_id,
            "status": "found",
            "personality": profile,
            "framework_alignment": framework,
        });
    } else if !framework.is_null() {
        record = json!({
            "contract": "arda.tool.manifest.v1",
            "agent_id": agent_id,
            "status": "partial",
            "framework_alignment": framework,
            "note": "personality record missing; framework alignment present",
        });
    }

    record
}

fn read_json_file(path: &Path) -> Value {
    match std::fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn not_found(id: &str) -> Value {
    json!({"contract":"arda.registry.not_found.v1","id":id,"status":"not_found"})
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let from = std::env::var("ARDA_ROOT")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    out.push(from.join("core/state/contract_registry.json"));
    out
}

fn id_matches_any(value: &Value, id: &str, keys: &[&str]) -> bool {
    for key in keys {
        if value
            .get(*key)
            .and_then(|v| v.as_str())
            .map(|v| v == id)
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn track_by_id(registry: &Value, track_id: &str) -> Option<Value> {
    let tracks = registry.get("tracks")?.as_array()?;
    let track = tracks
        .iter()
        .find(|t| t.get("track_id").and_then(|v| v.as_str()) == Some(track_id))?;
    Some(track.clone())
}
