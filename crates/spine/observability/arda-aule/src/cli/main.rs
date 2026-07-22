#![warn(rust_2018_idioms)]
#![recursion_limit = "256"]

use anyhow::Result;
use arda_governance::{
    build_governance_status_report, default_governance_readiness_report, global_governance_metrics,
    read_bacon_lite_summary, read_latest_bacon_lite_event, render_governance_status_human,
    BaconLiteLogPaths, BaconLiteReadWindow, MalformedLineBehavior,
};
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
    Receipt {
        id: String,
    },
    GovernancePolicy {
        policy_id: String,
    },
    GovernanceReceipt {
        receipt_id: String,
    },
    ServiceGraph,
    ToolManifest {
        agent_id: String,
    },
    RuntimeReceipt {
        run_id: String,
    },
    EvalRun {
        task_id: String,
    },
    LearningDelta {
        run_id: String,
    },
    /// Inspect persisted Plutus economics state.
    Plutus {
        #[command(subcommand)]
        command: PlutusCommands,
    },
    /// Summarize the durable Bacon-Lite governance ledger.
    BaconLiteSummary {
        /// Override the machine JSONL ledger path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Inclusive RFC 3339 lower bound.
        #[arg(long)]
        since: Option<String>,
        /// Inclusive RFC 3339 upper bound.
        #[arg(long)]
        until: Option<String>,
        /// Fail rather than count and skip malformed JSONL records.
        #[arg(long)]
        strict_malformed: bool,
        /// Emit the complete machine-readable summary as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Expose the in-process governance metric snapshot for scraping.
    GovernanceMetrics {
        /// Emit structured JSON instead of Prometheus text exposition.
        #[arg(long)]
        json: bool,
    },
    /// Join readiness, recent ledger evidence, and metrics without claiming autonomy.
    GovernanceStatus {
        /// Override the machine JSONL ledger path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Inclusive RFC 3339 lower bound for recent ledger aggregation.
        #[arg(long)]
        since: Option<String>,
        /// Inclusive RFC 3339 upper bound for recent ledger aggregation.
        #[arg(long)]
        until: Option<String>,
        /// Fail rather than count and skip malformed JSONL records.
        #[arg(long)]
        strict_malformed: bool,
        /// Emit the complete machine-readable report as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum PlutusCommands {
    /// Export the current economics, JouleWork, ledger, and governance state.
    Export {
        /// Override the runtime_status.json path.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Emit the complete machine-readable export envelope.
        #[arg(long)]
        json: bool,
    },
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
            let value = union_find_receipt(
                &registry,
                &["receipt_id", "run_id", "policy_id", "label"],
                &id,
            )
            .unwrap_or_else(|| not_found(&id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernancePolicy { policy_id } => {
            let registry = read_registry()?;
            let value =
                union_find_receipt(&registry, &["policy_id", "receipt_id", "label"], &policy_id)
                    .unwrap_or_else(|| not_found(&policy_id));
            println!("{}", serde_json::to_string_pretty(&value)?);
        }
        Commands::GovernanceReceipt { receipt_id } => {
            let registry = read_registry()?;
            let value = union_find_receipt(
                &registry,
                &["receipt_id", "policy_id", "label"],
                &receipt_id,
            )
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
        Commands::Plutus {
            command: PlutusCommands::Export { path, json },
        } => {
            let export = load_plutus_export(path)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&export)?);
            } else {
                print_plutus_export(&export);
            }
        }
        Commands::BaconLiteSummary {
            path,
            since,
            until,
            strict_malformed,
            json,
        } => {
            let root = std::env::var_os("ARDA_ROOT")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            let machine_path = path
                .or_else(|| std::env::var_os("ARDA_BACON_LITE_LOG_PATH").map(PathBuf::from))
                .unwrap_or_else(|| BaconLiteLogPaths::from_base_dir(root).machine);
            let parse_bound = |name: &str, value: Option<String>| -> Result<_> {
                value
                    .map(|value| {
                        chrono::DateTime::parse_from_rfc3339(&value)
                            .map(|date| date.with_timezone(&chrono::Utc))
                            .map_err(|error| anyhow::anyhow!("invalid --{name} timestamp: {error}"))
                    })
                    .transpose()
            };
            let window = BaconLiteReadWindow {
                since: parse_bound("since", since)?,
                until: parse_bound("until", until)?,
                malformed: if strict_malformed {
                    MalformedLineBehavior::Fail
                } else {
                    MalformedLineBehavior::CountAndSkip
                },
                include_rotated: true,
            };
            let summary = read_bacon_lite_summary(&machine_path, &window)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                println!(
                    "Bacon-Lite ledger: {} records, {} malformed",
                    summary.records, summary.malformed_records
                );
                for (crate_name, actions) in &summary.groups {
                    for (action, aggregate) in actions {
                        println!(
                            "- {crate_name}/{action}: count={} pass_rate={:.1}% mean_confidence={:.3} scorers={}",
                            aggregate.record_count,
                            aggregate.pass_rate * 100.0,
                            aggregate.mean_confidence,
                            aggregate
                                .scorer_versions
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(",")
                        );
                    }
                }
            }
        }
        Commands::GovernanceMetrics { json } => {
            let snapshot = global_governance_metrics().snapshot();
            if json {
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            } else {
                print!("{}", arda_aule::render_governance_prometheus(&snapshot));
            }
        }
        Commands::GovernanceStatus {
            path,
            since,
            until,
            strict_malformed,
            json,
        } => {
            let root = std::env::var_os("ARDA_ROOT")
                .map(PathBuf::from)
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;
            let machine_path = path
                .or_else(|| std::env::var_os("ARDA_BACON_LITE_LOG_PATH").map(PathBuf::from))
                .unwrap_or_else(|| BaconLiteLogPaths::from_base_dir(root).machine);
            let parse_bound = |name: &str, value: Option<String>| -> Result<_> {
                value
                    .map(|value| {
                        chrono::DateTime::parse_from_rfc3339(&value)
                            .map(|date| date.with_timezone(&chrono::Utc))
                            .map_err(|error| anyhow::anyhow!("invalid --{name} timestamp: {error}"))
                    })
                    .transpose()
            };
            let window = BaconLiteReadWindow {
                since: parse_bound("since", since)?,
                until: parse_bound("until", until)?,
                malformed: if strict_malformed {
                    MalformedLineBehavior::Fail
                } else {
                    MalformedLineBehavior::CountAndSkip
                },
                include_rotated: true,
            };
            let recent_ledger = read_bacon_lite_summary(&machine_path, &window)?;
            let latest_event = read_latest_bacon_lite_event(&machine_path, true)?;
            let report = build_governance_status_report(
                default_governance_readiness_report(),
                recent_ledger,
                global_governance_metrics().snapshot(),
                latest_event,
            );
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", render_governance_status_human(&report));
            }
        }
    }
    Ok(())
}

fn load_plutus_export(path: Option<PathBuf>) -> Result<Value> {
    let path = match path {
        Some(path) => path,
        None => {
            let home = std::env::var_os("ARDA_PLUTUS_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    std::env::var_os("ARDA_ROOT")
                        .map(PathBuf::from)
                        .unwrap_or_else(|| {
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                        })
                        .join("data/plutus")
                });
            home.join("runtime_status.json")
        }
    };
    let events_path = path.with_file_name("runtime_events.jsonl");
    let events_total = std::fs::read_to_string(&events_path)
        .map(|raw| raw.lines().filter(|line| !line.trim().is_empty()).count())
        .unwrap_or_default();

    if !path.exists() {
        return Ok(json!({
            "contract": "arda.plutus.export.v1",
            "found": false,
            "missing": true,
            "path": path,
            "events_path": events_path,
            "events_total": events_total,
        }));
    }

    let raw = std::fs::read_to_string(&path)?;
    let snapshot: Value = serde_json::from_str(&raw)
        .map_err(|error| anyhow::anyhow!("invalid Plutus snapshot {}: {error}", path.display()))?;
    Ok(json!({
        "contract": "arda.plutus.export.v1",
        "found": true,
        "missing": false,
        "path": path,
        "events_path": events_path,
        "events_total": events_total,
        "snapshot": snapshot,
    }))
}

fn print_plutus_export(export: &Value) {
    let path = export["path"].as_str().unwrap_or("unknown");
    if export["found"] != true {
        println!("Plutus state: not initialized");
        println!("- expected snapshot: {path}");
        return;
    }

    let snapshot = &export["snapshot"];
    println!("Plutus economics: {path}");
    println!(
        "- budget: spent={:.3} remaining={:.3} usage={:.1}% alert={}",
        snapshot["economics"]["total_spend"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_remaining"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_usage_percent"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["economics"]["budget_alert"]
            .as_str()
            .unwrap_or("none")
    );
    println!(
        "- providers={} accounts={} governance_records={} append_only_events={}",
        snapshot["economics"]["providers"]
            .as_array()
            .map(Vec::len)
            .unwrap_or_default(),
        snapshot["ledger"]["accounts_total"]
            .as_u64()
            .unwrap_or_default(),
        snapshot["governance"]["records_total"]
            .as_u64()
            .unwrap_or_default(),
        export["events_total"].as_u64().unwrap_or_default(),
    );
    println!(
        "- joulework_total={:.3} relationships={}",
        snapshot["joulework"]["total_joulework"]
            .as_f64()
            .unwrap_or_default(),
        snapshot["love_equation"]["relationships_total"]
            .as_u64()
            .unwrap_or_default(),
    );
}

fn read_registry() -> Result<Value> {
    let path = candidate_paths()
        .into_iter()
        .find(|p| p.exists())
        .ok_or_else(|| anyhow::anyhow!("missing registry"))?;
    let raw = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&raw)?)
}

fn union_find_receipt(registry: &Value, keys: &[&str], id: &str) -> Option<Value> {
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

    if let Some(profile) = personalities.as_object().and_then(|obj| obj.get(agent_id)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plutus_export_command() {
        let cli = Cli::try_parse_from(["arda-cli", "plutus", "export", "--json"])
            .expect("parse plutus export");
        assert!(matches!(
            cli.command,
            Commands::Plutus {
                command: PlutusCommands::Export { json: true, .. }
            }
        ));
    }

    #[test]
    fn plutus_export_reads_snapshot_and_event_count() {
        let temp = tempfile::tempdir().expect("tempdir");
        let status_path = temp.path().join("runtime_status.json");
        std::fs::write(
            &status_path,
            serde_json::to_vec(&json!({
                "schema_version": "arda.plutus.runtime.v2",
                "economics": {"total_spend": 2.0},
            }))
            .expect("snapshot json"),
        )
        .expect("snapshot");
        std::fs::write(
            temp.path().join("runtime_events.jsonl"),
            "{\"action\":\"one\"}\n{\"action\":\"two\"}\n",
        )
        .expect("events");

        let export = load_plutus_export(Some(status_path)).expect("export");
        assert_eq!(export["contract"], "arda.plutus.export.v1");
        assert_eq!(export["found"], true);
        assert_eq!(export["events_total"], 2);
        assert_eq!(export["snapshot"]["economics"]["total_spend"], 2.0);
    }

    #[test]
    fn plutus_export_reports_missing_state_without_error() {
        let temp = tempfile::tempdir().expect("tempdir");
        let export = load_plutus_export(Some(temp.path().join("missing.json"))).expect("export");
        assert_eq!(export["found"], false);
        assert_eq!(export["missing"], true);
    }
}
