//! Bounded repository adapters for Rúmil.
//!
//! Adapters receive a validated `AuditRequest` + `AuditPolicy` and return
//! provider output plus a capability outcome. They do not write memory,
//! mutate files, approve work, or call Warden directly.

use crate::contracts::CapabilityOutcome;
use serde::{Deserialize, Serialize};

#[cfg(feature = "cargo")]
pub mod cargo;
pub mod generic;
#[cfg(feature = "git")]
pub mod git;

#[cfg(feature = "cargo")]
pub use cargo::{
    CargoAdapter, CargoDependencyEdge, CargoPackageRecord, CargoTargetRecord,
    CargoWorkspaceSnapshot,
};
pub use generic::GenericAdapter;
#[cfg(feature = "git")]
pub use git::{GitAdapter, GitStateSnapshot, GitStatusEntry};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterCapability {
    pub capability: String,
    pub provider_id: String,
    pub available: bool,
    pub detail: Option<String>,
}

/// Discover compiled and project-applicable adapters without claiming that an
/// unavailable project type was audited.
pub fn discover_capabilities(project_root: &std::path::Path) -> Vec<AdapterCapability> {
    let mut capabilities = vec![AdapterCapability {
        capability: "generic_inventory".to_string(),
        provider_id: "rumil.generic_inventory.v1".to_string(),
        available: cfg!(feature = "walkdir") && project_root.is_dir(),
        detail: None,
    }];
    capabilities.push(AdapterCapability {
        capability: "cargo_workspace".to_string(),
        provider_id: "rumil.cargo_metadata.v1".to_string(),
        available: cargo_available(project_root),
        detail: Some("requires the cargo feature and a root Cargo.toml".to_string()),
    });
    capabilities.push(AdapterCapability {
        capability: "git_state".to_string(),
        provider_id: "rumil.git_readonly.v1".to_string(),
        available: git_available(project_root),
        detail: Some("requires the git feature and a Git work tree".to_string()),
    });
    capabilities
}

#[cfg(feature = "cargo")]
fn cargo_available(project_root: &std::path::Path) -> bool {
    project_root.join("Cargo.toml").is_file()
}

#[cfg(not(feature = "cargo"))]
fn cargo_available(_project_root: &std::path::Path) -> bool {
    false
}

#[cfg(feature = "git")]
fn git_available(project_root: &std::path::Path) -> bool {
    std::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(project_root)
        .output()
        .map(|output| output.status.success() && output.stdout.starts_with(b"true"))
        .unwrap_or(false)
}

#[cfg(not(feature = "git"))]
fn git_available(_project_root: &std::path::Path) -> bool {
    false
}

pub trait ProviderAdapter {
    fn capability(&self) -> &str;
    fn provider_id(&self) -> &str;

    fn run(
        &self,
        request: &crate::contracts::AuditRequest,
        policy: &crate::policy::AuditPolicy,
        project_root: &std::path::Path,
    ) -> crate::error::Result<(serde_json::Value, CapabilityOutcome)>;
}

pub(crate) fn provider_allowed(policy: &crate::policy::AuditPolicy, provider: &str) -> bool {
    policy
        .provider_allowlist
        .iter()
        .any(|allowed| allowed == provider)
}

pub(crate) fn outcome(
    capability: &str,
    provider_id: &str,
    status: &str,
    detail: Option<String>,
) -> CapabilityOutcome {
    CapabilityOutcome {
        capability: capability.to_string(),
        status: status.to_string(),
        provider_id: Some(provider_id.to_string()),
        detail,
    }
}
