//! Stable service identity and lifecycle shims.
//!
//! `ArdaServiceRegistryStatus` and `startup_order` live here because they are
//! part of Arda's public integration surface, but they still rely on the
//! consensus `ServiceRegistry` in this crate.
//!
//! The typed handles below are why `service` exists as a first-class module:
//! engine code depends on `ServiceKind`/`ServiceStatus` directly, and
//! contracts should not need to reconstruct them from strings.

use std::{
    fmt,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::contract::{ServiceContract, ServiceKind as ContractServiceKind};

// ---------------------------------------------------------------------------
// Typed domain types
// ---------------------------------------------------------------------------

/// Stable identifier for a registered service.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceKind {
    #[default]
    Generic,
    Governance,
    Mnemosyne,
    Plutus,
    Oracle,
    Charon,
    Gateway,
    Daemon,
    Launcher,
}

impl ServiceKind {
    pub fn classify(name: &str) -> Self {
        let normalized = name.to_ascii_lowercase();
        if normalized.contains("manwe") || normalized.contains("gateway") {
            return Self::Gateway;
        }
        if normalized.contains("charon") {
            return Self::Charon;
        }
        if normalized.contains("oracle") {
            return Self::Oracle;
        }
        if normalized.contains("plutus") {
            return Self::Plutus;
        }
        if normalized.contains("mnemosyne") {
            return Self::Mnemosyne;
        }
        if normalized.contains("regis") || normalized.contains("registry") {
            return Self::Governance;
        }
        Normalized::from_segments(normalized.split('-'))
            .and_then(match_kind)
            .unwrap_or(Self::Generic)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatus {
    Pending,
    Running,
    #[default]
    Stopped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRecord {
    pub contract: ServiceContract,
    pub status: ServiceStatus,
    pub handle: Option<ServiceHandle>,
}

impl ServiceRecord {
    pub const fn new(contract: ServiceContract) -> Self {
        Self {
            contract,
            status: ServiceStatus::Pending,
            handle: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ServiceHandle(u64);

impl ServiceHandle {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
    pub const fn value(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceRegistryState {
    pub records: Vec<ServiceRecord>,
}

// ---------------------------------------------------------------------------
// Display / conversion helpers
// ---------------------------------------------------------------------------

impl fmt::Display for ServiceKind {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            fmt,
            "{}",
            match self {
                Self::Generic => "generic",
                Self::Governance => "governance",
                Self::Mnemosyne => "mnemosyne",
                Self::Plutus => "plutus",
                Self::Oracle => "oracle",
                Self::Charon => "charon",
                Self::Gateway => "gateway",
                Self::Daemon => "daemon",
                Self::Launcher => "launcher",
            }
        )
    }
}

impl From<ServiceKind> for ContractServiceKind {
    fn from(value: ServiceKind) -> Self {
        match value {
            ServiceKind::Generic => ContractServiceKind::Gateway,
            ServiceKind::Governance => ContractServiceKind::Governance,
            ServiceKind::Mnemosyne => ContractServiceKind::Mnemosyne,
            ServiceKind::Plutus => ContractServiceKind::Plutus,
            ServiceKind::Oracle => ContractServiceKind::Oracle,
            ServiceKind::Charon => ContractServiceKind::Charon,
            ServiceKind::Gateway => ContractServiceKind::Gateway,
            ServiceKind::Daemon => ContractServiceKind::Gateway,
            ServiceKind::Launcher => ContractServiceKind::Gateway,
        }
    }
}

impl From<ServiceKind> for String {
    fn from(value: ServiceKind) -> Self {
        value.to_string()
    }
}

impl std::str::FromStr for ServiceKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "generic" => Ok(Self::Generic),
            "governance" => Ok(Self::Governance),
            "mnemosyne" => Ok(Self::Mnemosyne),
            "plutus" => Ok(Self::Plutus),
            "oracle" => Ok(Self::Oracle),
            "charon" => Ok(Self::Charon),
            "gateway" => Ok(Self::Gateway),
            "daemon" => Ok(Self::Daemon),
            "launcher" => Ok(Self::Launcher),
            other => Err(format!("unknown service kind: {other}")),
        }
    }
}

struct Normalized<'a> {
    value: &'a str,
}

impl<'a> Normalized<'a> {
    fn from_segments(mut segments: impl Iterator<Item = &'a str>) -> Option<Self> {
        let first = segments.next()?;
        let normal = first.trim_start_matches("arda-");
        if normal == first && !normal.is_empty() {
            return Some(Self { value: normal });
        }
        None
    }
}

fn match_kind(input: Normalized<'_>) -> Option<ServiceKind> {
    match input.value {
        "governance" => Some(ServiceKind::Governance),
        "mnemosyne" => Some(ServiceKind::Mnemosyne),
        "plutus" => Some(ServiceKind::Plutus),
        "oracle" => Some(ServiceKind::Oracle),
        "charon" => Some(ServiceKind::Charon),
        "manwe" => Some(ServiceKind::Gateway),
        "gateway" => Some(ServiceKind::Gateway),
        "daemon" => Some(ServiceKind::Daemon),
        "launcher" => Some(ServiceKind::Launcher),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Contract helper
// ---------------------------------------------------------------------------

pub fn fetch_service_contract(name: &str) -> Option<ServiceContract> {
    Normalized::from_segments(name.split('-'))
        .and_then(match_kind)
        .map(|kind| ServiceContract {
            name: name.into(),
            kind: kind.into(),
            command: name.into(),
            args: Vec::default(),
            working_directory: PathBuf::default(),
            environment: Vec::default(),
        })
}

// ---------------------------------------------------------------------------
// Storage I/O
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum StateError {
    #[error("state read failed: {0}")]
    Read(String),
    #[error("state write failed: {0}")]
    Write(String),
    #[error("state path is not UTF-8: {0}")]
    NonUtf8Path(Box<PathBuf>),
    #[error("state parse failed: {0}")]
    Parse(String),
    #[error("state serialize failed: {0}")]
    Serialize(String),
}

impl From<StateError> for crate::registry::RegistryError {
    fn from(value: StateError) -> Self {
        crate::registry::RegistryError::Invalid("state".into(), value.to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceRegistryStateValidator {
    pub require_governance: bool,
    pub snapshots: Vec<ServiceRegistryState>,
    pub last_error: Option<String>,
}

impl ServiceRegistryStateValidator {
    pub fn load(
        &self,
        target: impl AsRef<Path>,
    ) -> Result<Option<ServiceRegistryState>, StateError> {
        let path = target.as_ref();
        if !path.exists() {
            return Ok(None);
        }
        let data = fs::read_to_string(path)
            .map_err(|source| StateError::Read(source.to_string()))?;
        let snapshot = serde_json::from_str(&data)
            .map_err(|source| StateError::Parse(source.to_string()))?;
        Ok(Some(snapshot))
    }

    pub fn snapshot(&self, state_dir: impl AsRef<Path>) -> ServiceRegistryState {
        let state_dir = state_dir.as_ref();
        let _ = fs::create_dir_all(state_dir);
        let latest = fs::read_dir(state_dir)
            .ok()
            .and_then(|mut entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .max_by_key(|entry| {
                        entry
                            .metadata()
                            .and_then(|meta| meta.modified())
                            .ok()
                    })
            })
            .and_then(|entry| fs::read_to_string(entry.path()).ok())
            .and_then(|data| serde_json::from_str::<ServiceRegistryState>(&data).ok());

        let records = latest
            .map(|state| state.records)
            .unwrap_or_default();

        ServiceRegistryState { records }
    }

    pub fn persist_snapshot(
        &self,
        state_dir: impl AsRef<Path>,
        state_name: &str,
        snapshot: ServiceRegistryState,
    ) -> Result<(), StateError> {
        let state_dir = state_dir.as_ref();
        let _ = fs::create_dir_all(state_dir);
        let data = serde_json::to_vec_pretty(&snapshot)
            .map_err(|source| StateError::Serialize(source.to_string()))?;
        let target = state_dir.join(format!("{state_name}.json"));
        fs::write(&target, data)
            .map_err(|source| StateError::Write(source.to_string()))?;
        Ok(())
    }

    pub fn validate_or_refresh(
        &mut self,
        snapshot: ServiceRegistryState,
    ) -> Result<Option<ServiceRegistryState>, StateError> {
        let seen_governance = snapshot
            .records
            .iter()
            .any(|record| matches!(record.contract.kind, ContractServiceKind::Governance));

        if self.require_governance && !seen_governance {
            self.last_error = Some("snapshot missing governance service".into());
            return Ok(None);
        }
        self.snapshots.push(snapshot.clone());
        self.last_error = None;
        Ok(Some(snapshot))
    }
}

// ---------------------------------------------------------------------------
// Arda integration surface
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GovernanceConfig {
    pub triad_required: bool,
    pub bacon_lite_required: bool,
    pub joulework_required: bool,
    pub love_equation_required: bool,
    pub soterion_trace_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContinuityConfig {
    pub task_ledger_linked: bool,
    pub memory_checkpoint_expected: bool,
    pub arda_visibility_defined: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractConfig {
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance: GovernanceConfig,
    pub continuity: ContinuityConfig,
}

impl Default for ContractConfig {
    fn default() -> Self {
        Self {
            realm: "arda",
            productizable: true,
            state_export_path: "./state/registry.json",
            governance: GovernanceConfig {
                triad_required: true,
                bacon_lite_required: true,
                joulework_required: true,
                love_equation_required: true,
                soterion_trace_required: true,
            },
            continuity: ContinuityConfig {
                task_ledger_linked: true,
                memory_checkpoint_expected: false,
                arda_visibility_defined: true,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArdaServiceRegistryStatus {
    pub crate_name: &'static str,
    pub realm: &'static str,
    pub productizable: bool,
    pub state_export_path: &'static str,
    pub governance_ready: bool,
    pub deterministic_startup_supported: bool,
}

pub fn contract() -> ContractConfig {
    ContractConfig::default()
}

pub fn status() -> ArdaServiceRegistryStatus {
    let base = contract();
    let governance_ready = base.governance.triad_required
        && base.governance.bacon_lite_required
        && base.governance.joulework_required
        && base.governance.love_equation_required
        && base.governance.soterion_trace_required
        && base.continuity.task_ledger_linked
        && base.continuity.memory_checkpoint_expected
        && base.continuity.arda_visibility_defined;

    ArdaServiceRegistryStatus {
        crate_name: "arda-service-registry",
        realm: base.realm,
        productizable: base.productizable,
        state_export_path: base.state_export_path,
        governance_ready,
        deterministic_startup_supported: true,
    }
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

impl crate::registry::ServiceRegistry {
    pub fn startup_order(&self) -> Result<Vec<String>, String> {
        let mut names: Vec<_> = self
            .records()
            .iter()
            .filter_map(|(_, record)| {
                if matches!(record.status, ServiceStatus::Pending | ServiceStatus::Running) {
                    return Some(record.contract.name.clone());
                }
                None
            })
            .collect::<Vec<_>>();

        let priority = |name: &str| match ServiceKind::classify(name) {
            ServiceKind::Governance => 0,
            ServiceKind::Mnemosyne => 1,
            ServiceKind::Plutus => 2,
            ServiceKind::Oracle => 3,
            ServiceKind::Charon => 4,
            ServiceKind::Gateway => 5,
            ServiceKind::Daemon => 6,
            ServiceKind::Launcher => 7,
            ServiceKind::Generic => 8,
        };

        names.sort_by_cached_key(|name| priority(name));
        Ok(names)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_service_record_defaults_to_pending() {
        let record = ServiceRecord::new(ServiceContract::new(
            "test",
            ContractServiceKind::Gateway,
            "cmd",
            ".",
        ));
        assert_eq!(record.status, ServiceStatus::Pending);
        assert_eq!(record.contract.kind, ContractServiceKind::Gateway);
        assert_eq!(record.handle, None);
    }
}

