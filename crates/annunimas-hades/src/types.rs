// sigil: REPAIR
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SigilState {
    Ankh,
    Eye,
    Scroll,
    Coin,
    Repair,
    OrphanTemp,
    Quarantine,
    Condemned,
    Archived,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    InvestigateOrphan,
    Remove,
    Archive,
    Quarantine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskItem {
    pub task_id: String,
    pub queued_at_utc: String,
    pub action: ActionKind,
    pub file: String,
    pub authorized_by: Option<String>,
    pub reason: String,
    pub execute_after_utc: Option<String>,
    pub quorum_proof: Option<QuorumProof>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumProof {
    pub approvers: Vec<String>,
    pub evidence: Vec<String>,
    pub asserted_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepResult {
    pub sweep_type: String,
    pub started_at_utc: String,
    pub completed_at_utc: String,
    pub files_scanned: usize,
    pub actions_taken: usize,
    pub orphans_found: usize,
    pub held_for_review: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRecord {
    pub ts: String,
    pub event: String,
    pub file: Option<String>,
    #[serde(default)]
    pub sigil_code: Option<String>,
    #[serde(default)]
    pub sigil_tags: Vec<String>,
    #[serde(default)]
    pub sigil_retention: Option<String>,
    #[serde(default)]
    pub sigil_source: Option<String>,
    pub details: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SigilVacuumRule {
    #[serde(default)]
    pub code_regex: Option<String>,
    #[serde(default)]
    pub retention: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanLifecycleReviewItem {
    pub contract: String,
    pub review_id: String,
    pub queued_at_utc: String,
    pub source_contract: String,
    pub source_path: String,
    pub content_hash: String,
    pub detected_status: String,
    pub detected_authority: String,
    pub source_type: String,
    pub severity: String,
    pub lifecycle_action: String,
    pub allowed_actions: Vec<String>,
    pub evidence: serde_json::Value,
    pub review_required: bool,
    pub destructive_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HumanLifecycleImportReport {
    pub contract: String,
    pub source_path: String,
    pub queue_path: String,
    pub scanned_total: usize,
    pub queued_total: usize,
    pub skipped_total: usize,
    pub malformed_total: usize,
    pub generated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleAuditFinding {
    pub finding_id: String,
    pub finding_type: String,
    pub lifecycle_class: String,
    pub path: String,
    pub severity: String,
    pub recommendation: String,
    pub evidence: serde_json::Value,
    pub review_required: bool,
    pub destructive_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleAuditReport {
    pub contract: String,
    pub generated_at_utc: String,
    pub root_path: String,
    pub findings_total: usize,
    pub stale_plan_total: usize,
    pub archive_candidate_total: usize,
    pub task_queue_hygiene_total: usize,
    pub scanned_files_total: usize,
    pub no_delete: bool,
    pub findings: Vec<LifecycleAuditFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesQueueStats {
    pub path: String,
    pub line_count: usize,
    pub sha256: String,
    pub retention_required: bool,
    pub clear_allowed: bool,
    pub archive_allowed_without_operator_approval: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesEvidenceArtifact {
    pub contract: String,
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesEvidenceArtifactRecord {
    pub contract: String,
    pub generated_at_utc: String,
    pub review_id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_queue_line: Option<usize>,
    pub record: serde_json::Value,
    pub record_sha256: String,
    pub mutation_authorized: bool,
    pub destructive_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesQueueOutcome {
    pub status: String,
    pub recommended_next_action: String,
    pub mutation_gate: String,
    pub evidence_required: Vec<String>,
    pub append_only_closeout: bool,
    pub destructive_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesReviewItem {
    pub review_id: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_queue_line: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_type: Option<String>,
    pub severity: String,
    pub classification: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finding_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_disposition: Option<String>,
    pub record: serde_json::Value,
    pub evidence_artifact: WardenHadesEvidenceArtifact,
    pub outcome: WardenHadesQueueOutcome,
    pub approval_status: String,
    pub review_required: bool,
    pub destructive_allowed: bool,
    pub allowed_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenHadesReviewPacket {
    pub contract: String,
    pub generated_at_utc: String,
    pub root_path: String,
    pub packet_path: String,
    pub review_queue_path: String,
    pub evidence_dir: String,
    pub markdown_summary_path: String,
    pub raw_queue: WardenHadesQueueStats,
    pub raw_queue_retained: bool,
    pub policy_report_path: serde_json::Value,
    pub policy_report_contract: serde_json::Value,
    pub review_items_total: usize,
    pub review_items: Vec<WardenHadesReviewItem>,
    pub operator_decision_required: bool,
    pub packet_is_authorization: bool,
    pub clear_archive_allowed: bool,
    pub delete_allowed: bool,
    pub move_allowed: bool,
    pub archive_allowed: bool,
    pub requires_explicit_operator_approval_for_any_mutation: bool,
    pub no_file_moves_or_deletes_performed: bool,
}
