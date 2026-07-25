#![cfg(feature = "full-cli")]

use serde_json::Value;
use std::process::Command;

fn arda_cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_arda-cli"))
}

#[test]
fn governance_metrics_command_emits_machine_readable_contract() {
    let output = arda_cli()
        .args(["governance-metrics", "--json"])
        .output()
        .expect("run governance metrics command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let snapshot: Value = serde_json::from_slice(&output.stdout).expect("metrics JSON");
    assert!(snapshot.get("collection_mode").is_some());
    assert_eq!(snapshot["owns_http_server"], false);
    assert!(snapshot["counters"].is_array());
    assert!(snapshot["histograms"].is_array());
}

#[test]
fn governance_status_command_reports_conservative_readiness() {
    let temp = tempfile::tempdir().expect("tempdir");
    let ledger = temp.path().join("bacon_lite.jsonl");
    std::fs::write(&ledger, "").expect("empty ledger");

    let output = arda_cli()
        .arg("governance-status")
        .arg("--path")
        .arg(&ledger)
        .arg("--json")
        .output()
        .expect("run governance status command");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout).expect("status JSON");
    assert!(report.get("readiness").is_some());
    assert!(report.get("recent_ledger").is_some());
    assert!(report.get("metrics").is_some());
}
