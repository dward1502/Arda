use annunimas_fleet::{EdgeHealthMonitor, FleetNode, NodeHealthStatus};

#[test]
fn health_snapshot_creates_state_directory_for_node_telemetry() {
    let dir = tempfile::tempdir().expect("tempdir");
    let monitor = EdgeHealthMonitor::new(dir.path());
    let nodes = vec![FleetNode {
        id: "node-pi5-warden".to_owned(),
        hostname: "warden".to_owned(),
        tailscale_ip: "100.64.0.3".to_owned(),
        ..FleetNode::default()
    }];

    monitor.init_from_fleet(&nodes);

    monitor
        .write_health_snapshot()
        .expect("health snapshot should create its state directory");

    let snapshot_path = dir.path().join("core/state/edge_health.json");
    let snapshot = std::fs::read_to_string(snapshot_path).expect("snapshot should be readable");
    let value: serde_json::Value =
        serde_json::from_str(&snapshot).expect("snapshot should be json");

    assert_eq!(value["schema_version"], "annunimas.edge-health.v1");
    assert_eq!(value["nodes"].as_array().map(Vec::len), Some(1));
    assert_eq!(value["unreachable_count"], 1);
    assert_eq!(
        monitor.get_all_health()[0].status,
        NodeHealthStatus::Unreachable
    );
}
