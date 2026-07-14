use annunimas_chronos::{build_runtime_snapshot, execute_scheduled_audit_tasks, ChronosAgent};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let root = std::env::var("ANNUNIMAS_ROOT")
        .map(PathBuf::from)
        .unwrap_or(std::env::current_dir()?);

    let agent = ChronosAgent::new();
    agent
        .initialize()
        .map_err(|err| anyhow::anyhow!("chronos initialization failed: {err}"))?;

    let audit_run = execute_scheduled_audit_tasks(&root, Utc::now())?;
    let snapshot = serde_json::to_value(build_runtime_snapshot(&root, Utc::now()))?;

    write_runtime_snapshot(&root, &snapshot)?;
    println!(
        "Chronos scheduled audit receipts written: {} to {}",
        audit_run.written_receipt_count,
        root.join(&audit_run.receipt_path).display()
    );
    println!(
        "Chronos runtime snapshot written: {}",
        root.join("core/state/chronos_runtime.json").display()
    );

    Ok(())
}

fn write_runtime_snapshot(root: &Path, snapshot: &serde_json::Value) -> anyhow::Result<()> {
    let path = root.join("core/state/chronos_runtime.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(snapshot)? + "\n")?;
    Ok(())
}
