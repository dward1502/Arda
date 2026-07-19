#![cfg(feature = "full-cli")]
use crate::service::{append_jsonl, PrometheusService};
use annunimas_core::error::Result;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use uuid::Uuid;

impl PrometheusService {
    pub fn interrupt_reroute(&self, payload: serde_json::Value) -> Result<serde_json::Value> {
        let interruption_id = payload
            .get("event_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_interrupt")
            .to_string();
        let content = payload
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let source = payload
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("hermes")
            .to_string();
        let sender = payload
            .get("sender")
            .and_then(|v| v.as_str())
            .unwrap_or("operator")
            .to_string();
        let run_id = payload
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let session_id = payload
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(|v| v.to_string());
        let triad_passed = payload
            .get("triad_passed")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let triad_score = payload
            .get("triad_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let policy_safe = payload
            .get("policy_safe")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let requires_operator_review = payload
            .get("requires_operator_review")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let mut target_task_ids = payload
            .get("context")
            .and_then(|v| v.get("task_ids"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if target_task_ids.is_empty() {
            if let Some(task_id) = payload
                .get("task_id")
                .and_then(|v| v.as_str())
                .filter(|v| !v.trim().is_empty())
            {
                target_task_ids.push(task_id.to_string());
            }
        }
        if target_task_ids.is_empty() {
            target_task_ids.push("task_unspecified".to_string());
        }

        let now = chrono::Utc::now().to_rfc3339();
        let latest_by_target = self.latest_intent_ids_by_target()?;
        let existing_by_idempotency = self.latest_intents_by_idempotency_key()?;
        let mut events = Vec::new();
        let mut duplicates = Vec::new();
        for target_task_id in target_task_ids {
            let idempotency_key = format!("{}::{}", interruption_id, target_task_id);
            if let Some(existing) = existing_by_idempotency.get(&idempotency_key) {
                let duplicate_event = serde_json::json!({
                    "ts_utc": now,
                    "intent_id": existing.get("intent_id").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "action": "duplicate_ignored",
                    "status": existing.get("status").and_then(|v| v.as_str()).unwrap_or("queued"),
                    "source": "hermes_interrupt",
                    "interruption_id": interruption_id,
                    "idempotency_key": idempotency_key,
                    "target_task_id": target_task_id,
                });
                append_jsonl(&self.execution_intents_path, &duplicate_event)?;
                duplicates.push(serde_json::json!({
                    "idempotency_key": idempotency_key,
                    "action": "duplicate_ignored",
                    "intent_id": existing.get("intent_id").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "target_task_id": target_task_id
                }));
                continue;
            }
            let existing = latest_by_target.get(&target_task_id).cloned();
            let intent_id = existing
                .unwrap_or_else(|| format!("intn_{}", &Uuid::new_v4().simple().to_string()[..10]));
            let action = if latest_by_target.contains_key(&target_task_id) {
                "adjust"
            } else {
                "create"
            };
            let event = serde_json::json!({
                "ts_utc": now,
                "intent_id": intent_id,
                "action": action,
                "status": if triad_passed { "queued" } else { "pending_review" },
                "source": "hermes_interrupt",
                "interruption_id": interruption_id,
                "idempotency_key": idempotency_key,
                "target_task_id": target_task_id,
                "request": {
                    "task_type": "interrupt_reroute",
                    "description": content,
                    "priority": if requires_operator_review { "high" } else { "normal" },
                    "run_id": run_id,
                    "session_id": session_id,
                },
                "safety": {
                    "triad_passed": triad_passed,
                    "triad_score": triad_score,
                    "policy_safe": policy_safe,
                    "requires_operator_review": requires_operator_review
                },
                "actor": {
                    "source": source,
                    "sender": sender
                }
            });
            append_jsonl(&self.execution_intents_path, &event)?;
            events.push(event);
        }

        let _ = self.thought_ledger.append(
            "audit",
            "interrupt_reroute_intent",
            &format!(
                "PROMETHEUS queued {} interrupt reroute intent(s) from {}; duplicates_ignored={}.",
                events.len(),
                interruption_id,
                duplicates.len()
            ),
        );
        Ok(serde_json::json!({
            "ack": {
                "contract_version": "v1",
                "acknowledged": true,
                "ack_id": format!("ack_{}", &Uuid::new_v4().simple().to_string()[..10]),
                "ack_ts_utc": chrono::Utc::now().to_rfc3339(),
                "source": "prometheus"
            },
            "queued": events.len(),
            "duplicates_ignored": duplicates.len(),
            "duplicates": duplicates,
            "intents": events,
            "execution_intents_path": self.execution_intents_path
        }))
    }

    pub fn execution_intents(
        &self,
        limit: usize,
        include_terminal: bool,
    ) -> Result<Vec<serde_json::Value>> {
        let latest = self.latest_execution_intents_by_id()?;
        let mut out = latest
            .into_values()
            .filter(|v| {
                if include_terminal {
                    return true;
                }
                !matches!(
                    v.get("status").and_then(|s| s.as_str()),
                    Some("superseded" | "expired")
                )
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| {
            let ats = a.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            let bts = b.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            bts.cmp(ats)
        });
        out.truncate(limit.max(1));
        Ok(out)
    }

    pub fn execution_intents_recovery(&self) -> Result<serde_json::Value> {
        if self.execution_intents_recovery_path.exists() {
            let raw = fs::read_to_string(&self.execution_intents_recovery_path)?;
            let value: serde_json::Value = serde_json::from_str(&raw)?;
            return Ok(value);
        }
        Self::write_execution_intents_recovery(
            &self.execution_intents_path,
            &self.execution_intents_recovery_path,
        )
    }

    pub fn transition_execution_intent(
        &self,
        intent_id: &str,
        new_status: &str,
        note: Option<&str>,
    ) -> Result<serde_json::Value> {
        let latest = self.latest_execution_intents_by_id()?;
        let current = latest.get(intent_id).ok_or_else(|| {
            annunimas_core::error::AnnunimasError::Task(format!("intent not found: {intent_id}"))
        })?;
        let from_status = current
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("queued");
        if !is_valid_intent_status(new_status) {
            return Err(annunimas_core::error::AnnunimasError::Task(format!(
                "invalid intent status: {new_status}"
            )));
        }
        if !is_valid_intent_transition(from_status, new_status) {
            return Err(annunimas_core::error::AnnunimasError::Task(format!(
                "invalid transition {from_status} -> {new_status}"
            )));
        }

        let event = serde_json::json!({
            "ts_utc": chrono::Utc::now().to_rfc3339(),
            "intent_id": intent_id,
            "action": "transition",
            "from_status": from_status,
            "status": new_status,
            "note": note,
            "source": "prometheus_operator"
        });
        append_jsonl(&self.execution_intents_path, &event)?;
        Ok(event)
    }

    pub fn compact_execution_intents(
        &self,
        retention_days: i64,
        max_keep: usize,
    ) -> Result<serde_json::Value> {
        let content = fs::read_to_string(&self.execution_intents_path)?;
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::days(retention_days.max(1));
        let mut latest_by_id = HashMap::<String, serde_json::Value>::new();
        for line in content.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(intent_id) = value.get("intent_id").and_then(|v| v.as_str()) else {
                continue;
            };
            latest_by_id.insert(intent_id.to_string(), value);
        }

        let mut kept = latest_by_id
            .into_values()
            .filter(|value| {
                let status = value
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("queued");
                if !matches!(status, "superseded" | "expired") {
                    return true;
                }
                let ts = value
                    .get("ts_utc")
                    .and_then(|v| v.as_str())
                    .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
                    .map(|v| v.with_timezone(&chrono::Utc));
                match ts {
                    Some(ts) => ts >= cutoff,
                    None => false,
                }
            })
            .collect::<Vec<_>>();
        kept.sort_by(|a, b| {
            let ats = a.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            let bts = b.get("ts_utc").and_then(|v| v.as_str()).unwrap_or("");
            bts.cmp(ats)
        });
        if kept.len() > max_keep.max(1) {
            kept.truncate(max_keep.max(1));
        }
        kept.reverse();

        let mut out = String::new();
        for value in &kept {
            out.push_str(&serde_json::to_string(value)?);
            out.push('\n');
        }
        fs::write(&self.execution_intents_path, out)?;
        Ok(serde_json::json!({
            "compacted": true,
            "kept": kept.len(),
            "retention_days": retention_days.max(1),
            "max_keep": max_keep.max(1),
            "execution_intents_path": self.execution_intents_path
        }))
    }

    fn latest_intent_ids_by_target(&self) -> Result<HashMap<String, String>> {
        let content = fs::read_to_string(&self.execution_intents_path)?;
        let mut latest = HashMap::new();
        for line in content.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(target) = value.get("target_task_id").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(intent_id) = value.get("intent_id").and_then(|v| v.as_str()) else {
                continue;
            };
            latest.insert(target.to_string(), intent_id.to_string());
        }
        Ok(latest)
    }

    fn latest_intents_by_idempotency_key(&self) -> Result<HashMap<String, serde_json::Value>> {
        let content = fs::read_to_string(&self.execution_intents_path)?;
        let mut latest = HashMap::new();
        for line in content.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(key) = value.get("idempotency_key").and_then(|v| v.as_str()) else {
                continue;
            };
            latest.insert(key.to_string(), value);
        }
        Ok(latest)
    }

    fn latest_execution_intents_by_id(&self) -> Result<HashMap<String, serde_json::Value>> {
        let content = fs::read_to_string(&self.execution_intents_path)?;
        let mut latest = HashMap::new();
        for line in content.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let Some(intent_id) = value.get("intent_id").and_then(|v| v.as_str()) else {
                continue;
            };
            latest.insert(intent_id.to_string(), value);
        }
        Ok(latest)
    }

    pub(crate) fn write_execution_intents_recovery(
        intents_path: &Path,
        recovery_path: &Path,
    ) -> Result<serde_json::Value> {
        let content = fs::read_to_string(intents_path).unwrap_or_default();
        let mut latest = HashMap::<String, serde_json::Value>::new();
        let mut idempotency_seen = HashSet::<String>::new();
        let mut duplicate_idempotency = 0usize;
        for line in content.lines() {
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            if let Some(key) = value.get("idempotency_key").and_then(|v| v.as_str()) {
                if !idempotency_seen.insert(key.to_string()) {
                    duplicate_idempotency += 1;
                }
            }
            let Some(intent_id) = value.get("intent_id").and_then(|v| v.as_str()) else {
                continue;
            };
            latest.insert(intent_id.to_string(), value);
        }
        let pending = latest
            .values()
            .filter(|v| {
                matches!(
                    v.get("status").and_then(|s| s.as_str()),
                    Some("pending_review" | "queued")
                )
            })
            .count();
        let summary = serde_json::json!({
            "ts_utc": chrono::Utc::now().to_rfc3339(),
            "source": "prometheus_startup_recovery",
            "execution_intents_path": intents_path,
            "total_latest_intents": latest.len(),
            "pending_recovered": pending,
            "duplicate_idempotency_records": duplicate_idempotency,
            "replay_safe": true
        });
        if let Some(parent) = recovery_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::write(
            recovery_path,
            serde_json::to_string_pretty(&summary)? + "\n",
        )?;
        Ok(summary)
    }
}

fn is_valid_intent_status(status: &str) -> bool {
    matches!(
        status,
        "pending_review" | "queued" | "assigned" | "superseded" | "expired"
    )
}

fn is_valid_intent_transition(from: &str, to: &str) -> bool {
    if from == to {
        return true;
    }
    match from {
        "pending_review" => matches!(to, "queued" | "superseded" | "expired"),
        "queued" => matches!(to, "assigned" | "superseded" | "expired"),
        "assigned" => matches!(to, "superseded" | "expired"),
        "superseded" | "expired" => false,
        _ => false,
    }
}
