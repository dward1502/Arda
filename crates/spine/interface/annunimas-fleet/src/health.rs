use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    pub node_id: String,
    pub hostname: String,
    pub tailscale_ip: String,
    pub online: bool,
    pub last_check: String,
    pub response_time_ms: Option<u64>,
    pub consecutive_failures: u32,
    pub status: NodeHealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeHealthStatus {
    Healthy,
    Degraded,
    Unreachable,
}

pub struct EdgeHealthMonitor {
    nodes: RwLock<HashMap<String, NodeHealth>>,
    state_path: String,
}

impl Default for EdgeHealthMonitor {
    fn default() -> Self {
        Self::new(".")
    }
}

impl EdgeHealthMonitor {
    pub fn new(config_root: impl AsRef<Path>) -> Self {
        let state_path = config_root
            .as_ref()
            .join("core/state")
            .to_string_lossy()
            .into_owned();

        Self {
            nodes: RwLock::new(HashMap::new()),
            state_path,
        }
    }

    pub fn init_from_fleet(&self, fleet_nodes: &[crate::FleetNode]) {
        if let Ok(mut nodes) = self.nodes.write() {
            for node in fleet_nodes {
                let health = NodeHealth {
                    node_id: node.id.clone(),
                    hostname: node.hostname.clone(),
                    tailscale_ip: node.tailscale_ip.clone(),
                    online: false,
                    last_check: chrono::Utc::now().to_rfc3339(),
                    response_time_ms: None,
                    consecutive_failures: 0,
                    status: NodeHealthStatus::Unreachable,
                };
                nodes.insert(node.id.clone(), health);
            }
        }
    }

    pub async fn check_node(&self, node_id: &str) -> Option<NodeHealth> {
        let ip = {
            let nodes = self.nodes.read().ok()?;
            nodes.get(node_id)?.tailscale_ip.clone()
        };

        let start = Instant::now();

        // Try SSH connection test
        let result = tokio::process::Command::new("ssh")
            .args([
                "-o",
                "StrictHostKeyChecking=no",
                "-o",
                "BatchMode=yes",
                "-o",
                "ConnectTimeout=5",
                &format!("citadel@{}", ip),
                "echo ok",
            ])
            .output()
            .await;

        let response_time = start.elapsed().as_millis() as u64;

        let (online, status, failures) = match result {
            Ok(output) if output.status.success() => (true, NodeHealthStatus::Healthy, 0),
            Ok(_) => (true, NodeHealthStatus::Degraded, 0),
            Err(_) => (false, NodeHealthStatus::Unreachable, 1),
        };

        let health = NodeHealth {
            node_id: node_id.to_owned(),
            hostname: {
                let nodes = self.nodes.read().ok()?;
                nodes
                    .get(node_id)
                    .map(|n| n.hostname.clone())
                    .unwrap_or_default()
            },
            tailscale_ip: ip,
            online,
            last_check: chrono::Utc::now().to_rfc3339(),
            response_time_ms: Some(response_time),
            consecutive_failures: failures,
            status,
        };

        if let Ok(mut nodes) = self.nodes.write() {
            if let Some(existing) = nodes.get_mut(node_id) {
                existing.online = health.online;
                existing.last_check = health.last_check.clone();
                existing.response_time_ms = health.response_time_ms;
                existing.status = health.status.clone();
                if !online {
                    existing.consecutive_failures += 1;
                } else {
                    existing.consecutive_failures = 0;
                }
            }
        }

        Some(health)
    }

    pub async fn check_all_nodes(&self) -> Vec<NodeHealth> {
        let node_ids: Vec<String> = {
            let nodes = match self.nodes.read() {
                Ok(n) => n,
                Err(_) => return Vec::new(),
            };
            nodes.keys().cloned().collect()
        };

        let mut results = Vec::new();
        for node_id in node_ids {
            if let Some(health) = self.check_node(&node_id).await {
                results.push(health);
            }
        }

        results
    }

    pub fn get_healthy_nodes(&self) -> Vec<String> {
        self.nodes
            .read()
            .ok()
            .map(|nodes| {
                nodes
                    .values()
                    .filter(|n| n.status == NodeHealthStatus::Healthy)
                    .map(|n| n.node_id.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn get_all_health(&self) -> Vec<NodeHealth> {
        self.nodes
            .read()
            .ok()
            .map(|nodes| nodes.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn write_health_snapshot(&self) -> anyhow::Result<()> {
        let health = self.get_all_health();
        let snapshot = serde_json::json!({
            "schema_version": "annunimas.edge-health.v1",
            "generated_at_utc": chrono::Utc::now().to_rfc3339(),
            "nodes": health,
            "healthy_count": health.iter().filter(|n| n.status == NodeHealthStatus::Healthy).count(),
            "degraded_count": health.iter().filter(|n| n.status == NodeHealthStatus::Degraded).count(),
            "unreachable_count": health.iter().filter(|n| n.status == NodeHealthStatus::Unreachable).count(),
        });

        std::fs::create_dir_all(&self.state_path)?;
        let path = format!("{}/edge_health.json", self.state_path);
        std::fs::write(&path, serde_json::to_string_pretty(&snapshot)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_from_fleet_and_snapshot_reflects_health_counts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let state_dir = dir.path().join("core/state");
        std::fs::create_dir_all(&state_dir).expect("state dir");

        let monitor = EdgeHealthMonitor::new(dir.path());
        let nodes = vec![
            crate::FleetNode {
                id: "node-ser9-worker".to_owned(),
                hostname: "beelink".to_owned(),
                tailscale_ip: "100.64.0.2".to_owned(),
                ..crate::FleetNode::default()
            },
            crate::FleetNode {
                id: "node-pi5-warden".to_owned(),
                hostname: "warden".to_owned(),
                tailscale_ip: "100.64.0.3".to_owned(),
                ..crate::FleetNode::default()
            },
        ];

        monitor.init_from_fleet(&nodes);
        assert_eq!(monitor.get_all_health().len(), 2);
        assert!(monitor.get_healthy_nodes().is_empty());

        monitor.write_health_snapshot().expect("snapshot");
        let snapshot = std::fs::read_to_string(state_dir.join("edge_health.json")).expect("read");
        let value: serde_json::Value = serde_json::from_str(&snapshot).expect("json");

        assert_eq!(value["nodes"].as_array().map(Vec::len), Some(2));
        assert_eq!(value["unreachable_count"], 2);
        assert_eq!(value["healthy_count"], 0);
    }
}
