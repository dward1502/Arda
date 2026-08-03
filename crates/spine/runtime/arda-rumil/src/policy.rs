//! Audit policy and root identity for Rúmil.
//!
//! Policies are declarative: roots, exclusions, providers, budgets,
//! organization rules, retention, and redaction.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Project identity derived from the workspace root, independent of host paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootIdentity {
    pub project_id: Uuid,
    pub name: String,
    pub kind: String,
    pub remote_url: Option<String>,
}

/// Budget constraints that bound every scan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BudgetPolicy {
    pub max_depth: usize,
    pub max_files: u64,
    pub max_total_bytes: u64,
    pub max_excerpt_bytes: u64,
    #[serde(default = "default_scan_timeout_seconds")]
    pub scan_timeout_seconds: u64,
    pub command_timeout_seconds: u64,
}

const fn default_scan_timeout_seconds() -> u64 {
    60
}

impl Default for BudgetPolicy {
    fn default() -> Self {
        Self {
            max_depth: 12,
            max_files: 100_000,
            max_total_bytes: 256 * 1024 * 1024,
            max_excerpt_bytes: 64 * 1024,
            scan_timeout_seconds: default_scan_timeout_seconds(),
            command_timeout_seconds: 60,
        }
    }
}

/// Path exclusion rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExclusionRule {
    pub pattern: String,
    pub kind: ExclusionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExclusionKind {
    Directory,
    File,
    Glob,
}

/// Top-level audit policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditPolicy {
    pub profile_id: String,
    pub root_identity: RootIdentity,
    /// POSIX-relative root path used for traversal.
    pub root_relative: String,
    #[serde(default)]
    pub exclusion_rules: Vec<ExclusionRule>,
    pub budget: BudgetPolicy,
    #[serde(default)]
    pub provider_allowlist: Vec<String>,
    #[serde(default)]
    pub redaction_policy: Vec<String>,
}

/// Full project policy combining root identity and audit policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectPolicy {
    pub identity: RootIdentity,
    pub audit: AuditPolicy,
}
