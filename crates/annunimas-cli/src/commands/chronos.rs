use std::fs;
use std::path::PathBuf;

use annunimas_chronos::build_runtime_snapshot;
use chrono::Utc;
use clap::{Subcommand, ValueEnum};
use serde_json::json;

use crate::annunimas_root;

#[derive(Clone, Debug, ValueEnum)]
pub(crate) enum ChronosStatusFormat {
    Json,
    Compact,
}

#[derive(Subcommand)]
pub(crate) enum ChronosCommands {
    /// Publish and print the local Chronos status projection.
    Status {
        /// Projection output path.
        #[arg(long, default_value = "core/state/chronos_status.json")]
        out: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = ChronosStatusFormat::Json)]
        format: ChronosStatusFormat,
    },
}

pub(crate) fn handle(command: ChronosCommands) -> anyhow::Result<()> {
    match command {
        ChronosCommands::Status { out, format } => {
            let root = annunimas_root();
            let now = Utc::now();
            let runtime = build_runtime_snapshot(&root, now);
            let projection = json!({
                "schema_version": "annunimas.chronos-status.v1",
                "generated_at_utc": now,
                "authority": "annunimas-cli chronos status",
                "source_runtime_schema": runtime.schema_version,
                "source_runtime_mode": runtime.mode,
                "status": runtime.status,
                "mode": "local_runtime_visibility_projection",
                "activation_boundary": {
                    "approved": true,
                    "approval_source": "core/projects/tasks/queue.jsonl",
                    "packet_id": "CHRONOS-P3",
                    "policy": "local status visibility only; no service restart, scheduling mutation, credential use, or external send"
                },
                "capabilities": runtime.capabilities,
                "feed_summary": runtime.feed_summary,
                "audit_runner": runtime.audit_runner,
                "next_integration_steps": runtime.next_integration_steps,
            });
            let out_path = if out.is_absolute() {
                out
            } else {
                root.join(out)
            };
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out_path, serde_json::to_string_pretty(&projection)? + "\n")?;
            match format {
                ChronosStatusFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&projection)?);
                }
                ChronosStatusFormat::Compact => {
                    println!(
                        "Chronos status: {} | feeds: {}/{} present | stale: {} | ready audits: {} | projection: {}",
                        projection["status"].as_str().unwrap_or("unknown"),
                        projection["feed_summary"]["present_count"].as_u64().unwrap_or(0),
                        projection["feed_summary"]["feed_count"].as_u64().unwrap_or(0),
                        projection["feed_summary"]["stale_count"].as_u64().unwrap_or(0),
                        projection["audit_runner"]["ready_task_count"].as_u64().unwrap_or(0),
                        out_path.display()
                    );
                }
            }
        }
    }
    Ok(())
}
