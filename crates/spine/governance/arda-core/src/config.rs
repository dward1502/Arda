// sigil: REPAIR
use crate::error::Result;
use crate::llm::LlmConfig;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub system: SystemConfig,
    pub agents: AgentsConfig,
    pub paths: PathsConfig,
    pub joulework: JouleWorkConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub name: String,
    pub log_level: String,
    pub max_concurrent_tasks: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    pub default_timeout_secs: u64,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub data_dir: String,
    pub ledger_dir: String,
    pub knowledge_dir: String,
    pub config_dir: String,
    pub registry_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleWorkConfig {
    pub base_cost: u64,          // Base joule cost per task
    pub threshold: u64,          // Max joules before fail/retry
    pub game_theory_factor: f64, // Multiplier for complex tasks (e.g., 1.5 for ingests)
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            crate::error::ArdaError::Config(format!("Failed to read config: {}", e))
        })?;
        toml::from_str(&content).map_err(|e| {
            crate::error::ArdaError::Config(format!("Failed to parse config: {}", e))
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system: SystemConfig {
                name: "Arda".to_string(),
                log_level: "info".to_string(),
                max_concurrent_tasks: 4,
            },
            agents: AgentsConfig {
                default_timeout_secs: 300,
                max_retries: 3,
            },
            paths: PathsConfig {
                data_dir: "data".to_string(),
                ledger_dir: "core/state/ledger".to_string(),
                knowledge_dir: "data/knowledge".to_string(),
                config_dir: "config".to_string(),
                registry_path: "registry.toml".to_string(),
            },
            joulework: JouleWorkConfig {
                base_cost: 10,
                threshold: 100,
                game_theory_factor: 1.2,
            },
            llm: LlmConfig::default(),
        }
    }
}
