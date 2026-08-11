//! Versioned `arda.rumil.*` packet structs.
//!
//! Every packet here is evidence/state, not execution authorization. Each struct
//! implements `RumilPacket` to expose its envelope kind tag.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{
    CURRENT_SCHEMA_VERSION, KIND_AUDIT_REPORT, KIND_AUDIT_REQUEST, KIND_COMMAND_RECEIPT,
    KIND_COMPARISON, KIND_FINDING, KIND_ORGANIZATION_PLAN,
};

/// Envelope kind for the current schema version.
pub const AUDIT_SCHEMA_VERSION: &str = CURRENT_SCHEMA_VERSION;

/// Marker trait for Rúmil packets. Every packet struct implements `kind()` to
/// expose its canonical envelope tag. Serialize/Deserialize bounds are applied
/// at the call site rather than on the trait to avoid `for<'de>` conflicts.
pub trait RumilPacket {
    /// The canonical `arda.rumil.<name>.v1` string for this packet.
    fn kind() -> &'static str;
}

/// Common envelope fields required on every portable Rúmil packet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PacketEnvelope {
    pub kind: String,
    #[serde(rename = "schema_version")]
    pub version: String,
    pub project_id: Uuid,
    pub source_revision: Option<String>,
    pub authority: String,
    pub generated_at_utc: DateTime<Utc>,
}

/// Project identity independent of local absolute paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RootIdentity {
    pub project_id: Uuid,
    pub name: String,
    pub kind: String,
    pub remote_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditRequest {
    pub request_id: Uuid,
    pub project_id: Uuid,
    pub profile_id: String,
    pub source_revision_expectation: Option<String>,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
    pub root_policy: String,
    #[serde(default)]
    pub path_exclusions: Vec<String>,
    pub file_count_budget: u64,
    pub byte_budget: u64,
    pub source_excerpt_budget: u64,
    pub command_timeout_seconds: u64,
    #[serde(default)]
    pub provider_allowlist: Vec<String>,
    #[serde(default)]
    pub redaction_policy: Vec<String>,
    pub prior_audit_id: Option<Uuid>,
    pub requested_by: String,
    pub expires_at_utc: DateTime<Utc>,
    /// Advisory read-only authority for Rúmil output.
    pub authority: String,
}

impl RumilPacket for AuditRequest {
    fn kind() -> &'static str {
        KIND_AUDIT_REQUEST
    }
}

/// Completeness classification for an audit report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditReportCompleteness {
    Complete,
    Partial,
    StructureOnly,
    Failed,
    NotRequested,
}

impl AuditReportCompleteness {
    /// Returns true only for `Complete` when all required capabilities ran
    /// without disclosure, truncation, or policy failure.
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Summary counts for the inventory section of an audit report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventorySummary {
    pub total_files: u64,
    pub total_directories: u64,
    pub total_symlinks: u64,
    pub total_bytes: u64,
    pub sampled_files: u64,
}

/// Capability execution outcome inside a report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityOutcome {
    pub capability: String,
    pub status: String,
    pub provider_id: Option<String>,
    pub detail: Option<String>,
}

/// Top-level audit report packet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditReport {
    pub audit_id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub project_kind: String,
    pub root_identity: RootIdentity,
    pub source_revision: Option<String>,
    pub profile_id: String,
    pub generated_at_utc: chrono::DateTime<Utc>,
    pub completed_at_utc: Option<chrono::DateTime<Utc>>,
    pub completeness: AuditReportCompleteness,
    #[serde(default)]
    pub capabilities_requested: Vec<String>,
    #[serde(default)]
    pub capabilities_completed: Vec<CapabilityOutcome>,
    #[serde(default)]
    pub capabilities_unavailable: Vec<CapabilityOutcome>,
    pub inventory_summary: InventorySummary,
    pub tree_digest: Option<String>,
    #[serde(default)]
    pub file_record_references: Vec<String>,
    #[serde(default)]
    pub package_records: serde_json::Value,
    #[serde(default)]
    pub module_records: serde_json::Value,
    pub dependency_graph_reference: Option<String>,
    #[serde(default)]
    pub command_receipts: Vec<String>,
    #[serde(default)]
    pub finding_references: Vec<String>,
    pub organization_plan_reference: Option<String>,
    pub comparison_reference: Option<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub truncation: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
    /// Advisory read-only authority.
    pub authority: String,
}

impl RumilPacket for AuditReport {
    fn kind() -> &'static str {
        KIND_AUDIT_REPORT
    }
}

/// File-record kind classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileRecordKind {
    File,
    Directory,
    Symlink,
    Unreadable,
    Excluded,
}

/// Redaction state for a bounded file-record entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionState {
    Observed,
    Redacted,
    Unavailable,
}

/// Bounded inventory entry for a single path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRecord {
    pub path: String,
    pub kind: FileRecordKind,
    pub size_bytes: Option<u64>,
    pub content_sha256: Option<String>,
    pub mime_or_extension: Option<String>,
    pub executable: Option<bool>,
    pub symlink_target_digest: Option<String>,
    #[serde(default)]
    pub source_excerpt_ids: Vec<String>,
    pub redaction_state: RedactionState,
    pub observed_at_utc: chrono::DateTime<Utc>,
}

/// Status of a command/provider receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandReceiptStatus {
    Completed,
    Failed,
    TimedOut,
    Denied,
    Unavailable,
}

/// Prove what an approved provider command actually did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommandReceipt {
    pub command_id: uuid::Uuid,
    pub provider_id: String,
    pub argv_digest: String,
    pub working_directory_relative: String,
    pub policy_id: String,
    pub started_at_utc: chrono::DateTime<Utc>,
    pub finished_at_utc: Option<chrono::DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub stdout_digest: Option<String>,
    pub stderr_digest: Option<String>,
    pub stdout_bytes_retained: u64,
    pub stderr_bytes_retained: u64,
    pub truncated: bool,
    pub timed_out: bool,
    pub status: CommandReceiptStatus,
    pub tool_version: Option<String>,
    pub configuration_digest: Option<String>,
    /// Advisory read-only authority.
    pub authority: String,
}

impl RumilPacket for CommandReceipt {
    fn kind() -> &'static str {
        KIND_COMMAND_RECEIPT
    }
}

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Confidence classification for a finding's evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingConfidenceClass {
    ToolBacked,
    SourceBacked,
    Heuristic,
    Historical,
    Unavailable,
}

/// Lifecycle status of a finding across audits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    New,
    Persistent,
    Changed,
    Resolved,
    Stale,
    Unverifiable,
}

/// Normalized finding from a tool, heuristic, comparison, or organization rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    pub finding_id: uuid::Uuid,
    pub audit_id: uuid::Uuid,
    pub category: String,
    pub severity: FindingSeverity,
    pub status: FindingStatus,
    pub confidence_class: FindingConfidenceClass,
    pub path_or_scope: String,
    pub summary: String,
    pub recommendation: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    pub provider_id: Option<String>,
    pub source_command_id: Option<uuid::Uuid>,
    pub prior_finding_id: Option<uuid::Uuid>,
    pub review_required: bool,
    /// Always false for Rúmil findings — mutations require a separate path.
    pub mutation_allowed: bool,
}

impl RumilPacket for Finding {
    fn kind() -> &'static str {
        KIND_FINDING
    }
}

/// Risk level for an organization candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationRisk {
    Low,
    Medium,
    High,
}

/// One candidate in an organization plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrganizationCandidate {
    pub candidate_id: uuid::Uuid,
    pub path: String,
    pub candidate_type: String,
    pub risk: OrganizationRisk,
    pub recommended_action: String,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub affected_paths: Vec<String>,
    pub rollback_note: Option<String>,
}

/// Status of an organization plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrganizationPlanStatus {
    Proposed,
    Reviewed,
    Superseded,
}

/// Review-only organization proposal. No mutations are performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OrganizationPlan {
    pub plan_id: uuid::Uuid,
    pub audit_id: uuid::Uuid,
    pub profile_id: String,
    pub scope: String,
    pub no_delete: bool,
    pub no_move: bool,
    pub no_rewrite: bool,
    pub operator_review_required: bool,
    pub mutation_authorized: bool,
    pub generated_at_utc: chrono::DateTime<Utc>,
    #[serde(default)]
    pub candidates: Vec<OrganizationCandidate>,
    pub status: OrganizationPlanStatus,
}

impl RumilPacket for OrganizationPlan {
    fn kind() -> &'static str {
        KIND_ORGANIZATION_PLAN
    }
}

/// Revision relation between current and prior audit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RevisionRelation {
    Same,
    Ahead,
    Behind,
    Diverged,
    Unknown,
}

/// Compare a current audit to a prior audit for the same project identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Comparison {
    pub comparison_id: uuid::Uuid,
    pub current_audit_id: uuid::Uuid,
    pub prior_audit_id: uuid::Uuid,
    pub identity_match: bool,
    pub revision_relation: RevisionRelation,
    #[serde(default)]
    pub new_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub persistent_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub changed_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub resolved_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub stale_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub unverifiable_findings: Vec<uuid::Uuid>,
    #[serde(default)]
    pub baseline_warnings: Vec<String>,
}

impl RumilPacket for Comparison {
    fn kind() -> &'static str {
        KIND_COMPARISON
    }
}

/// Bounded handoff to `arda-vaire`. Contains only summary data, no raw source excerpts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryObservation {
    pub observation_id: uuid::Uuid,
    pub source_audit_id: uuid::Uuid,
    pub project_id: uuid::Uuid,
    pub source_revision: Option<String>,
    pub summary: String,
    pub completeness: String,
    #[serde(default)]
    pub finding_counts: std::collections::BTreeMap<String, u64>,
    pub comparison_digest: Option<String>,
    #[serde(default)]
    pub receipt_refs: Vec<String>,
    #[serde(default)]
    pub eligible_tags: Vec<String>,
    pub retention_class: String,
    pub provenance: String,
}

/// Preserve historical HADES evidence during migration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyHadesImport {
    pub import_id: uuid::Uuid,
    #[serde(default = "default_legacy_hades_source")]
    pub legacy_source: String,
    pub legacy_contract: String,
    pub legacy_path: String,
    pub legacy_sha256: String,
    pub legacy_generated_at_utc: chrono::DateTime<Utc>,
    pub mapped_project_id: uuid::Uuid,
    pub mapped_rumil_audit_id: uuid::Uuid,
    pub mapping_quality: String,
    #[serde(default)]
    pub unmapped_fields: Vec<String>,
    /// Always true — these are historical, not native Rúmil evidence.
    pub historical_only: bool,
    /// Always false — these are not native Rúmil contracts.
    pub native_rumil_evidence: bool,
}

fn default_legacy_hades_source() -> String {
    "hades".to_string()
}

/// Serialize a Rúmil packet as a JSON object with an outer `kind`/`payload` envelope.
///
/// This mirrors the plan's "envelope" convention: the packet struct is nestled
/// under `payload`, with the canonical kind string at `kind` and the current
/// schema version at `schema_version`.
pub fn serialize_packet<T: RumilPacket + serde::Serialize>(
    packet: &T,
) -> crate::error::Result<String> {
    let payload = serde_json::to_value(packet)?;
    let envelope = serde_json::json!({
        "kind": T::kind(),
        "schema_version": CURRENT_SCHEMA_VERSION,
        "payload": payload,
    });
    serde_json::to_string(&envelope).map_err(Into::into)
}

/// Deserialize an envelope string back into a typed packet, validating the kind tag.
pub fn deserialize_packet<T>(raw: &str, expected_kind: &str) -> crate::error::Result<T>
where
    T: RumilPacket + serde::de::DeserializeOwned,
{
    let value: serde_json::Value = serde_json::from_str(raw)?;
    validate_packet_envelope(&value)?;
    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::RumilError::PacketValidation("missing kind field".into()))?;
    if kind != expected_kind {
        return Err(crate::error::RumilError::PacketValidation(format!(
            "kind mismatch: expected {expected_kind}, got {kind}"
        )));
    }
    let payload = value.get("payload").ok_or_else(|| {
        crate::error::RumilError::PacketValidation("missing payload field".into())
    })?;
    serde_json::from_value(payload.clone()).map_err(Into::into)
}

/// Validate an envelope: check kind, schema_version, and required common fields.
pub fn validate_packet_envelope(value: &serde_json::Value) -> crate::error::Result<()> {
    let object = value.as_object().ok_or_else(|| {
        crate::error::RumilError::PacketValidation("packet envelope must be an object".into())
    })?;
    if let Some(field) = object
        .keys()
        .find(|field| !matches!(field.as_str(), "kind" | "schema_version" | "payload"))
    {
        return Err(crate::error::RumilError::PacketValidation(format!(
            "unknown envelope field: {field}"
        )));
    }
    let kind = value
        .get("kind")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::error::RumilError::PacketValidation("missing kind field".into()))?;
    if !kind.starts_with("arda.rumil.") {
        return Err(crate::error::RumilError::PacketValidation(format!(
            "kind does not start with arda.rumil.: {kind}"
        )));
    }
    let version = value
        .get("schema_version")
        .or_else(|| value.get("payload").and_then(|p| p.get("schema_version")))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::RumilError::PacketValidation("missing schema_version".into())
        })?;
    if version != CURRENT_SCHEMA_VERSION {
        return Err(crate::error::RumilError::UnsupportedVersion(
            version.to_string(),
        ));
    }
    if !value
        .get("payload")
        .is_some_and(serde_json::Value::is_object)
    {
        return Err(crate::error::RumilError::PacketValidation(
            "packet envelope payload must be an object".into(),
        ));
    }
    Ok(())
}
