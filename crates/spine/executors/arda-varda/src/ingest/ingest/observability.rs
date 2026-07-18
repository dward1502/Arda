// sigil: REPAIR
//
// Observability surface: AthenaStore::status assembles the full snapshot
// (corpus counts, deep-queue health, ingest metrics, policy breakdown,
// planning-task emission receipts). Also hosts the small `recent_*` readers
// and their JSONL aggregation helpers.

use arda_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::Path;

use super::{
    AthenaAutonomyRecommendation, AthenaKnowledgeVaultSourceLaneObservation,
    AthenaKnowledgeVaultStatus, AthenaStatus, AthenaStore, AthenaVaultSynthesisQueueItem,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct PlanningTaskReceiptSummary {
    pub(super) total: usize,
    pub(super) queued: usize,
    pub(super) skipped: usize,
    pub(super) last_run_at_utc: Option<String>,
}

pub(super) struct PolicyReadinessBreakdown {
    pub(super) policy_ready_count: usize,
    pub(super) reference_only_count: usize,
    pub(super) malformed_records: usize,
    pub(super) primary_policy_ready_count: usize,
    pub(super) primary_reference_only_count: usize,
    pub(super) synthetic_policy_ready_count: usize,
    pub(super) synthetic_reference_only_count: usize,
}

pub(super) struct IngestObservabilitySummary {
    pub(super) ingest_success_total: usize,
    pub(super) deduplicated_ingests_total: usize,
    pub(super) duplicate_hit_rate: f64,
}

impl AthenaStore {
    pub fn status(&self) -> Result<AthenaStatus> {
        let books_count = fs::read_dir(&self.books_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|v| v.to_str()) == Some("jsonl"))
            .count();

        let digest_events = line_count(&self.digest_path)?;
        let (deep_queue_depth, deep_queue_failed) =
            deep_queue_status_counts(&self.deep_queue_path)?;
        // Update the deep queue depth gauge
        self.metrics.set_deep_queue_depth(deep_queue_depth as u64);
        let deep_graph_events = line_count(&self.deep_graph_path)?;
        let ingest_metrics = ingest_observability_summary(&self.digest_path)?;
        let avg_deep_queue_latency_seconds =
            average_deep_queue_latency_seconds(&self.deep_queue_path)?;
        let policy_breakdown =
            policy_readiness_summary(&self.policy_readiness_path, &self.digest_path)?;
        self.metrics
            .set_policy_readiness_malformed_records(policy_breakdown.malformed_records as u64);
        let (policy_ready_promotions_total, policy_ready_regressions_total) =
            policy_readiness_delta_summary(&self.policy_readiness_path)?;
        let source_provenance_coverage_ratio = self.source_provenance_coverage_ratio()?;
        let task_receipts = planning_task_receipt_summary(&self.planning_task_receipts_path)?;
        let knowledge_vault = self.knowledge_vault_status()?;

        Ok(AthenaStatus {
            storage_root: self.root.display().to_string(),
            digest_path: self.digest_path.display().to_string(),
            deep_queue_path: self.deep_queue_path.display().to_string(),
            deep_graph_path: self.deep_graph_path.display().to_string(),
            policy_readiness_path: self.policy_readiness_path.display().to_string(),
            planning_task_receipts_path: self.planning_task_receipts_path.display().to_string(),
            books_count,
            digest_events,
            deep_queue_depth,
            deep_queue_failed,
            deep_graph_events,
            ingest_success_total: ingest_metrics.ingest_success_total,
            deduplicated_ingests_total: ingest_metrics.deduplicated_ingests_total,
            duplicate_hit_rate: ingest_metrics.duplicate_hit_rate,
            avg_deep_queue_latency_seconds,
            policy_ready_count: policy_breakdown.policy_ready_count,
            reference_only_count: policy_breakdown.reference_only_count,
            policy_ready_promotions_total,
            policy_ready_regressions_total,
            policy_readiness_malformed_records: policy_breakdown.malformed_records,
            primary_policy_ready_count: policy_breakdown.primary_policy_ready_count,
            primary_reference_only_count: policy_breakdown.primary_reference_only_count,
            synthetic_policy_ready_count: policy_breakdown.synthetic_policy_ready_count,
            synthetic_reference_only_count: policy_breakdown.synthetic_reference_only_count,
            execution_authority: "workstation".to_string(),
            execution_posture: "workstation_first_laptop_operator_fallback".to_string(),
            operator_ingress_role: "laptop_terminal_optional_fallback".to_string(),
            source_provenance_coverage_ratio,
            memory_lanes: vec![
                "episodic".to_string(),
                "source_book".to_string(),
                "policy_ready".to_string(),
                "implementation_ready".to_string(),
            ],
            task_emission_receipts_total: task_receipts.total,
            task_emission_success_total: task_receipts.queued,
            task_emission_skipped_total: task_receipts.skipped,
            task_emission_last_run_at_utc: task_receipts.last_run_at_utc,
            knowledge_vault,
        })
    }

    pub fn knowledge_vault_status(&self) -> Result<AthenaKnowledgeVaultStatus> {
        let source_lane_observations = knowledge_vault_source_lane_observations(
            &self.digest_path,
            &self.policy_readiness_path,
        )?;
        let autonomy_recommendations =
            knowledge_vault_autonomy_recommendations(&source_lane_observations);
        let synthesis_queue = knowledge_vault_synthesis_queue(&autonomy_recommendations);
        Ok(AthenaKnowledgeVaultStatus {
            doctrine_path: "docs/ARDA_AUTONOMY_DOCTRINE.md".to_string(),
            authority: "athena_knowledge_sovereign".to_string(),
            local_first_recall: true,
            layers: vec![
                "source_acquisition".to_string(),
                "evidence_vault".to_string(),
                "knowledge_extraction".to_string(),
                "memory_index".to_string(),
                "synthesis".to_string(),
                "autonomy_feed".to_string(),
            ],
            source_lanes: vec![
                "github".to_string(),
                "documentation".to_string(),
                "papers".to_string(),
                "blogs_rss".to_string(),
                "forums".to_string(),
                "x_reddit".to_string(),
                "youtube_transcripts".to_string(),
                "pdfs_books".to_string(),
                "local_notes".to_string(),
                "chat_exports".to_string(),
                "codebases".to_string(),
                "runtime_logs".to_string(),
            ],
            autonomy_feed: vec![
                "skill_updates".to_string(),
                "docs".to_string(),
                "plans".to_string(),
                "task_proposals".to_string(),
                "safe_local_task_candidates".to_string(),
                "high_risk_escalation_packets".to_string(),
            ],
            source_lane_observations_total: source_lane_observations.len(),
            source_lane_observations,
            autonomy_recommendations_total: autonomy_recommendations.len(),
            autonomy_recommendations,
            synthesis_queue_total: synthesis_queue.len(),
            synthesis_queue,
        })
    }

    pub fn recent_deep_queue_events(&self, limit: usize) -> Result<Vec<Value>> {
        read_recent_jsonl(&self.deep_queue_path, limit)
    }

    pub fn recent_deep_graph_events(&self, limit: usize) -> Result<Vec<Value>> {
        read_recent_jsonl(&self.deep_graph_path, limit)
    }
}

pub(super) fn line_count(path: &Path) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    Ok(content.lines().count())
}

pub(super) fn read_recent_jsonl(path: &Path, limit: usize) -> Result<Vec<Value>> {
    let content = fs::read_to_string(path)?;
    let mut items = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(line) {
            items.push(value);
        }
    }
    let limit = limit.max(1);
    if items.len() > limit {
        let start = items.len().saturating_sub(limit);
        Ok(items.split_off(start))
    } else {
        Ok(items)
    }
}

pub(super) fn deep_queue_status_counts(path: &Path) -> Result<(usize, usize)> {
    let content = fs::read_to_string(path)?;
    let mut latest = std::collections::HashMap::<String, String>::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
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
    let pending = latest
        .values()
        .filter(|status| status.as_str() == "pending_deep")
        .count();
    let failed = latest
        .values()
        .filter(|status| status.as_str() == "failed")
        .count();
    Ok((pending, failed))
}

pub(super) fn policy_readiness_summary(
    path: &Path,
    digest_path: &Path,
) -> Result<PolicyReadinessBreakdown> {
    let content = fs::read_to_string(path)?;
    let mut latest = std::collections::HashMap::<String, String>::new();
    let mut malformed_records = 0usize;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => {
                malformed_records += 1;
                continue;
            }
        };
        let Some(source_id) = value.get("source_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let status = value
            .get("policy_readiness")
            .and_then(|v| v.as_str())
            .unwrap_or("reference_only");
        latest.insert(source_id.to_string(), status.to_string());
    }

    let digest = fs::read_to_string(digest_path)?;
    let mut task_context_by_source = std::collections::HashMap::<String, String>::new();
    for line in digest.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(source_id) = value.get("id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(task_context) = value.get("task_context").and_then(|v| v.as_str()) else {
            continue;
        };
        task_context_by_source.insert(source_id.to_string(), task_context.to_string());
    }

    let mut primary_policy_ready_count = 0usize;
    let mut primary_reference_only_count = 0usize;
    let mut synthetic_policy_ready_count = 0usize;
    let mut synthetic_reference_only_count = 0usize;
    for (source_id, status) in latest {
        let synthetic = task_context_by_source
            .get(&source_id)
            .map(|ctx| ctx.starts_with("opposition_for:"))
            .unwrap_or(false);
        match (synthetic, status.as_str()) {
            (true, "policy_ready") => synthetic_policy_ready_count += 1,
            (true, _) => synthetic_reference_only_count += 1,
            (false, "policy_ready") => primary_policy_ready_count += 1,
            (false, _) => primary_reference_only_count += 1,
        }
    }

    Ok(PolicyReadinessBreakdown {
        policy_ready_count: primary_policy_ready_count + synthetic_policy_ready_count,
        reference_only_count: primary_reference_only_count + synthetic_reference_only_count,
        malformed_records,
        primary_policy_ready_count,
        primary_reference_only_count,
        synthetic_policy_ready_count,
        synthetic_reference_only_count,
    })
}

pub(super) fn knowledge_vault_source_lane_observations(
    digest_path: &Path,
    policy_readiness_path: &Path,
) -> Result<Vec<AthenaKnowledgeVaultSourceLaneObservation>> {
    let mut policy_ready_by_source = std::collections::HashMap::<String, bool>::new();
    let policy_content = fs::read_to_string(policy_readiness_path)?;
    for line in policy_content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(source_id) = value.get("source_id").and_then(Value::as_str) else {
            continue;
        };
        let readiness = value
            .get("policy_readiness")
            .and_then(Value::as_str)
            .unwrap_or("reference_only");
        policy_ready_by_source.insert(source_id.to_string(), readiness == "policy_ready");
    }

    let digest_content = fs::read_to_string(digest_path)?;
    let mut observations =
        std::collections::BTreeMap::<String, AthenaKnowledgeVaultSourceLaneObservation>::new();
    for line in digest_content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if value.get("raw_input").is_none() {
            continue;
        }
        let Some(source_id) = value.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(source_type) = value.get("source_type").and_then(Value::as_str) else {
            continue;
        };
        let Some(lane) = knowledge_vault_lane_for_source_type(source_type) else {
            continue;
        };
        let observed_at = value
            .get("processed_at_utc")
            .or_else(|| value.get("received_at_utc"))
            .and_then(Value::as_str)
            .map(str::to_string);
        let entry = observations.entry(lane.to_string()).or_insert_with(|| {
            AthenaKnowledgeVaultSourceLaneObservation {
                lane: lane.to_string(),
                ingested_sources_total: 0,
                policy_ready_sources_total: 0,
                latest_observed_at_utc: None,
            }
        });
        entry.ingested_sources_total += 1;
        if policy_ready_by_source
            .get(source_id)
            .copied()
            .unwrap_or(false)
        {
            entry.policy_ready_sources_total += 1;
        }
        if observed_at > entry.latest_observed_at_utc {
            entry.latest_observed_at_utc = observed_at;
        }
    }

    Ok(observations.into_values().collect())
}

fn knowledge_vault_autonomy_recommendations(
    observations: &[AthenaKnowledgeVaultSourceLaneObservation],
) -> Vec<AthenaAutonomyRecommendation> {
    observations
        .iter()
        .filter(|observation| observation.ingested_sources_total > 0)
        .map(|observation| AthenaAutonomyRecommendation {
            recommendation_id: format!(
                "athena.vault.{}.safe_local_ingest_review",
                observation.lane
            ),
            lane: observation.lane.clone(),
            action: "review_ingested_lane_for_synthesis".to_string(),
            rationale: format!(
                "{} ingested source(s) are available for safe-local synthesis review",
                observation.ingested_sources_total
            ),
            safe_local: true,
            human_gate_required: false,
            evidence_count: observation.ingested_sources_total,
        })
        .collect()
}

fn knowledge_vault_synthesis_queue(
    recommendations: &[AthenaAutonomyRecommendation],
) -> Vec<AthenaVaultSynthesisQueueItem> {
    let mut items = recommendations
        .iter()
        .filter(|recommendation| recommendation.safe_local && !recommendation.human_gate_required)
        .map(|recommendation| AthenaVaultSynthesisQueueItem {
            synthesis_id: String::new(),
            rank: 0,
            lane: recommendation.lane.clone(),
            recommended_action: "synthesize_lane_digest".to_string(),
            rationale: format!(
                "{} evidence item(s) make {} ready for safe-local synthesis",
                recommendation.evidence_count, recommendation.lane
            ),
            evidence_count: recommendation.evidence_count,
            priority_score: recommendation.evidence_count as f64,
            safe_local: true,
            human_gate_required: false,
            risk: "low".to_string(),
        })
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        right
            .evidence_count
            .cmp(&left.evidence_count)
            .then_with(|| left.lane.cmp(&right.lane))
    });

    for (index, item) in items.iter_mut().enumerate() {
        item.rank = index + 1;
        item.synthesis_id = format!("athena.vault.{}.synthesis.rank_{}", item.lane, item.rank);
    }

    items
}

fn knowledge_vault_lane_for_source_type(source_type: &str) -> Option<&'static str> {
    match source_type {
        "github_repo" | "github_file" => Some("github"),
        "scholarly_link" => Some("papers"),
        "documentation" => Some("documentation"),
        "news_article" => Some("blogs_rss"),
        "government_doc" => Some("documentation"),
        "raw_note" => Some("local_notes"),
        "code_snippet" => Some("codebases"),
        "pdf_document" => Some("pdfs_books"),
        "x_post" | "x_bookmark" => Some("x_reddit"),
        "chat_export" => Some("chat_exports"),
        _ => None,
    }
}

pub(super) fn ingest_observability_summary(path: &Path) -> Result<IngestObservabilitySummary> {
    let content = fs::read_to_string(path)?;
    let mut ingest_success_total = 0usize;
    let mut deduplicated_ingests_total = 0usize;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if value.get("raw_input").is_none() {
            continue;
        }
        ingest_success_total += 1;
        if value
            .get("deduplicated")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            deduplicated_ingests_total += 1;
        }
    }
    let duplicate_hit_rate = if ingest_success_total == 0 {
        0.0
    } else {
        deduplicated_ingests_total as f64 / ingest_success_total as f64
    };
    Ok(IngestObservabilitySummary {
        ingest_success_total,
        deduplicated_ingests_total,
        duplicate_hit_rate,
    })
}

pub(super) fn average_deep_queue_latency_seconds(path: &Path) -> Result<f64> {
    let content = fs::read_to_string(path)?;
    let mut queued_at = std::collections::HashMap::<String, chrono::DateTime<Utc>>::new();
    let mut latencies = Vec::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(source_id) = value.get("source_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(ts_raw) = value.get("ts").and_then(|v| v.as_str()) else {
            continue;
        };
        let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_raw) else {
            continue;
        };
        let ts = ts.with_timezone(&Utc);
        match value
            .get("event")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
        {
            "deep_queued" => {
                queued_at.insert(source_id.to_string(), ts);
            }
            "deep_complete" => {
                if let Some(start) = queued_at.remove(source_id) {
                    let delta = (ts - start).num_milliseconds() as f64 / 1000.0;
                    if delta >= 0.0 {
                        latencies.push(delta);
                    }
                }
            }
            _ => {}
        }
    }
    if latencies.is_empty() {
        return Ok(0.0);
    }
    Ok(latencies.iter().sum::<f64>() / latencies.len() as f64)
}

pub(super) fn policy_readiness_delta_summary(path: &Path) -> Result<(usize, usize)> {
    let content = fs::read_to_string(path)?;
    let mut latest = std::collections::HashMap::<String, String>::new();
    let mut promotions = 0usize;
    let mut regressions = 0usize;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(source_id) = value.get("source_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let current = value
            .get("policy_readiness")
            .and_then(|v| v.as_str())
            .unwrap_or("reference_only")
            .to_string();
        if let Some(previous) = latest.insert(source_id.to_string(), current.clone()) {
            if previous != "policy_ready" && current == "policy_ready" {
                promotions += 1;
            } else if previous == "policy_ready" && current != "policy_ready" {
                regressions += 1;
            }
        }
    }
    Ok((promotions, regressions))
}

pub(super) fn planning_task_receipt_summary(path: &Path) -> Result<PlanningTaskReceiptSummary> {
    let mut summary = PlanningTaskReceiptSummary::default();
    if !path.exists() {
        return Ok(summary);
    }
    let content = fs::read_to_string(path)?;
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        summary.total += 1;
        match value.get("disposition").and_then(Value::as_str) {
            Some("queued") => summary.queued += 1,
            Some("skipped") => summary.skipped += 1,
            _ => {}
        }
        if let Some(ts) = value.get("ts_utc").and_then(Value::as_str) {
            summary.last_run_at_utc = Some(ts.to_string());
        }
    }
    Ok(summary)
}
