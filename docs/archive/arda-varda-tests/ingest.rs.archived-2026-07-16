// sigil: REPAIR
use arda_core::error::{ArdaError, Result};
use arda_core::llm::LlmProvider;
use arda_core::task::Task;
use arda_core::try_run_bounded;
use arda_governance::{
    calculate_resonance_basic, record_bacon_lite, triad_validate, GateOutcome, TriadConfig,
};
use arda_economics::JouleWorkUnit;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;

mod crawl;
mod curriculum_generator;
mod deep;
mod extraction;
mod github;
mod importers;
mod index;
mod interceptor;
mod io;
mod layout;
mod metrics;
mod observability;
mod policy;
mod query;
mod remediation;
mod routing;
mod scholarly;
mod source;
mod uncertainty_sampler;
mod views;
// mod dashboard_upgrade;
// mod joulework_tracking;
// mod validation_test;

use deep::{deep_summary_for_source, implementation_brief_for_source, scholarly_title_for_deep};
use interceptor::{
    DigestEvent, HadesQueueInterceptor, IngestCtx, MnemosyneInterceptor, WardenQueueInterceptor,
};
use layout::{human_library_root, machine_library_root};
use observability::deep_queue_status_counts;
use policy::{evaluate_policy_readiness, opposition_coverage_count};
use source::{
    build_shallow_analysis, canonicalize_ingest_input, classify_source, estimate_joule_cost,
    extract_url, love_equation_from_tags, normalize_graph_token, source_id_from_input,
};

pub use crawl::{
    crawl4ai_fetch_markdown, resolve_crawl_provider_order, scrapling_fetch_markdown,
    CrawlCaptureReceipt, CrawlMarkdownResult,
};
pub use extraction::ExtractedKnowledge;
pub use interceptor::{DigestEvent as AthenaDigestEvent, IngestInterceptor, IngestPipeline};
pub use metrics::{AthenaMetrics, AthenaMetricsSnapshot};

fn athena_error(message: impl Into<String>) -> ArdaError {
    ArdaError::Agent {
        agent: "athena".to_string(),
        message: message.into(),
    }
}

fn source_kind_label(kind: &SourceType) -> &'static str {
    match kind {
        SourceType::GithubRepo => "github_repo",
        SourceType::GithubFile => "github_file",
        SourceType::ScholarlyLink => "scholarly_link",
        SourceType::Documentation => "documentation",
        SourceType::NewsArticle => "news_article",
        SourceType::GovernmentDoc => "government_doc",
        SourceType::RawNote => "raw_note",
        SourceType::CodeSnippet => "code_snippet",
        SourceType::PdfDocument => "pdf_document",
        SourceType::XPost => "x_post",
        SourceType::XBookmark => "x_bookmark",
        SourceType::ChatExport => "chat_export",
    }
}

fn shallow_has_extractable_material(shallow: &ShallowAnalysis) -> bool {
    if let Some(meta) = &shallow.github_metadata {
        return meta.description.is_some()
            || meta
                .readme_excerpt
                .as_deref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
            || !meta.key_dependencies.is_empty();
    }
    if let Some(meta) = &shallow.scholarly_metadata {
        return !meta.abstract_text.trim().is_empty();
    }
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    GithubRepo,
    GithubFile,
    ScholarlyLink,
    Documentation,
    NewsArticle,
    GovernmentDoc,
    RawNote,
    CodeSnippet,
    PdfDocument,
    XPost,
    XBookmark,
    ChatExport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShallowAnalysis {
    pub title: String,
    pub summary: String,
    pub language: Option<String>,
    pub key_dependencies: Vec<String>,
    pub relevance_tags: Vec<String>,
    pub license: Option<String>,
    pub components_available: Vec<String>,
    pub reuse_potential: Option<f64>,
    pub deep_analysis_recommended: bool,
    pub deep_analysis_reason: String,
    #[serde(default)]
    pub scholarly_metadata: Option<ScholarlyMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_metadata: Option<GithubMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubMetadata {
    pub owner: String,
    pub repo: String,
    pub full_name: String,
    pub description: Option<String>,
    pub primary_language: Option<String>,
    pub license: Option<String>,
    pub default_branch: Option<String>,
    pub stargazers_count: Option<u64>,
    pub forks_count: Option<u64>,
    pub open_issues_count: Option<u64>,
    pub pushed_at: Option<String>,
    pub topics: Vec<String>,
    pub readme_excerpt: Option<String>,
    pub manifest_kind: Option<String>,
    pub key_dependencies: Vec<String>,
    pub file_path: Option<String>,
    pub ref_name: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScholarlyMetadata {
    pub paper_title: String,
    pub authors: Vec<String>,
    pub abstract_text: String,
    pub subjects: Vec<String>,
    pub comments: Option<String>,
    pub doi: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRecord {
    pub id: String,
    pub received_at_utc: String,
    pub processed_at_utc: String,
    pub source_type: SourceType,
    pub url: Option<String>,
    pub raw_input: String,
    pub submitted_by: String,
    pub task_context: String,
    pub digest_status: String,
    pub sigil: String,
    pub book_ref: String,
    pub shallow: ShallowAnalysis,
    pub deduplicated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTriageSoterion {
    pub sigil: String,
    pub glyph: String,
    pub retention: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeTriageEntry {
    pub schema_version: String,
    pub path: String,
    pub title: String,
    pub classification: String,
    pub soterion: KnowledgeTriageSoterion,
    pub canonical_home: String,
    pub domain: String,
    pub authority: String,
    pub recommended_action: String,
    pub rationale: String,
    pub headings: Vec<String>,
    pub bytes: usize,
    pub sha256_12: String,
    pub triaged_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestReceipt {
    pub source_id: String,
    pub input: String,
    pub canonical_input: String,
    pub url: Option<String>,
    pub deduplicated: bool,
    pub digest_status: String,
    pub book_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchIngestReport {
    pub total_inputs: usize,
    pub accepted_inputs: usize,
    pub deduplicated_inputs: usize,
    pub invalid_inputs: usize,
    pub receipts: Vec<BatchIngestReceipt>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookEntry {
    pub version: u32,
    pub stage: String,
    pub written_at_utc: String,
    pub sigil: String,
    pub data: ShallowAnalysis,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriadAnalysis {
    pub aurelius: GateOutcome,
    pub bacon: GateOutcome,
    pub sun_tzu: GateOutcome,
    pub aurelius_score: f64,
    pub bacon_score: f64,
    pub sun_tzu_score: f64,
    pub passed: bool,
    pub consensus_threshold: f64,
    pub pass_count: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JouleworkAnalysis {
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub balance_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoveEquationAnalysis {
    pub alignment_score: f64,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepAnalysisData {
    pub title: String,
    pub full_summary: String,
    pub relevance_tags: Vec<String>,
    #[serde(default)]
    pub inference_route: serde_json::Value,
    pub triad_analysis: TriadAnalysis,
    pub joulework: JouleworkAnalysis,
    pub love_equation: LoveEquationAnalysis,
    pub deep_analysis_recommended: bool,
    pub confidence: f64,
    #[serde(default)]
    pub policy_readiness: String,
    #[serde(default)]
    pub policy_gate: serde_json::Value,
    #[serde(default)]
    pub implementation_brief: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extracted_knowledge: Option<ExtractedKnowledge>,
    #[serde(default)]
    pub extraction_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepBookEntry {
    pub version: u32,
    pub stage: String,
    pub written_at_utc: String,
    pub sigil: String,
    pub data: DeepAnalysisData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepQueueRecord {
    pub ts: String,
    pub event: String,
    pub source_id: String,
    pub agent: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMatch {
    pub source_id: String,
    pub book_ref: String,
    pub score: f64,
    pub digest_status: String,
    pub title: String,
    pub summary: String,
    pub relevance_tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub concepts_hit: Vec<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub extraction_status: String,
    #[serde(default, skip_serializing_if = "is_zero_f64")]
    pub confidence_self_report: f64,
}

fn is_zero_f64(value: &f64) -> bool {
    *value == 0.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub query: String,
    pub total_matches: usize,
    pub matches: Vec<QueryMatch>,
    pub suggestion: Option<String>,
}

#[derive(Clone)]
pub struct AthenaStore {
    root: PathBuf,
    books_dir: PathBuf,
    digest_path: PathBuf,
    crawl_receipts_path: PathBuf,
    uncertainty_selections_path: PathBuf,
    crawl_artifacts_dir: PathBuf,
    deep_queue_path: PathBuf,
    deep_graph_path: PathBuf,
    policy_readiness_path: PathBuf,
    planning_task_receipts_path: PathBuf,
    human_sources_dir: PathBuf,
    machine_index_path: PathBuf,
    interceptors: IngestPipeline,
    llm: Option<Arc<dyn LlmProvider>>,
    digest_index: Arc<std::sync::RwLock<Option<index::DigestIndex>>>,
    metrics: Arc<AthenaMetrics>,
}

impl std::fmt::Debug for AthenaStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AthenaStore")
            .field("root", &self.root)
            .field("books_dir", &self.books_dir)
            .field("digest_path", &self.digest_path)
            .field("deep_queue_path", &self.deep_queue_path)
            .field("llm_attached", &self.llm.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaKnowledgeVaultSourceLaneObservation {
    pub lane: String,
    pub ingested_sources_total: usize,
    pub policy_ready_sources_total: usize,
    pub latest_observed_at_utc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaAutonomyRecommendation {
    pub recommendation_id: String,
    pub lane: String,
    pub action: String,
    pub rationale: String,
    pub safe_local: bool,
    pub human_gate_required: bool,
    pub evidence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaVaultSynthesisQueueItem {
    pub synthesis_id: String,
    pub rank: usize,
    pub lane: String,
    pub recommended_action: String,
    pub rationale: String,
    pub evidence_count: usize,
    pub priority_score: f64,
    pub safe_local: bool,
    pub human_gate_required: bool,
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaKnowledgeVaultStatus {
    pub doctrine_path: String,
    pub authority: String,
    pub local_first_recall: bool,
    pub layers: Vec<String>,
    pub source_lanes: Vec<String>,
    pub autonomy_feed: Vec<String>,
    pub source_lane_observations_total: usize,
    pub source_lane_observations: Vec<AthenaKnowledgeVaultSourceLaneObservation>,
    pub autonomy_recommendations_total: usize,
    pub autonomy_recommendations: Vec<AthenaAutonomyRecommendation>,
    pub synthesis_queue_total: usize,
    pub synthesis_queue: Vec<AthenaVaultSynthesisQueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AthenaStatus {
    pub storage_root: String,
    pub digest_path: String,
    pub deep_queue_path: String,
    pub deep_graph_path: String,
    pub policy_readiness_path: String,
    pub planning_task_receipts_path: String,
    pub books_count: usize,
    pub digest_events: usize,
    pub deep_queue_depth: usize,
    pub deep_queue_failed: usize,
    pub deep_graph_events: usize,
    pub ingest_success_total: usize,
    pub deduplicated_ingests_total: usize,
    pub duplicate_hit_rate: f64,
    pub avg_deep_queue_latency_seconds: f64,
    pub policy_ready_count: usize,
    pub reference_only_count: usize,
    pub policy_ready_promotions_total: usize,
    pub policy_ready_regressions_total: usize,
    pub policy_readiness_malformed_records: usize,
    pub primary_policy_ready_count: usize,
    pub primary_reference_only_count: usize,
    pub synthetic_policy_ready_count: usize,
    pub synthetic_reference_only_count: usize,
    pub execution_authority: String,
    pub execution_posture: String,
    pub operator_ingress_role: String,
    pub source_provenance_coverage_ratio: f64,
    pub memory_lanes: Vec<String>,
    pub task_emission_receipts_total: usize,
    pub task_emission_success_total: usize,
    pub task_emission_skipped_total: usize,
    pub task_emission_last_run_at_utc: Option<String>,
    pub knowledge_vault: AthenaKnowledgeVaultStatus,
}

impl AthenaStore {
    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let books_dir = root.join("books");
        let digest_path = root.join("digest.jsonl");
        let crawl_receipts_path = root.join("crawl_receipts.jsonl");
        let uncertainty_selections_path = root.join("uncertainty_selections.jsonl");
        let crawl_artifacts_dir = root.join("crawls");
        let deep_queue_path = root.join("deep_queue.jsonl");
        let deep_graph_path = root.join("deep_graph.jsonl");
        let policy_readiness_path = root.join("policy_readiness.jsonl");
        let planning_task_receipts_path = root.join("planning_task_receipts.jsonl");
        let hades_queue_path = std::env::var("ARDA_HADES_ACTION_QUEUE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                crate::ingest::layout::arda_root().join("data/hades/action_queue.jsonl")
            });
        let warden_queue_path = std::env::var("ARDA_WARDEN_QUEUE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                crate::ingest::layout::arda_root().join("data/warden/informant_queue.jsonl")
            });
        let human_sources_dir = human_library_root().join("sources");
        let machine_index_path = machine_library_root().join("index").join("sources.jsonl");

        fs::create_dir_all(&books_dir)?;
        fs::create_dir_all(&crawl_artifacts_dir)?;
        fs::create_dir_all(&human_sources_dir)?;
        if let Some(parent) = machine_index_path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&digest_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&crawl_receipts_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&uncertainty_selections_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&deep_queue_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&deep_graph_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&policy_readiness_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&planning_task_receipts_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&machine_index_path)?;
        if let Some(parent) = hades_queue_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if let Some(parent) = warden_queue_path.parent() {
            fs::create_dir_all(parent)?;
        }
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&hades_queue_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&warden_queue_path)?;

        let interceptors = IngestPipeline::new();
        interceptors.register(HadesQueueInterceptor::new(&hades_queue_path));
        interceptors.register(WardenQueueInterceptor::new(&warden_queue_path));
        interceptors.register(MnemosyneInterceptor::from_default());

        Ok(Self {
            root,
            books_dir,
            digest_path,
            crawl_receipts_path,
            uncertainty_selections_path,
            crawl_artifacts_dir,
            deep_queue_path,
            deep_graph_path,
            policy_readiness_path,
            planning_task_receipts_path,
            human_sources_dir,
            machine_index_path,
            interceptors,
            llm: None,
            digest_index: Arc::new(std::sync::RwLock::new(None)),
            metrics: Arc::new(AthenaMetrics::new()),
        })
    }

    /// Borrow the in-process metrics handle for external readers (daemon
    /// status command, HTTP `/metrics`, IPC `metrics` command). Cheap clone
    /// since the inner store is behind an `Arc`.
    pub fn metrics(&self) -> Arc<AthenaMetrics> {
        self.metrics.clone()
    }

    /// Build (or rebuild) the in-memory digest index. Reads every book file
    /// from disk and assembles a flat searchable snapshot. Subsequent
    /// queries hit RAM until the books-dir mtime changes or the TTL expires.
    pub fn warm_digest_index(&self) -> Result<usize> {
        let books_dir = self.books_dir.clone();
        let books_dir_for_ref = self.books_dir.clone();
        let new_index = index::rebuild_index(&books_dir, |id| {
            books_dir_for_ref
                .join(format!("{id}.jsonl"))
                .display()
                .to_string()
        })?;
        let count = new_index.entries.len();
        let mut guard = self
            .digest_index
            .write()
            .map_err(|e| athena_error(format!("digest index lock poisoned: {e}")))?;
        *guard = Some(new_index);
        Ok(count)
    }

    pub(in crate::ingest) fn invalidate_digest_index(&self) {
        if let Ok(mut guard) = self.digest_index.write() {
            *guard = None;
        }
    }

    pub(in crate::ingest) fn with_digest_index<F, R>(&self, f: F) -> Result<R>
    where
        F: FnOnce(&index::DigestIndex) -> R,
    {
        let current_mtime = index::books_dir_mtime(&self.books_dir);
        {
            let guard = self
                .digest_index
                .read()
                .map_err(|e| athena_error(format!("digest index lock poisoned: {e}")))?;
            if let Some(idx) = guard.as_ref() {
                if idx.is_fresh(current_mtime) {
                    return Ok(f(idx));
                }
            }
        }
        // Rebuild under write lock — release after.
        let books_dir = self.books_dir.clone();
        let books_dir_for_ref = self.books_dir.clone();
        let rebuilt = index::rebuild_index(&books_dir, |id| {
            books_dir_for_ref
                .join(format!("{id}.jsonl"))
                .display()
                .to_string()
        })?;
        let mut guard = self
            .digest_index
            .write()
            .map_err(|e| athena_error(format!("digest index lock poisoned: {e}")))?;
        *guard = Some(rebuilt);
        let Some(view) = guard.as_ref() else {
            return Err(athena_error("digest index rebuild did not populate cache"));
        };
        Ok(f(view))
    }

    /// Attach an LLM provider for deep-analysis knowledge extraction.
    /// When `None` (the default), `deep_analyze` falls back to the
    /// deterministic governance scaffold without an extraction phase.
    pub fn with_llm(mut self, llm: Arc<dyn LlmProvider>) -> Self {
        self.llm = Some(llm);
        self
    }

    /// Returns true if an LLM provider is wired in for digestion extraction.
    pub fn llm_attached(&self) -> bool {
        self.llm.is_some()
    }

    pub fn crawl_receipts_path(&self) -> &Path {
        &self.crawl_receipts_path
    }

    pub fn uncertainty_selections_path(&self) -> &Path {
        &self.uncertainty_selections_path
    }

    pub fn interceptor_names(&self) -> Vec<String> {
        self.interceptors.names()
    }

    fn event_ctx(&self, operation: &str, source_id: &str) -> IngestCtx {
        IngestCtx::new(
            operation,
            source_id,
            "",
            "",
            "athena",
            "digest lifecycle side-effect",
        )
    }

    pub fn ingest(
        &self,
        raw_input: &str,
        submitted_by: &str,
        task_context: &str,
    ) -> Result<IngestRecord> {
        let Some(result) = try_run_bounded("athena_ingest", athena_ingest_limit(), || {
            let normalized = raw_input.trim();
            if normalized.is_empty() {
                return Err(athena_error("ingest payload cannot be empty"));
            }

            let canonical_input = canonicalize_ingest_input(normalized);
            let source_id = source_id_from_input(&canonical_input);
            let book_path = self.books_dir.join(format!("{source_id}.jsonl"));
            let book_ref = self.book_ref_for(&source_id);
            let now = Utc::now().to_rfc3339();
            let source_type = classify_source(&canonical_input);
            let url = extract_url(&canonical_input);
            let deduplicated = book_path.exists();
            let mut ingest_ctx = IngestCtx::new(
                "athena_ingest",
                &source_id,
                normalized,
                &canonical_input,
                submitted_by,
                task_context,
            );
            ingest_ctx.source_type = Some(source_type.clone());
            ingest_ctx.url = url.clone();
            ingest_ctx.metadata = serde_json::json!({
                "deduplicated": deduplicated,
                "book_ref": book_ref.clone()
            });
            self.interceptors.before(&mut ingest_ctx);

            let shallow = build_shallow_analysis(
                &canonical_input,
                &source_type,
                deduplicated,
                url.as_deref(),
            );
            let digest_status = if deduplicated {
                "shallow_existing"
            } else {
                "shallow"
            }
            .to_string();

            let record = IngestRecord {
                id: source_id.clone(),
                received_at_utc: now.clone(),
                processed_at_utc: now.clone(),
                source_type,
                url,
                raw_input: normalized.to_string(),
                submitted_by: submitted_by.to_string(),
                task_context: task_context.to_string(),
                digest_status,
                sigil: "ANKH".to_string(),
                book_ref: book_ref.clone(),
                shallow: shallow.clone(),
                deduplicated,
                error: None,
            };

            self.append_jsonl(&self.digest_path, &record)?;

            let source_kind_label = source_kind_label(&record.source_type);
            self.metrics.observe_ingest(
                source_kind_label,
                metrics::classify_ingest_outcome(deduplicated, None),
            );

            if !deduplicated {
                let book_entry = BookEntry {
                    version: 1,
                    stage: "shallow".to_string(),
                    written_at_utc: now,
                    sigil: "ANKH".to_string(),
                    data: shallow,
                };
                self.append_jsonl(&book_path, &book_entry)?;
                self.invalidate_digest_index();
            }
            let existing_deep = self.latest_deep_book_entry(&record.id).ok().flatten();
            if let Err(err) = self.sync_knowledge_views(
                &record.id,
                Some(&record),
                Some(&record.shallow),
                existing_deep.as_ref(),
            ) {
                tracing::warn!(error = %err, source_id = %record.id, "ATHENA knowledge view sync failed");
            }
            if let Err(err) = self.emit_ingest_triage_entry(&record) {
                tracing::warn!(error = %err, source_id = %record.id, "ATHENA triage registry emission failed");
            }
            let mut bacon_task = Task::new(format!("ingest {}", record.raw_input), "ingest");
            bacon_task.clarifications_resolved = if record.url.is_some() { 1 } else { 0 };
            if let Err(err) = record_bacon_lite(
                "athena",
                "ingest",
                &bacon_task,
                serde_json::json!({
                    "source_id": record.id,
                    "deduplicated": record.deduplicated,
                    "source_type": format!("{:?}", record.source_type),
                }),
            ) {
                tracing::debug!(error = %err, "ATHENA bacon-lite record failed");
            }
            self.interceptors.after(
                &ingest_ctx,
                &DigestEvent::ShallowSynced {
                    source_id: record.id.clone(),
                    source_type: record.source_type.clone(),
                    url: record.url.clone(),
                    deduplicated: record.deduplicated,
                },
            );

            Ok(record)
        }) else {
            return Err(athena_error("ingest concurrency gate saturated"));
        };

        result
    }

    pub fn ingest_batch(
        &self,
        inputs: &[String],
        submitted_by: &str,
        task_context: &str,
    ) -> Result<BatchIngestReport> {
        let mut receipts = Vec::new();
        let mut accepted_inputs = 0usize;
        let mut deduplicated_inputs = 0usize;
        let mut invalid_inputs = 0usize;

        for input in inputs {
            let trimmed = input.trim();
            if trimmed.is_empty() {
                invalid_inputs += 1;
                continue;
            }

            let canonical_input = canonicalize_ingest_input(trimmed);
            let record = self.ingest(&canonical_input, submitted_by, task_context)?;
            accepted_inputs += 1;
            if record.deduplicated {
                deduplicated_inputs += 1;
            }

            receipts.push(BatchIngestReceipt {
                source_id: record.id,
                input: trimmed.to_string(),
                canonical_input,
                url: record.url,
                deduplicated: record.deduplicated,
                digest_status: record.digest_status,
                book_ref: record.book_ref,
            });
        }

        Ok(BatchIngestReport {
            total_inputs: inputs.len(),
            accepted_inputs,
            deduplicated_inputs,
            invalid_inputs,
            receipts,
        })
    }

    pub fn queue_deep_analysis(
        &self,
        source_id: &str,
        agent: &str,
        reason: &str,
    ) -> Result<DeepQueueRecord> {
        let normalized = source_id.trim();
        if normalized.is_empty() {
            return Err(athena_error("source_id cannot be empty"));
        }

        let record = DeepQueueRecord {
            ts: Utc::now().to_rfc3339(),
            event: "deep_queued".to_string(),
            source_id: normalized.to_string(),
            agent: agent.to_string(),
            status: "pending_deep".to_string(),
            reason: reason.to_string(),
        };
        self.append_jsonl(&self.deep_queue_path, &record)?;
        self.append_jsonl(&self.digest_path, &record)?;
        let ctx = self.event_ctx("athena_deep_queued", normalized);
        self.interceptors.after(
            &ctx,
            &DigestEvent::DeepQueued {
                source_id: normalized.to_string(),
                agent: agent.to_string(),
                reason: reason.to_string(),
            },
        );
        let (pending_deep, _) = deep_queue_status_counts(&self.deep_queue_path)?;
        if pending_deep == 101 {
            let warning = serde_json::json!({
                "ts": Utc::now().to_rfc3339(),
                "event": "deep_queue_backlog_warning",
                "source_id": normalized,
                "agent": "athena",
                "status": "warning",
                "reason": format!("ATHENA deep queue backlog exceeded threshold: {}", pending_deep)
            });
            self.append_jsonl(&self.digest_path, &warning)?;
            self.interceptors.after(
                &ctx,
                &DigestEvent::BacklogWarning {
                    source_id: normalized.to_string(),
                    pending: pending_deep,
                    threshold: 100,
                },
            );
        }
        Ok(record)
    }

    /// Run the LLM extraction phase for a shallow record. Returns the
    /// extracted knowledge and a status string for telemetry. Falls back
    /// gracefully when no LLM is attached or when the call/parse fails.
    fn run_extraction(&self, shallow: &ShallowAnalysis) -> (Option<ExtractedKnowledge>, String) {
        let Some(llm) = self.llm.clone() else {
            return (None, "no_llm_attached".to_string());
        };
        if !shallow_has_extractable_material(shallow) {
            return (None, "no_extractable_material".to_string());
        }
        let shallow_for_async = shallow.clone();
        let fut = async move { extraction::extract_knowledge(llm, &shallow_for_async).await };
        match routing::run_async_for_sync(fut) {
            Ok(knowledge) => {
                let status = if knowledge.parse_error.is_some() {
                    "llm_extraction_parse_failed".to_string()
                } else {
                    "llm_extraction_complete".to_string()
                };
                (Some(knowledge), status)
            }
            Err(err) => {
                tracing::warn!(error = %err, "ATHENA extraction failed");
                (None, "llm_extraction_failed".to_string())
            }
        }
    }

    pub fn deep_analyze(&self, source_id: &str) -> Result<DeepBookEntry> {
        let Some(result) = try_run_bounded(
            "athena_deep_analyze",
            athena_deep_queue_limit(),
            || {
                let source_id = source_id.trim();
                let book_path = self.books_dir.join(format!("{source_id}.jsonl"));
                if !book_path.exists() {
                    return Err(athena_error(format!(
                        "source not found in books: {source_id}"
                    )));
                }

                let content = fs::read_to_string(&book_path)?;
                let line_count = content.lines().count() as u32;
                let shallow = content
                    .lines()
                    .find_map(|line| {
                        let value: serde_json::Value = serde_json::from_str(line).ok()?;
                        if value.get("stage").and_then(|s| s.as_str()) == Some("shallow") {
                            return serde_json::from_value::<BookEntry>(value).ok();
                        }
                        None
                    })
                    .ok_or_else(|| {
                        athena_error(format!("missing shallow entry for source: {source_id}"))
                    })?;
                let shallow = self.recover_shallow_analysis(source_id, shallow)?;

                let mut task = Task::new(
                    format!(
                        "deep analyze {} {}",
                        shallow.data.title, shallow.data.deep_analysis_reason
                    ),
                    "deep_analyze",
                );
                task.joule_cost_estimated = estimate_joule_cost(&shallow.data);
                task.joule_cost_actual = task.joule_cost_estimated * 1.08;
                task.clarifications_requested = 1;
                task.clarifications_resolved = 1;
                task.complete(serde_json::json!({ "source_id": source_id }));

                let triad = triad_validate(
                    &task,
                    Some(&TriadConfig {
                        strict: false,
                        required_passes: Some(2),
                    }),
                );
                let pass_count = [
                    triad.aurelius == GateOutcome::Pass,
                    triad.bacon == GateOutcome::Pass,
                    triad.sun_tzu == GateOutcome::Pass,
                ]
                .iter()
                .filter(|&&x| x)
                .count() as u8;
                let triad_average =
                    (triad.aurelius_score + triad.bacon_score + triad.sun_tzu_score) / 3.0;
                let opposition_coverage = opposition_coverage_count(&self.digest_path, source_id)?;
                // Evidence-weighted uplift: opposing viewpoints reduce brittleness of the raw triad average.
                let triad_evidence_uplift = (opposition_coverage.min(2) as f64) * 0.06;
                let triad_effective = (triad_average + triad_evidence_uplift).clamp(0.0, 1.0);
                let consensus_threshold = if opposition_coverage >= 2 { 0.65 } else { 0.75 };
                let triad_passed = pass_count >= 2 && triad_effective >= consensus_threshold;

                let resonance = calculate_resonance_basic(&task);
                let joule_balance_score = resonance
                    .ecst_components
                    .as_ref()
                    .map(|c| c.joule_balance)
                    .unwrap_or(50.0);
                let (love_score, love_rationale) =
                    love_equation_from_tags(&shallow.data.relevance_tags);
                let inference_route = routing::resolve_inference_route_snapshot(
                    source_id,
                    &shallow.data.title,
                    &shallow.data.relevance_tags,
                );

                let (extracted_knowledge, extraction_status) = self.run_extraction(&shallow.data);

                let deep_data = DeepAnalysisData {
                    title: scholarly_title_for_deep(&shallow.data),
                    full_summary: deep_summary_for_source(&shallow.data),
                    relevance_tags: shallow.data.relevance_tags.clone(),
                    inference_route,
                    triad_analysis: TriadAnalysis {
                        aurelius: triad.aurelius,
                        bacon: triad.bacon,
                        sun_tzu: triad.sun_tzu,
                        aurelius_score: triad.aurelius_score,
                        bacon_score: triad.bacon_score,
                        sun_tzu_score: triad.sun_tzu_score,
                        passed: triad_passed,
                        consensus_threshold,
                        pass_count,
                    },
                    joulework: JouleworkAnalysis {
                        estimated_cost: task.joule_cost_estimated,
                        actual_cost: task.joule_cost_actual,
                        balance_score: joule_balance_score,
                    },
                    love_equation: LoveEquationAnalysis {
                        alignment_score: love_score,
                        rationale: love_rationale,
                    },
                    deep_analysis_recommended: !triad_passed || love_score < 0.7,
                    confidence: ((triad_effective + love_score) / 2.0).min(1.0),
                    policy_readiness: String::new(),
                    policy_gate: serde_json::json!({}),
                    implementation_brief: implementation_brief_for_source(&shallow.data),
                    extracted_knowledge,
                    extraction_status,
                };
                let (policy_readiness, policy_gate) =
                    evaluate_policy_readiness(&shallow, &deep_data, source_id, opposition_coverage);

                let deep_entry = DeepBookEntry {
                    version: line_count + 1,
                    stage: "deep".to_string(),
                    written_at_utc: Utc::now().to_rfc3339(),
                    sigil: "EYE".to_string(),
                    data: DeepAnalysisData {
                        policy_readiness,
                        policy_gate,
                        ..deep_data
                    },
                };
                self.append_jsonl(&book_path, &deep_entry)?;
                self.invalidate_digest_index();
                let _ = self.append_jsonl(
                    &self.policy_readiness_path,
                    &serde_json::json!({
                        "ts_utc": Utc::now().to_rfc3339(),
                        "source_id": source_id,
                        "policy_readiness": deep_entry.data.policy_readiness,
                        "gate": deep_entry.data.policy_gate
                    }),
                );

                let event_reason = match deep_entry.data.extraction_status.as_str() {
                    "llm_extraction_complete" => "llm_extraction_complete".to_string(),
                    "llm_extraction_failed" => {
                        "deterministic scaffold complete (llm extraction failed)".to_string()
                    }
                    _ => "deterministic scaffold complete".to_string(),
                };
                let event = DeepQueueRecord {
                    ts: Utc::now().to_rfc3339(),
                    event: "deep_complete".to_string(),
                    source_id: source_id.to_string(),
                    agent: "athena".to_string(),
                    status: "deep".to_string(),
                    reason: event_reason,
                };
                self.append_jsonl(&self.deep_queue_path, &event)?;
                self.append_jsonl(&self.digest_path, &event)?;
                let ctx = self.event_ctx("athena_deep_complete", source_id);
                self.interceptors.after(
                    &ctx,
                    &DigestEvent::DeepSynced {
                        source_id: source_id.to_string(),
                        policy_readiness: deep_entry.data.policy_readiness.clone(),
                        confidence: deep_entry.data.confidence,
                    },
                );
                if deep_entry.data.policy_readiness == "policy_ready" {
                    self.emit_relationship_signal_background(
                        "athena".to_string(),
                        "knowledge_corpus".to_string(),
                        deep_entry.data.confidence.clamp(0.35, 0.96),
                        deep_entry
                            .data
                            .love_equation
                            .alignment_score
                            .clamp(0.2, 0.95),
                        0.82,
                        "athena_deep_complete",
                    );
                    self.emit_work_signal_background(
                        "athena".to_string(),
                        deep_entry.data.confidence.clamp(0.35, 0.95),
                        JouleWorkUnit::Reasoning,
                        "athena_deep_complete",
                    );
                }
                let ingest_record = self.latest_ingest_record(source_id).ok().flatten();
                if let Err(err) = self.sync_knowledge_views(
                    source_id,
                    ingest_record.as_ref(),
                    Some(&shallow.data),
                    Some(&deep_entry),
                ) {
                    tracing::warn!(error = %err, source_id = %source_id, "ATHENA deep knowledge view sync failed");
                }
                if let Err(err) =
                    self.append_deep_graph_event(source_id, &shallow.data, &deep_entry)
                {
                    tracing::warn!(error = %err, source_id = %source_id, "ATHENA deep graph append failed");
                }
                if let Err(err) = record_bacon_lite(
                    "athena",
                    "deep_analyze",
                    &task,
                    serde_json::json!({
                        "source_id": source_id,
                        "triad_passed": deep_entry.data.triad_analysis.passed,
                        "confidence": deep_entry.data.confidence,
                    }),
                ) {
                    tracing::debug!(error = %err, "ATHENA deep bacon-lite record failed");
                }

                Ok(deep_entry)
            },
        ) else {
            return Err(athena_error("deep analysis concurrency gate saturated"));
        };

        result
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn digest_path(&self) -> &Path {
        &self.digest_path
    }

    pub fn deep_queue_path(&self) -> &Path {
        &self.deep_queue_path
    }

    pub fn process_deep_queue(
        &self,
        limit: usize,
        retry_failed: bool,
    ) -> Result<serde_json::Value> {
        let Some(result) = try_run_bounded("athena_deep_queue", athena_deep_queue_limit(), || {
            let content = fs::read_to_string(&self.deep_queue_path)?;
            let mut latest = std::collections::HashMap::<String, String>::new();
            for line in content.lines() {
                if line.trim().is_empty() {
                    continue;
                }
                let value: serde_json::Value = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let Some(source_id) = value.get("source_id").and_then(|v| v.as_str()) else {
                    continue;
                };
                let status = value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending_deep");
                latest.insert(source_id.to_string(), status.to_string());
            }

            let mut processed = 0usize;
            let mut success = 0usize;
            let mut failed = 0usize;
            let mut details = Vec::new();
            for (source_id, status) in latest {
                let can_run = status == "pending_deep" || (retry_failed && status == "failed");
                if !can_run {
                    continue;
                }
                if processed >= limit.max(1) {
                    break;
                }
                processed += 1;
                match self.deep_analyze(&source_id) {
                    Ok(entry) => {
                        success += 1;
                        details.push(serde_json::json!({
                            "source_id": source_id,
                            "ok": true,
                            "confidence": entry.data.confidence
                        }));
                    }
                    Err(err) => {
                        failed += 1;
                        let event = DeepQueueRecord {
                            ts: Utc::now().to_rfc3339(),
                            event: "deep_failed".to_string(),
                            source_id: source_id.clone(),
                            agent: "athena".to_string(),
                            status: "failed".to_string(),
                            reason: err.to_string(),
                        };
                        let _ = self.append_jsonl(&self.deep_queue_path, &event);
                        let _ = self.append_jsonl(&self.digest_path, &event);
                        let ctx = self.event_ctx("athena_deep_failed", &source_id);
                        self.interceptors.after(
                            &ctx,
                            &DigestEvent::DeepFailed {
                                source_id: source_id.clone(),
                                reason: err.to_string(),
                            },
                        );
                        details.push(serde_json::json!({
                            "source_id": source_id,
                            "ok": false,
                            "error": err.to_string()
                        }));
                    }
                }
            }
            Ok(serde_json::json!({
                "processed": processed,
                "success": success,
                "failed": failed,
                "retry_failed": retry_failed,
                "details": details,
            }))
        }) else {
            return Err(athena_error("deep queue concurrency gate saturated"));
        };

        result
    }
}

fn athena_ingest_limit() -> usize {
    #[cfg(test)]
    let default = 128;
    #[cfg(not(test))]
    let default = 2;

    std::env::var("ARDA_VARDA_INGEST_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn athena_deep_queue_limit() -> usize {
    #[cfg(test)]
    let default = 64;
    #[cfg(not(test))]
    let default = 1;

    std::env::var("ARDA_VARDA_DEEP_QUEUE_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn athena_crawl_limit() -> usize {
    #[cfg(test)]
    let default = 32;
    #[cfg(not(test))]
    let default = 1;

    std::env::var("ARDA_VARDA_CRAWL_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

#[cfg(test)]
// Tests that mutate process environment variables must serialize across await
// points so no other test observes a partially configured runtime. This is
// test-scaffolding only; production code must not hold std mutex guards across
// async boundaries.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::scholarly::{offline_scholarly_metadata, parse_arxiv_api_response};
    use super::{
        crawl4ai_fetch_markdown, resolve_crawl_provider_order, scrapling_fetch_markdown,
        source_id_from_input, AthenaStore, CrawlMarkdownResult,
    };
    use arda_core::try_run_bounded_async;
    use arda_plutus::PlutusService;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::path::Path;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::sync::oneshot;

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_support::env_guard()
    }

    #[test]
    fn ingest_writes_digest_and_book() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        let record = store
            .ingest(
                "https://github.com/example/project",
                "orchestrator",
                "test ingest",
            )
            .expect("ingest");

        let digest = fs::read_to_string(store.digest_path()).expect("digest");
        assert!(digest.contains(&record.id));

        let book_path = dir.path().join(record.book_ref);
        let book = fs::read_to_string(book_path).expect("book");
        assert!(book.contains("\"stage\":\"shallow\""));
    }

    #[test]
    fn ingest_skips_workspace_triage_registry_for_noncanonical_store() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let previous_root = std::env::var_os("ARDA_ROOT");
        let previous_registry = std::env::var_os("ARDA_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        // SAFETY: guarded by `env_guard`, so no sibling ATHENA test mutates or
        // reads these process-wide environment variables concurrently.
        unsafe {
            std::env::remove_var("ARDA_ROOT");
            std::env::remove_var("ARDA_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
        }

        let store = AthenaStore::new(dir.path().join("athena")).expect("store");
        let record = store
            .ingest(
                "manual operator memory seed about imported corpus",
                "orchestrator",
                "knowledge triage contract",
            )
            .expect("ingest");

        assert!(!record.id.is_empty());
        let workspace_registry = crate::ingest::layout::arda_root()
            .join("core/state/knowledge_triage_registry.jsonl");
        let registry = fs::read_to_string(&workspace_registry).unwrap_or_default();
        let expected_path = format!("data/athena/books/{}.jsonl", record.id);
        assert!(
            !registry.contains(&expected_path),
            "non-canonical test store must not append to workspace registry"
        );

        // SAFETY: guarded by `env_guard`, so restoration is serialized with all
        // ATHENA tests that manipulate the process environment.
        unsafe {
            if let Some(value) = previous_root {
                std::env::set_var("ARDA_ROOT", value);
            } else {
                std::env::remove_var("ARDA_ROOT");
            }
            if let Some(value) = previous_registry {
                std::env::set_var("ARDA_KNOWLEDGE_TRIAGE_REGISTRY_PATH", value);
            } else {
                std::env::remove_var("ARDA_KNOWLEDGE_TRIAGE_REGISTRY_PATH");
            }
        }
    }

    #[test]
    fn ingest_emits_knowledge_triage_entry() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("core/state")).expect("core state dir");
        fs::create_dir_all(root.join("data/knowledge")).expect("knowledge dir");

        let previous_root = std::env::var_os("ARDA_ROOT");
        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_ROOT", &root);
        }

        let store = AthenaStore::new(dir.path().join("athena")).expect("store");
        let record = store
            .ingest(
                "manual operator memory seed about imported corpus",
                "orchestrator",
                "knowledge triage contract",
            )
            .expect("ingest");

        let registry_path = root.join("core/state/knowledge_triage_registry.jsonl");
        let registry = fs::read_to_string(&registry_path).expect("registry");
        let expected_path = format!("data/athena/books/{}.jsonl", record.id);
        let entry: serde_json::Value = registry
            .lines()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|entry| {
                entry.get("path").and_then(|path| path.as_str()) == Some(expected_path.as_str())
            })
            .unwrap_or_else(|| panic!("missing registry entry for {expected_path}"));

        assert_eq!(entry["schema_version"], "arda.knowledge_triage.v1");
        assert_eq!(entry["path"], expected_path);
        assert_eq!(entry["classification"], "memory_seed");
        assert_eq!(entry["canonical_home"], "data/athena");
        assert_eq!(entry["soterion"]["glyph"], "🜄");

        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            if let Some(value) = previous_root {
                std::env::set_var("ARDA_ROOT", value);
            } else {
                std::env::remove_var("ARDA_ROOT");
            }
        }
    }

    #[test]
    fn ingest_skips_fixture_domain_triage_entries() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join("root");
        fs::create_dir_all(root.join("core/state")).expect("core state dir");

        let previous_root = std::env::var_os("ARDA_ROOT");
        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_ROOT", &root);
        }

        let store = AthenaStore::new(dir.path().join("athena")).expect("store");
        let _record = store
            .ingest(
                "https://example.com/governance-report",
                "orchestrator",
                "fixture promotion guard",
            )
            .expect("ingest");

        let registry_path = root.join("core/state/knowledge_triage_registry.jsonl");
        if registry_path.exists() {
            let registry = fs::read_to_string(&registry_path).expect("registry");
            assert!(
                !registry.contains("https://example.com/governance-report"),
                "fixture domain entry should not be promoted: {registry}"
            );
        }

        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            if let Some(value) = previous_root {
                std::env::set_var("ARDA_ROOT", value);
            } else {
                std::env::remove_var("ARDA_ROOT");
            }
        }
    }

    #[test]
    fn ingest_deduplicates_existing_source() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let input = "https://github.com/example/project";

        let first = store.ingest(input, "orchestrator", "one").expect("first");
        let second = store.ingest(input, "orchestrator", "two").expect("second");

        assert_eq!(first.id, second.id);
        assert!(second.deduplicated);

        let book_path = dir.path().join(first.book_ref);
        let lines = fs::read_to_string(book_path).expect("book");
        assert_eq!(lines.lines().count(), 1);
    }

    #[test]
    fn query_returns_relevant_match() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        store
            .ingest(
                "https://github.com/example/rust-api",
                "orchestrator",
                "rust api docs",
            )
            .expect("ingest");

        let response = store.query("rust", 5).expect("query");
        assert!(response.total_matches >= 1);
        assert_eq!(response.suggestion, None);
    }

    #[test]
    fn book_ref_is_consistent_across_ingest_and_query() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://github.com/example/rust-api",
                "orchestrator",
                "book ref contract",
            )
            .expect("ingest");

        let response = store.query("rust", 5).expect("query");
        let matched = response
            .matches
            .iter()
            .find(|entry| entry.source_id == record.id)
            .expect("match for ingested source");

        assert_eq!(record.book_ref, matched.book_ref);
        assert!(record
            .book_ref
            .ends_with(&format!("{}/{}.jsonl", "books", record.id)));
    }

    #[test]
    fn status_exposes_knowledge_vault_scaffold() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        let status = store.status().expect("status");

        assert_eq!(
            status.knowledge_vault.doctrine_path,
            "docs/ARDA_AUTONOMY_DOCTRINE.md"
        );
        assert_eq!(
            status.knowledge_vault.authority,
            "athena_knowledge_sovereign"
        );
        assert!(status.knowledge_vault.local_first_recall);
        assert!(status
            .knowledge_vault
            .layers
            .contains(&"source_acquisition".to_string()));
        assert!(status
            .knowledge_vault
            .source_lanes
            .contains(&"github".to_string()));
        assert!(status
            .knowledge_vault
            .source_lanes
            .contains(&"papers".to_string()));
        assert!(status
            .knowledge_vault
            .autonomy_feed
            .contains(&"safe_local_task_candidates".to_string()));
    }

    #[test]
    fn status_exposes_knowledge_vault_source_lane_observations() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        store
            .ingest(
                "https://docs.rs/tokio/latest/tokio/",
                "orchestrator",
                "vault lane observation docs",
            )
            .expect("documentation ingest");
        store
            .ingest(
                "https://arxiv.org/abs/2401.00001",
                "orchestrator",
                "vault lane observation papers",
            )
            .expect("paper ingest");

        let status = store.status().expect("status");

        assert_eq!(status.knowledge_vault.source_lane_observations_total, 2);
        let docs_lane = status
            .knowledge_vault
            .source_lane_observations
            .iter()
            .find(|lane| lane.lane == "documentation")
            .expect("documentation lane observation");
        assert_eq!(docs_lane.ingested_sources_total, 1);
        assert_eq!(docs_lane.policy_ready_sources_total, 0);
        assert!(docs_lane.latest_observed_at_utc.is_some());

        let papers_lane = status
            .knowledge_vault
            .source_lane_observations
            .iter()
            .find(|lane| lane.lane == "papers")
            .expect("papers lane observation");
        assert_eq!(papers_lane.ingested_sources_total, 1);
        assert_eq!(papers_lane.policy_ready_sources_total, 0);
    }

    #[test]
    fn status_exposes_knowledge_vault_autonomy_recommendations() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        store
            .ingest(
                "https://docs.rs/tokio/latest/tokio/",
                "orchestrator",
                "vault recommendation docs",
            )
            .expect("documentation ingest");
        store
            .ingest(
                "https://arxiv.org/abs/2401.00001",
                "orchestrator",
                "vault recommendation papers",
            )
            .expect("paper ingest");

        let status = store.status().expect("status");

        assert_eq!(status.knowledge_vault.autonomy_recommendations_total, 2);
        let recommendation = status
            .knowledge_vault
            .autonomy_recommendations
            .iter()
            .find(|packet| packet.lane == "documentation")
            .expect("documentation recommendation packet");
        assert_eq!(
            recommendation.recommendation_id,
            "athena.vault.documentation.safe_local_ingest_review"
        );
        assert_eq!(recommendation.action, "review_ingested_lane_for_synthesis");
        assert!(recommendation.safe_local);
        assert!(!recommendation.human_gate_required);
        assert_eq!(recommendation.evidence_count, 1);
    }

    #[test]
    fn status_exposes_ranked_knowledge_vault_synthesis_queue() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        store
            .ingest(
                "https://docs.rs/tokio/latest/tokio/",
                "orchestrator",
                "vault synthesis docs one",
            )
            .expect("documentation ingest one");
        store
            .ingest(
                "https://docs.rs/serde/latest/serde/",
                "orchestrator",
                "vault synthesis docs two",
            )
            .expect("documentation ingest two");
        store
            .ingest(
                "https://arxiv.org/abs/2401.00001",
                "orchestrator",
                "vault synthesis papers",
            )
            .expect("paper ingest");

        let status = store.status().expect("status");

        assert_eq!(status.knowledge_vault.synthesis_queue_total, 2);
        let first = status
            .knowledge_vault
            .synthesis_queue
            .first()
            .expect("ranked synthesis queue item");
        assert_eq!(first.rank, 1);
        assert_eq!(first.lane, "documentation");
        assert_eq!(
            first.synthesis_id,
            "athena.vault.documentation.synthesis.rank_1"
        );
        assert_eq!(first.recommended_action, "synthesize_lane_digest");
        assert_eq!(first.evidence_count, 2);
        assert!(first.priority_score > 0.0);
        assert!(first.safe_local);
        assert!(!first.human_gate_required);
        assert_eq!(first.risk, "low");
    }

    #[test]
    fn status_reports_ingest_and_policy_observability() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");

        let first = store
            .ingest(
                "governance routing memory safety context tooling",
                "orchestrator",
                "observability",
            )
            .expect("first ingest");
        let _duplicate = store
            .ingest(
                "governance routing memory safety context tooling",
                "orchestrator",
                "observability duplicate",
            )
            .expect("duplicate ingest");
        let queued = store
            .queue_deep_analysis(&first.id, "orchestrator", "observability deep")
            .expect("queue");
        assert_eq!(queued.status, "pending_deep");
        let _deep = store.deep_analyze(&first.id).expect("deep");
        let _harvest = store
            .harvest_opposition_evidence(&first.id, None, "test")
            .expect("harvest opposition");
        let _processed = store.process_deep_queue(8, false).expect("deep process");
        let _deep_promoted = store.deep_analyze(&first.id).expect("deep promoted");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&store.policy_readiness_path)
            .expect("open policy readiness")
            .write_all(b"not-json\n")
            .expect("append malformed policy readiness");

        let status = store.status().expect("status");
        assert_eq!(status.ingest_success_total, 4);
        assert_eq!(status.deduplicated_ingests_total, 1);
        assert!(status.duplicate_hit_rate > 0.0);
        assert!(status.avg_deep_queue_latency_seconds >= 0.0);
        assert!(status.policy_ready_promotions_total >= 1);
        assert_eq!(status.policy_ready_regressions_total, 0);
        assert_eq!(status.policy_readiness_malformed_records, 1);
    }

    #[test]
    fn parses_arxiv_api_metadata() {
        let xml = r#"
        <feed xmlns:arxiv="http://arxiv.org/schemas/atom">
          <entry>
            <title>Terminal-Bench: A Benchmark for Interactive Coding Agents</title>
            <summary>We study terminal coding agents with context, memory, routing, and safety concerns.</summary>
            <author><name>Alice Example</name></author>
            <author><name>Bob Example</name></author>
            <arxiv:comment>12 pages</arxiv:comment>
            <arxiv:doi>10.1000/example</arxiv:doi>
            <category term="cs.SE"/>
            <category term="cs.AI"/>
          </entry>
        </feed>
        "#;
        let parsed = parse_arxiv_api_response(xml, "https://arxiv.org/abs/2603.05344")
            .expect("parsed arxiv metadata");
        assert_eq!(
            parsed.paper_title,
            "Terminal-Bench: A Benchmark for Interactive Coding Agents"
        );
        assert_eq!(parsed.authors.len(), 2);
        assert!(parsed
            .abstract_text
            .contains("context, memory, routing, and safety"));
        assert!(parsed.subjects.contains(&"cs.SE".to_string()));
        assert_eq!(parsed.comments.as_deref(), Some("12 pages"));
        assert_eq!(parsed.doi.as_deref(), Some("10.1000/example"));
    }

    #[test]
    fn offline_arxiv_metadata_fixture_supports_recovery() {
        let parsed = offline_scholarly_metadata("2603.05344", "https://arxiv.org/abs/2603.05344")
            .expect("offline scholarly metadata");
        assert_eq!(
            parsed.paper_title,
            "Terminal-Bench: A Benchmark for Interactive Coding Agents"
        );
        assert!(parsed.abstract_text.contains("context management"));
        assert!(parsed.subjects.contains(&"cs.SE".to_string()));
    }

    #[test]
    fn deep_analysis_recovers_missing_scholarly_metadata_from_ingest_url() {
        let _guard = env_guard();
        std::env::set_var("ARDA_VARDA_FORCE_OFFLINE_SCHOLARLY_METADATA", "true");
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://arxiv.org/abs/2603.05344",
                "orchestrator",
                "scholarly recovery",
            )
            .expect("ingest");
        let book_path = dir.path().join(format!("books/{}.jsonl", record.id));
        let original = fs::read_to_string(&book_path).expect("book");
        let shallow = original.lines().next().expect("shallow line");
        let mut shallow_value: serde_json::Value =
            serde_json::from_str(shallow).expect("shallow json");
        shallow_value["data"]["title"] = serde_json::json!("https://arxiv.org/abs/2603.05344");
        shallow_value["data"]["summary"] =
            serde_json::json!("Initial shallow ingest completed for ScholarlyLink.");
        let obj = shallow_value["data"]
            .as_object_mut()
            .expect("shallow data object");
        obj.remove("scholarly_metadata");
        obj.insert(
            "relevance_tags".to_string(),
            serde_json::json!(["scholarlylink"]),
        );
        fs::write(
            &book_path,
            format!(
                "{}\n",
                serde_json::to_string(&shallow_value).expect("serialize")
            ),
        )
        .expect("rewrite shallow-only book");

        let deep = store.deep_analyze(&record.id).expect("deep");
        assert_eq!(
            deep.data.title,
            "Terminal-Bench: A Benchmark for Interactive Coding Agents"
        );
        assert!(deep.data.full_summary.contains("context management"));
        assert!(deep.data.implementation_brief.is_some());
        std::env::remove_var("ARDA_VARDA_FORCE_OFFLINE_SCHOLARLY_METADATA");
    }

    #[test]
    fn generate_planning_tasks_from_evidence() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_PROJECT_TASK_QUEUE_PATH", &queue_path);
        }
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "context memory routing safety tooling terminal agent harness",
                "orchestrator",
                "planning",
            )
            .expect("ingest");
        let out = store
            .generate_planning_tasks(&record.id, 8)
            .expect("generate tasks");
        assert!(out["queued_tasks"].as_u64().unwrap_or_default() >= 4);
    }

    #[test]
    fn github_repo_source_emits_implementation_brief() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://github.com/unclecode/crawl4ai",
                "orchestrator",
                "planning",
            )
            .expect("ingest");
        store
            .harvest_opposition_evidence(&record.id, Some("crawl runtime"), "orchestrator")
            .expect("opposition");
        let _ = store.deep_analyze(&record.id).expect("deep");

        let book_path = dir
            .path()
            .join("books")
            .join(format!("{}.jsonl", record.id));
        let content = fs::read_to_string(book_path).expect("book");
        let mut implementation_brief = None;
        for line in content.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("json");
            if value.get("stage").and_then(|v| v.as_str()) == Some("deep") {
                implementation_brief = value
                    .get("data")
                    .and_then(|v| v.get("implementation_brief"))
                    .cloned();
            }
        }
        assert!(implementation_brief.is_some());
    }

    #[test]
    fn github_repo_implementation_brief_generates_execution_tasks() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let queue_path = dir.path().join("queue.jsonl");
        // SAFETY: warden-owned by `arda-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_PROJECT_TASK_QUEUE_PATH", &queue_path);
        }
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://github.com/D4Vinci/Scrapling",
                "orchestrator",
                "planning",
            )
            .expect("ingest");
        store
            .harvest_opposition_evidence(&record.id, Some("crawl runtime"), "orchestrator")
            .expect("opposition");
        let _ = store.deep_analyze(&record.id).expect("deep");

        let out = store
            .generate_planning_tasks(&record.id, 8)
            .expect("generate tasks");
        assert!(out["queued_tasks"].as_u64().unwrap_or_default() >= 2);
    }

    #[test]
    fn deep_analysis_appends_deep_entry_and_logs_queue() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        }
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://example.com/governance-report",
                "orchestrator",
                "analysis",
            )
            .expect("ingest");

        let _queue = store
            .queue_deep_analysis(&record.id, "orchestrator", "manual trigger")
            .expect("queue");
        let deep = store.deep_analyze(&record.id).expect("deep");

        assert_eq!(deep.stage, "deep");
        assert_eq!(deep.sigil, "EYE");
        assert!(deep.version >= 2);
        assert_eq!(
            deep.data
                .inference_route
                .get("mode")
                .and_then(|v| v.as_str()),
            Some("unconfigured")
        );

        let book_path = dir.path().join(format!("books/{}.jsonl", record.id));
        let book = fs::read_to_string(book_path).expect("book");
        assert!(book.contains("\"stage\":\"deep\""));

        let queue_log = fs::read_to_string(store.deep_queue_path()).expect("queue");
        assert!(queue_log.contains("deep_queued"));
        assert_ne!(deep.data.policy_readiness, "policy_ready");
        std::thread::sleep(Duration::from_millis(150));
        let plutus_status = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                PlutusService::from_home(&plutus_home)
                    .expect("plutus service")
                    .status()
                    .await
                    .expect("plutus status")
            });
        assert_eq!(
            plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default(),
            0
        );
        assert_eq!(
            plutus_status["joulework"]["total"]
                .as_f64()
                .unwrap_or_default(),
            0.0
        );
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_PLUTUS_HOME");
        }
    }

    #[test]
    fn deep_process_runs_pending_queue_items() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://example.com/rust-governance",
                "orchestrator",
                "analysis",
            )
            .expect("ingest");
        store
            .queue_deep_analysis(&record.id, "orchestrator", "queued")
            .expect("queued");

        let out = store.process_deep_queue(10, false).expect("process");
        assert_eq!(out.get("processed").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(out.get("success").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn deduplicated_ingest_preserves_deep_human_source_view() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://example.com/scrapling-like-source",
                "orchestrator",
                "analysis",
            )
            .expect("ingest");
        store
            .queue_deep_analysis(&record.id, "orchestrator", "queued")
            .expect("queued");
        let _ = store.process_deep_queue(10, false).expect("process");

        let _dedup = store
            .ingest(
                "https://example.com/scrapling-like-source",
                "orchestrator",
                "dedup verification",
            )
            .expect("dedup ingest");

        let human_path = store.human_sources_dir.join(format!("{}.md", record.id));
        let human = fs::read_to_string(human_path).expect("human source view");
        assert!(human.contains("- status: `deep`"));
        assert!(human.contains("## Deep Analysis"));
    }

    #[test]
    fn deep_process_ignores_malformed_queue_records() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "https://example.com/rust-governance",
                "orchestrator",
                "analysis",
            )
            .expect("ingest");
        fs::write(
            store.deep_queue_path(),
            "not-json\n{\"source_id\":\"missing_status\"}\n",
        )
        .expect("seed malformed queue");
        store
            .queue_deep_analysis(&record.id, "orchestrator", "queued")
            .expect("queued");

        let out = store.process_deep_queue(10, false).expect("process");
        assert_eq!(out.get("processed").and_then(|v| v.as_u64()), Some(2));
        assert_eq!(out.get("success").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(out.get("failed").and_then(|v| v.as_u64()), Some(1));
    }

    #[test]
    fn lifecycle_events_write_hades_and_warden_queues() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let hades_queue = dir.path().join("hades_queue.jsonl");
        let warden_queue = dir.path().join("warden_queue.jsonl");
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_HADES_ACTION_QUEUE_PATH", &hades_queue);
            std::env::set_var("ARDA_WARDEN_QUEUE_PATH", &warden_queue);
        }

        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest("https://example.com/research", "orchestrator", "analysis")
            .expect("ingest");
        store
            .queue_deep_analysis(&record.id, "orchestrator", "queued")
            .expect("queued");
        let _ = store.process_deep_queue(10, false).expect("process");

        let hades = fs::read_to_string(hades_queue).expect("hades queue");
        let warden = fs::read_to_string(warden_queue).expect("warden queue");
        assert!(hades.contains("athena lifecycle event"));
        assert!(warden.contains("athena_deep_"));
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_HADES_ACTION_QUEUE_PATH");
            std::env::remove_var("ARDA_WARDEN_QUEUE_PATH");
        }
    }

    #[test]
    fn deep_queue_backlog_threshold_emits_warning() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let hades_queue = dir.path().join("hades_queue.jsonl");
        let warden_queue = dir.path().join("warden_queue.jsonl");
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_HADES_ACTION_QUEUE_PATH", &hades_queue);
            std::env::set_var("ARDA_WARDEN_QUEUE_PATH", &warden_queue);
        }

        let store = AthenaStore::new(dir.path()).expect("store");
        for idx in 0..101 {
            let source_id = format!("src_backlog_{idx:03}");
            store
                .queue_deep_analysis(&source_id, "orchestrator", "backlog seed")
                .expect("queue");
        }

        let digest = fs::read_to_string(store.digest_path()).expect("digest");
        let warden = fs::read_to_string(warden_queue).expect("warden queue");
        assert!(digest.contains("deep_queue_backlog_warning"));
        assert!(warden.contains("athena_deep_backlog_warning"));
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_HADES_ACTION_QUEUE_PATH");
            std::env::remove_var("ARDA_WARDEN_QUEUE_PATH");
        }
    }

    #[test]
    fn policy_promote_holds_plutus_signal_without_policy_or_task_receipts() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        }
        let store = AthenaStore::new(dir.path()).expect("store");

        let out = store.promote_policy_readiness(4, false).expect("promote");
        assert_eq!(
            out.get("queued_tasks").and_then(|value| value.as_u64()),
            Some(0)
        );
        assert_eq!(
            out.get("policy_ready_recent")
                .and_then(|value| value.as_u64()),
            Some(0)
        );
        assert_eq!(
            out.get("promotion_receipt_available")
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        std::thread::sleep(Duration::from_millis(150));
        let plutus_status = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                PlutusService::from_home(&plutus_home)
                    .expect("plutus service")
                    .status()
                    .await
                    .expect("plutus status")
            });
        assert_eq!(
            plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default(),
            0
        );
        assert_eq!(
            plutus_status["governance"]["records_total"]
                .as_u64()
                .unwrap_or_default(),
            0
        );
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_PLUTUS_HOME");
        }
    }

    #[test]
    fn policy_promote_emits_plutus_relationship_signal_when_receipt_available() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        }
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest(
                "context memory routing safety tooling terminal agent harness",
                "orchestrator",
                "policy",
            )
            .expect("ingest");
        let _ = store.deep_analyze(&record.id).expect("deep");
        let task_emission = store
            .generate_planning_tasks(&record.id, 8)
            .expect("task emission");
        assert!(task_emission["receipts_total"].as_u64().unwrap_or_default() > 0);

        let out = store.promote_policy_readiness(4, true).expect("promote");
        assert_eq!(
            out.get("promotion_receipt_available")
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(
            out.get("policy_ready_recent")
                .and_then(|value| value.as_u64())
                .unwrap_or_default()
                > 0
                || out
                    .get("task_emission_receipts_total")
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default()
                    > 0
        );
        std::thread::sleep(Duration::from_millis(150));
        let plutus_status = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                PlutusService::from_home(&plutus_home)
                    .expect("plutus service")
                    .status()
                    .await
                    .expect("plutus status")
            });
        assert!(
            plutus_status["love_equation"]["relationships_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );
        assert!(
            plutus_status["governance"]["records_total"]
                .as_u64()
                .unwrap_or_default()
                >= 1
        );
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_PLUTUS_HOME");
        }
    }

    #[test]
    fn read_digest_ignores_malformed_records() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        fs::write(
            store.digest_path(),
            "not-json\n{\"source_id\":\"src_ok\",\"status\":\"pending_deep\"}\n",
        )
        .expect("seed digest");

        let out = store.read_digest(None, 10).expect("digest");
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].get("source_id").and_then(|v| v.as_str()),
            Some("src_ok")
        );
    }

    #[test]
    fn query_ignores_malformed_book_records() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest("https://example.com/governance", "orchestrator", "analysis")
            .expect("ingest");
        let book_path = store.books_dir.join(format!("{}.jsonl", record.id));
        let seeded = fs::read_to_string(&book_path).expect("read seeded book");
        fs::write(&book_path, format!("not-json\n{seeded}")).expect("corrupt seeded book");

        let out = store.query("governance", 10).expect("query");
        assert!(!out.matches.is_empty());
        assert_eq!(out.matches[0].source_id, record.id);
    }

    #[test]
    fn append_jsonl_serializes_concurrent_writers() {
        use std::sync::{Arc, Barrier};

        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let path = dir.path().join("concurrent.jsonl");
        let barrier = Arc::new(Barrier::new(9));
        let store = Arc::new(store);
        let mut handles = Vec::new();

        for thread_id in 0..8 {
            let barrier = Arc::clone(&barrier);
            let store = Arc::clone(&store);
            let path = path.clone();
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                for item_id in 0..50 {
                    store
                        .append_jsonl(
                            &path,
                            &serde_json::json!({
                                "thread_id": thread_id,
                                "item_id": item_id
                            }),
                        )
                        .expect("append");
                }
            }));
        }

        barrier.wait();
        for handle in handles {
            handle.join().expect("join");
        }

        let content = fs::read_to_string(&path).expect("read concurrent jsonl");
        let lines = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 400);
        for line in lines {
            let value = serde_json::from_str::<serde_json::Value>(line).expect("valid json");
            assert!(value.get("thread_id").is_some());
            assert!(value.get("item_id").is_some());
        }
    }

    #[test]
    fn record_crawl_capture_writes_artifact_and_receipt() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let crawl = CrawlMarkdownResult {
            url: "https://example.com".to_string(),
            filter: "fit".to_string(),
            query: Some("governance".to_string()),
            markdown: "# Example\n\nCrawled markdown.".to_string(),
            success: true,
            provider: "crawl4ai".to_string(),
        };

        let receipt = store
            .record_crawl_capture(
                "https://example.com",
                "cli",
                "crawl test",
                &format!("http://{}:{}", "127.0.0.1", 11235),
                &crawl,
            )
            .expect("crawl receipt");

        assert_eq!(
            receipt.source_id,
            source_id_from_input("https://example.com")
        );
        assert!(Path::new(&receipt.artifact_path).exists());
        let artifact = fs::read_to_string(&receipt.artifact_path).expect("artifact");
        assert!(artifact.contains("Crawled markdown"));
        let ledger = fs::read_to_string(store.crawl_receipts_path()).expect("crawl ledger");
        let crawl_service_url = format!("http://{}:{}", "127.0.0.1", 11235);
        assert!(ledger.contains(&format!("\"crawl_service_url\":\"{crawl_service_url}\"")));
    }

    #[test]
    fn crawl4ai_fetch_markdown_parses_response_body() {
        let listener = match TcpListener::bind("127.0.0.1:0") {
            Ok(listener) => listener,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(err) => panic!("bind: {err}"),
        };
        let addr = listener.local_addr().expect("addr");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            let mut request = [0u8; 4096];
            let _ = stream.read(&mut request).expect("read request");
            let body =
                r##"{"url":"https://example.com","markdown":"# Example\nbody\n","success":true}"##;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });

        let out = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                crawl4ai_fetch_markdown(
                    &format!("http://{}", addr),
                    "https://example.com",
                    "fit",
                    Some("governance"),
                )
                .await
            })
            .expect("crawl fetch");
        server.join().expect("join server");

        assert_eq!(out.url, "https://example.com");
        assert_eq!(out.filter, "fit");
        assert_eq!(out.query.as_deref(), Some("governance"));
        assert!(out.markdown.contains("# Example"));
        assert!(out.success);
    }

    #[test]
    fn scrapling_fetch_markdown_supports_raw_html_shim() {
        let _guard = env_guard();
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_SCRAPLING_RUNTIME_MODE");
        }
        let out = scrapling_fetch_markdown(
            "raw:<html><body><main><h1>ARDA Scrapling</h1><p>Shim body.</p></main></body></html>",
            "fit",
            Some("knowledge"),
        )
        .expect("scrapling fetch");

        assert_eq!(
            out.provider,
            std::env::var("ARDA_SCRAPLING_EXPECTED_PROVIDER")
                .unwrap_or_else(|_| "scrapling_shim".to_string())
        );
        assert!(out.markdown.contains("Arda Scrapling"));
        assert!(out.markdown.contains("Shim body."));
        assert!(out.success);
    }

    #[test]
    fn scrapling_fetch_markdown_fails_when_native_required_without_package() {
        let _guard = env_guard();
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::set_var("ARDA_SCRAPLING_RUNTIME_MODE", "native_required");
        }
        let err = scrapling_fetch_markdown(
            "raw:<html><body><main><h1>ARDA Scrapling</h1></main></body></html>",
            "fit",
            None,
        )
        .expect_err("native-required failure");
        assert!(err
            .to_string()
            .contains("native Scrapling runtime required"));
        // SAFETY: warden-owned by `ARDA-athena` test scaffolding — single-threaded
        // test process with no concurrent env reader at this point.
        unsafe {
            std::env::remove_var("ARDA_SCRAPLING_RUNTIME_MODE");
        }
    }

    #[test]
    fn resolve_crawl_provider_order_prefers_profile_defaults() {
        assert_eq!(
            resolve_crawl_provider_order(None, Some("production")),
            vec!["crawl4ai".to_string(), "scrapling".to_string()]
        );
        assert_eq!(
            resolve_crawl_provider_order(None, Some("research")),
            vec!["scrapling".to_string(), "crawl4ai".to_string()]
        );
        assert_eq!(
            resolve_crawl_provider_order(Some("scrapling,crawl4ai,scrapling"), Some("production")),
            vec!["scrapling".to_string(), "crawl4ai".to_string()]
        );
    }

    #[tokio::test]
    async fn deep_analyze_sheds_excess_burst_work() {
        let _guard = env_guard();
        let dir = tempdir().expect("tempdir");
        std::env::set_var("ARDA_VARDA_DEEP_QUEUE_MAX_CONCURRENCY", "1");
        let store = AthenaStore::new(dir.path()).expect("store");
        let record = store
            .ingest("https://example.com/burst-deep", "test", "burst")
            .expect("ingest");
        let (ready_tx, ready_rx) = oneshot::channel::<()>();
        let (tx, rx) = oneshot::channel::<()>();

        let holder = tokio::spawn(async move {
            let _ = try_run_bounded_async("athena_deep_analyze", 1, || async move {
                let _ = ready_tx.send(());
                let _ = rx.await;
            })
            .await;
        });
        ready_rx.await.expect("holder ready");

        let err = store.deep_analyze(&record.id).expect_err("saturated");
        assert!(err
            .to_string()
            .contains("deep analysis concurrency gate saturated"));

        let _ = tx.send(());
        holder.await.expect("holder");
        std::env::remove_var("ARDA_VARDA_DEEP_QUEUE_MAX_CONCURRENCY");
    }
}
