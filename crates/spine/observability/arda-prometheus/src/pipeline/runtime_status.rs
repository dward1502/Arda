use crate::pipeline::Pipeline;
use arda_core::message::Message;

impl Pipeline {
    pub async fn run_fleet_health_check(&self) {
        if let Some(ref monitor) = self.health_monitor {
            if let Some(ref fleet) = self.fleet_manager {
                let nodes: Vec<arda_fleet::FleetNode> =
                    fleet.get_all_nodes().into_iter().cloned().collect();
                monitor.init_from_fleet(&nodes);
            }

            let results = monitor.check_all_nodes().await;

            for health in &results {
                self.ledger
                    .append(&Message::event(
                        "fleet",
                        "node_health",
                        serde_json::to_value(health).unwrap_or_default(),
                    ))
                    .ok();
            }

            if let Err(err) = monitor.write_health_snapshot() {
                tracing::debug!(error = %err, "failed to write fleet health snapshot");
            }

            tracing::info!(
                healthy = results
                    .iter()
                    .filter(|h| h.status == arda_fleet::NodeHealthStatus::Healthy)
                    .count(),
                total = results.len(),
                "fleet health check complete"
            );
        }
    }
}
