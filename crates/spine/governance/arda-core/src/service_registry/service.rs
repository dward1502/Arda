//! Service identity and lifecycle types for the registry.

use serde::{Deserialize, Serialize};
use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

/// Stable runtime status for a registered service.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    #[default]
    Registered,
    Running,
    Stopped,
    Failed,
}

/// Stable service identity used by the registry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArdaServiceRegistryStatus {
    #[default]
    Unknown,
    Healthy,
    Degraded,
    Unhealthy,
}

impl fmt::Display for ArdaServiceRegistryStatus {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            ArdaServiceRegistryStatus::Unknown => "unknown",
            ArdaServiceRegistryStatus::Healthy => "healthy",
            ArdaServiceRegistryStatus::Degraded => "degraded",
            ArdaServiceRegistryStatus::Unhealthy => "unhealthy",
        };
        write!(fmt, "{label}")
    }
}

/// Persisted governance options for a registered service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GovernanceConfig {
    pub mutation_requires_human_gate: bool,
    pub destructive_allowed_by_default: bool,
    pub notes: Vec<String>,
}

/// Contract overrides applied during registration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractConfig {
    pub owner: String,
    pub tags: Vec<String>,
    pub environment_overrides: Vec<(String, String)>,
}

/// Continuity hints for a registered service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContinuityConfig {
    pub recommended_restart_policy: String,
    pub max_restart_attempts: usize,
    pub cooldown_seconds: u64,
}

/// Opaque handle returned when a service is started from the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHandle {
    pub pid: u32,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
}

impl ServiceHandle {
    pub fn new(
        pid: u32,
        command: impl Into<String>,
        args: Vec<String>,
        cwd: impl Into<PathBuf>,
    ) -> Self {
        Self {
            pid,
            command: command.into(),
            args,
            cwd: cwd.into(),
        }
    }
}

/// Runtime snapshot for a single registered service.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRegistryState {
    pub registry_version: u64,
    pub updated_at: String,
    pub services: Vec<ServiceRecord>,
}

impl Default for ServiceRegistryState {
    fn default() -> Self {
        Self {
            registry_version: 0,
            updated_at: now_utc(),
            services: Vec::new(),
        }
    }
}

/// Validation result for a registry snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceRegistryStateValidator {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Identity and runtime record for one registered service.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub identity: crate::service_registry::crate_identity::CrateIdentity,
    pub contract: crate::service_registry::contract::ServiceContract,
    pub status: ServiceStatus,
    pub health: ArdaServiceRegistryStatus,
    pub continuity: ContinuityConfig,
    pub contract_config: ContractConfig,
    pub governance: GovernanceConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handle: Option<ServiceHandle>,
    #[serde(skip)]
    pub state_file: Option<PathBuf>,
}

impl ServiceRecord {
    pub fn new(contract: crate::service_registry::contract::ServiceContract) -> Self {
        Self {
            identity: crate::service_registry::crate_identity::CrateIdentity::from_contract(
                &contract,
            ),
            contract,
            status: ServiceStatus::default(),
            health: ArdaServiceRegistryStatus::default(),
            continuity: ContinuityConfig::default(),
            contract_config: ContractConfig::default(),
            governance: GovernanceConfig::default(),
            handle: None,
            state_file: None,
        }
    }

    pub fn state_path(base: impl AsRef<Path>, contract_name: impl AsRef<str>) -> PathBuf {
        base.as_ref()
            .join(format!("{}.json", contract_name.as_ref().replace('/', "-")))
    }

    pub fn load(
        base: impl AsRef<Path>,
        contract: crate::service_registry::contract::ServiceContract,
    ) -> Self {
        let path = Self::state_path(base, &contract.name);
        let mut record = Self::new(contract);
        record.state_file = Some(path);
        record
    }

    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = &self.state_file else {
            return Ok(());
        };

        let payload = serde_json::to_vec_pretty(self).map_err(io_error)?;
        fs::write(path, payload)
    }

    pub fn hydrate(file: impl AsRef<Path>) -> std::io::Result<Self> {
        let payload = fs::read(file.as_ref())?;
        let mut record: Self = serde_json::from_slice(&payload).map_err(io_error)?;
        record.state_file = Some(file.as_ref().into());
        Ok(record)
    }
}

fn now_utc() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn io_error(err: impl std::fmt::Display) -> std::io::Error {
    std::io::Error::other(err.to_string())
}
