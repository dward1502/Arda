//! Direct Warden consumer for bounded, review-only Rúmil audits.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use arda_outpost_protocol::{
    AuthorityClass, ObservationClassification, ObservationScope, OutpostObservation,
};
use arda_rumil::{
    inventory_repo, AuditPolicy, AuditReport, AuditReportCompleteness, AuditRequest, BudgetPolicy,
    CapabilityOutcome, ContractRootIdentity, ExclusionKind, ExclusionRule, FileRecord,
    InventorySummary, RootIdentity,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

const ADVISORY_AUTHORITY: &str = "advisory_read_only";
const ROOT_POLICY: &str = "bounded_request_root";
const INVENTORY_CAPABILITY: &str = "inventory";
const MAX_FOLLOWUP_RECORDS: usize = 100;

#[derive(Debug, Error)]
pub enum ScoutAuditError {
    #[error("audit request rejected: {0}")]
    Rejected(String),
    #[error("audit packet not found: {0}")]
    NotFound(Uuid),
    #[error("audit I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("audit serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Rúmil audit failed: {0}")]
    Rumil(#[from] arda_rumil::RumilError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScoutAuditRequest {
    /// Relative project root beneath the Warden runtime's configured root.
    pub root: PathBuf,
    pub project_name: String,
    pub project_kind: String,
    pub remote_url: Option<String>,
    pub request: AuditRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedAuditPacket {
    pub report: AuditReport,
    pub file_records: Vec<FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditSummary {
    pub completeness: AuditReportCompleteness,
    pub inventory: InventorySummary,
    pub warning_count: usize,
    pub error_count: usize,
    pub truncation_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoutAuditOutcome {
    pub report: AuditReport,
    pub packet_path: String,
    pub packet_sha256: String,
    pub observation: OutpostObservation,
    pub replayed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditFollowupSection {
    Summary,
    Warnings,
    Exclusions,
    FileRecords,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditFollowupRequest {
    pub audit_id: Uuid,
    pub sections: Vec<AuditFollowupSection>,
    pub path_prefix: Option<String>,
    pub file_record_limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditFollowupResponse {
    pub audit_id: Uuid,
    pub summary: Option<AuditSummary>,
    pub warnings: Vec<String>,
    pub exclusions: Vec<String>,
    pub file_records: Vec<FileRecord>,
    pub truncated: bool,
    pub authority: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditReceipt {
    request_id: Uuid,
    request_sha256: String,
    audit_id: Uuid,
    packet_path: String,
    packet_sha256: String,
    completeness: AuditReportCompleteness,
    completed_at_utc: DateTime<Utc>,
    authority: String,
}

#[derive(Debug, Clone)]
pub struct ScoutAuditService {
    runtime_root: PathBuf,
    source: String,
}

impl ScoutAuditService {
    pub fn new(runtime_root: impl Into<PathBuf>, source: impl Into<String>) -> Self {
        Self {
            runtime_root: runtime_root.into(),
            source: source.into(),
        }
    }

    pub fn execute(
        &self,
        request: ScoutAuditRequest,
        now: DateTime<Utc>,
    ) -> Result<ScoutAuditOutcome, ScoutAuditError> {
        self.validate_request(&request, now)?;
        let request_sha256 = arda_rumil::hash::sha256_bytes(&serde_json::to_vec(&request)?);
        if let Some(receipt) = self.receipt_for_request(request.request.request_id)? {
            if receipt.request_sha256 != request_sha256 {
                return Err(ScoutAuditError::Rejected(
                    "request_id was already used for a different audit request".to_string(),
                ));
            }
            return self.load_outcome(&receipt, true);
        }

        let project_root = self.resolve_project_root(&request.root)?;
        let policy = audit_policy(&request);
        let inventory = inventory_repo(&project_root, &policy)?;
        let file_records = inventory.file_records();
        let unavailable_capabilities = request
            .request
            .requested_capabilities
            .iter()
            .filter(|capability| capability.as_str() != INVENTORY_CAPABILITY)
            .map(|capability| CapabilityOutcome {
                capability: capability.clone(),
                status: "unavailable".to_string(),
                provider_id: None,
                detail: Some(
                    "Warden direct consumer currently supports bounded inventory only".to_string(),
                ),
            })
            .collect::<Vec<_>>();
        let completeness = if inventory.is_complete() && unavailable_capabilities.is_empty() {
            AuditReportCompleteness::Complete
        } else {
            AuditReportCompleteness::Partial
        };
        let audit_id = Uuid::new_v5(
            &request.request.project_id,
            request.request.request_id.as_bytes(),
        );
        let packet_path = format!("data/warden/rumil_audits/{audit_id}.json");
        let report = AuditReport {
            audit_id,
            project_id: request.request.project_id,
            project_kind: request.project_kind.clone(),
            root_identity: ContractRootIdentity {
                project_id: request.request.project_id,
                name: request.project_name.clone(),
                kind: request.project_kind.clone(),
                remote_url: request.remote_url.clone(),
            },
            source_revision: request.request.source_revision_expectation.clone(),
            profile_id: request.request.profile_id.clone(),
            generated_at_utc: now,
            completed_at_utc: Some(now),
            completeness,
            capabilities_requested: request.request.requested_capabilities.clone(),
            capabilities_completed: vec![CapabilityOutcome {
                capability: INVENTORY_CAPABILITY.to_string(),
                status: if completeness.is_complete() {
                    "completed".to_string()
                } else {
                    "partial".to_string()
                },
                provider_id: Some("arda-rumil.generic-inventory".to_string()),
                detail: None,
            }],
            capabilities_unavailable: unavailable_capabilities,
            inventory_summary: inventory.summary(),
            tree_digest: Some(arda_rumil::hash::sha256_bytes(&serde_json::to_vec(
                &file_records,
            )?)),
            file_record_references: vec![format!("{packet_path}#file_records")],
            package_records: serde_json::Value::Null,
            module_records: serde_json::Value::Null,
            dependency_graph_reference: None,
            command_receipts: Vec::new(),
            finding_references: Vec::new(),
            organization_plan_reference: None,
            comparison_reference: None,
            exclusions: inventory.exclusion_summary,
            truncation: inventory.truncation_reasons,
            warnings: Vec::new(),
            errors: Vec::new(),
            authority: ADVISORY_AUTHORITY.to_string(),
        };
        let packet = PersistedAuditPacket {
            report: report.clone(),
            file_records,
        };
        let packet_bytes = serde_json::to_vec_pretty(&packet)?;
        let packet_sha256 = arda_rumil::hash::sha256_bytes(&packet_bytes);
        self.persist_packet(&packet_path, &packet_bytes)?;
        let receipt = AuditReceipt {
            request_id: request.request.request_id,
            request_sha256,
            audit_id,
            packet_path: packet_path.clone(),
            packet_sha256: packet_sha256.clone(),
            completeness,
            completed_at_utc: now,
            authority: ADVISORY_AUTHORITY.to_string(),
        };
        self.append_receipt(&receipt)?;

        Ok(ScoutAuditOutcome {
            observation: self.build_observation(&report, &packet_path, &packet_sha256),
            report,
            packet_path,
            packet_sha256,
            replayed: false,
        })
    }

    pub fn followup(
        &self,
        request: AuditFollowupRequest,
    ) -> Result<AuditFollowupResponse, ScoutAuditError> {
        if request.sections.is_empty() || request.sections.len() > 4 {
            return Err(ScoutAuditError::Rejected(
                "follow-up must select between one and four sections".to_string(),
            ));
        }
        if request.file_record_limit == 0 || request.file_record_limit > MAX_FOLLOWUP_RECORDS {
            return Err(ScoutAuditError::Rejected(format!(
                "file_record_limit must be between 1 and {MAX_FOLLOWUP_RECORDS}"
            )));
        }
        let prefix = request
            .path_prefix
            .as_deref()
            .map(validate_relative_prefix)
            .transpose()?;
        let receipt = self
            .receipt_for_audit(request.audit_id)?
            .ok_or(ScoutAuditError::NotFound(request.audit_id))?;
        let packet = self.load_packet(&receipt.packet_path)?;
        let selected = |section| request.sections.contains(&section);
        let matching = packet
            .file_records
            .iter()
            .filter(|record| {
                prefix
                    .as_deref()
                    .is_none_or(|prefix| path_matches(&record.path, prefix))
            })
            .collect::<Vec<_>>();
        let truncated = selected(AuditFollowupSection::FileRecords)
            && matching.len() > request.file_record_limit;
        let file_records = if selected(AuditFollowupSection::FileRecords) {
            matching
                .into_iter()
                .take(request.file_record_limit)
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        Ok(AuditFollowupResponse {
            audit_id: request.audit_id,
            summary: selected(AuditFollowupSection::Summary)
                .then(|| AuditSummary::from_report(&packet.report)),
            warnings: if selected(AuditFollowupSection::Warnings) {
                packet.report.warnings.clone()
            } else {
                Vec::new()
            },
            exclusions: if selected(AuditFollowupSection::Exclusions) {
                packet.report.exclusions.clone()
            } else {
                Vec::new()
            },
            file_records,
            truncated,
            authority: ADVISORY_AUTHORITY.to_string(),
        })
    }

    fn validate_request(
        &self,
        request: &ScoutAuditRequest,
        now: DateTime<Utc>,
    ) -> Result<(), ScoutAuditError> {
        if request.request.expires_at_utc <= now {
            return Err(ScoutAuditError::Rejected("request has expired".to_string()));
        }
        if request.request.authority != ADVISORY_AUTHORITY {
            return Err(ScoutAuditError::Rejected(
                "request authority must be advisory_read_only".to_string(),
            ));
        }
        if request.request.root_policy != ROOT_POLICY {
            return Err(ScoutAuditError::Rejected(
                "unsupported root policy".to_string(),
            ));
        }
        if !request
            .request
            .requested_capabilities
            .iter()
            .any(|capability| capability == INVENTORY_CAPABILITY)
        {
            return Err(ScoutAuditError::Rejected(
                "inventory capability is required".to_string(),
            ));
        }
        if request.request.file_count_budget == 0
            || request.request.byte_budget == 0
            || request.request.source_excerpt_budget == 0
            || request.request.command_timeout_seconds == 0
        {
            return Err(ScoutAuditError::Rejected(
                "all audit budgets must be non-zero".to_string(),
            ));
        }
        if request.project_name.trim().is_empty() || request.project_kind.trim().is_empty() {
            return Err(ScoutAuditError::Rejected(
                "project identity fields must be non-empty".to_string(),
            ));
        }
        validate_relative_path(&request.root)
    }

    fn resolve_project_root(&self, root: &Path) -> Result<PathBuf, ScoutAuditError> {
        let runtime_root = self.runtime_root.canonicalize()?;
        let project_root = runtime_root.join(root).canonicalize()?;
        if !project_root.starts_with(&runtime_root) || !project_root.is_dir() {
            return Err(ScoutAuditError::Rejected(
                "project root escapes the configured Warden root".to_string(),
            ));
        }
        Ok(project_root)
    }

    fn build_observation(
        &self,
        report: &AuditReport,
        packet_path: &str,
        packet_sha256: &str,
    ) -> OutpostObservation {
        OutpostObservation::new(
            self.source.clone(),
            ObservationScope::Custom("rumil_audit_receipt".to_string()),
            ObservationClassification::DerivedEstimate,
            AuthorityClass::Advisory,
            serde_json::json!({
                "audit_id": report.audit_id,
                "project_id": report.project_id,
                "project_kind": report.project_kind,
                "completeness": report.completeness,
                "inventory_summary": report.inventory_summary,
                "packet_reference": packet_path,
                "packet_sha256": packet_sha256,
                "warning_count": report.warnings.len(),
                "error_count": report.errors.len(),
                "truncation_count": report.truncation.len(),
                "authority": ADVISORY_AUTHORITY,
            }),
        )
        .with_confidence(if report.completeness.is_complete() {
            0.9
        } else {
            0.6
        })
        .with_provenance(format!("arda-rumil://audit/{}", report.audit_id))
        .local_only()
    }

    fn receipt_for_request(
        &self,
        request_id: Uuid,
    ) -> Result<Option<AuditReceipt>, ScoutAuditError> {
        Ok(self
            .receipts()?
            .into_iter()
            .find(|receipt| receipt.request_id == request_id))
    }

    fn receipt_for_audit(&self, audit_id: Uuid) -> Result<Option<AuditReceipt>, ScoutAuditError> {
        Ok(self
            .receipts()?
            .into_iter()
            .find(|receipt| receipt.audit_id == audit_id))
    }

    fn receipts(&self) -> Result<Vec<AuditReceipt>, ScoutAuditError> {
        let path = self.receipt_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        fs::read_to_string(path)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| serde_json::from_str(line).map_err(ScoutAuditError::from))
            .collect()
    }

    fn append_receipt(&self, receipt: &AuditReceipt) -> Result<(), ScoutAuditError> {
        let path = self.receipt_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        serde_json::to_writer(&mut file, receipt)?;
        file.write_all(b"\n")?;
        file.sync_data()?;
        Ok(())
    }

    fn receipt_path(&self) -> PathBuf {
        self.runtime_root
            .join("data/warden/rumil_audit_receipts.jsonl")
    }

    fn persist_packet(&self, relative: &str, bytes: &[u8]) -> Result<(), ScoutAuditError> {
        let path = self.runtime_root.join(relative);
        let parent = path
            .parent()
            .ok_or_else(|| ScoutAuditError::Rejected("packet path has no parent".to_string()))?;
        fs::create_dir_all(parent)?;
        let temporary = path.with_extension("json.tmp");
        fs::write(&temporary, bytes)?;
        fs::rename(temporary, path)?;
        Ok(())
    }

    fn load_packet(&self, relative: &str) -> Result<PersistedAuditPacket, ScoutAuditError> {
        validate_relative_path(Path::new(relative))?;
        Ok(serde_json::from_slice(&fs::read(
            self.runtime_root.join(relative),
        )?)?)
    }

    fn load_outcome(
        &self,
        receipt: &AuditReceipt,
        replayed: bool,
    ) -> Result<ScoutAuditOutcome, ScoutAuditError> {
        let packet = self.load_packet(&receipt.packet_path)?;
        let digest = arda_rumil::hash::sha256_bytes(&fs::read(
            self.runtime_root.join(&receipt.packet_path),
        )?);
        if digest != receipt.packet_sha256 {
            return Err(ScoutAuditError::Rejected(
                "stored audit packet digest does not match its receipt".to_string(),
            ));
        }
        Ok(ScoutAuditOutcome {
            observation: self.build_observation(
                &packet.report,
                &receipt.packet_path,
                &receipt.packet_sha256,
            ),
            report: packet.report,
            packet_path: receipt.packet_path.clone(),
            packet_sha256: receipt.packet_sha256.clone(),
            replayed,
        })
    }
}

impl AuditSummary {
    fn from_report(report: &AuditReport) -> Self {
        Self {
            completeness: report.completeness,
            inventory: report.inventory_summary.clone(),
            warning_count: report.warnings.len(),
            error_count: report.errors.len(),
            truncation_count: report.truncation.len(),
        }
    }
}

fn audit_policy(request: &ScoutAuditRequest) -> AuditPolicy {
    AuditPolicy {
        profile_id: request.request.profile_id.clone(),
        root_identity: RootIdentity {
            project_id: request.request.project_id,
            name: request.project_name.clone(),
            kind: request.project_kind.clone(),
            remote_url: request.remote_url.clone(),
        },
        root_relative: ".".to_string(),
        exclusion_rules: request
            .request
            .path_exclusions
            .iter()
            .map(|pattern| ExclusionRule {
                pattern: pattern.clone(),
                kind: if pattern.contains('*') || pattern.contains('?') {
                    ExclusionKind::Glob
                } else {
                    ExclusionKind::Directory
                },
            })
            .collect(),
        budget: BudgetPolicy {
            max_depth: 12,
            max_files: request.request.file_count_budget,
            max_total_bytes: request.request.byte_budget,
            max_excerpt_bytes: request.request.source_excerpt_budget,
            scan_timeout_seconds: request.request.command_timeout_seconds,
            command_timeout_seconds: request.request.command_timeout_seconds,
        },
        provider_allowlist: request.request.provider_allowlist.clone(),
        redaction_policy: request.request.redaction_policy.clone(),
    }
}

fn validate_relative_path(path: &Path) -> Result<(), ScoutAuditError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(ScoutAuditError::Rejected(
            "root must be a non-empty relative path without parent traversal".to_string(),
        ));
    }
    Ok(())
}

fn validate_relative_prefix(prefix: &str) -> Result<String, ScoutAuditError> {
    let trimmed = prefix.trim_matches('/');
    validate_relative_path(Path::new(trimmed))?;
    Ok(trimmed.replace('\\', "/"))
}

fn path_matches(path: &str, prefix: &str) -> bool {
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}
