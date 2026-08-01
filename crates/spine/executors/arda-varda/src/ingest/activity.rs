// sigil: REPAIR
//
// Live activity observability: process-local active crawl tracking plus durable
// recent-pipeline and error recovery from append-only ATHENA ledgers.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use arda_core::error::Result;

use super::IngestRecord;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AthenaActiveCrawl {
    pub pipeline_id: String,
    pub url: String,
    pub provider: String,
    pub started_at_utc: String,
    pub elapsed_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AthenaCompletedPipeline {
    pub pipeline_id: String,
    pub source_id: String,
    pub stage: String,
    pub outcome: String,
    pub completed_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AthenaActivityError {
    pub pipeline_id: Option<String>,
    pub source_id: Option<String>,
    pub stage: String,
    pub occurred_at_utc: String,
    pub message: String,
}

#[derive(Debug, Clone)]
struct TrackedCrawl {
    pipeline_id: String,
    url: String,
    provider: String,
    started_at_utc: String,
}

#[derive(Debug, Default)]
struct ActivityState {
    active_crawls: BTreeMap<String, TrackedCrawl>,
    last_error: Option<AthenaActivityError>,
}

#[derive(Debug, Default)]
pub(super) struct ActivityTracker {
    state: Mutex<ActivityState>,
}

pub(super) struct CrawlActivityGuard {
    tracker: Arc<ActivityTracker>,
    pipeline_id: String,
    finished: bool,
}

impl CrawlActivityGuard {
    pub(super) fn complete(mut self) {
        self.tracker.finish_crawl(&self.pipeline_id);
        self.finished = true;
    }

    pub(super) fn fail(mut self, url: &str, message: &str) {
        self.tracker.fail_crawl(&self.pipeline_id, url, message);
        self.finished = true;
    }
}

impl Drop for CrawlActivityGuard {
    fn drop(&mut self) {
        if !self.finished {
            self.tracker.finish_crawl(&self.pipeline_id);
        }
    }
}

impl ActivityTracker {
    fn lock(&self) -> MutexGuard<'_, ActivityState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub(super) fn begin_crawl(
        self: &Arc<Self>,
        pipeline_id: &str,
        url: &str,
        provider: &str,
    ) -> CrawlActivityGuard {
        self.lock().active_crawls.insert(
            pipeline_id.to_string(),
            TrackedCrawl {
                pipeline_id: pipeline_id.to_string(),
                url: redact_url(url),
                provider: provider.to_string(),
                started_at_utc: Utc::now().to_rfc3339(),
            },
        );
        CrawlActivityGuard {
            tracker: Arc::clone(self),
            pipeline_id: pipeline_id.to_string(),
            finished: false,
        }
    }

    pub(super) fn finish_crawl(&self, pipeline_id: &str) {
        self.lock().active_crawls.remove(pipeline_id);
    }

    pub(super) fn fail_crawl(&self, pipeline_id: &str, url: &str, message: &str) {
        let mut state = self.lock();
        state.active_crawls.remove(pipeline_id);
        state.last_error = Some(AthenaActivityError {
            pipeline_id: Some(pipeline_id.to_string()),
            source_id: None,
            stage: "crawl".to_string(),
            occurred_at_utc: Utc::now().to_rfc3339(),
            message: format!("{}: {message}", redact_url(url)),
        });
    }

    pub(super) fn snapshot(
        &self,
        now: DateTime<Utc>,
    ) -> (Vec<AthenaActiveCrawl>, Option<AthenaActivityError>) {
        let state = self.lock();
        let active = state
            .active_crawls
            .values()
            .map(|crawl| AthenaActiveCrawl {
                pipeline_id: crawl.pipeline_id.clone(),
                url: crawl.url.clone(),
                provider: crawl.provider.clone(),
                started_at_utc: crawl.started_at_utc.clone(),
                elapsed_seconds: age_seconds(&crawl.started_at_utc, now).unwrap_or(0),
            })
            .collect();
        (active, state.last_error.clone())
    }
}

pub(super) fn recent_completed_pipelines(
    digest_path: &Path,
    crawl_receipts_path: &Path,
    limit: usize,
) -> Result<Vec<AthenaCompletedPipeline>> {
    let mut candidates = Vec::<AthenaCompletedPipeline>::new();

    for line in fs::read_to_string(digest_path)?.lines() {
        let Ok(record) = serde_json::from_str::<IngestRecord>(line) else {
            continue;
        };
        if record.pipeline_id.is_empty() {
            continue;
        }
        if parsed_timestamp(&record.processed_at_utc).is_none() {
            continue;
        }
        candidates.push(AthenaCompletedPipeline {
            pipeline_id: record.pipeline_id,
            source_id: record.id,
            stage: "ingest".to_string(),
            outcome: if record.error.is_some() {
                "failed"
            } else if record.deduplicated {
                "deduplicated"
            } else {
                "completed"
            }
            .to_string(),
            completed_at_utc: record.processed_at_utc,
        });
    }

    for line in fs::read_to_string(crawl_receipts_path)?.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(pipeline_id) = nonempty_string(&value, "pipeline_id") else {
            continue;
        };
        let Some(completed_at_utc) = nonempty_string(&value, "captured_at_utc") else {
            continue;
        };
        if parsed_timestamp(&completed_at_utc).is_none() {
            continue;
        }
        candidates.push(AthenaCompletedPipeline {
            pipeline_id,
            source_id: nonempty_string(&value, "source_id").unwrap_or_default(),
            stage: "crawl".to_string(),
            outcome: if value.get("success").and_then(Value::as_bool) == Some(false) {
                "failed"
            } else {
                "completed"
            }
            .to_string(),
            completed_at_utc,
        });
    }

    candidates.sort_by(|left, right| {
        parsed_timestamp(&left.completed_at_utc).cmp(&parsed_timestamp(&right.completed_at_utc))
    });
    let mut seen = HashSet::new();
    let mut recent = Vec::new();
    for candidate in candidates.into_iter().rev() {
        if seen.insert(candidate.pipeline_id.clone()) {
            recent.push(candidate);
            if recent.len() >= limit.max(1) {
                break;
            }
        }
    }
    Ok(recent)
}

pub(super) fn latest_durable_error(
    deep_queue_path: &Path,
    scholarly_reenrichment_path: &Path,
) -> Result<Option<AthenaActivityError>> {
    let mut errors = Vec::new();
    collect_failed_events(deep_queue_path, "deep", "ts", "reason", &mut errors)?;
    collect_failed_events(
        scholarly_reenrichment_path,
        "scholarly_reenrichment",
        "ts_utc",
        "last_error",
        &mut errors,
    )?;
    errors.sort_by(|left, right| {
        parsed_timestamp(&left.occurred_at_utc).cmp(&parsed_timestamp(&right.occurred_at_utc))
    });
    Ok(errors.pop())
}

pub(super) fn latest_error(
    process_error: Option<AthenaActivityError>,
    durable_error: Option<AthenaActivityError>,
) -> Option<AthenaActivityError> {
    match (process_error, durable_error) {
        (Some(process), Some(durable)) => {
            if parsed_timestamp(&process.occurred_at_utc)
                >= parsed_timestamp(&durable.occurred_at_utc)
            {
                Some(process)
            } else {
                Some(durable)
            }
        }
        (Some(process), None) => Some(process),
        (None, Some(durable)) => Some(durable),
        (None, None) => None,
    }
}

fn collect_failed_events(
    path: &Path,
    stage: &str,
    timestamp_field: &str,
    message_field: &str,
    errors: &mut Vec<AthenaActivityError>,
) -> Result<()> {
    for line in fs::read_to_string(path)?.lines() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if value.get("status").and_then(Value::as_str) != Some("failed") {
            continue;
        }
        let Some(occurred_at_utc) = nonempty_string(&value, timestamp_field) else {
            continue;
        };
        if parsed_timestamp(&occurred_at_utc).is_none() {
            continue;
        }
        let Some(message) = nonempty_string(&value, message_field) else {
            continue;
        };
        errors.push(AthenaActivityError {
            pipeline_id: nonempty_string(&value, "pipeline_id"),
            source_id: nonempty_string(&value, "source_id"),
            stage: stage.to_string(),
            occurred_at_utc,
            message,
        });
    }
    Ok(())
}

fn nonempty_string(value: &Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn parsed_timestamp(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn redact_url(raw: &str) -> String {
    if raw.starts_with("raw:") {
        return "raw:<redacted>".to_string();
    }
    let Ok(mut url) = reqwest::Url::parse(raw) else {
        return raw.to_string();
    };
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_query(None);
    url.set_fragment(None);
    url.to_string()
}

fn age_seconds(raw: &str, now: DateTime<Utc>) -> Option<u64> {
    let timestamp = parsed_timestamp(raw)?;
    Some(now.signed_duration_since(timestamp).num_seconds().max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::{
        latest_durable_error, latest_error, recent_completed_pipelines, ActivityTracker,
        AthenaActivityError,
    };
    use chrono::{TimeZone, Utc};
    use std::collections::HashSet;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    #[test]
    fn tracker_exposes_active_crawl_and_latest_process_error() {
        let tracker = Arc::new(ActivityTracker::default());
        let guard = tracker.begin_crawl(
            "athpl_test",
            "https://user:secret@example.com/path?token=secret",
            "crawl4ai",
        );
        let (active, error) = tracker.snapshot(Utc::now());
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].pipeline_id, "athpl_test");
        assert_eq!(active[0].url, "https://example.com/path");
        assert!(error.is_none());

        guard.fail("https://example.com", "upstream timeout");
        let (active, error) = tracker.snapshot(Utc::now());
        assert!(active.is_empty());
        assert!(error
            .expect("crawl error")
            .message
            .contains("upstream timeout"));
    }

    #[test]
    fn dropping_crawl_guard_removes_cancelled_activity() {
        let tracker = Arc::new(ActivityTracker::default());
        let guard = tracker.begin_crawl("athpl_cancelled", "https://example.com", "crawl4ai");
        assert_eq!(tracker.snapshot(Utc::now()).0.len(), 1);
        drop(guard);
        assert!(tracker.snapshot(Utc::now()).0.is_empty());
    }

    #[test]
    fn latest_error_selects_the_newer_event() {
        let older = AthenaActivityError {
            pipeline_id: None,
            source_id: None,
            stage: "deep".to_string(),
            occurred_at_utc: Utc
                .with_ymd_and_hms(2026, 7, 25, 1, 0, 0)
                .unwrap()
                .to_rfc3339(),
            message: "older".to_string(),
        };
        let newer = AthenaActivityError {
            message: "newer".to_string(),
            occurred_at_utc: Utc
                .with_ymd_and_hms(2026, 7, 25, 2, 0, 0)
                .unwrap()
                .to_rfc3339(),
            ..older.clone()
        };
        assert_eq!(
            latest_error(Some(older), Some(newer))
                .expect("latest error")
                .message,
            "newer"
        );
    }

    #[test]
    fn durable_activity_is_bounded_unique_newest_first_and_malformed_tolerant() {
        let dir = tempdir().expect("tempdir");
        let digest = dir.path().join("digest.jsonl");
        let receipts = dir.path().join("crawl.jsonl");
        fs::write(&digest, "not-json\n").expect("digest");

        let mut lines = Vec::new();
        for hour in 0..10 {
            lines.push(
                serde_json::json!({
                    "pipeline_id": format!("athpl_{hour}"),
                    "source_id": format!("src_{hour}"),
                    "captured_at_utc": format!("2026-07-25T{hour:02}:00:00Z"),
                    "success": true
                })
                .to_string(),
            );
        }
        lines.push(
            serde_json::json!({
                "pipeline_id": "athpl_5",
                "source_id": "src_5",
                "captured_at_utc": "2026-07-25T12:00:00Z",
                "success": true
            })
            .to_string(),
        );
        lines.push(
            "{\"pipeline_id\":\"\",\"captured_at_utc\":\"2026-07-25T13:00:00Z\"}".to_string(),
        );
        lines.push(
            "{\"pipeline_id\":\"athpl_invalid\",\"captured_at_utc\":\"invalid\"}".to_string(),
        );
        lines.push("not-json".to_string());
        fs::write(&receipts, format!("{}\n", lines.join("\n"))).expect("receipts");

        let recent = recent_completed_pipelines(&digest, &receipts, 8).expect("recent");
        assert_eq!(recent.len(), 8);
        assert_eq!(recent[0].pipeline_id, "athpl_5");
        assert_eq!(recent[1].pipeline_id, "athpl_9");
        assert_eq!(
            recent
                .iter()
                .map(|pipeline| &pipeline.pipeline_id)
                .collect::<HashSet<_>>()
                .len(),
            8
        );
    }

    #[test]
    fn latest_durable_error_selects_newest_deep_or_scholarly_failure() {
        let dir = tempdir().expect("tempdir");
        let deep = dir.path().join("deep.jsonl");
        let scholarly = dir.path().join("scholarly.jsonl");
        fs::write(
            &deep,
            "not-json\n{\"status\":\"failed\",\"ts\":\"2026-07-25T10:00:00Z\",\"pipeline_id\":\"athpl_deep\",\"reason\":\"deep failed\"}\n",
        )
        .expect("deep");
        fs::write(
            &scholarly,
            "{\"status\":\"failed\",\"ts_utc\":\"2026-07-25T11:00:00Z\",\"pipeline_id\":\"athpl_scholarly\",\"last_error\":\"metadata failed\"}\n",
        )
        .expect("scholarly");

        let error = latest_durable_error(&deep, &scholarly)
            .expect("error scan")
            .expect("latest error");
        assert_eq!(error.pipeline_id.as_deref(), Some("athpl_scholarly"));
        assert_eq!(error.stage, "scholarly_reenrichment");
        assert_eq!(error.message, "metadata failed");
    }
}
