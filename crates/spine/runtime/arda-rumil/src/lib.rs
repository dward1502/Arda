//! # arda-rumil
//!
//! Project audit and organization coordinator for Arda.
//!
//! `arda-rumil` is a **review-only** audit coordinator. It inspects bounded
//! project trees through versioned adapters, normalizes tool output into
//! versioned evidence packets (`arda.rumil.*`), compares current state against
//! prior observations, and produces **review-only** organization plans.
//!
//! ## Authority invariants
//!
//! - Rúmil observes and coordinates; it does not approve or execute destructive changes.
//! - Every scan is bounded by root policy, path exclusions, file/byte/token limits, timeout, and a tool allowlist.
//! - Every command is represented by a receipt with argv digest, working directory, exit status, output digests, truncation, and authority class.
//! - Every finding links to evidence or explicitly states that evidence is unavailable.
//! - Organization output is a dry-run plan by default. Moves, archives, rewrites, and deletes require a separate operator/governance path.
//!
//! ## Status: RUMIL-0 through RUMIL-8 baseline
//!
//! The crate exposes the versioned contract envelope and a generic bounded
//! inventory adapter plus opt-in Cargo-workspace, read-only Git-state, and
//! policy-gated analysis providers. Findings and selected historical baselines
//! are normalized and compared deterministically. Organization planning is
//! profile-gated, deterministic, and review-only. Warden consumes bounded audit
//! requests directly through `arda-outpost-scout`, while Vairë receives only a
//! compact advisory receipt projection. Bounded references are classified by
//! Mandos, evaluated without execution authority by Varda, and projected into
//! Workbench/HUD research briefs. Declarative profiles cover Arda, Rust, Node,
//! Python, and mixed roots, with explicit host/Pi execution boundaries and a
//! provenance-preserving historical HADES import.

pub mod adapters;
pub mod baseline;
pub mod comparison;
pub mod constants;
pub mod contracts;
pub mod error;
pub mod evaluation;
pub mod findings;
#[cfg(feature = "crypto")]
pub mod hash;
pub mod inventory;
pub mod organization;
pub mod policy;
pub mod profile;
#[cfg(feature = "provider")]
pub mod providers;
pub mod tree;

pub use baseline::{build_memory_observation, AuditBaseline};
#[cfg(feature = "crypto")]
pub use baseline::{import_legacy_hades_findings, LegacyHadesBaselineImport};
pub use comparison::compare_baselines;
pub use constants::{
    CONTRACT_DOMAIN, CONTRACT_VERSION_MAJOR, CURRENT_SCHEMA_VERSION, KIND_AUDIT_REPORT,
    KIND_AUDIT_REQUEST, PROVIDER_COMPLETED, PROVIDER_DENIED_BY_BUDGET, PROVIDER_FAILED,
    PROVIDER_MALFORMED_OUTPUT, PROVIDER_SKIPPED_BY_POLICY, PROVIDER_TIMED_OUT,
    PROVIDER_UNAVAILABLE, RUMIL_SCHEMA_VERSION,
};
pub use contracts::{
    deserialize_packet, serialize_packet, validate_packet_envelope, AuditReport,
    AuditReportCompleteness, AuditRequest, CapabilityOutcome, CommandReceipt, CommandReceiptStatus,
    Comparison, FileRecord, FileRecordKind, Finding, FindingConfidenceClass, FindingSeverity,
    FindingStatus, InventorySummary, LegacyHadesImport, MemoryObservation, OrganizationCandidate,
    OrganizationPlan, OrganizationPlanStatus, OrganizationRisk, PacketEnvelope, RedactionState,
    RevisionRelation, RootIdentity as ContractRootIdentity, RumilPacket,
};
pub use error::{EnvelopeProbe, PacketKind, Result, RumilError};
pub use evaluation::{project_evidence_reference, RumilEvidenceClass, RumilEvidenceReference};
pub use findings::{normalize_finding, FindingDisposition, FindingDraft, FindingFeedback};
pub use inventory::{inventory_repo, InventoryConfig, InventoryReport, TreeWalker};
#[cfg(feature = "crypto")]
pub use organization::import_legacy_hades_organization_report;
pub use organization::{
    plan_organization, MutationHandoffBoundary, OrganizationDryRunReceipt, OrganizationIssueKind,
    OrganizationObservation, OrganizationPlanBundle, OrganizationProfile, OrganizationRule,
};
pub use policy::{
    AuditPolicy, BudgetPolicy, ExclusionKind, ExclusionRule, ProjectPolicy, RootIdentity,
};
pub use profile::{
    audit_with_profile, builtin_profile, validate_execution_target, ExecutionTarget,
    ProfileInventory, ProfileOrganization, ProfileRetention, ProjectKind, ProjectProfile,
};
pub use tree::{TreeEntry, TreeEntryKind};
