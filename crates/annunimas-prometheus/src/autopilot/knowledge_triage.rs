// sigil: REPAIR
//! Read-only knowledge triage foundation for PROMETHEUS autopilot.
//!
//! This module discovers broad human/docs knowledge sources, classifies them
//! deterministically, and emits reviewable local artifacts only when explicitly
//! requested. It does not mutate the canonical task queue.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

pub const KNOWLEDGE_TRIAGE_SCHEMA: &str = "annunimas.knowledge_triage.v1";
pub const KNOWLEDGE_SOURCE_INVENTORY_SCHEMA: &str = "annunimas.knowledge_source_inventory.v1";
pub const KNOWLEDGE_ACTIONABLE_REVIEW_SCHEMA: &str = "annunimas.knowledge_actionable_review.v1";
pub const KNOWLEDGE_TASK_PROMOTION_RECEIPT_SCHEMA: &str =
    "annunimas.knowledge_task_promotion_receipt.v1";
pub const KNOWLEDGE_TASK_EXECUTION_RECEIPT_SCHEMA: &str =
    "annunimas.knowledge_task_execution_receipt.v1";
pub const KNOWLEDGE_ACTIONABLE_REVIEW_GATE: &str =
    "human_review_required_before_task_queue_mutation";
pub const KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE: &str = "prometheus_safe_local_task_promotion";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeClassification {
    OperationalSignal,
    ArchitectureReference,
    PlanOrBenchmark,
    MemorySeed,
    PersonalKnowledge,
    ArchiveNoise,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceDomain {
    Human,
    Docs,
    DocsPlans,
    Eregion,
    CoreState,
    ExternalFuture,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentType {
    PersonalNote,
    ProjectNote,
    BusinessPlan,
    ArchitectureDoc,
    CompletedPlan,
    ActivePlan,
    StalePlan,
    ReferenceMaterial,
    TaskCandidate,
    Research,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessRelevance {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyRelevance {
    None,
    ContextOnly,
    PlanningInput,
    TaskSource,
    ExecutionInstruction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageStatus {
    Unknown,
    Active,
    Completed,
    Blocked,
    Stale,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskClass {
    SafeLocal,
    NeedsReview,
    ExternalSideEffect,
    Financial,
    CredentialSensitive,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecommendedAction {
    IgnoreForNow,
    PreserveAsContext,
    Summarize,
    LinkToProject,
    CreateTask,
    UpdateExistingTask,
    RequestHumanDecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLane {
    ObserveOnly,
    CandidateOnly,
    AutoCreateInternalTask,
    AutoExecuteSafeLocalTask,
    HumanApprovalRequired,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PromotionDecision {
    ContextOnly,
    CandidateOnly,
    AutoPromoteInternalTask,
    HumanReviewRequired,
    Blocked,
    DuplicateSkipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoterionMapping {
    pub sigil: String,
    pub glyph: String,
    pub retention: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeSourceInventoryRecord {
    pub schema_version: String,
    pub path: PathBuf,
    pub source_root: PathBuf,
    pub bytes: u64,
    pub sha256_12: String,
    pub read_only_discovery: bool,
    pub discovered_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTriageRecord {
    pub schema_version: String,
    pub path: PathBuf,
    pub source_path: PathBuf,
    pub title: String,
    pub classification: KnowledgeClassification,
    pub source_domain: SourceDomain,
    pub content_type: ContentType,
    pub business_relevance: BusinessRelevance,
    pub autonomy_relevance: AutonomyRelevance,
    pub status: TriageStatus,
    pub risk_class: RiskClass,
    pub recommended_action: RecommendedAction,
    pub evidence: Vec<String>,
    pub confidence: Confidence,
    pub autonomy_lane: AutonomyLane,
    pub requires_human: bool,
    pub promotion_decision: PromotionDecision,
    pub promotion_reason: String,
    pub dedupe_key: String,
    pub source_excerpt: String,
    pub source_line_range: Option<(usize, usize)>,
    pub created_task_id: Option<String>,
    pub actionable_operational_signal: bool,
    pub mutate_task_queue: bool,
    pub soterion: SoterionMapping,
    pub canonical_home: String,
    pub domain: String,
    pub authority: String,
    pub recommended_action_label: String,
    pub rationale: String,
    pub headings: Vec<String>,
    pub bytes: u64,
    pub sha256_12: String,
    pub triaged_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeActionableReviewRecord {
    pub schema_version: String,
    pub source_path: PathBuf,
    pub title: String,
    pub classification: KnowledgeClassification,
    pub source_domain: SourceDomain,
    pub status: TriageStatus,
    pub risk_class: RiskClass,
    pub recommended_action: RecommendedAction,
    pub recommended_action_label: String,
    pub confidence: Confidence,
    pub evidence: Vec<String>,
    pub review_gate: String,
    pub triad_required: bool,
    pub mutate_task_queue: bool,
    pub rationale: String,
    pub triaged_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTaskPromotionReceipt {
    pub schema_version: String,
    pub source_path: PathBuf,
    pub title: String,
    pub dedupe_key: String,
    pub autonomy_lane: AutonomyLane,
    pub promotion_decision: PromotionDecision,
    pub requires_human: bool,
    pub risk_class: RiskClass,
    pub confidence: Confidence,
    pub source_excerpt: String,
    pub source_line_range: Option<(usize, usize)>,
    pub created_task_id: Option<String>,
    pub approval_evidence: Option<String>,
    pub receipt_reason: String,
    pub written_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTaskPromotionReport {
    pub schema_version: String,
    pub dry_run: bool,
    pub queue_path: PathBuf,
    pub receipt_path: PathBuf,
    pub candidates_seen: usize,
    pub tasks_created: usize,
    pub human_review_required: usize,
    pub duplicates_skipped: usize,
    pub queue_mutation_authorized: bool,
    pub approval_evidence_required: bool,
    pub approval_evidence_supplied: bool,
    pub artifacts_written: bool,
    pub receipts: Vec<KnowledgeTaskPromotionReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeExecutionDecision {
    EligibleForArandur,
    DryRunEligible,
    HumanReviewRequired,
    Blocked,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTaskExecutionReceipt {
    pub schema_version: String,
    pub task_id: String,
    pub title: String,
    pub source_path: Option<PathBuf>,
    pub dedupe_key: Option<String>,
    pub autonomy_lane: Option<String>,
    pub execution_decision: KnowledgeExecutionDecision,
    pub requires_human: bool,
    pub receipt_reason: String,
    pub written_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTaskExecutionReport {
    pub schema_version: String,
    pub dry_run: bool,
    pub queue_path: PathBuf,
    pub receipt_path: PathBuf,
    pub tasks_seen: usize,
    pub eligible_for_arandur: usize,
    pub human_review_required: usize,
    pub blocked: usize,
    pub artifacts_written: bool,
    pub receipts: Vec<KnowledgeTaskExecutionReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct KnowledgeTriageReport {
    pub schema_version: String,
    pub dry_run: bool,
    pub read_only: bool,
    pub source_roots: Vec<PathBuf>,
    pub inventory_path: PathBuf,
    pub registry_path: PathBuf,
    pub review_queue_path: PathBuf,
    pub sources_discovered: usize,
    pub records_classified: usize,
    pub actionable_operational_signals: usize,
    pub counts_by_domain: BTreeMap<String, usize>,
    pub counts_by_status: BTreeMap<String, usize>,
    pub counts_by_recommended_action: BTreeMap<String, usize>,
    pub artifacts_written: bool,
    pub inventory_records: Vec<KnowledgeSourceInventoryRecord>,
    pub registry_records: Vec<KnowledgeTriageRecord>,
    pub actionable_review_records: Vec<KnowledgeActionableReviewRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgeTriageConfig {
    pub root: PathBuf,
    pub source_roots: Vec<PathBuf>,
    pub inventory_path: PathBuf,
    pub registry_path: PathBuf,
    pub review_queue_path: PathBuf,
    pub promotion_receipts_path: PathBuf,
    pub execution_receipts_path: PathBuf,
    pub task_queue_path: PathBuf,
    pub dry_run: bool,
    pub approval_evidence: Option<String>,
    pub max_file_bytes: u64,
}

impl KnowledgeTriageConfig {
    pub fn for_root(root: impl AsRef<Path>) -> Self {
        let root = root.as_ref().to_path_buf();
        let home = root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.clone());
        Self {
            source_roots: vec![
                root.join("human"),
                root.join("docs"),
                root.join("docs/plans"),
                home.join("Eregion"),
            ],
            inventory_path: root.join("core/knowledge/source_inventory.jsonl"),
            registry_path: root.join("core/knowledge/triage_registry.jsonl"),
            review_queue_path: root.join("core/knowledge/actionable_review_queue.jsonl"),
            promotion_receipts_path: root.join("core/knowledge/task_promotion_receipts.jsonl"),
            execution_receipts_path: root
                .join("data/arandur/knowledge_task_execution_receipts.jsonl"),
            task_queue_path: root.join("core/projects/tasks/queue.jsonl"),
            root,
            dry_run: true,
            approval_evidence: None,
            max_file_bytes: 512 * 1024,
        }
    }

    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    pub fn with_source_roots(mut self, source_roots: Vec<PathBuf>) -> Self {
        self.source_roots = source_roots;
        self
    }

    pub fn with_approval_evidence(mut self, approval_evidence: impl Into<String>) -> Self {
        let approval_evidence = approval_evidence.into();
        self.approval_evidence = if approval_evidence.trim().is_empty() {
            None
        } else {
            Some(approval_evidence)
        };
        self
    }
}

pub fn run_knowledge_triage(cfg: &KnowledgeTriageConfig) -> std::io::Result<KnowledgeTriageReport> {
    let discovered_at_utc = Utc::now();
    let source_paths = discover_source_paths(cfg)?;
    let mut inventory_records = Vec::with_capacity(source_paths.len());
    let mut registry_records = Vec::with_capacity(source_paths.len());

    for (path, source_root) in source_paths {
        let metadata = fs::metadata(&path)?;
        let content = fs::read_to_string(&path).unwrap_or_else(|_| String::new());
        let sha256_12 = sha256_12(content.as_bytes());
        let display_path = relative_to_root(&path, &cfg.root);
        inventory_records.push(KnowledgeSourceInventoryRecord {
            schema_version: KNOWLEDGE_SOURCE_INVENTORY_SCHEMA.into(),
            path: display_path.clone(),
            source_root: relative_to_root(&source_root, &cfg.root),
            bytes: metadata.len(),
            sha256_12: sha256_12.clone(),
            read_only_discovery: true,
            discovered_at_utc,
        });

        let mut triage = classify_knowledge_source(&display_path.to_string_lossy(), &content);
        triage.bytes = metadata.len();
        triage.sha256_12 = sha256_12;
        triage.triaged_at_utc = discovered_at_utc;
        registry_records.push(triage);
    }

    let actionable_review_records = registry_records
        .iter()
        .filter(|record| record.requires_human)
        .map(actionable_review_record_for_triage)
        .collect::<Vec<_>>();

    let artifacts_written = if cfg.dry_run {
        false
    } else {
        write_jsonl(&cfg.inventory_path, &inventory_records)?;
        write_jsonl(&cfg.registry_path, &registry_records)?;
        write_jsonl(&cfg.review_queue_path, &actionable_review_records)?;
        true
    };

    let actionable_operational_signals = registry_records
        .iter()
        .filter(|record| record.actionable_operational_signal)
        .count();
    let mut counts_by_domain = BTreeMap::new();
    let mut counts_by_status = BTreeMap::new();
    let mut counts_by_recommended_action = BTreeMap::new();
    for record in &registry_records {
        increment_count(&mut counts_by_domain, &record.source_domain);
        increment_count(&mut counts_by_status, &record.status);
        increment_count(
            &mut counts_by_recommended_action,
            &record.recommended_action,
        );
    }

    Ok(KnowledgeTriageReport {
        schema_version: KNOWLEDGE_TRIAGE_SCHEMA.into(),
        dry_run: cfg.dry_run,
        read_only: true,
        source_roots: cfg.source_roots.clone(),
        inventory_path: cfg.inventory_path.clone(),
        registry_path: cfg.registry_path.clone(),
        review_queue_path: cfg.review_queue_path.clone(),
        sources_discovered: inventory_records.len(),
        records_classified: registry_records.len(),
        actionable_operational_signals,
        counts_by_domain,
        counts_by_status,
        counts_by_recommended_action,
        artifacts_written,
        inventory_records,
        registry_records,
        actionable_review_records,
    })
}

pub fn classify_knowledge_source(path: &str, content: &str) -> KnowledgeTriageRecord {
    let path_lower = path.to_ascii_lowercase();
    let content_lower = content.to_ascii_lowercase();
    let headings = extract_headings(content);
    let title = headings
        .first()
        .cloned()
        .unwrap_or_else(|| fallback_title(path));

    let source_domain = source_domain_for_path(&path_lower);
    let status = status_for_source(&path_lower, &content_lower);
    let task_like = contains_task_signal(&content_lower);
    let template_source = is_template_source(&path_lower, &content_lower);
    let actionable = task_like
        && !template_source
        && is_actionable_source(&source_domain, &status, &content_lower);

    let classification = if matches!(status, TriageStatus::Archived)
        || path_lower.contains("archive/")
        || path_lower.contains(".trash/")
    {
        KnowledgeClassification::ArchiveNoise
    } else if actionable {
        KnowledgeClassification::OperationalSignal
    } else if path_lower.contains("docs/plans")
        || path_lower.contains("benchmark")
        || path_lower.contains("plan")
    {
        KnowledgeClassification::PlanOrBenchmark
    } else if path_lower.contains("architecture") || content_lower.contains("architecture") {
        KnowledgeClassification::ArchitectureReference
    } else if path_lower.contains("human/")
        && contains_any(
            &path_lower,
            &["life", "journal", "personal", "people", "daily"],
        )
    {
        KnowledgeClassification::PersonalKnowledge
    } else if path_lower.contains("human/")
        || path_lower.contains("knowledge")
        || path_lower.contains("docs/")
    {
        KnowledgeClassification::MemorySeed
    } else {
        KnowledgeClassification::Unknown
    };

    let risk_class = risk_class_for_source(&content_lower);
    let content_type = content_type_for_source(&classification, &source_domain, &status);
    let autonomy_relevance = autonomy_relevance_for_source(actionable, &classification);
    let business_relevance = business_relevance_for_source(&source_domain, &classification);
    let recommended_action = recommended_action_for_source(
        actionable,
        &risk_class,
        &classification,
        &source_domain,
        &status,
    );
    let confidence = confidence_for_source(&source_domain, &classification, &headings);
    let evidence = evidence_for_source(path, actionable, &classification, &status, &risk_class);
    let (source_excerpt, source_line_range) = source_evidence_excerpt(content);
    let autonomy_lane = autonomy_lane_for_source(
        actionable,
        &source_domain,
        &status,
        &risk_class,
        &confidence,
        &classification,
    );
    let requires_human = matches!(autonomy_lane, AutonomyLane::HumanApprovalRequired);
    let promotion_decision = promotion_decision_for_lane(&autonomy_lane, actionable);
    let promotion_reason =
        promotion_reason_for_lane(&autonomy_lane, &source_domain, &risk_class, actionable);
    let dedupe_key = dedupe_key_for(path, &title, &source_excerpt);

    let (canonical_home, domain, authority, recommended_action_label, rationale, soterion) =
        classification_metadata(&classification, actionable);

    KnowledgeTriageRecord {
        schema_version: KNOWLEDGE_TRIAGE_SCHEMA.into(),
        path: PathBuf::from(path),
        source_path: PathBuf::from(path),
        title,
        classification,
        source_domain,
        content_type,
        business_relevance,
        autonomy_relevance,
        status,
        risk_class,
        recommended_action,
        evidence,
        confidence,
        autonomy_lane,
        requires_human,
        promotion_decision,
        promotion_reason,
        dedupe_key,
        source_excerpt,
        source_line_range,
        created_task_id: None,
        actionable_operational_signal: actionable,
        mutate_task_queue: false,
        soterion,
        canonical_home,
        domain,
        authority,
        recommended_action_label,
        rationale,
        headings,
        bytes: 0,
        sha256_12: String::new(),
        triaged_at_utc: Utc::now(),
    }
}

pub fn promote_knowledge_tasks(
    cfg: &KnowledgeTriageConfig,
) -> std::io::Result<KnowledgeTaskPromotionReport> {
    let triage = run_knowledge_triage(&cfg.clone().with_dry_run(true))?;
    let mut known_dedupe_keys = load_existing_knowledge_dedupe_keys(&cfg.task_queue_path)?;
    let now = Utc::now();
    let mut receipts = Vec::new();
    let mut task_records = Vec::new();
    let approval_evidence = cfg
        .approval_evidence
        .as_deref()
        .map(str::trim)
        .filter(|evidence| !evidence.is_empty())
        .map(str::to_string);
    let approval_evidence_required = !cfg.dry_run;
    let queue_mutation_authorized = cfg.dry_run || approval_evidence.is_some();

    for record in triage
        .registry_records
        .iter()
        .filter(|record| record.actionable_operational_signal)
    {
        let mut decision = record.promotion_decision.clone();
        let mut task_id = None;
        let mut reason = record.promotion_reason.clone();

        if matches!(decision, PromotionDecision::AutoPromoteInternalTask)
            && known_dedupe_keys.contains(&record.dedupe_key)
        {
            decision = PromotionDecision::DuplicateSkipped;
            reason = format!(
                "duplicate dedupe_key already present in task queue: {}",
                record.dedupe_key
            );
        }

        if matches!(decision, PromotionDecision::AutoPromoteInternalTask) {
            let created = format!(
                "tsk_{}_knowledge_{}",
                now.format("%Y%m%dT%H%M%SZ"),
                record.sha256_12
            );
            if cfg.dry_run {
                reason = format!("dry-run: would create safe-local internal task; {reason}");
            } else if !queue_mutation_authorized {
                reason = format!(
                    "explicit approval evidence required before safe-local task queue mutation; {reason}"
                );
            } else {
                task_id = Some(created.clone());
                task_records.push(json!({
                    "id": created,
                    "title": record.title,
                    "owner": "prometheus",
                    "priority": "medium",
                    "status": "pending",
                    "task_type": "safe_local_knowledge_task",
                    "queued_at_utc": now.to_rfc3339(),
                    "meta": {
                        "origin": "prometheus_knowledge_task_promotion",
                        "source_path": &record.source_path,
                        "source_excerpt": &record.source_excerpt,
                        "source_line_range": record.source_line_range,
                        "dedupe_key": &record.dedupe_key,
                        "autonomy_lane": &record.autonomy_lane,
                        "approval_evidence": approval_evidence.as_deref(),
                        "promotion_gate": KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE,
                        "no_execution_during_promotion": true
                    },
                    "glyphs": ["∇", "↝"],
                }));
                known_dedupe_keys.insert(record.dedupe_key.clone());
            }
        }

        receipts.push(KnowledgeTaskPromotionReceipt {
            schema_version: KNOWLEDGE_TASK_PROMOTION_RECEIPT_SCHEMA.into(),
            source_path: record.source_path.clone(),
            title: record.title.clone(),
            dedupe_key: record.dedupe_key.clone(),
            autonomy_lane: record.autonomy_lane.clone(),
            promotion_decision: decision,
            requires_human: record.requires_human,
            risk_class: record.risk_class.clone(),
            confidence: record.confidence.clone(),
            source_excerpt: record.source_excerpt.clone(),
            source_line_range: record.source_line_range,
            created_task_id: task_id,
            approval_evidence: approval_evidence.clone(),
            receipt_reason: reason,
            written_at_utc: now,
        });
    }

    let artifacts_written = if cfg.dry_run || !queue_mutation_authorized {
        false
    } else {
        append_jsonl_values(&cfg.task_queue_path, &task_records)?;
        append_jsonl_records(&cfg.promotion_receipts_path, &receipts)?;
        true
    };

    Ok(KnowledgeTaskPromotionReport {
        schema_version: KNOWLEDGE_TASK_PROMOTION_RECEIPT_SCHEMA.into(),
        dry_run: cfg.dry_run,
        queue_path: cfg.task_queue_path.clone(),
        receipt_path: cfg.promotion_receipts_path.clone(),
        candidates_seen: receipts.len(),
        tasks_created: receipts
            .iter()
            .filter(|receipt| receipt.created_task_id.is_some())
            .count(),
        human_review_required: receipts
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.promotion_decision,
                    PromotionDecision::HumanReviewRequired
                )
            })
            .count(),
        duplicates_skipped: receipts
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.promotion_decision,
                    PromotionDecision::DuplicateSkipped
                )
            })
            .count(),
        queue_mutation_authorized,
        approval_evidence_required,
        approval_evidence_supplied: approval_evidence.is_some(),
        artifacts_written,
        receipts,
    })
}

pub fn execute_knowledge_task_queue(
    cfg: &KnowledgeTriageConfig,
) -> std::io::Result<KnowledgeTaskExecutionReport> {
    let now = Utc::now();
    let queue_records = read_jsonl_values(&cfg.task_queue_path)?;
    let mut receipts = Vec::new();

    for task in queue_records
        .iter()
        .filter(|task| is_knowledge_promoted_task(task))
    {
        receipts.push(execution_receipt_for_task(task, cfg.dry_run, now));
    }

    let artifacts_written = if cfg.dry_run {
        false
    } else {
        append_jsonl_records(&cfg.execution_receipts_path, &receipts)?;
        true
    };

    Ok(KnowledgeTaskExecutionReport {
        schema_version: KNOWLEDGE_TASK_EXECUTION_RECEIPT_SCHEMA.into(),
        dry_run: cfg.dry_run,
        queue_path: cfg.task_queue_path.clone(),
        receipt_path: cfg.execution_receipts_path.clone(),
        tasks_seen: receipts.len(),
        eligible_for_arandur: receipts
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.execution_decision,
                    KnowledgeExecutionDecision::EligibleForArandur
                        | KnowledgeExecutionDecision::DryRunEligible
                )
            })
            .count(),
        human_review_required: receipts
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.execution_decision,
                    KnowledgeExecutionDecision::HumanReviewRequired
                )
            })
            .count(),
        blocked: receipts
            .iter()
            .filter(|receipt| {
                matches!(
                    receipt.execution_decision,
                    KnowledgeExecutionDecision::Blocked
                )
            })
            .count(),
        artifacts_written,
        receipts,
    })
}

fn is_knowledge_promoted_task(task: &serde_json::Value) -> bool {
    task.get("meta")
        .and_then(|meta| meta.get("origin"))
        .and_then(|origin| origin.as_str())
        .is_some_and(|origin| origin == "prometheus_knowledge_task_promotion")
}

fn execution_receipt_for_task(
    task: &serde_json::Value,
    dry_run: bool,
    now: DateTime<Utc>,
) -> KnowledgeTaskExecutionReceipt {
    let task_id = string_field(task, "id").unwrap_or_else(|| "unknown_task".to_string());
    let title = string_field(task, "title").unwrap_or_else(|| task_id.clone());
    let status = string_field(task, "status").unwrap_or_default();
    let task_type = string_field(task, "task_type").unwrap_or_default();
    let meta = task.get("meta").unwrap_or(&serde_json::Value::Null);
    let autonomy_lane = string_field(meta, "autonomy_lane");
    let risk_class = string_field(meta, "risk_class").unwrap_or_else(|| "safe_local".to_string());
    let source_path = string_field(meta, "source_path").map(PathBuf::from);
    let dedupe_key = string_field(meta, "dedupe_key");
    let promotion_gate = string_field(meta, "promotion_gate");

    let (execution_decision, requires_human, receipt_reason) = if status != "pending" {
        (
            KnowledgeExecutionDecision::Blocked,
            false,
            format!("blocked: task status '{status}' is not pending"),
        )
    } else if task_type != "safe_local_knowledge_task" {
        (
            KnowledgeExecutionDecision::Blocked,
            false,
            format!("blocked: task_type '{task_type}' is not safe_local_knowledge_task"),
        )
    } else if promotion_gate.as_deref() != Some(KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE) {
        (
            KnowledgeExecutionDecision::HumanReviewRequired,
            true,
            "risk boundary stops Arandur execution: missing safe-local promotion gate".to_string(),
        )
    } else if !is_safe_local_execution_lane(autonomy_lane.as_deref(), &risk_class) {
        (
            KnowledgeExecutionDecision::HumanReviewRequired,
            true,
            format!(
                "risk boundary stops Arandur execution: autonomy_lane={:?}, risk_class={risk_class}",
                autonomy_lane
            ),
        )
    } else if dry_run {
        (
            KnowledgeExecutionDecision::DryRunEligible,
            false,
            "dry-run: would hand off safe-local internal task to Arandur with receipt-only guard"
                .to_string(),
        )
    } else {
        (
            KnowledgeExecutionDecision::EligibleForArandur,
            false,
            "safe-local internal task is eligible for bounded Arandur execution; receipt written before execution handoff"
                .to_string(),
        )
    };

    KnowledgeTaskExecutionReceipt {
        schema_version: KNOWLEDGE_TASK_EXECUTION_RECEIPT_SCHEMA.into(),
        task_id,
        title,
        source_path,
        dedupe_key,
        autonomy_lane,
        execution_decision,
        requires_human,
        receipt_reason,
        written_at_utc: now,
    }
}

fn is_safe_local_execution_lane(autonomy_lane: Option<&str>, risk_class: &str) -> bool {
    matches!(
        autonomy_lane,
        Some("auto_create_internal_task" | "auto_execute_safe_local_task")
    ) && matches!(risk_class, "safe_local" | "SafeLocal")
}

fn string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|field| field.as_str())
        .map(ToOwned::to_owned)
}

fn read_jsonl_values(path: &Path) -> std::io::Result<Vec<serde_json::Value>> {
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let mut values = Vec::new();
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            values.push(value);
        }
    }
    Ok(values)
}

fn actionable_review_record_for_triage(
    record: &KnowledgeTriageRecord,
) -> KnowledgeActionableReviewRecord {
    KnowledgeActionableReviewRecord {
        schema_version: KNOWLEDGE_ACTIONABLE_REVIEW_SCHEMA.into(),
        source_path: record.source_path.clone(),
        title: record.title.clone(),
        classification: record.classification.clone(),
        source_domain: record.source_domain.clone(),
        status: record.status.clone(),
        risk_class: record.risk_class.clone(),
        recommended_action: record.recommended_action.clone(),
        recommended_action_label: record.recommended_action_label.clone(),
        confidence: record.confidence.clone(),
        evidence: record.evidence.clone(),
        review_gate: KNOWLEDGE_ACTIONABLE_REVIEW_GATE.into(),
        triad_required: true,
        mutate_task_queue: false,
        rationale: format!(
            "Review-only operational signal from {}. Task queue mutation remains disabled until an explicit task-pivot or human decision approves the work.",
            record.source_path.display()
        ),
        triaged_at_utc: record.triaged_at_utc,
    }
}

fn discover_source_paths(cfg: &KnowledgeTriageConfig) -> std::io::Result<Vec<(PathBuf, PathBuf)>> {
    let mut seen = BTreeSet::new();
    let mut paths = Vec::new();
    for root in &cfg.source_roots {
        if !root.exists() {
            continue;
        }
        collect_sources(root, root, cfg.max_file_bytes, &mut seen, &mut paths)?;
    }
    paths.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(paths)
}

fn collect_sources(
    base: &Path,
    current: &Path,
    max_file_bytes: u64,
    seen: &mut BTreeSet<PathBuf>,
    paths: &mut Vec<(PathBuf, PathBuf)>,
) -> std::io::Result<()> {
    let mut entries = fs::read_dir(current)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if should_skip_path(file_name) {
            continue;
        }
        let metadata = entry.metadata()?;
        if metadata.is_dir() {
            collect_sources(base, &path, max_file_bytes, seen, paths)?;
        } else if metadata.is_file()
            && metadata.len() <= max_file_bytes
            && is_supported_source_file(&path)
            && seen.insert(path.clone())
        {
            paths.push((path, base.to_path_buf()));
        }
    }
    Ok(())
}

fn is_supported_source_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("md" | "txt" | "json" | "jsonl" | "toml" | "yaml" | "yml")
    )
}

fn should_skip_path(file_name: &str) -> bool {
    file_name.starts_with('.')
        || matches!(
            file_name,
            "target" | "node_modules" | "dist" | "logs" | "tmp" | "__pycache__"
        )
}

fn extract_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                Some(trimmed.trim_start_matches('#').trim().to_string())
            } else {
                None
            }
        })
        .filter(|heading| !heading.is_empty())
        .take(8)
        .collect()
}

fn fallback_title(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string())
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_task_signal(content_lower: &str) -> bool {
    has_explicit_task_source_marker(content_lower)
        || content_lower.lines().any(has_actionable_evidence_line)
}

fn has_actionable_evidence_line(line_lower: &str) -> bool {
    let trimmed = line_lower.trim();
    if trimmed.is_empty() || contains_template_placeholder(trimmed) {
        return false;
    }

    let without_heading = trimmed.trim_start_matches('#').trim();
    let without_bullet = without_heading
        .trim_start_matches('-')
        .trim_start_matches('*')
        .trim();

    has_prefixed_task_body(without_bullet, "next step:")
        || has_prefixed_task_body(without_bullet, "next steps:")
        || has_prefixed_task_body(without_bullet, "action item:")
        || has_prefixed_task_body(without_bullet, "action items:")
        || has_prefixed_task_body(without_bullet, "todo:")
        || has_prefixed_task_body(without_bullet, "follow-up:")
        || has_prefixed_task_body(without_bullet, "follow up:")
        || has_prefixed_task_body(without_bullet, "acceptance criteria:")
        || without_bullet
            .strip_prefix("[ ]")
            .is_some_and(has_non_placeholder_task_text)
}

fn has_prefixed_task_body(line: &str, prefix: &str) -> bool {
    line.strip_prefix(prefix)
        .is_some_and(has_non_placeholder_task_text)
}

fn has_non_placeholder_task_text(task_text: &str) -> bool {
    let trimmed = task_text.trim().trim_matches('*').trim();
    !trimmed.is_empty() && !contains_template_placeholder(trimmed)
}

fn contains_template_placeholder(text: &str) -> bool {
    contains_any(
        text,
        &["{{", "}}", "[project]", "[item]", "[task]", "[date]"],
    )
}

fn is_template_source(path_lower: &str, content_lower: &str) -> bool {
    path_lower.contains("/templates/")
        || path_lower.contains("template.md")
        || contains_template_placeholder(content_lower)
        || is_generic_audit_note_template(path_lower, content_lower)
}

fn is_generic_audit_note_template(path_lower: &str, content_lower: &str) -> bool {
    path_lower.ends_with("/audit-notes.md")
        && contains_any(
            content_lower,
            &[
                "## audit status",
                "- [ ] dependencies audit",
                "- [ ] code quality check",
                "- [ ] security review",
                "- [ ] performance analysis",
                "- [ ] build verification",
            ],
        )
        && contains_any(
            content_lower,
            &[
                "(add findings as you discover them)",
                "(add detailed notes here)",
            ],
        )
}

fn is_actionable_source(
    source_domain: &SourceDomain,
    status: &TriageStatus,
    content_lower: &str,
) -> bool {
    if matches!(
        status,
        TriageStatus::Archived | TriageStatus::Completed | TriageStatus::Stale
    ) {
        return false;
    }

    match source_domain {
        SourceDomain::DocsPlans | SourceDomain::Docs | SourceDomain::Eregion => true,
        SourceDomain::Human => has_explicit_task_source_marker(content_lower),
        _ => false,
    }
}

fn has_explicit_task_source_marker(content_lower: &str) -> bool {
    contains_any(
        content_lower,
        &[
            "autopilot: true",
            "autopilot_task: true",
            "task_source: true",
            "operational_signal: true",
            "annunimas_task: true",
            "## annunimas tasks",
            "## project actions",
            "## autopilot candidates",
        ],
    )
}

fn autonomy_lane_for_source(
    actionable: bool,
    source_domain: &SourceDomain,
    status: &TriageStatus,
    risk_class: &RiskClass,
    confidence: &Confidence,
    classification: &KnowledgeClassification,
) -> AutonomyLane {
    if matches!(
        status,
        TriageStatus::Archived | TriageStatus::Completed | TriageStatus::Stale
    ) || matches!(classification, KnowledgeClassification::ArchiveNoise)
    {
        return AutonomyLane::Blocked;
    }
    if !actionable {
        return match classification {
            KnowledgeClassification::ArchitectureReference
            | KnowledgeClassification::MemorySeed => AutonomyLane::ObserveOnly,
            KnowledgeClassification::PlanOrBenchmark => AutonomyLane::CandidateOnly,
            _ => AutonomyLane::ObserveOnly,
        };
    }
    if !matches!(risk_class, RiskClass::SafeLocal) || matches!(confidence, Confidence::Low) {
        return AutonomyLane::HumanApprovalRequired;
    }
    match source_domain {
        SourceDomain::DocsPlans
        | SourceDomain::Docs
        | SourceDomain::Eregion
        | SourceDomain::Human => AutonomyLane::AutoCreateInternalTask,
        _ => AutonomyLane::CandidateOnly,
    }
}

fn promotion_decision_for_lane(lane: &AutonomyLane, actionable: bool) -> PromotionDecision {
    match lane {
        AutonomyLane::ObserveOnly => PromotionDecision::ContextOnly,
        AutonomyLane::CandidateOnly => PromotionDecision::CandidateOnly,
        AutonomyLane::AutoCreateInternalTask | AutonomyLane::AutoExecuteSafeLocalTask => {
            if actionable {
                PromotionDecision::AutoPromoteInternalTask
            } else {
                PromotionDecision::CandidateOnly
            }
        }
        AutonomyLane::HumanApprovalRequired => PromotionDecision::HumanReviewRequired,
        AutonomyLane::Blocked => PromotionDecision::Blocked,
    }
}

fn promotion_reason_for_lane(
    lane: &AutonomyLane,
    source_domain: &SourceDomain,
    risk_class: &RiskClass,
    actionable: bool,
) -> String {
    match lane {
        AutonomyLane::AutoCreateInternalTask => format!(
            "explicit actionable evidence from {source_domain:?}; safe-local risk permits task creation without execution"
        ),
        AutonomyLane::HumanApprovalRequired => format!(
            "human review required because risk_class={risk_class:?} or confidence/source ambiguity crosses safe-local bounds"
        ),
        AutonomyLane::Blocked => "source is archived, completed, stale, unsafe, or outside active scope".into(),
        AutonomyLane::CandidateOnly if actionable => {
            "actionable evidence exists but source is not eligible for automatic task creation".into()
        }
        AutonomyLane::CandidateOnly => "planning context only; no executable task promotion".into(),
        AutonomyLane::AutoExecuteSafeLocalTask => {
            "safe-local execution lane reserved for Arandur; promotion only creates a task".into()
        }
        AutonomyLane::ObserveOnly => "observe/context only; no task candidate evidence".into(),
    }
}

fn source_evidence_excerpt(content: &str) -> (String, Option<(usize, usize)>) {
    for (idx, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        if has_actionable_evidence_line(&lower) {
            let line_number = idx + 1;
            return (line.trim().to_string(), Some((line_number, line_number)));
        }
    }
    let excerpt = content
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    (excerpt.trim().chars().take(240).collect(), None)
}

fn dedupe_key_for(path: &str, title: &str, source_excerpt: &str) -> String {
    let seed = format!(
        "{}::{}::{}",
        path.to_ascii_lowercase(),
        title.to_ascii_lowercase(),
        source_excerpt.to_ascii_lowercase()
    );
    format!("knowledge:{}", sha256_12(seed.as_bytes()))
}

fn load_existing_knowledge_dedupe_keys(path: &Path) -> std::io::Result<BTreeSet<String>> {
    let mut keys = BTreeSet::new();
    let Ok(contents) = fs::read_to_string(path) else {
        return Ok(keys);
    };
    for line in contents.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(key) = value
            .get("meta")
            .and_then(|meta| meta.get("dedupe_key"))
            .and_then(|key| key.as_str())
        {
            keys.insert(key.to_string());
        }
    }
    Ok(keys)
}

fn append_jsonl_values(path: &Path, records: &[serde_json::Value]) -> std::io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for record in records {
        serde_json::to_writer(&mut file, record)?;
        file.write_all(b"\n")?;
    }
    file.flush()
}

fn increment_count<T>(counts: &mut BTreeMap<String, usize>, value: &T)
where
    T: Serialize,
{
    let key = serde_json::to_value(value)
        .ok()
        .and_then(|json| json.as_str().map(ToOwned::to_owned))
        .unwrap_or_else(|| "unknown".to_string());
    let count = counts.entry(key).or_insert(0);
    *count += 1;
}

fn source_domain_for_path(path_lower: &str) -> SourceDomain {
    if path_lower.contains("docs/plans") {
        SourceDomain::DocsPlans
    } else if path_lower.contains("human/") {
        SourceDomain::Human
    } else if path_lower.contains("docs/") {
        SourceDomain::Docs
    } else if path_lower.contains("/eregion") || path_lower.starts_with("eregion/") {
        SourceDomain::Eregion
    } else if path_lower.contains("core/state") || path_lower.contains("core/knowledge") {
        SourceDomain::CoreState
    } else {
        SourceDomain::Unknown
    }
}

fn status_for_source(path_lower: &str, content_lower: &str) -> TriageStatus {
    if path_lower.contains("archive/")
        || path_lower.contains("archived")
        || path_lower.contains("/09-archive/")
        || path_lower.contains("/archive/")
    {
        TriageStatus::Archived
    } else if has_explicit_status(content_lower, &["completed", "complete", "done"])
        || contains_any(path_lower, &["completed", "done"])
    {
        TriageStatus::Completed
    } else if has_explicit_status(content_lower, &["blocked"])
        || contains_any(content_lower, &["blocker"])
    {
        TriageStatus::Blocked
    } else if has_explicit_status(content_lower, &["stale", "superseded", "deprecated"])
        || contains_any(content_lower, &["superseded", "deprecated"])
    {
        TriageStatus::Stale
    } else if has_explicit_status(content_lower, &["active", "in progress"])
        || contains_any(content_lower, &["next step", "todo", "gate"])
    {
        TriageStatus::Active
    } else {
        TriageStatus::Unknown
    }
}

fn has_explicit_status(content_lower: &str, statuses: &[&str]) -> bool {
    content_lower.lines().any(|line| {
        let trimmed = line
            .trim()
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim();
        let value = trimmed
            .strip_prefix("status:")
            .or_else(|| trimmed.strip_prefix("status ="))
            .map(str::trim);
        value.is_some_and(|status| {
            statuses
                .iter()
                .any(|candidate| status.starts_with(candidate))
        })
    })
}

fn risk_class_for_source(content_lower: &str) -> RiskClass {
    if contains_any(
        content_lower,
        &["secret", "credential", "password", "api key", "token"],
    ) {
        RiskClass::CredentialSensitive
    } else if contains_any(
        content_lower,
        &["pay", "invoice", "purchase", "money", "bank"],
    ) {
        RiskClass::Financial
    } else if contains_any(content_lower, &["delete", "destroy", "remove all", "wipe"]) {
        RiskClass::Destructive
    } else if contains_any(
        content_lower,
        &["send", "publish", "post", "email", "external"],
    ) {
        RiskClass::ExternalSideEffect
    } else if contains_any(content_lower, &["approve", "approval", "human decision"]) {
        RiskClass::NeedsReview
    } else {
        RiskClass::SafeLocal
    }
}

fn content_type_for_source(
    classification: &KnowledgeClassification,
    source_domain: &SourceDomain,
    status: &TriageStatus,
) -> ContentType {
    match (classification, source_domain, status) {
        (_, _, TriageStatus::Completed) => ContentType::CompletedPlan,
        (_, _, TriageStatus::Stale) => ContentType::StalePlan,
        (KnowledgeClassification::OperationalSignal, _, _) => ContentType::TaskCandidate,
        (KnowledgeClassification::ArchitectureReference, _, _) => ContentType::ArchitectureDoc,
        (KnowledgeClassification::PersonalKnowledge, _, _) => ContentType::PersonalNote,
        (KnowledgeClassification::PlanOrBenchmark, _, TriageStatus::Active) => {
            ContentType::ActivePlan
        }
        (KnowledgeClassification::PlanOrBenchmark, _, _) => ContentType::ProjectNote,
        (_, SourceDomain::Eregion, _) => ContentType::BusinessPlan,
        (KnowledgeClassification::MemorySeed, _, _) => ContentType::ReferenceMaterial,
        _ => ContentType::Unknown,
    }
}

fn autonomy_relevance_for_source(
    actionable: bool,
    classification: &KnowledgeClassification,
) -> AutonomyRelevance {
    if actionable {
        AutonomyRelevance::TaskSource
    } else {
        match classification {
            KnowledgeClassification::OperationalSignal => AutonomyRelevance::TaskSource,
            KnowledgeClassification::PlanOrBenchmark => AutonomyRelevance::PlanningInput,
            KnowledgeClassification::ArchitectureReference
            | KnowledgeClassification::MemorySeed => AutonomyRelevance::ContextOnly,
            _ => AutonomyRelevance::None,
        }
    }
}

fn business_relevance_for_source(
    source_domain: &SourceDomain,
    classification: &KnowledgeClassification,
) -> BusinessRelevance {
    match (source_domain, classification) {
        (SourceDomain::Eregion, _) => BusinessRelevance::High,
        (_, KnowledgeClassification::OperationalSignal) => BusinessRelevance::High,
        (SourceDomain::DocsPlans | SourceDomain::Docs, _) => BusinessRelevance::Medium,
        (SourceDomain::Human, KnowledgeClassification::PersonalKnowledge) => BusinessRelevance::Low,
        (SourceDomain::Human, _) => BusinessRelevance::Medium,
        _ => BusinessRelevance::None,
    }
}

fn recommended_action_for_source(
    actionable: bool,
    risk_class: &RiskClass,
    classification: &KnowledgeClassification,
    source_domain: &SourceDomain,
    status: &TriageStatus,
) -> RecommendedAction {
    if matches!(status, TriageStatus::Archived)
        || matches!(classification, KnowledgeClassification::ArchiveNoise)
    {
        RecommendedAction::IgnoreForNow
    } else if matches!(status, TriageStatus::Completed | TriageStatus::Stale) {
        match source_domain {
            SourceDomain::DocsPlans | SourceDomain::Docs | SourceDomain::Eregion => {
                RecommendedAction::LinkToProject
            }
            SourceDomain::Human => RecommendedAction::PreserveAsContext,
            _ => RecommendedAction::PreserveAsContext,
        }
    } else if !matches!(risk_class, RiskClass::SafeLocal) {
        RecommendedAction::RequestHumanDecision
    } else if actionable {
        RecommendedAction::CreateTask
    } else {
        match classification {
            KnowledgeClassification::PersonalKnowledge => RecommendedAction::PreserveAsContext,
            KnowledgeClassification::ArchitectureReference
            | KnowledgeClassification::MemorySeed => RecommendedAction::Summarize,
            KnowledgeClassification::PlanOrBenchmark => RecommendedAction::LinkToProject,
            KnowledgeClassification::ArchiveNoise => RecommendedAction::IgnoreForNow,
            _ => RecommendedAction::PreserveAsContext,
        }
    }
}

fn confidence_for_source(
    source_domain: &SourceDomain,
    classification: &KnowledgeClassification,
    headings: &[String],
) -> Confidence {
    if !headings.is_empty() && !matches!(classification, KnowledgeClassification::Unknown) {
        Confidence::High
    } else if !matches!(source_domain, SourceDomain::Unknown) {
        Confidence::Medium
    } else {
        Confidence::Low
    }
}

fn evidence_for_source(
    path: &str,
    actionable: bool,
    classification: &KnowledgeClassification,
    status: &TriageStatus,
    risk_class: &RiskClass,
) -> Vec<String> {
    let mut evidence = vec![
        format!("path={path}"),
        format!("classification={classification:?}"),
    ];
    if actionable {
        evidence.push("matched task-like language in an operational source".into());
    }
    if !matches!(status, TriageStatus::Unknown) {
        evidence.push(format!("status={status:?}"));
    }
    if !matches!(risk_class, RiskClass::SafeLocal) {
        evidence.push(format!("risk_class={risk_class:?}"));
    }
    evidence
}

fn classification_metadata(
    classification: &KnowledgeClassification,
    actionable: bool,
) -> (String, String, String, String, String, SoterionMapping) {
    if actionable {
        return (
            "core/knowledge".into(),
            "autopilot_triage".into(),
            "review_required".into(),
            "review as an operational signal; do not mutate the task queue without explicit task-pivot".into(),
            "Deterministic heuristics found task-like language in an operational knowledge source.".into(),
            SoterionMapping {
                sigil: "PROMETHEUS".into(),
                glyph: "↝".into(),
                retention: "review_before_action".into(),
            },
        );
    }

    match classification {
        KnowledgeClassification::PersonalKnowledge => (
            "human".into(),
            "personal_context".into(),
            "human_context".into(),
            "preserve as personal context; do not convert to task by default".into(),
            "Human vault note appears personal/life-context oriented rather than operational.".into(),
            SoterionMapping {
                sigil: "MNEMOSYNE".into(),
                glyph: "🜄".into(),
                retention: "context_only".into(),
            },
        ),
        KnowledgeClassification::ArchitectureReference => (
            "core/knowledge".into(),
            "architecture_reference".into(),
            "curated_reference".into(),
            "index as architecture reference for future planning".into(),
            "Source path or content indicates architecture-level knowledge.".into(),
            SoterionMapping {
                sigil: "SOTERION".into(),
                glyph: "📜".into(),
                retention: "reference".into(),
            },
        ),
        KnowledgeClassification::PlanOrBenchmark => (
            "docs/plans".into(),
            "plan_or_benchmark".into(),
            "operator_plan".into(),
            "review during readiness/autopilot planning".into(),
            "Source appears to be a plan, benchmark, or readiness artifact.".into(),
            SoterionMapping {
                sigil: "PROMETHEUS".into(),
                glyph: "∇".into(),
                retention: "planning_reference".into(),
            },
        ),
        KnowledgeClassification::ArchiveNoise => (
            "archive".into(),
            "archive_noise".into(),
            "historical".into(),
            "ignore unless explicitly auditing history".into(),
            "Path is under an archive/trash-like location.".into(),
            SoterionMapping {
                sigil: "HADES".into(),
                glyph: "↝".into(),
                retention: "archive".into(),
            },
        ),
        KnowledgeClassification::MemorySeed | KnowledgeClassification::Unknown => (
            "core/knowledge".into(),
            "memory_seed".into(),
            "curated_memory".into(),
            "encode/link as future Mnemosyne recall context after review".into(),
            "Source may carry useful background knowledge but did not qualify as an operational signal.".into(),
            SoterionMapping {
                sigil: "MNEMOSYNE".into(),
                glyph: "🜄".into(),
                retention: "encode_or_link".into(),
            },
        ),
        KnowledgeClassification::OperationalSignal => (
            "core/knowledge".into(),
            "autopilot_triage".into(),
            "review_required".into(),
            "route through Prometheus promotion policy before queue mutation".into(),
            "Operational signal classification is evidence only; Prometheus decides whether safe-local promotion or human review is required.".into(),
            SoterionMapping {
                sigil: "PROMETHEUS".into(),
                glyph: "↝".into(),
                retention: "review_before_action".into(),
            },
        ),
    }
}

fn relative_to_root(path: &Path, root: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

fn sha256_12(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}").chars().take(12).collect()
}

fn write_jsonl<T: Serialize>(path: &Path, records: &[T]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()
}

fn append_jsonl_records<T: Serialize>(path: &Path, records: &[T]) -> std::io::Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let mut writer = BufWriter::new(file);
    for record in records {
        serde_json::to_writer(&mut writer, record)?;
        writer.write_all(b"\n")?;
    }
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eregion_fixture_path(relative_path: &str) -> String {
        format!(
            "/portable-fixtures/Eregion/{}",
            relative_path.trim_start_matches('/')
        )
    }

    #[test]
    fn classifies_human_notes_without_treating_every_note_as_task() {
        let personal = classify_knowledge_source(
            "human/01-Life/journal.md",
            "# Dinner notes\nRemember that family dinner felt grounding.",
        );
        assert_eq!(
            personal.classification,
            KnowledgeClassification::PersonalKnowledge
        );
        assert_eq!(personal.source_domain, SourceDomain::Human);
        assert_eq!(personal.content_type, ContentType::PersonalNote);
        assert_eq!(personal.autonomy_relevance, AutonomyRelevance::None);
        assert_eq!(
            personal.recommended_action,
            RecommendedAction::PreserveAsContext
        );
        assert!(!personal.actionable_operational_signal);

        let operational = classify_knowledge_source(
            "docs/plans/gate-9.md",
            "# Gate 9\nTODO: implement dry-run autopilot knowledge triage foundation.",
        );
        assert_eq!(
            operational.classification,
            KnowledgeClassification::OperationalSignal
        );
        assert_eq!(operational.source_domain, SourceDomain::DocsPlans);
        assert_eq!(operational.content_type, ContentType::TaskCandidate);
        assert_eq!(
            operational.autonomy_relevance,
            AutonomyRelevance::TaskSource
        );
        assert_eq!(
            operational.recommended_action,
            RecommendedAction::CreateTask
        );
        assert_eq!(operational.risk_class, RiskClass::SafeLocal);
        assert!(operational.actionable_operational_signal);
        assert!(!operational.mutate_task_queue);
    }

    #[test]
    fn human_reference_notes_do_not_become_task_candidates_without_explicit_metadata() {
        let reference = classify_knowledge_source(
            "human/03-Knowledge/Architecture/README.md",
            "# Architecture Documentation\nTODO: keep this reference complete and useful for future agents.",
        );

        assert_eq!(reference.source_domain, SourceDomain::Human);
        assert_ne!(reference.recommended_action, RecommendedAction::CreateTask);
        assert!(!reference.actionable_operational_signal);
        assert!(matches!(
            reference.autonomy_relevance,
            AutonomyRelevance::ContextOnly | AutonomyRelevance::None
        ));
    }

    #[test]
    fn terminal_statuses_suppress_task_creation() {
        let completed = classify_knowledge_source(
            "docs/operations/BACKEND_LOCKDOWN.md",
            "# Backend Lockdown\nstatus: completed\nTODO: verify that the lockdown work is done.",
        );
        assert_eq!(completed.status, TriageStatus::Completed);
        assert_ne!(completed.recommended_action, RecommendedAction::CreateTask);
        assert!(!completed.actionable_operational_signal);

        let archived = classify_knowledge_source(
            "human/09-Archive/phase1-plan.md",
            "# Phase 1 Plan\nToken rotation TODO: send archived notes to the old provider.",
        );
        assert_eq!(archived.status, TriageStatus::Archived);
        assert_eq!(
            archived.classification,
            KnowledgeClassification::ArchiveNoise
        );
        assert_eq!(archived.recommended_action, RecommendedAction::IgnoreForNow);
        assert!(!archived.actionable_operational_signal);
    }

    #[test]
    fn loose_completion_words_do_not_mark_context_notes_complete() {
        let daily = classify_knowledge_source(
            "human/01-Daily/2026-05-10.md",
            "# Daily Note\nDinner felt complete and grounding today.",
        );

        assert_eq!(daily.status, TriageStatus::Unknown);
        assert_ne!(daily.content_type, ContentType::CompletedPlan);
    }

    #[test]
    fn active_docs_plan_safe_local_task_candidates_still_create_tasks() {
        let plan = classify_knowledge_source(
            "docs/plans/gate-9.md",
            "# Gate 9\nstatus: active\nNext step: implement safe-local knowledge triage hardening.",
        );

        assert_eq!(plan.source_domain, SourceDomain::DocsPlans);
        assert_eq!(plan.status, TriageStatus::Active);
        assert_eq!(plan.risk_class, RiskClass::SafeLocal);
        assert_eq!(plan.recommended_action, RecommendedAction::CreateTask);
        assert!(plan.actionable_operational_signal);
        assert!(!plan.mutate_task_queue);
    }

    #[test]
    fn incidental_keywords_do_not_create_tasks_without_explicit_evidence() {
        let docs_index = classify_knowledge_source(
            "docs/INDEX_TREE.md",
            "# Documentation Directory\n- ATHENA_ARDA_IMPLEMENTATION_GUIDE.md\n- prompts/\n",
        );
        assert_ne!(docs_index.recommended_action, RecommendedAction::CreateTask);
        assert!(!docs_index.actionable_operational_signal);

        let config = classify_knowledge_source(
            &eregion_fixture_path("site/components.json"),
            r##"{
                "tailwind": { "css": "src/index.css", "prefix": "" },
                "aliases": { "components": "@/components", "hooks": "@/hooks" }
            }"##,
        );
        assert_ne!(config.recommended_action, RecommendedAction::CreateTask);
        assert!(!config.actionable_operational_signal);

        let template = classify_knowledge_source(
            &eregion_fixture_path("templates/MEMORY.md"),
            "# Memory Audit - {{PROJECT_NAME}}\n## Recommendations\n- [ ] Implement caching layer\n- [ ] Add cleanup routines\n",
        );
        assert_ne!(template.recommended_action, RecommendedAction::CreateTask);
        assert!(!template.actionable_operational_signal);
    }

    #[test]
    fn eregion_audit_note_templates_do_not_create_tasks() {
        let audit_note = classify_knowledge_source(
            &eregion_fixture_path("CoverCoINC/audit-notes.md"),
            "# CoverCoINC Audit Notes\n\n## Project Overview\n- Type: TypeScript/JavaScript\n\n## Audit Status\n- [ ] Dependencies audit\n- [ ] Code quality check\n- [ ] Security review\n- [ ] Performance analysis\n- [ ] Build verification\n\n## Findings\n(Add findings as you discover them)\n\n## Notes\n(Add detailed notes here)\n",
        );

        assert_eq!(audit_note.source_domain, SourceDomain::Eregion);
        assert_ne!(audit_note.recommended_action, RecommendedAction::CreateTask);
        assert!(!audit_note.actionable_operational_signal);
    }

    #[test]
    fn historical_audit_summary_headings_do_not_create_tasks_without_item_evidence() {
        let summary = classify_knowledge_source(
            &eregion_fixture_path("audits/master-summary.md"),
            "# Eregion Audit Summary\n**Date:** 2026-03-26\n\n## Project Audits Completed\n| Project | Audits Done |\n| Annunimas | 4 |\n\n**Next Steps:**\n1. Review Annunimas critical issues\n2. Implement memory persistence\n3. Add monitoring infrastructure\n",
        );

        assert_eq!(summary.source_domain, SourceDomain::Eregion);
        assert_ne!(summary.recommended_action, RecommendedAction::CreateTask);
        assert!(!summary.actionable_operational_signal);
    }

    #[test]
    fn explicit_action_item_evidence_remains_task_eligible() {
        let action_item = classify_knowledge_source(
            &eregion_fixture_path("audits/master-summary.md"),
            "# Audit Summary\nstatus: active\n## Action Items\n- [ ] Add monitoring infrastructure for safe local services.\n",
        );

        assert_eq!(action_item.source_domain, SourceDomain::Eregion);
        assert_eq!(action_item.risk_class, RiskClass::SafeLocal);
        assert_eq!(
            action_item.recommended_action,
            RecommendedAction::CreateTask
        );
        assert!(action_item.actionable_operational_signal);
        assert!(!action_item.mutate_task_queue);
    }

    #[test]
    fn dry_run_discovers_sources_but_does_not_emit_artifacts() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let human = root.join("human/03-Knowledge/Architecture");
        std::fs::create_dir_all(&human).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            human.join("zero-human-spec.md"),
            "# Zero Human Company\nArchitecture reference for autonomous company operations.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(true);
        let report =
            run_knowledge_triage(&cfg).unwrap_or_else(|err| panic!("triage failed: {err}"));

        assert_eq!(report.sources_discovered, 1);
        assert_eq!(report.records_classified, 1);
        assert_eq!(report.counts_by_domain.get("human").copied(), Some(1));
        assert_eq!(
            report
                .counts_by_recommended_action
                .get("summarize")
                .copied(),
            Some(1)
        );
        assert!(!report.artifacts_written);
        assert!(!cfg.inventory_path.exists());
        assert!(!cfg.registry_path.exists());
        assert!(report
            .registry_records
            .iter()
            .all(|record| !record.mutate_task_queue));
    }

    #[test]
    fn write_mode_emits_review_queue_only_for_risky_candidates() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let docs = root.join("docs/plans");
        std::fs::create_dir_all(&docs).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            docs.join("gate.md"),
            "# Gate Plan
status: active
Next step: verify generated artifacts.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));
        std::fs::write(
            docs.join("credential.md"),
            "# Credential Plan
status: active
Next step: rotate production credential.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(false);
        let report =
            run_knowledge_triage(&cfg).unwrap_or_else(|err| panic!("triage failed: {err}"));

        assert!(report.artifacts_written);
        assert!(cfg.inventory_path.exists());
        assert!(cfg.registry_path.exists());
        assert!(cfg.review_queue_path.exists());
        assert_eq!(report.actionable_operational_signals, 2);
        assert_eq!(report.actionable_review_records.len(), 1);
        assert_eq!(
            report.actionable_review_records[0].risk_class,
            RiskClass::CredentialSensitive
        );
        assert_eq!(
            report.actionable_review_records[0].review_gate,
            "human_review_required_before_task_queue_mutation"
        );
        assert!(!report.actionable_review_records[0].mutate_task_queue);

        let registry = std::fs::read_to_string(&cfg.registry_path)
            .unwrap_or_else(|err| panic!("read registry failed: {err}"));
        assert!(registry.contains("auto_create_internal_task"));
        assert!(registry.contains("human_approval_required"));
        assert!(!registry.contains("core/projects/tasks/queue.jsonl"));

        let review_queue = std::fs::read_to_string(&cfg.review_queue_path)
            .unwrap_or_else(|err| panic!("read review queue failed: {err}"));
        assert_eq!(review_queue.lines().count(), 1);
        assert!(review_queue.contains("annunimas.knowledge_actionable_review.v1"));
        assert!(review_queue.contains("human_review_required_before_task_queue_mutation"));
    }

    #[test]
    fn graduated_autonomy_lanes_separate_safe_local_from_risky_and_stale_work() {
        let safe = classify_knowledge_source(
            "docs/plans/active.md",
            "# Active Plan
status: active
- [ ] Add tests for existing crate.",
        );
        assert_eq!(safe.autonomy_lane, AutonomyLane::AutoCreateInternalTask);
        assert_eq!(
            safe.promotion_decision,
            PromotionDecision::AutoPromoteInternalTask
        );
        assert!(!safe.requires_human);
        assert!(!safe.mutate_task_queue);

        let risky = classify_knowledge_source(
            "docs/operations/deploy.md",
            "# Deploy
status: active
Next step: restart production service after rotating secret.",
        );
        assert_eq!(risky.autonomy_lane, AutonomyLane::HumanApprovalRequired);
        assert_eq!(
            risky.promotion_decision,
            PromotionDecision::HumanReviewRequired
        );
        assert!(risky.requires_human);
        assert!(!risky.mutate_task_queue);

        let stale = classify_knowledge_source(
            "docs/plans/old.md",
            "# Old Plan
status: completed
TODO: implement historical task.",
        );
        assert_eq!(stale.autonomy_lane, AutonomyLane::Blocked);
        assert_eq!(stale.promotion_decision, PromotionDecision::Blocked);
        assert!(!stale.actionable_operational_signal);
    }

    #[test]
    fn athena_stage_records_never_mutate_task_queue_directly() {
        let human = classify_knowledge_source(
            "human/Projects/Annunimas/action.md",
            "# Annunimas Tasks
autopilot: true
- [ ] Annunimas: implement dry-run source triage CLI.",
        );
        assert_eq!(human.source_domain, SourceDomain::Human);
        assert!(human.actionable_operational_signal);
        assert_eq!(human.autonomy_lane, AutonomyLane::AutoCreateInternalTask);
        assert!(!human.mutate_task_queue);
        assert!(human.created_task_id.is_none());
    }

    #[test]
    fn dry_run_promotion_never_mutates_task_queue_or_receipts() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let docs = root.join("docs/plans");
        std::fs::create_dir_all(&docs).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            docs.join("safe.md"),
            "# Safe Plan
status: active
Next step: add tests for existing crate.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(true);
        let report =
            promote_knowledge_tasks(&cfg).unwrap_or_else(|err| panic!("promotion failed: {err}"));

        assert!(report.dry_run);
        assert_eq!(report.tasks_created, 0);
        assert!(!report.artifacts_written);
        assert!(!cfg.task_queue_path.exists());
        assert!(!cfg.promotion_receipts_path.exists());
        assert!(report
            .receipts
            .iter()
            .all(|receipt| !receipt.requires_human));
        assert!(report.receipts.iter().any(|receipt| receipt
            .receipt_reason
            .contains("dry-run: would create safe-local internal task")));
    }

    #[test]
    fn arandur_execution_guard_dry_run_selects_only_safe_local_lane3_tasks() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let docs = root.join("docs/plans");
        std::fs::create_dir_all(&docs).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            docs.join("safe.md"),
            "# Safe Plan
status: active
Next step: add tests for existing crate.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root)
            .with_dry_run(false)
            .with_approval_evidence("operator-approved:test-safe-local-execution-guard");
        promote_knowledge_tasks(&cfg).unwrap_or_else(|err| panic!("promotion failed: {err}"));
        let exec_cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(true);
        let report = execute_knowledge_task_queue(&exec_cfg)
            .unwrap_or_else(|err| panic!("execution guard failed: {err}"));

        assert!(report.dry_run);
        assert_eq!(report.tasks_seen, 1);
        assert_eq!(report.eligible_for_arandur, 1);
        assert_eq!(report.human_review_required, 0);
        assert_eq!(report.blocked, 0);
        assert!(!report.artifacts_written);
        assert!(!exec_cfg.execution_receipts_path.exists());
        assert_eq!(
            report.receipts[0].execution_decision,
            KnowledgeExecutionDecision::DryRunEligible
        );
        assert!(report.receipts[0]
            .receipt_reason
            .contains("dry-run: would hand off safe-local internal task to Arandur"));
    }

    #[test]
    fn arandur_execution_guard_stops_at_risk_boundaries() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(false);
        let risky_task = json!({
            "id": "tsk_risky",
            "title": "Rotate production credential",
            "owner": "prometheus",
            "status": "pending",
            "task_type": "safe_local_knowledge_task",
            "meta": {
                "origin": "prometheus_knowledge_task_promotion",
                "source_path": "docs/operations/credential.md",
                "dedupe_key": "knowledge:risky",
                "autonomy_lane": "human_approval_required",
                "risk_class": "credential_sensitive",
                "promotion_gate": KNOWLEDGE_SAFE_LOCAL_PROMOTION_GATE,
                "no_execution_during_promotion": true
            }
        });
        append_jsonl_values(&cfg.task_queue_path, &[risky_task])
            .unwrap_or_else(|err| panic!("write queue failed: {err}"));

        let report = execute_knowledge_task_queue(&cfg)
            .unwrap_or_else(|err| panic!("execution guard failed: {err}"));

        assert_eq!(report.tasks_seen, 1);
        assert_eq!(report.eligible_for_arandur, 0);
        assert_eq!(report.human_review_required, 1);
        assert_eq!(report.blocked, 0);
        assert!(report.artifacts_written);
        assert!(cfg.execution_receipts_path.exists());
        assert_eq!(
            report.receipts[0].execution_decision,
            KnowledgeExecutionDecision::HumanReviewRequired
        );
        assert!(report.receipts[0].requires_human);
        assert!(report.receipts[0]
            .receipt_reason
            .contains("risk boundary stops Arandur execution"));
    }

    #[test]
    fn write_promotion_requires_explicit_approval_evidence_before_task_queue_mutation() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let docs = root.join("docs/plans");
        std::fs::create_dir_all(&docs).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            docs.join("safe.md"),
            "# Safe Plan
status: active
Next step: add tests for existing crate.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root).with_dry_run(false);
        let report =
            promote_knowledge_tasks(&cfg).unwrap_or_else(|err| panic!("promotion failed: {err}"));

        assert!(!report.dry_run);
        assert!(report.approval_evidence_required);
        assert!(!report.approval_evidence_supplied);
        assert!(!report.queue_mutation_authorized);
        assert_eq!(report.tasks_created, 0);
        assert!(!report.artifacts_written);
        assert!(!cfg.task_queue_path.exists());
        assert!(!cfg.promotion_receipts_path.exists());
        assert!(report
            .receipts
            .iter()
            .any(|receipt| receipt.receipt_reason.contains(
                "explicit approval evidence required before safe-local task queue mutation"
            )));
    }

    #[test]
    fn write_promotion_appends_safe_local_tasks_and_skips_duplicates() {
        let dir = tempfile::tempdir().unwrap_or_else(|err| panic!("tempdir failed: {err}"));
        let root = dir.path();
        let docs = root.join("docs/plans");
        std::fs::create_dir_all(&docs).unwrap_or_else(|err| panic!("mkdir failed: {err}"));
        std::fs::write(
            docs.join("safe.md"),
            "# Safe Plan
status: active
Next step: add tests for existing crate.",
        )
        .unwrap_or_else(|err| panic!("write failed: {err}"));

        let cfg = KnowledgeTriageConfig::for_root(root)
            .with_dry_run(false)
            .with_approval_evidence("operator-approved:test-safe-local-promotion");
        let first = promote_knowledge_tasks(&cfg)
            .unwrap_or_else(|err| panic!("first promotion failed: {err}"));
        assert!(first.queue_mutation_authorized);
        assert!(first.approval_evidence_required);
        assert!(first.approval_evidence_supplied);
        assert_eq!(first.tasks_created, 1);
        assert!(first.artifacts_written);
        assert!(cfg.task_queue_path.exists());
        assert!(cfg.promotion_receipts_path.exists());

        let queue = std::fs::read_to_string(&cfg.task_queue_path)
            .unwrap_or_else(|err| panic!("read queue failed: {err}"));
        assert_eq!(queue.lines().count(), 1);
        assert!(queue.contains("operator-approved:test-safe-local-promotion"));
        assert!(queue.contains("prometheus_knowledge_task_promotion"));
        assert!(queue.contains("no_execution_during_promotion"));

        let second = promote_knowledge_tasks(&cfg)
            .unwrap_or_else(|err| panic!("second promotion failed: {err}"));
        assert_eq!(second.tasks_created, 0);
        assert_eq!(second.duplicates_skipped, 1);
        let queue_after = std::fs::read_to_string(&cfg.task_queue_path)
            .unwrap_or_else(|err| panic!("read queue after failed: {err}"));
        assert_eq!(queue_after.lines().count(), 1);
    }
}
