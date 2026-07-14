use arda_core::Task;
use arda_governance::{
    bacon_lite_validate, love_equation_score, profile_joulework, TriadResult,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

pub mod dispatch;
pub mod health;
pub mod providers;

pub use dispatch::{DispatchResult, EdgeDispatcher};
pub use health::{EdgeHealthMonitor, NodeHealth, NodeHealthStatus};
pub use providers::{ProviderTokenTracker, ProviderTokenUsageSnapshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetNode {
    pub id: String,
    pub role: String,
    pub hostname: String,
    pub tailscale_ip: String,
    pub node_class: String,
    pub enrollment_status: String,
    pub llm_runtime: String,
    pub execution_authority: String,
    pub scout_capacity: u32,
    pub cpu_cores: u32,
    pub ram_mb: u32,
    pub gpu: bool,
    pub gpu_model: Option<String>,
    pub system_ram_gib: u32,
    pub reliability_score: f64,
    pub network_tier: String,
    pub strengths: Vec<String>,
    pub constraints: Vec<String>,
    pub best_for: Vec<String>,
    pub online: bool,
}

impl Default for FleetNode {
    fn default() -> Self {
        Self {
            id: String::new(),
            role: String::new(),
            hostname: String::new(),
            tailscale_ip: String::new(),
            node_class: "edge_worker".to_string(),
            enrollment_status: "active".to_string(),
            llm_runtime: "local_inference_worker".to_string(),
            execution_authority: "secondary".to_string(),
            scout_capacity: 2,
            cpu_cores: 4,
            ram_mb: 4096,
            gpu: false,
            gpu_model: None,
            system_ram_gib: 4,
            reliability_score: 0.85,
            network_tier: "tailnet".to_string(),
            strengths: Vec::new(),
            constraints: Vec::new(),
            best_for: Vec::new(),
            online: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeLoad {
    pub active_agents: u32,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for NodeLoad {
    fn default() -> Self {
        Self {
            active_agents: 0,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetRoutingDecision {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub authority: String,
    pub task_id: String,
    pub task_type: String,
    pub decision: FleetDecision,
    pub governance: FleetGovernance,
    pub routing_context: RoutingContext,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetGovernance {
    pub triad_passed: bool,
    pub joule_efficient: bool,
    pub love_acceptable: bool,
    pub triad_result: TriadResult,
    pub joule_profile: arda_governance::JouleWorkProfile,
    pub love_score: arda_governance::LoveEquationScore,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingContext {
    pub local_node_available: bool,
    pub edge_nodes_available: u32,
    pub capacity_before_decision: u32,
    pub selected_node_authority: String,
}

#[allow(dead_code)]
pub struct FleetCapacityManager {
    nodes: HashMap<String, FleetNode>,
    node_loads: HashMap<String, NodeLoad>,
    config_path: String,
    primary_backbone: Option<String>,
    primary_operator: Option<String>,
}

impl FleetCapacityManager {
    pub fn new(config_root: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_path = config_root.as_ref().to_path_buf();
        let mut manager = Self {
            nodes: HashMap::new(),
            node_loads: HashMap::new(),
            config_path: config_path.to_string_lossy().to_string(),
            primary_backbone: None,
            primary_operator: None,
        };
        manager.load_fleet_config(&config_path)?;
        Ok(manager)
    }

    fn load_fleet_config(&mut self, config_root: &Path) -> anyhow::Result<()> {
        let fleet_toml = config_root.join("config/fleet.toml");
        if !fleet_toml.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&fleet_toml)?;
        let parsed: toml::Value = toml::from_str(&content)?;

        if let Some(nodes) = parsed.get("nodes").and_then(|n| n.as_array()) {
            for node in nodes {
                let id = node
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let role = node
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let hostname = node
                    .get("hostname")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let tailscale_ip = node
                    .get("tailscale_ip")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let node_class = node
                    .get("node_class")
                    .and_then(|v| v.as_str())
                    .unwrap_or("edge_worker")
                    .to_string();
                let enrollment_status = node
                    .get("enrollment_status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("active")
                    .to_string();
                let llm_runtime = node
                    .get("llm_runtime")
                    .and_then(|v| v.as_str())
                    .unwrap_or("local_inference_worker")
                    .to_string();

                let execution_authority = match node_class.as_str() {
                    "backbone_compute" => "primary",
                    "edge_worker" => "specialized",
                    _ => "secondary",
                };

                let scout_capacity = match node_class.as_str() {
                    "backbone_compute" => 8,
                    "edge_worker" => 4,
                    "edge_avatar_product" => 2,
                    "edge_guardhouse" => 3,
                    _ => 2,
                };

                let cpu_cores = match node_class.as_str() {
                    "backbone_compute" => 16,
                    "edge_worker" => 12,
                    _ => 4,
                };

                let ram_mb = match node_class.as_str() {
                    "backbone_compute" => 131072,
                    "edge_worker" => 32768,
                    _ => 4096,
                };

                let system_ram_gib = ram_mb / 1024;

                let gpu = matches!(node_class.as_str(), "backbone_compute" | "edge_worker");

                let gpu_model = match node_class.as_str() {
                    "backbone_compute" => Some("dual_rtx_2080_super_8g".to_string()),
                    "edge_worker" => Some("AMD Radeon 890M".to_string()),
                    _ => None,
                };

                let reliability_score = match enrollment_status.as_str() {
                    "active" => 0.95,
                    "active_staging" => 0.85,
                    _ => 0.7,
                };
                let online = matches!(enrollment_status.as_str(), "active" | "active_staging");

                let (strengths, best_for) = match node_class.as_str() {
                    "backbone_compute" => (
                        vec![
                            "parallel_services".to_string(),
                            "deep_reasoning".to_string(),
                            "large_memory_headroom".to_string(),
                        ],
                        vec![
                            "charon_primary_routing".to_string(),
                            "oracle_reasoning".to_string(),
                            "athena_deep_digest".to_string(),
                        ],
                    ),
                    "edge_worker" => (
                        vec![
                            "background_tasks".to_string(),
                            "supplemental_reasoning".to_string(),
                        ],
                        vec![
                            "edge_worker_execution".to_string(),
                            "background_tasks".to_string(),
                        ],
                    ),
                    "edge_guardhouse" => (
                        vec![],
                        vec![
                            "warden_monitoring".to_string(),
                            "guardhouse_alerting".to_string(),
                        ],
                    ),
                    _ => (Vec::new(), Vec::new()),
                };

                let constraints = if system_ram_gib < 8 {
                    vec!["limited_ram".to_string()]
                } else {
                    Vec::new()
                };

                let fleet_node = FleetNode {
                    id: id.clone(),
                    role,
                    hostname,
                    tailscale_ip,
                    node_class,
                    enrollment_status,
                    llm_runtime,
                    execution_authority: execution_authority.to_string(),
                    scout_capacity,
                    cpu_cores,
                    ram_mb,
                    gpu,
                    gpu_model,
                    system_ram_gib,
                    reliability_score,
                    network_tier: "tailnet".to_string(),
                    strengths,
                    constraints,
                    best_for,
                    online,
                };

                if execution_authority == "primary" && online {
                    self.primary_backbone = Some(id.clone());
                }

                self.nodes.insert(id.clone(), fleet_node);
                self.node_loads.insert(id, NodeLoad::default());
            }
        }

        Ok(())
    }

    pub fn get_node(&self, node_id: &str) -> Option<&FleetNode> {
        self.nodes.get(node_id)
    }

    pub fn get_all_nodes(&self) -> Vec<&FleetNode> {
        self.nodes.values().collect()
    }

    pub fn get_online_nodes(&self) -> Vec<&FleetNode> {
        self.nodes.values().filter(|n| n.online).collect()
    }

    pub fn get_primary_backbone(&self) -> Option<&FleetNode> {
        self.primary_backbone
            .as_ref()
            .and_then(|id| self.nodes.get(id))
    }

    pub fn update_load(
        &mut self,
        node_id: &str,
        active_agents: u32,
        cpu_usage: f64,
        memory_usage: f64,
    ) {
        if let Some(load) = self.node_loads.get_mut(node_id) {
            load.active_agents = active_agents;
            load.cpu_usage = cpu_usage;
            load.memory_usage = memory_usage;
            load.last_updated = chrono::Utc::now();
        }
    }

    pub fn get_available_capacity(&self, node_id: &str) -> u32 {
        let node = match self.nodes.get(node_id) {
            Some(n) => n,
            None => return 0,
        };

        let load = match self.node_loads.get(node_id) {
            Some(l) => l,
            None => return node.scout_capacity,
        };

        node.scout_capacity.saturating_sub(load.active_agents)
    }

    pub fn evaluate_task(&self, task: &Task) -> FleetRoutingDecision {
        let task_id = task.id;
        let task_type = task.task_type.clone();

        let local_available = self.can_execute_locally();
        let edge_count = self
            .get_online_nodes()
            .iter()
            .filter(|n| self.get_available_capacity(&n.id) > 0)
            .count() as u32;

        let bacon_result = bacon_lite_validate(task);
        let joule_profile = profile_joulework(task);
        let love_score = love_equation_score(task);

        let governance = FleetGovernance {
            triad_passed: bacon_result.passed,
            joule_efficient: joule_profile.efficient,
            love_acceptable: love_score.score > 0.3,
            triad_result: bacon_result.triad.clone(),
            joule_profile: joule_profile.clone(),
            love_score: love_score.clone(),
        };

        if !bacon_result.passed {
            return FleetRoutingDecision {
                schema_version: "arda.fleet-routing-decision.v1".to_string(),
                generated_at_utc: chrono::Utc::now().to_rfc3339(),
                authority: "fleet_capacity_manager + bacon_lite".to_string(),
                task_id: task_id.to_string(),
                task_type: task_type.clone(),
                decision: FleetDecision::Reject {
                    reason: bacon_result.rationale.clone(),
                },
                governance,
                routing_context: RoutingContext {
                    local_node_available: local_available,
                    edge_nodes_available: edge_count,
                    capacity_before_decision: 0,
                    selected_node_authority: "none".to_string(),
                },
            };
        }

        if !joule_profile.efficient {
            return FleetRoutingDecision {
                schema_version: "arda.fleet-routing-decision.v1".to_string(),
                generated_at_utc: chrono::Utc::now().to_rfc3339(),
                authority: "fleet_capacity_manager + triad".to_string(),
                task_id: task_id.to_string(),
                task_type: task_type.clone(),
                decision: FleetDecision::Reject {
                    reason: format!(
                        "Joule inefficient: variance={:.2}, honest_ratio={:.2}",
                        joule_profile.variance, joule_profile.honesty_ratio
                    ),
                },
                governance,
                routing_context: RoutingContext {
                    local_node_available: local_available,
                    edge_nodes_available: edge_count,
                    capacity_before_decision: 0,
                    selected_node_authority: "none".to_string(),
                },
            };
        }

        let best_node = self.select_best_node(task);

        let (decision, capacity_before, authority) = match best_node {
            Some(FleetDecision::Accept {
                ref node_id,
                available_capacity,
                node_score,
                triad_result,
                reasoning,
            }) => {
                let node = self.nodes.get(node_id);
                let auth = node
                    .map(|n| n.execution_authority.clone())
                    .unwrap_or_else(|| "unknown".to_string());
                (
                    FleetDecision::Accept {
                        node_id: node_id.clone(),
                        available_capacity,
                        node_score,
                        triad_result,
                        reasoning,
                    },
                    available_capacity,
                    auth,
                )
            }
            _ => (
                FleetDecision::Reject {
                    reason: "No suitable node found".to_string(),
                },
                0,
                "none".to_string(),
            ),
        };

        FleetRoutingDecision {
            schema_version: "arda.fleet-routing-decision.v1".to_string(),
            generated_at_utc: chrono::Utc::now().to_rfc3339(),
            authority: "fleet_capacity_manager + triad".to_string(),
            task_id: task_id.to_string(),
            task_type,
            decision,
            governance,
            routing_context: RoutingContext {
                local_node_available: local_available,
                edge_nodes_available: edge_count,
                capacity_before_decision: capacity_before,
                selected_node_authority: authority,
            },
        }
    }

    pub fn select_best_node(&self, task: &Task) -> Option<FleetDecision> {
        let mut candidates: Vec<(String, u32)> = self
            .get_online_nodes()
            .iter()
            .map(|n| {
                let available = self.get_available_capacity(&n.id);
                (n.id.clone(), available)
            })
            .filter(|(_, available)| *available > 0)
            .collect();

        candidates.sort_by(|a, b| b.1.cmp(&a.1));

        for (node_id, available) in candidates {
            let node = self.nodes.get(&node_id)?;
            let bacon_result = bacon_lite_validate(task);

            if !bacon_result.passed {
                continue;
            }

            let score = self.calculate_node_score(node, available);
            return Some(FleetDecision::Accept {
                node_id,
                node_score: score,
                available_capacity: available,
                triad_result: bacon_result.triad.clone(),
                reasoning: format!("Node score: {:.2}", score),
            });
        }

        None
    }

    fn calculate_node_score(&self, node: &FleetNode, available: u32) -> f64 {
        let capacity_factor = (available as f64 / node.scout_capacity as f64).min(1.0);
        let reliability_factor = node.reliability_score;
        let gpu_factor = if node.gpu { 1.2 } else { 1.0 };

        (reliability_factor * gpu_factor * capacity_factor).min(1.0)
    }

    pub fn can_execute_locally(&self) -> bool {
        if let Some(backbone) = self.get_primary_backbone() {
            return self.get_available_capacity(&backbone.id) > 0;
        }

        self.get_online_nodes()
            .iter()
            .any(|node| self.get_available_capacity(&node.id) > 0)
    }

    pub fn get_local_node_id(&self) -> Option<String> {
        self.primary_backbone.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FleetDecision {
    Accept {
        node_id: String,
        node_score: f64,
        available_capacity: u32,
        triad_result: TriadResult,
        reasoning: String,
    },
    Reject {
        reason: String,
    },
}

impl FleetDecision {
    pub fn is_accepted(&self) -> bool {
        matches!(self, FleetDecision::Accept { .. })
    }

    pub fn node_id(&self) -> Option<&str> {
        match self {
            FleetDecision::Accept { node_id, .. } => Some(node_id),
            FleetDecision::Reject { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_fleet_config(root: &Path, content: &str) {
        let config_dir = root.join("config");
        std::fs::create_dir_all(&config_dir).expect("config dir");
        std::fs::write(config_dir.join("fleet.toml"), content).expect("fleet config");
    }

    #[test]
    fn loads_fleet_config_and_identifies_primary_backbone() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_fleet_config(
            dir.path(),
            r#"
[[nodes]]
id = "node-core-hub"
role = "main_hub"
hostname = "arda-core"
tailscale_ip = "100.64.0.1"
node_class = "core_compute"
enrollment_status = "active"
llm_runtime = "mesh_root"

[[nodes]]
id = "node-backbone-server"
role = "backbone_inference"
hostname = "arda-server"
tailscale_ip = "100.64.0.2"
node_class = "backbone_compute"
enrollment_status = "offline"
llm_runtime = "multi_gpu_sovereign_backbone"

[[nodes]]
id = "node-ser9-worker"
role = "ser9_sovereign_worker"
hostname = "beelink"
tailscale_ip = "100.64.0.3"
node_class = "edge_worker"
enrollment_status = "active"
llm_runtime = "hades"
"#,
        );

        let manager = FleetCapacityManager::new(dir.path()).expect("manager");
        let backbone = manager
            .get_node("node-backbone-server")
            .expect("backbone node");
        let edge = manager
            .get_node("node-ser9-worker")
            .expect("edge worker node");
        let hub = manager.get_node("node-core-hub").expect("core hub");

        assert_eq!(backbone.id, "node-backbone-server");
        assert!(!backbone.online);
        assert_eq!(backbone.execution_authority, "primary");
        assert_eq!(backbone.scout_capacity, 8);
        assert_eq!(edge.execution_authority, "specialized");
        assert_eq!(edge.gpu_model.as_deref(), Some("AMD Radeon 890M"));
        assert!(hub.online);
        assert!(manager.get_primary_backbone().is_none());
        assert_eq!(manager.get_online_nodes().len(), 2);
    }

    #[test]
    fn evaluate_task_prefers_available_backbone_node() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_fleet_config(
            dir.path(),
            r#"
[[nodes]]
id = "node-core-hub"
role = "main_hub"
hostname = "arda-core"
tailscale_ip = "100.64.0.1"
node_class = "core_compute"
enrollment_status = "active"
llm_runtime = "mesh_root"

[[nodes]]
id = "node-backbone-server"
role = "backbone_inference"
hostname = "arda-server"
tailscale_ip = "100.64.0.2"
node_class = "backbone_compute"
enrollment_status = "offline"
llm_runtime = "multi_gpu_sovereign_backbone"

[[nodes]]
id = "node-ser9-worker"
role = "ser9_sovereign_worker"
hostname = "beelink"
tailscale_ip = "100.64.0.3"
node_class = "edge_worker"
enrollment_status = "active"
llm_runtime = "hades"
"#,
        );

        let manager = FleetCapacityManager::new(dir.path()).expect("manager");
        let mut task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        task.joule_cost_estimated = 4.0;
        task.joule_cost_actual = 4.0;
        let decision = manager.evaluate_task(&task);

        assert!(decision.decision.is_accepted());
        assert_eq!(decision.decision.node_id(), Some("node-ser9-worker"));
        assert!(decision.routing_context.local_node_available);
        assert_eq!(
            decision.routing_context.selected_node_authority,
            "specialized"
        );
        assert!(decision.governance.triad_passed);
    }

    #[test]
    fn local_execution_and_selection_fail_when_online_nodes_are_saturated() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_fleet_config(
            dir.path(),
            r#"
[[nodes]]
id = "node-backbone-server"
role = "backbone_inference"
hostname = "arda-server"
tailscale_ip = "100.64.0.2"
node_class = "backbone_compute"
enrollment_status = "active"
llm_runtime = "multi_gpu_sovereign_backbone"

[[nodes]]
id = "node-ser9-worker"
role = "ser9_sovereign_worker"
hostname = "beelink"
tailscale_ip = "100.64.0.3"
node_class = "edge_worker"
enrollment_status = "active"
llm_runtime = "hades"
"#,
        );

        let mut manager = FleetCapacityManager::new(dir.path()).expect("manager");
        manager.update_load("node-backbone-server", 8, 0.92, 0.81);
        manager.update_load("node-ser9-worker", 4, 0.88, 0.73);

        let mut task = Task::new(
            "ingest https://example.com because source evidence is official",
            "ingest",
        );
        task.joule_cost_estimated = 4.0;
        task.joule_cost_actual = 4.0;

        assert!(!manager.can_execute_locally());
        assert!(manager.select_best_node(&task).is_none());

        let decision = manager.evaluate_task(&task);
        assert!(!decision.decision.is_accepted());
        assert_eq!(decision.routing_context.capacity_before_decision, 0);
        assert_eq!(decision.routing_context.selected_node_authority, "none");
        assert!(!decision.routing_context.local_node_available);
        assert_eq!(decision.routing_context.edge_nodes_available, 0);
    }
}
