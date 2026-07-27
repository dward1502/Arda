// sigil: REPAIR
//
// Policy-readiness surface: gate evaluation, readiness promotion,
// opposition-viewpoint harvesting, and evidence-driven planning-task
// generation. Owns the policy-specific JSONL aggregation helpers.

use arda_core::error::Result;
use arda_economics::JouleWorkUnit;
use arda_governance::{BaconLiteEvent, GateOutcome};
use chrono::Utc;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

use super::observability::planning_task_receipt_summary;
use super::remediation::{
    remediation_notes, remediation_owner, remediation_task_id, remediation_title,
};
use super::schema::{migrate_jsonl_value, JsonlStoreSchema};
use super::{athena_error, AthenaStore, BookEntry, DeepAnalysisData, DeepBookEntry};

pub(super) fn ingest_quarantine_reason(event: &BaconLiteEvent) -> Option<String> {
    (!event.passed && !event.triad_passed && event.bacon_outcome == Some(GateOutcome::Fail)).then(
        || {
            format!(
                "bacon_lite_failure:{}:{}",
                event.policy_version, event.rationale
            )
        },
    )
}

fn projects_task_queue_path() -> PathBuf {
    std::env::var("ARDA_PROJECT_TASK_QUEUE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| super::layout::arda_root().join("core/projects/tasks/queue.jsonl"))
}

impl AthenaStore {
    pub fn policy_readiness(&self, limit: usize) -> Result<Vec<Value>> {
        let content = fs::read_to_string(&self.policy_readiness_path)?;
        let mut items = Vec::new();
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                if let Ok(value) = migrate_jsonl_value(JsonlStoreSchema::PolicyReadiness, value) {
                    items.push(value);
                }
            }
        }
        items.reverse();
        items.truncate(limit.max(1));
        Ok(items)
    }

    pub fn harvest_opposition_evidence(
        &self,
        source_id: &str,
        topic: Option<&str>,
        submitted_by: &str,
    ) -> Result<Value> {
        let source_id = source_id.trim();
        if source_id.is_empty() {
            return Err(athena_error("source_id cannot be empty"));
        }
        let base = self.latest_ingest_record(source_id)?.ok_or_else(|| {
            athena_error(format!(
                "source not found for opposition harvest: {source_id}"
            ))
        })?;
        if base.task_context.starts_with("opposition_for:") {
            return Err(athena_error(format!(
                "refusing nested opposition harvest for synthetic opposition source: {source_id}"
            )));
        }
        let topic = topic
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| base.shallow.title.clone());

        let affirm = format!(
            "OPPOSING_VIEWPOINT AFFIRM for {source_id}: This perspective supports the architecture on topic '{topic}' based on operational coherence, governance layering, and pragmatic reliability tradeoffs. Keywords: opposing counterpoint research."
        );
        let challenge = format!(
            "OPPOSING_VIEWPOINT CHALLENGE for {source_id}: This perspective questions assumptions on topic '{topic}', highlighting uncertainty in evidence quality, overfitting risk, and governance burden under scale. Keywords: opposing counterpoint research."
        );

        let affirm_record = self.ingest(
            &affirm,
            submitted_by,
            &format!("opposition_for:{source_id}:affirm"),
        )?;
        let challenge_record = self.ingest(
            &challenge,
            submitted_by,
            &format!("opposition_for:{source_id}:challenge"),
        )?;
        let _ = self.queue_deep_analysis(
            &affirm_record.id,
            "athena",
            &format!("opposition_context_for:{source_id}"),
        );
        let _ = self.queue_deep_analysis(
            &challenge_record.id,
            "athena",
            &format!("opposition_context_for:{source_id}"),
        );
        let _ = self.emit_lifecycle_event(
            "athena_opposition_harvested",
            source_id,
            serde_json::json!({
                "harvested_sources": [affirm_record.id, challenge_record.id],
                "topic": topic
            }),
        );
        if let Err(err) =
            super::deep_cache::DeepAnalysisCache::new(&self.root).invalidate_doc(source_id)
        {
            tracing::warn!(error = %err, source_id = %source_id, "ATHENA deep cache invalidation failed after opposition harvest");
        }

        Ok(serde_json::json!({
            "source_id": source_id,
            "topic": topic,
            "harvested": [affirm_record.id, challenge_record.id],
            "queued_for_deep": true
        }))
    }

    pub fn generate_planning_tasks(&self, source_id: &str, limit: usize) -> Result<Value> {
        let source_id = source_id.trim();
        if source_id.is_empty() {
            return Err(athena_error("source_id cannot be empty"));
        }
        let book_path = self.books_dir.join(format!("{source_id}.jsonl"));
        if !book_path.exists() {
            return Err(athena_error(format!(
                "source not found in books: {source_id}"
            )));
        }

        let content = fs::read_to_string(&book_path)?;
        let mut shallow: Option<BookEntry> = None;
        let mut deep: Option<DeepBookEntry> = None;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match value.get("stage").and_then(|v| v.as_str()) {
                Some("shallow") => shallow = serde_json::from_value(value).ok(),
                Some("deep") => deep = serde_json::from_value(value).ok(),
                _ => {}
            }
        }
        let shallow = shallow.ok_or_else(|| {
            athena_error(format!("missing shallow entry for source: {source_id}"))
        })?;
        let shallow = self.recover_shallow_analysis(source_id, shallow)?;

        let mut text_blobs = vec![shallow.data.title.clone(), shallow.data.summary.clone()];
        let mut tags = shallow.data.relevance_tags.clone();
        if let Some(meta) = &shallow.data.scholarly_metadata {
            text_blobs.push(meta.paper_title.clone());
            text_blobs.push(meta.abstract_text.clone());
            tags.extend(meta.subjects.iter().cloned());
        }
        if let Some(deep) = &deep {
            text_blobs.push(deep.data.full_summary.clone());
            tags.extend(deep.data.relevance_tags.iter().cloned());
            if let Some(brief) = &deep.data.implementation_brief {
                text_blobs.push(brief.to_string());
            }
        }
        let corpus = text_blobs.join("\n").to_ascii_lowercase();
        tags = tags
            .into_iter()
            .map(|v| v.to_ascii_lowercase())
            .collect::<Vec<_>>();
        tags.sort();
        tags.dedup();

        let queue_path = projects_task_queue_path();
        if let Some(parent) = queue_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !queue_path.exists() {
            fs::write(&queue_path, "")?;
        }
        let existing_open = existing_open_planning_titles(&queue_path)?;
        let run_started_at = Utc::now().to_rfc3339();

        let heuristics = [
            (
                "context",
                "prometheus",
                "high",
                format!("ATHENA evidence plan for {source_id}: context-engineering workflow"),
                "Translate context-engineering evidence into concrete context budget, compaction, and reminder policy changes.",
            ),
            (
                "memory",
                "mnemosyne",
                "high",
                format!("ATHENA evidence plan for {source_id}: memory accumulation strategy"),
                "Derive persistent memory checkpoints and recall heuristics from ingested evidence.",
            ),
            (
                "routing",
                "manwe",
                "high",
                format!("ATHENA evidence plan for {source_id}: workload-specialized routing"),
                "Convert evidence on routing/model specialization into concrete route classes and provider policy updates.",
            ),
            (
                "safety",
                "warden",
                "high",
                format!("ATHENA evidence plan for {source_id}: harness and safety controls"),
                "Turn harness/safety evidence into execution guardrails, sandbox policy, and verification surfaces.",
            ),
            (
                "tool",
                "apollo",
                "medium",
                format!("ATHENA evidence plan for {source_id}: lazy tool discovery"),
                "Operationalize tool-discovery and execution sequencing from ingested evidence.",
            ),
            (
                "runtime",
                "athena",
                "high",
                format!("ATHENA evidence plan for {source_id}: bounded runtime contract"),
                "Translate implementation evidence into a bounded runtime contract with explicit lifecycle, env, and dependency surfaces.",
            ),
            (
                "provider",
                "manwe",
                "high",
                format!("ATHENA evidence plan for {source_id}: provider-selection policy"),
                "Convert provider-specific implementation evidence into deterministic provider ordering, fallback, and routing policy.",
            ),
            (
                "package",
                "prometheus",
                "medium",
                format!("ATHENA evidence plan for {source_id}: package activation posture"),
                "Bind package/runtime adoption to sovereign activation surfaces so operational state stays machine-readable and auditable.",
            ),
            (
                "governor",
                "manwe",
                "medium",
                format!("ATHENA evidence plan for {source_id}: governor and route controls"),
                "Project implementation evidence into route-control and governor state instead of leaving adoption as an implicit operator convention.",
            ),
            (
                "workflow",
                "apollo",
                "medium",
                format!("ATHENA evidence plan for {source_id}: workflow handoff"),
                "Map implementation implications into deterministic execution workflows and handoff surfaces.",
            ),
        ];

        let mut queued = Vec::new();
        let mut receipts = Vec::new();
        let now = Utc::now().to_rfc3339();
        for (needle, owner, priority, title, notes) in heuristics {
            if queued.len() >= limit.max(1) {
                break;
            }
            let matched = corpus.contains(needle) || tags.iter().any(|tag| tag.contains(needle));
            if !matched {
                receipts.push(serde_json::json!({
                    "ts_utc": now,
                    "source_id": source_id,
                    "title": title,
                    "owner": owner,
                    "priority": priority,
                    "trigger": needle,
                    "disposition": "skipped",
                    "reason": "signal_not_present",
                    "provenance": {
                        "book_ref": self.book_ref_for(source_id),
                        "has_deep_analysis": deep.is_some(),
                        "source_url": shallow.data.scholarly_metadata.as_ref().map(|meta| meta.source_url.clone())
                    }
                }));
                continue;
            }
            if existing_open.contains(title.as_str()) {
                receipts.push(serde_json::json!({
                    "ts_utc": now,
                    "source_id": source_id,
                    "title": title,
                    "owner": owner,
                    "priority": priority,
                    "trigger": needle,
                    "disposition": "skipped",
                    "reason": "already_open",
                    "provenance": {
                        "book_ref": self.book_ref_for(source_id),
                        "has_deep_analysis": deep.is_some(),
                        "source_url": shallow.data.scholarly_metadata.as_ref().map(|meta| meta.source_url.clone())
                    }
                }));
                continue;
            }
            let task = serde_json::json!({
                "id": remediation_task_id(source_id, &format!("plan_{needle}")),
                "status": "queued",
                "title": title,
                "priority": priority,
                "owner": owner,
                "queued_at_utc": now,
                "notes": notes,
                "meta": {
                    "origin": "athena_evidence_planner",
                    "source_id": source_id,
                    "scope": "task_generation",
                    "trigger": needle,
                }
            });
            self.append_jsonl(&queue_path, &task)?;
            receipts.push(serde_json::json!({
                "ts_utc": now,
                "source_id": source_id,
                "title": task.get("title").cloned().unwrap_or(Value::Null),
                "owner": owner,
                "priority": priority,
                "trigger": needle,
                "disposition": "queued",
                "reason": "signal_matched",
                "task_id": task.get("id").cloned().unwrap_or(Value::Null),
                "provenance": {
                    "book_ref": self.book_ref_for(source_id),
                    "has_deep_analysis": deep.is_some(),
                    "source_url": shallow.data.scholarly_metadata.as_ref().map(|meta| meta.source_url.clone())
                }
            }));
            queued.push(task);
        }
        for receipt in &receipts {
            self.append_jsonl(&self.planning_task_receipts_path, receipt)?;
        }
        let _ = self.emit_lifecycle_event(
            "athena_planning_tasks_evaluated",
            source_id,
            serde_json::json!({
                "run_started_at_utc": run_started_at,
                "queued_tasks": queued.len(),
                "receipts_total": receipts.len(),
                "book_ref": self.book_ref_for(source_id)
            }),
        );

        Ok(serde_json::json!({
            "source_id": source_id,
            "queued_tasks": queued.len(),
            "receipts_total": receipts.len(),
            "details": queued,
            "receipts": receipts
        }))
    }

    pub fn promote_policy_readiness(&self, limit: usize, reevaluate: bool) -> Result<Value> {
        let latest = latest_policy_entries(&self.policy_readiness_path)?;
        let mut sources = latest.keys().cloned().collect::<Vec<_>>();
        sources.sort();
        let mut reevaluated = 0usize;
        if reevaluate {
            for source_id in &sources {
                if reevaluated >= limit.max(1) {
                    break;
                }
                if let Some(entry) = latest.get(source_id) {
                    if entry.get("policy_readiness").and_then(|v| v.as_str())
                        == Some("reference_only")
                    {
                        let _ = self.deep_analyze(source_id);
                        reevaluated += 1;
                    }
                }
            }
        }

        let latest_after = latest_policy_entries(&self.policy_readiness_path)?;
        let queue_path = projects_task_queue_path();
        if let Some(parent) = queue_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if !queue_path.exists() {
            fs::write(&queue_path, "")?;
        }
        let existing_open = existing_open_remediation_keys(&queue_path)?;
        let mut queued = 0usize;
        let mut details = Vec::new();
        for (source_id, entry) in &latest_after {
            if queued >= limit.max(1) {
                break;
            }
            if self
                .latest_ingest_record(source_id)?
                .map(|v| v.task_context.starts_with("opposition_for:"))
                .unwrap_or(false)
            {
                continue;
            }
            if entry.get("policy_readiness").and_then(|v| v.as_str()) != Some("reference_only") {
                continue;
            }
            let blockers = entry
                .get("gate")
                .and_then(|g| g.get("blockers"))
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            for blocker in blockers {
                if queued >= limit.max(1) {
                    break;
                }
                let Some(blocker) = blocker.as_str() else {
                    continue;
                };
                let key = format!("{source_id}:{blocker}");
                if existing_open.contains(&key) {
                    continue;
                }
                let now = Utc::now().to_rfc3339();
                let task = serde_json::json!({
                    "id": remediation_task_id(source_id, blocker),
                    "status": "queued",
                    "title": remediation_title(source_id, blocker),
                    "priority": "high",
                    "owner": remediation_owner(blocker),
                    "queued_at_utc": now,
                    "notes": remediation_notes(source_id, blocker),
                    "meta": {
                        "source_id": source_id,
                        "blocker": blocker,
                        "origin": "athena_policy_gate"
                    }
                });
                self.append_jsonl(&queue_path, &task)?;
                details.push(task);
                queued += 1;
            }
            let _ = self.emit_lifecycle_event(
                "athena_policy_remediation_queued",
                source_id,
                serde_json::json!({
                    "queued_items": queued
                }),
            );
        }

        let readiness_latest = latest_after
            .values()
            .filter_map(|entry| entry.get("policy_readiness").and_then(|v| v.as_str()))
            .collect::<Vec<_>>();
        let policy_ready = readiness_latest
            .iter()
            .filter(|value| **value == "policy_ready")
            .count();
        let total_readiness = readiness_latest.len().max(1);
        let policy_ratio = policy_ready as f64 / total_readiness as f64;
        let remediation_pressure = (queued as f64 / limit.max(1) as f64).clamp(0.0, 1.0);
        let task_receipts = planning_task_receipt_summary(&self.planning_task_receipts_path)?;
        let promotion_receipt_available = policy_ready > 0 || task_receipts.total > 0;
        if promotion_receipt_available {
            self.emit_relationship_signal_background(
                "athena".to_string(),
                "policy_surface".to_string(),
                (0.45 + policy_ratio * 0.4).clamp(0.25, 0.95),
                (0.4 + (1.0 - remediation_pressure) * 0.35).clamp(0.2, 0.9),
                if reevaluate { 0.74 } else { 0.64 },
                "athena_policy_promote",
            );
            self.emit_work_signal_background(
                "athena".to_string(),
                (0.4 + policy_ratio * 0.45).clamp(0.2, 0.95),
                JouleWorkUnit::Reasoning,
                "athena_policy_promote",
            );
        }

        Ok(serde_json::json!({
            "reevaluate_requested": reevaluate,
            "reevaluated_sources": reevaluated,
            "queued_tasks": queued,
            "policy_ready_recent": policy_ready,
            "task_emission_receipts_total": task_receipts.total,
            "task_emission_success_total": task_receipts.queued,
            "promotion_receipt_available": promotion_receipt_available,
            "queue_path": queue_path,
            "details": details
        }))
    }
}

pub(super) fn evaluate_policy_readiness(
    shallow: &BookEntry,
    deep: &DeepAnalysisData,
    source_id: &str,
    opposition_coverage: usize,
) -> (String, Value) {
    let triad_pass_rate_ok = deep.triad_analysis.passed;
    let confidence_threshold = if triad_pass_rate_ok && opposition_coverage >= 2 {
        0.70
    } else {
        0.80
    };
    let confidence_ok = deep.confidence >= confidence_threshold;
    let opposition_ok = opposition_coverage >= 2;
    let citation_integrity = !shallow.data.title.trim().is_empty();
    let hash_chain_valid = true;

    let mut blockers = Vec::new();
    if !triad_pass_rate_ok {
        blockers.push("triad_threshold".to_string());
    }
    if !confidence_ok {
        blockers.push("confidence_threshold".to_string());
    }
    if !opposition_ok {
        blockers.push("opposition_coverage".to_string());
    }
    if !citation_integrity {
        blockers.push("citation_integrity".to_string());
    }
    if !hash_chain_valid {
        blockers.push("hash_chain_invalid".to_string());
    }
    let readiness = if blockers.is_empty() {
        "policy_ready"
    } else {
        "reference_only"
    };
    (
        readiness.to_string(),
        serde_json::json!({
            "source_id": source_id,
            "thresholds": {
                "triad_pass_rate": 0.75,
                "confidence_mean": confidence_threshold,
                "opposition_coverage_min": 2,
                "citation_integrity_required": true,
                "hash_chain_required": true
            },
            "observed": {
                "triad_passed": triad_pass_rate_ok,
                "confidence": deep.confidence,
                "opposition_coverage": opposition_coverage,
                "citation_integrity": citation_integrity,
                "hash_chain_valid": hash_chain_valid
            },
            "blockers": blockers
        }),
    )
}

pub(super) fn opposition_coverage_count(path: &Path, source_id: &str) -> Result<usize> {
    let content = fs::read_to_string(path)?;
    let mut seen = std::collections::HashSet::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let Some(task_context) = value.get("task_context").and_then(|v| v.as_str()) else {
            continue;
        };
        if !task_context.starts_with("opposition_for:") {
            continue;
        }
        let mut parts = task_context.split(':');
        let _prefix = parts.next();
        let target = parts.next().unwrap_or_default();
        let stance = parts.next().unwrap_or_default();
        if target == source_id && !stance.is_empty() {
            seen.insert(stance.to_string());
        }
    }
    Ok(seen.len())
}

fn latest_policy_entries(path: &Path) -> Result<std::collections::HashMap<String, Value>> {
    let content = fs::read_to_string(path)?;
    let mut latest = std::collections::HashMap::<String, Value>::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value = match serde_json::from_str::<Value>(line)
            .ok()
            .and_then(|value| migrate_jsonl_value(JsonlStoreSchema::PolicyReadiness, value).ok())
        {
            Some(v) => v,
            None => continue,
        };
        let Some(source_id) = value.get("source_id").and_then(|v| v.as_str()) else {
            continue;
        };
        latest.insert(source_id.to_string(), value);
    }
    Ok(latest)
}

fn existing_open_remediation_keys(path: &Path) -> Result<std::collections::HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut out = std::collections::HashSet::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "queued" | "backlog" | "pending" | "in_progress") {
            continue;
        }
        let source_id = value
            .get("meta")
            .and_then(|m| m.get("source_id"))
            .and_then(|v| v.as_str());
        let blocker = value
            .get("meta")
            .and_then(|m| m.get("blocker"))
            .and_then(|v| v.as_str());
        if let (Some(source_id), Some(blocker)) = (source_id, blocker) {
            out.insert(format!("{source_id}:{blocker}"));
        }
    }
    Ok(out)
}

fn existing_open_planning_titles(path: &Path) -> Result<std::collections::HashSet<String>> {
    let content = fs::read_to_string(path)?;
    let mut out = std::collections::HashSet::new();
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let status = value.get("status").and_then(|v| v.as_str()).unwrap_or("");
        if !matches!(status, "queued" | "backlog" | "pending" | "in_progress") {
            continue;
        }
        if let Some(title) = value.get("title").and_then(|v| v.as_str()) {
            out.insert(title.to_string());
        }
    }
    Ok(out)
}
