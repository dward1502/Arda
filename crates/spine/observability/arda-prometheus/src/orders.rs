// sigil: REPAIR
use arda_core::error::{ArdaError, Result};
use chrono::{DateTime, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    Open,
    Assigned,
    Complete,
    Failed,
    Escalated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderEvent {
    pub ts: String,
    pub task_id: String,
    pub task_type: String,
    pub status: OrderStatus,
    pub agent: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationStatus {
    Pending,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationEvent {
    pub escalation_id: String,
    pub ts: String,
    pub task_id: String,
    pub status: EscalationStatus,
    pub reason: String,
    pub confidence: f64,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeReconcileCandidate {
    pub id: String,
    pub ts: String,
    pub kind: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeReconcileSummary {
    pub apply: bool,
    pub cutoff_utc: String,
    pub completed_orders: Vec<RuntimeReconcileCandidate>,
    pub resolved_escalations: Vec<RuntimeReconcileCandidate>,
}

#[derive(Debug, Clone)]
pub struct OrderStore {
    root: PathBuf,
    orders_path: PathBuf,
    escalations_path: PathBuf,
}

impl OrderStore {
    pub fn from_default_or_fallback() -> Result<Self> {
        let primary = default_root();
        match Self::new(&primary) {
            Ok(v) => Ok(v),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                Self::new(arda_root().join("data").join("prometheus"))
            }
        }
    }

    pub fn new(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        let orders_path = root.join("orders.jsonl");
        let escalations_path = root.join("escalations.jsonl");
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&orders_path)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&escalations_path)?;
        Ok(Self {
            root,
            orders_path,
            escalations_path,
        })
    }

    pub fn append_order(
        &self,
        task_id: Uuid,
        task_type: &str,
        status: OrderStatus,
        agent: Option<&str>,
        note: &str,
    ) -> Result<()> {
        let event = OrderEvent {
            ts: Utc::now().to_rfc3339(),
            task_id: task_id.to_string(),
            task_type: task_type.to_string(),
            status,
            agent: agent.map(|v| v.to_string()),
            note: note.to_string(),
        };
        append_jsonl(&self.orders_path, &event)
    }

    pub fn append_escalation(
        &self,
        task_id: Uuid,
        reason: &str,
        confidence: f64,
    ) -> Result<String> {
        let escalation_id = format!("esc_{}", &Uuid::new_v4().simple().to_string()[..8]);
        let event = EscalationEvent {
            escalation_id: escalation_id.clone(),
            ts: Utc::now().to_rfc3339(),
            task_id: task_id.to_string(),
            status: EscalationStatus::Pending,
            reason: reason.to_string(),
            confidence,
            note: None,
        };
        append_jsonl(&self.escalations_path, &event)?;
        Ok(escalation_id)
    }

    pub fn resolve_escalation(&self, escalation_id: &str, note: &str) -> Result<EscalationEvent> {
        let latest = self.latest_escalations_by_id()?;
        let current = latest.get(escalation_id).ok_or_else(|| {
            ArdaError::Task(format!("escalation not found: {escalation_id}"))
        })?;
        if matches!(current.status, EscalationStatus::Resolved) {
            return Err(ArdaError::Task(format!(
                "escalation already resolved: {escalation_id}"
            )));
        }

        let resolved = EscalationEvent {
            escalation_id: current.escalation_id.clone(),
            ts: Utc::now().to_rfc3339(),
            task_id: current.task_id.clone(),
            status: EscalationStatus::Resolved,
            reason: current.reason.clone(),
            confidence: current.confidence,
            note: Some(note.to_string()),
        };
        append_jsonl(&self.escalations_path, &resolved)?;
        Ok(resolved)
    }

    pub fn list_escalations(
        &self,
        include_resolved: bool,
        limit: usize,
    ) -> Result<Vec<EscalationEvent>> {
        let mut events: Vec<EscalationEvent> =
            self.latest_escalations_by_id()?.into_values().collect();
        if !include_resolved {
            events.retain(|e| matches!(e.status, EscalationStatus::Pending));
        }
        events.sort_by(|a, b| b.ts.cmp(&a.ts));
        events.truncate(limit.max(1));
        Ok(events)
    }

    pub fn active_orders_count(&self) -> Result<usize> {
        let latest = self.latest_orders_by_id()?;
        let count = latest
            .values()
            .filter(|event| matches!(event.status, OrderStatus::Open | OrderStatus::Assigned))
            .count();
        Ok(count)
    }

    pub fn pending_escalations_count(&self) -> Result<usize> {
        let latest = self.latest_escalations_by_id()?;
        Ok(latest
            .values()
            .filter(|event| matches!(event.status, EscalationStatus::Pending))
            .count())
    }

    pub fn reconcile_stale_runtime(
        &self,
        cutoff: DateTime<Utc>,
        apply: bool,
        note: &str,
    ) -> Result<RuntimeReconcileSummary> {
        let latest_orders = self.latest_orders_by_id()?;
        let latest_escalations = self.latest_escalations_by_id()?;
        let mut completed_orders = Vec::new();
        let mut resolved_escalations = Vec::new();
        let now = Utc::now().to_rfc3339();

        for event in latest_orders.values() {
            if !matches!(event.status, OrderStatus::Open | OrderStatus::Assigned) {
                continue;
            }
            if !is_before_cutoff(&event.ts, cutoff) {
                continue;
            }
            completed_orders.push(RuntimeReconcileCandidate {
                id: event.task_id.clone(),
                ts: event.ts.clone(),
                kind: event.task_type.clone(),
                status: format!("{:?}", event.status).to_lowercase(),
            });
            if apply {
                let completed = OrderEvent {
                    ts: now.clone(),
                    task_id: event.task_id.clone(),
                    task_type: event.task_type.clone(),
                    status: OrderStatus::Complete,
                    agent: event
                        .agent
                        .clone()
                        .or_else(|| Some("prometheus".to_string())),
                    note: note.to_string(),
                };
                append_jsonl(&self.orders_path, &completed)?;
            }
        }

        for event in latest_escalations.values() {
            if !matches!(event.status, EscalationStatus::Pending) {
                continue;
            }
            if !is_before_cutoff(&event.ts, cutoff) {
                continue;
            }
            resolved_escalations.push(RuntimeReconcileCandidate {
                id: event.escalation_id.clone(),
                ts: event.ts.clone(),
                kind: event.reason.clone(),
                status: "pending".to_string(),
            });
            if apply {
                let resolved = EscalationEvent {
                    escalation_id: event.escalation_id.clone(),
                    ts: now.clone(),
                    task_id: event.task_id.clone(),
                    status: EscalationStatus::Resolved,
                    reason: event.reason.clone(),
                    confidence: event.confidence,
                    note: Some(note.to_string()),
                };
                append_jsonl(&self.escalations_path, &resolved)?;
            }
        }

        completed_orders.sort_by(|a, b| a.ts.cmp(&b.ts));
        resolved_escalations.sort_by(|a, b| a.ts.cmp(&b.ts));
        Ok(RuntimeReconcileSummary {
            apply,
            cutoff_utc: cutoff.to_rfc3339(),
            completed_orders,
            resolved_escalations,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn latest_escalations_by_id(&self) -> Result<HashMap<String, EscalationEvent>> {
        let content = fs::read_to_string(&self.escalations_path)?;
        let mut latest: HashMap<String, EscalationEvent> = HashMap::new();
        for event in parse_json_value_stream::<EscalationEvent>(&content)? {
            latest.insert(event.escalation_id.clone(), event);
        }
        Ok(latest)
    }

    fn latest_orders_by_id(&self) -> Result<HashMap<String, OrderEvent>> {
        let content = fs::read_to_string(&self.orders_path)?;
        let mut latest: HashMap<String, OrderEvent> = HashMap::new();
        for event in parse_json_value_stream::<OrderEvent>(&content)? {
            latest.insert(event.task_id.clone(), event);
        }
        Ok(latest)
    }
}

fn is_before_cutoff(ts: &str, cutoff: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(ts)
        .map(|parsed| parsed.with_timezone(&Utc) < cutoff)
        .unwrap_or(false)
}

fn parse_json_value_stream<T>(content: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let mut values = Vec::new();
    for item in serde_json::Deserializer::from_str(content).into_iter::<serde_json::Value>() {
        let value = item?;
        if let Ok(parsed) = serde_json::from_value(value) {
            values.push(parsed);
        }
    }
    Ok(values)
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn default_root() -> PathBuf {
    if let Ok(custom) = std::env::var("ARDA_PROMETHEUS_HOME") {
        return PathBuf::from(custom);
    }
    arda_root().join("data/prometheus")
}

fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}

#[cfg(test)]
mod tests {
    use super::{EscalationStatus, OrderStatus, OrderStore};
    use proptest::prelude::*;
    use tempfile::tempdir;
    use uuid::Uuid;

    #[test]
    fn counts_active_orders_and_pending_escalations() {
        let dir = tempdir().expect("tempdir");
        let store = OrderStore::new(dir.path()).expect("store");
        let task = Uuid::new_v4();

        store
            .append_order(task, "query", OrderStatus::Open, None, "received")
            .expect("open");
        assert_eq!(store.active_orders_count().expect("count"), 1);

        store
            .append_order(task, "query", OrderStatus::Complete, Some("athena"), "done")
            .expect("complete");
        assert_eq!(store.active_orders_count().expect("count"), 0);

        store
            .append_escalation(task, "confidence low", 0.4)
            .expect("escalate");
        assert_eq!(store.pending_escalations_count().expect("count"), 1);

        let list = store.list_escalations(false, 10).expect("list");
        assert_eq!(list.len(), 1);
        let esc_id = list[0].escalation_id.clone();
        let resolved = store
            .resolve_escalation(&esc_id, "approved by illuvatar")
            .expect("resolve");
        assert!(matches!(resolved.status, EscalationStatus::Resolved));
        assert_eq!(store.pending_escalations_count().expect("count"), 0);
    }

    #[test]
    fn counts_orders_from_concatenated_jsonl_records() {
        let dir = tempdir().expect("tempdir");
        let store = OrderStore::new(dir.path()).expect("store");
        let task_one = Uuid::new_v4();
        let task_two = Uuid::new_v4();

        std::fs::write(
            &store.orders_path,
            format!(
                "{{\"ts\":\"2026-06-06T00:00:00Z\",\"task_id\":\"{task_one}\",\"task_type\":\"ingest\",\"status\":\"open\",\"agent\":null,\"note\":\"task received\"}}{{\"ts\":\"2026-06-06T00:00:01Z\",\"task_id\":\"{task_two}\",\"task_type\":\"ingest\",\"status\":\"open\",\"agent\":null,\"note\":\"task received\"}}\n\n{{\"ts\":\"2026-06-06T00:00:02Z\",\"task_id\":\"{task_one}\",\"task_type\":\"ingest\",\"status\":\"complete\",\"agent\":\"athena\",\"note\":\"done\"}}\n"
            ),
        )
        .expect("write concatenated orders");

        assert_eq!(store.active_orders_count().expect("count"), 1);
    }

    #[test]
    fn ignores_mixed_json_records_in_escalation_ledger() {
        let dir = tempdir().expect("tempdir");
        let store = OrderStore::new(dir.path()).expect("store");
        let task = Uuid::new_v4();

        std::fs::write(
            &store.escalations_path,
            format!(
                "{{\"escalation_id\":\"esc_valid\",\"ts\":\"2026-06-06T00:00:00Z\",\"task_id\":\"{task}\",\"status\":\"pending\",\"reason\":\"confidence_below_threshold\",\"confidence\":0.55,\"note\":null}}{{\"detail\":\"policy_denied sender=guest disposition=override\",\"disposition\":\"override\",\"event_id\":\"int_test\",\"reason\":\"interrupt_authority_policy.denied\",\"sender\":\"guest\",\"severity\":\"warning\",\"source\":\"hermes_interrupt_policy\",\"ts_utc\":\"2026-06-06T00:00:01Z\"}}\n"
            ),
        )
        .expect("write mixed escalations");

        assert_eq!(store.pending_escalations_count().expect("count"), 1);
    }

    #[test]
    fn reconciles_stale_runtime_records_only_when_applied() {
        let dir = tempdir().expect("tempdir");
        let store = OrderStore::new(dir.path()).expect("store");
        let order_task = Uuid::new_v4();
        let escalation_task = Uuid::new_v4();
        let cutoff = chrono::DateTime::parse_from_rfc3339("2026-06-01T00:00:00Z")
            .expect("cutoff")
            .with_timezone(&chrono::Utc);

        std::fs::write(
            &store.orders_path,
            format!(
                "{{\"ts\":\"2026-03-13T00:00:00Z\",\"task_id\":\"{order_task}\",\"task_type\":\"ingest\",\"status\":\"open\",\"agent\":null,\"note\":\"task received\"}}\n"
            ),
        )
        .expect("write orders");
        std::fs::write(
            &store.escalations_path,
            format!(
                "{{\"escalation_id\":\"esc_stale\",\"ts\":\"2026-05-31T00:00:00Z\",\"task_id\":\"{escalation_task}\",\"status\":\"pending\",\"reason\":\"confidence_below_threshold\",\"confidence\":0.55,\"note\":null}}\n"
            ),
        )
        .expect("write escalations");

        let dry_run = store
            .reconcile_stale_runtime(cutoff, false, "dry run")
            .expect("dry run");
        assert!(!dry_run.apply);
        assert_eq!(dry_run.completed_orders.len(), 1);
        assert_eq!(dry_run.resolved_escalations.len(), 1);
        assert_eq!(store.active_orders_count().expect("active"), 1);
        assert_eq!(store.pending_escalations_count().expect("pending"), 1);

        let applied = store
            .reconcile_stale_runtime(cutoff, true, "stale runtime reconciliation")
            .expect("apply");
        assert!(applied.apply);
        assert_eq!(applied.completed_orders.len(), 1);
        assert_eq!(applied.resolved_escalations.len(), 1);
        assert_eq!(store.active_orders_count().expect("active"), 0);
        assert_eq!(store.pending_escalations_count().expect("pending"), 0);
    }

    proptest! {
        #[test]
        fn active_count_never_exceeds_open_append_count(n in 1usize..20) {
            let dir = tempdir().expect("tempdir");
            let store = OrderStore::new(dir.path()).expect("store");
            for _ in 0..n {
                let id = Uuid::new_v4();
                store.append_order(id, "query", OrderStatus::Open, None, "bench").expect("append");
            }
            let count = store.active_orders_count().expect("count");
            prop_assert!(count <= n);
        }

        #[test]
        fn completing_every_task_yields_zero_active(n in 1usize..20) {
            let dir = tempdir().expect("tempdir");
            let store = OrderStore::new(dir.path()).expect("store");
            let ids: Vec<Uuid> = (0..n).map(|_| Uuid::new_v4()).collect();
            for &id in &ids {
                store.append_order(id, "query", OrderStatus::Open, None, "open").expect("open");
            }
            for &id in &ids {
                store.append_order(id, "query", OrderStatus::Complete, Some("athena"), "done").expect("complete");
            }
            let count = store.active_orders_count().expect("count");
            prop_assert_eq!(count, 0);
        }

        #[test]
        fn list_escalations_respects_limit(n in 1usize..30, limit in 1usize..15) {
            let dir = tempdir().expect("tempdir");
            let store = OrderStore::new(dir.path()).expect("store");
            for _ in 0..n {
                let id = Uuid::new_v4();
                store.append_escalation(id, "confidence low", 0.4).expect("escalate");
            }
            let list = store.list_escalations(false, limit).expect("list");
            prop_assert!(list.len() <= limit);
        }

        #[test]
        fn pending_count_plus_resolved_equals_total_unique_escalations(n in 1usize..15) {
            let dir = tempdir().expect("tempdir");
            let store = OrderStore::new(dir.path()).expect("store");
            let mut esc_ids = Vec::new();
            for _ in 0..n {
                let id = Uuid::new_v4();
                let esc_id = store.append_escalation(id, "low confidence", 0.4).expect("escalate");
                esc_ids.push(esc_id);
            }
            // resolve half
            let resolve_n = n / 2;
            for esc_id in esc_ids.iter().take(resolve_n) {
                store.resolve_escalation(esc_id, "approved").expect("resolve");
            }
            let pending = store.pending_escalations_count().expect("pending");
            let all = store.list_escalations(true, 100).expect("all");
            prop_assert_eq!(pending, n - resolve_n);
            prop_assert_eq!(all.len(), n);
        }
    }
}
