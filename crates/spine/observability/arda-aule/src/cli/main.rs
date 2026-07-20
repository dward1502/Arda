#![warn(rust_2018_idioms)]
#![recursion_limit = "256"]

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::path::PathBuf;

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
            let value = find_receipt(&registry, &id).unwrap_or_else(|| not_found(&id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernancePolicy { policy_id } => {
            let registry = read_registry()?;
            let value =
                find_receipt(&registry, &policy_id).unwrap_or_else(|| not_found(&policy_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernanceReceipt { receipt_id } => {
            let registry = read_registry()?;
            let value =
                find_receipt(&registry, &receipt_id).unwrap_or_else(|| not_found(&receipt_id));
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
            let out = json!({"contract":"arda.tool.manifest.v1","agent_id":agent_id,"status":"not_found"});
            println!("{}", out);
        }
        Commands::RuntimeReceipt { run_id } => {
            let registry = read_registry()?;
            let value = find_receipt(&registry, &run_id).unwrap_or_else(|| not_found(&run_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::EvalRun { task_id } => {
            let out = json!({"contract":"arda.eval.run.v1","task_id":task_id,"status":"queued"});
            println!("{}", out);
        }
        Commands::LearningDelta { run_id } => {
            let registry = read_registry()?;
            let value = find_receipt(&registry, &run_id).unwrap_or_else(|| not_found(&run_id));
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
                                if matches_id(&value, id) {
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
                            .cloned()
                            .unwrap_or_default();
                        for rec in arr {
                            if matches_id(&rec, id) {
                                return Some(rec);
                            }
                        }
                    }
                }
            }
        }
    }
    None
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

fn matches_id(value: &Value, id: &str) -> bool {
    value
        .get("receipt_id")
        .and_then(|v| v.as_str())
        .or_else(|| value.get("run_id").and_then(|v| v.as_str()))
        .or_else(|| value.get("policy_id").and_then(|v| v.as_str()))
        .map(|v| v == id)
        .unwrap_or(false)
}

fn track_by_id(registry: &Value, track_id: &str) -> Option<Value> {
    let tracks = registry.get("tracks")?.as_array()?;
    let track = tracks
        .iter()
        .find(|t| t.get("track_id").and_then(|v| v.as_str()) == Some(track_id))?;
    Some(track.clone())
}
