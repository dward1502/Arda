//! Bundled constants for the versioned `arda.rumil.*` contract names.
//!
//! These constants are used as the `kind` tag in packet envelopes so that
//! envelope validators can dispatch without instantiating every struct.

/// Top-level contract namespace for this crate.
pub const CONTRACT_DOMAIN: &str = "arda.rumil";
pub const CONTRACT_VERSION_MAJOR: &str = "v1";

// Packet kind constants — canonical string tags used in the `kind` envelope field.
pub const KIND_AUDIT_REQUEST: &str = "arda.rumil.audit-request.v1";
pub const KIND_AUDIT_REPORT: &str = "arda.rumil.audit-report.v1";
pub const KIND_FILE_RECORD: &str = "arda.rumil.file-record.v1";
pub const KIND_COMMAND_RECEIPT: &str = "arda.rumil.command-receipt.v1";
pub const KIND_FINDING: &str = "arda.rumil.finding.v1";
pub const KIND_ORGANIZATION_PLAN: &str = "arda.rumil.organization-plan.v1";
pub const KIND_COMPARISON: &str = "arda.rumil.comparison.v1";
pub const KIND_MEMORY_OBSERVATION: &str = "arda.rumil.memory-observation.v1";
pub const KIND_LEGACY_HADES_IMPORT: &str = "arda.rumil.legacy-hades-import.v1";

/// Current full schema version string.
pub const CURRENT_SCHEMA_VERSION: &str = "arda.rumil.v1";
/// Alias for the full schema version string.
pub const RUMIL_SCHEMA_VERSION: &str = CURRENT_SCHEMA_VERSION;

/// Provider status strings — shared by the normalization layer and tests.
pub const PROVIDER_COMPLETED: &str = "completed";
pub const PROVIDER_FAILED: &str = "failed";
pub const PROVIDER_UNAVAILABLE: &str = "unavailable";
pub const PROVIDER_SKIPPED_BY_POLICY: &str = "skipped_by_policy";
pub const PROVIDER_DENIED_BY_BUDGET: &str = "denied_by_budget";
pub const PROVIDER_TIMED_OUT: &str = "timed_out";
pub const PROVIDER_MALFORMED_OUTPUT: &str = "malformed_output";

pub const ALL_PROVIDER_STATUSES: &[&str] = &[
    PROVIDER_COMPLETED,
    PROVIDER_FAILED,
    PROVIDER_UNAVAILABLE,
    PROVIDER_SKIPPED_BY_POLICY,
    PROVIDER_DENIED_BY_BUDGET,
    PROVIDER_TIMED_OUT,
    PROVIDER_MALFORMED_OUTPUT,
];

pub fn is_known_provider_status(value: &str) -> bool {
    ALL_PROVIDER_STATUSES.contains(&value)
}
