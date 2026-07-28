use std::fs;
use std::process::Command;

#[test]
fn metrics_snapshot_reads_explicit_arda_root_and_emits_prometheus_text() {
    let temp = tempfile::tempdir().expect("temp root");
    let root = temp.path();
    fs::create_dir_all(root.join("core/state")).expect("state dir");
    fs::create_dir_all(root.join("core/metrics/by_crate/prometheus")).expect("metrics dir");
    fs::create_dir_all(root.join("core/metrics/by_crate/mnemosyne")).expect("memory metrics dir");
    fs::write(
        root.join("core/state/autonomy_runtime.json"),
        r#"{"mode":"normal","violations":[]}"#,
    )
    .expect("autonomy projection");
    fs::write(
        root.join("core/metrics/by_crate/prometheus/queue_observability.json"),
        r#"{"breakdown":{"core_queue":{"pending_records":3}},"summary":{"total_active_internal_tasks":3}}"#,
    )
    .expect("queue projection");
    fs::write(
        root.join("core/state/runtime_admission_pressure.json"),
        r#"{"status":"ok","violations":[],"observed":{"storage_pressure":{"oversize_files_gte_100mb":7}}}"#,
    )
    .expect("canonical pressure projection");
    fs::write(
        root.join("core/metrics/by_crate/mnemosyne/observability.json"),
        r#"{"schema_version":"arda.mnemosyne.observability.v1","metrics":{"recall_requests_total":9,"recall_results_total":21,"last_recall_fidelity":0.875,"last_recall_latency_ms":14,"queue_observations_total":7,"last_queue_latency_ms":3,"last_consolidation_depth":18,"promotion_receipts_total":6}}"#,
    )
    .expect("mnemosyne observability");

    let output = Command::new(env!("CARGO_BIN_EXE_arda-cli"))
        .args(["metrics", "snapshot", "--root"])
        .arg(root)
        .output()
        .expect("run arda metrics snapshot");

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8 metrics");
    assert!(stdout.contains("annunimas_metrics_exporter_refresh_success 1"));
    assert!(stdout.contains("annunimas_queue_depth{queue=\"core_queue\"} 3"));
    assert!(stdout.contains("annunimas_pressure_guard_status{status=\"ok\"} 1"));
    assert!(stdout.contains("annunimas_pressure_guard_oversize_files_total 7"));
    assert!(stdout
        .contains("annunimas_audit_health_status{status=\"ok\",surface=\"pressure_guard\"} 1"));
    assert!(stdout.contains("arda_mnemosyne_events_total{signal=\"recall_requests\"} 9"));
    assert!(stdout.contains("arda_mnemosyne_events_total{signal=\"promotion_receipts\"} 6"));
    assert!(stdout.contains("arda_mnemosyne_latency_milliseconds{operation=\"recall\"} 14"));
    assert!(stdout.contains("arda_mnemosyne_latency_milliseconds{operation=\"queue\"} 3"));
    assert!(stdout.contains("arda_mnemosyne_recall_fidelity 0.875"));
    assert!(stdout.contains("arda_mnemosyne_consolidation_depth 18"));
}
