#![cfg(feature = "full-cli")]
// sigil: REPAIR
use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct CoreAutonomyProfile {
    pub heartbeat_ms: u64,
    pub triad_bypass: bool,
    pub base_costs: HashMap<String, f64>,
    pub world_status: Option<String>,
    pub world_resonance: Option<f64>,
    pub source_root: PathBuf,
}

impl CoreAutonomyProfile {
    pub fn load(core_root: impl AsRef<Path>) -> Option<Self> {
        let core_root = core_root.as_ref().to_path_buf();
        let boot_path = core_root.join("realm").join("boot.toml");
        let world_path = core_root.join("state").join("world.json");

        let boot = read_boot(&boot_path)?;
        let world = read_world(&world_path);

        Some(Self {
            heartbeat_ms: boot.ceo.heartbeat_ms.unwrap_or(500),
            triad_bypass: boot.ceo.triad_bypass.unwrap_or(false),
            base_costs: boot
                .joulework
                .and_then(|j| j.base_costs)
                .unwrap_or_default(),
            world_status: world.as_ref().map(|w| w.system.status.clone()),
            world_resonance: world.map(|w| w.metrics.system_resonance),
            source_root: core_root,
        })
    }

    pub fn base_cost_for(&self, task_type: &str) -> Option<f64> {
        self.base_costs
            .get(&task_type.to_ascii_lowercase())
            .copied()
            .or_else(|| self.base_costs.get(task_type).copied())
    }
}

#[derive(Debug, Deserialize)]
struct BootConfig {
    ceo: BootCeoConfig,
    joulework: Option<BootJouleWorkConfig>,
}

#[derive(Debug, Deserialize)]
struct BootCeoConfig {
    heartbeat_ms: Option<u64>,
    triad_bypass: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct BootJouleWorkConfig {
    base_costs: Option<HashMap<String, f64>>,
}

#[derive(Debug, Deserialize)]
struct WorldState {
    system: WorldSystem,
    metrics: WorldMetrics,
}

#[derive(Debug, Deserialize)]
struct WorldSystem {
    status: String,
}

#[derive(Debug, Deserialize)]
struct WorldMetrics {
    system_resonance: f64,
}

fn read_boot(path: &Path) -> Option<BootConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str::<BootConfig>(&content).ok()
}

fn read_world(path: &Path) -> Option<WorldState> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<WorldState>(&content).ok()
}
