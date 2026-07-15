use super::{append_jsonl_batch, HadesService};
use crate::types::{HumanLifecycleImportReport, HumanLifecycleReviewItem};
use arda_core::error::Result;
use chrono::Utc;
use serde::Deserialize;
use std::fs;
use std::path::Path;

const HUMAN_INGESTION_CONTRACT: &str = "arda.human_ingestion_result.v1";
const HADES_REVIEW_CONTRACT: &str = "arda.hades.human_lifecycle_review.v1";

#[derive(Debug, Deserialize)]
struct HumanIngestionRecord {
    contract: String,
    source_path: String,
    content_hash: String,
    detected_status: String,
    detected_authority: String,
    source_type: String,
    #[serde(default)]
    affected_agents: Vec<String>,
    #[serde(default)]
    affected_paths: Vec<String>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    conflicts: Vec<String>,
    #[serde(default)]
    recommendation: String,
    #[serde(default)]
    review_required: bool,
    #[serde(default)]
    frontmatter_valid: bool,
    #[serde(default)]
    missing_frontmatter_keys: Vec<String>,
    #[serde(default)]
    generated_at_utc: String,
}

impl HadesService {
    pub fn import_human_lifecycle_reviews(
        &self,
        input_path: impl AsRef<Path>,
        limit: usize,
    ) -> Result<HumanLifecycleImportReport> {
        let input_path = input_path.as_ref();
        let content = fs::read_to_string(input_path)?;
        let mut scanned_total = 0usize;
        let mut skipped_total = 0usize;
        let mut malformed_total = 0usize;
        let mut reviews = Vec::new();
        let bounded_limit = limit.max(1);

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if scanned_total >= bounded_limit {
                break;
            }
            scanned_total += 1;

            let record: HumanIngestionRecord = match serde_json::from_str(line) {
                Ok(record) => record,
                Err(err) => {
                    malformed_total += 1;
                    self.log_event(
                        "human_lifecycle_import_malformed",
                        Some(&input_path.display().to_string()),
                        serde_json::json!({ "error": err.to_string() }),
                    )?;
                    continue;
                }
            };

            if record.contract != HUMAN_INGESTION_CONTRACT {
                skipped_total += 1;
                continue;
            }

            if !record.review_required
                && record.conflicts.is_empty()
                && record.frontmatter_valid
                && !record.recommendation.contains("review")
            {
                skipped_total += 1;
                continue;
            }

            reviews.push(self.human_review_item(record));
        }

        append_jsonl_batch(&self.athena_handoff_queue_path, &reviews)?;
        let report = HumanLifecycleImportReport {
            contract: "arda.hades.human_lifecycle_import_report.v1".to_owned(),
            source_path: input_path.display().to_string(),
            queue_path: self.athena_handoff_queue_path.display().to_string(),
            scanned_total,
            queued_total: reviews.len(),
            skipped_total,
            malformed_total,
            generated_at_utc: Utc::now().to_rfc3339(),
        };
        self.log_event(
            "human_lifecycle_reviews_imported",
            Some(&input_path.display().to_string()),
            serde_json::to_value(&report).unwrap_or_else(|_| serde_json::json!({})),
        )?;
        Ok(report)
    }

    fn human_review_item(&self, record: HumanIngestionRecord) -> HumanLifecycleReviewItem {
        let severity = if !record.conflicts.is_empty() {
            "high"
        } else if !record.frontmatter_valid || !record.missing_frontmatter_keys.is_empty() {
            "medium"
        } else {
            "low"
        };

        HumanLifecycleReviewItem {
            contract: HADES_REVIEW_CONTRACT.to_owned(),
            review_id: format!("hhr_{}", uuid::Uuid::new_v4().simple()),
            queued_at_utc: Utc::now().to_rfc3339(),
            source_contract: record.contract,
            source_path: record.source_path,
            content_hash: record.content_hash,
            detected_status: record.detected_status,
            detected_authority: record.detected_authority,
            source_type: record.source_type,
            severity: severity.to_owned(),
            lifecycle_action: "review_required".to_owned(),
            allowed_actions: vec![
                "retain-working".to_owned(),
                "request-human-clarification".to_owned(),
                "propose-frontmatter-repair".to_owned(),
                "propose-canonical-promotion".to_owned(),
                "archive-after-approval".to_owned(),
            ],
            evidence: serde_json::json!({
                "summary": record.summary,
                "recommendation": record.recommendation,
                "conflicts": record.conflicts,
                "affected_agents": record.affected_agents,
                "affected_paths": record.affected_paths,
                "frontmatter_valid": record.frontmatter_valid,
                "missing_frontmatter_keys": record.missing_frontmatter_keys,
                "athena_generated_at_utc": record.generated_at_utc,
                "safety": {
                    "read_only_import": true,
                    "moves_files": false,
                    "deletes_files": false,
                    "promotes_to_canonical": false,
                    "destructive_actions_require_quorum": true
                }
            }),
            review_required: true,
            destructive_allowed: false,
        }
    }
}
