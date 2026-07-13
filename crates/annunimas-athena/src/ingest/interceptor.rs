// Ingest pipeline interceptors.
//
// Side-effects that are not part of ATHENA's core book/digest write path live
// here so new source capture flows can attach behavior without changing ingest.

use annunimas_mnemosyne::{InformantEvent, MnemosyneService};
use chrono::Utc;
use fs2::FileExt;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use super::{layout, SourceType};

#[derive(Debug, Clone)]
pub struct IngestCtx {
    pub operation: String,
    pub source_id: String,
    pub raw_input: String,
    pub canonical_input: String,
    pub submitted_by: String,
    pub task_context: String,
    pub source_type: Option<SourceType>,
    pub url: Option<String>,
    pub metadata: Value,
}

impl IngestCtx {
    pub fn new(
        operation: impl Into<String>,
        source_id: impl Into<String>,
        raw_input: impl Into<String>,
        canonical_input: impl Into<String>,
        submitted_by: impl Into<String>,
        task_context: impl Into<String>,
    ) -> Self {
        Self {
            operation: operation.into(),
            source_id: source_id.into(),
            raw_input: raw_input.into(),
            canonical_input: canonical_input.into(),
            submitted_by: submitted_by.into(),
            task_context: task_context.into(),
            source_type: None,
            url: None,
            metadata: json!({}),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DigestEvent {
    ShallowSynced {
        source_id: String,
        source_type: SourceType,
        url: Option<String>,
        deduplicated: bool,
    },
    DeepQueued {
        source_id: String,
        agent: String,
        reason: String,
    },
    DeepSynced {
        source_id: String,
        policy_readiness: String,
        confidence: f64,
    },
    DeepFailed {
        source_id: String,
        reason: String,
    },
    BacklogWarning {
        source_id: String,
        pending: usize,
        threshold: usize,
    },
    PolicyPromoted {
        source_id: String,
        readiness: String,
    },
    Lifecycle {
        name: String,
        source_id: String,
        details: Value,
    },
}

impl DigestEvent {
    pub fn source_id(&self) -> &str {
        match self {
            DigestEvent::ShallowSynced { source_id, .. }
            | DigestEvent::DeepQueued { source_id, .. }
            | DigestEvent::DeepSynced { source_id, .. }
            | DigestEvent::DeepFailed { source_id, .. }
            | DigestEvent::BacklogWarning { source_id, .. }
            | DigestEvent::PolicyPromoted { source_id, .. }
            | DigestEvent::Lifecycle { source_id, .. } => source_id,
        }
    }

    pub fn event_name(&self) -> String {
        match self {
            DigestEvent::ShallowSynced { .. } => "athena_shallow_synced".to_string(),
            DigestEvent::DeepQueued { .. } => "athena_deep_queued".to_string(),
            DigestEvent::DeepSynced { .. } => "athena_deep_complete".to_string(),
            DigestEvent::DeepFailed { .. } => "athena_deep_failed".to_string(),
            DigestEvent::BacklogWarning { .. } => "athena_deep_backlog_warning".to_string(),
            DigestEvent::PolicyPromoted { .. } => "athena_policy_promoted".to_string(),
            DigestEvent::Lifecycle { name, .. } => name.clone(),
        }
    }

    pub fn details(&self) -> Value {
        match self {
            DigestEvent::ShallowSynced {
                source_type,
                url,
                deduplicated,
                ..
            } => json!({
                "source_type": format!("{source_type:?}"),
                "url": url,
                "deduplicated": deduplicated
            }),
            DigestEvent::DeepQueued { agent, reason, .. } => {
                json!({"agent": agent, "reason": reason})
            }
            DigestEvent::DeepSynced {
                policy_readiness,
                confidence,
                ..
            } => json!({"policy_readiness": policy_readiness, "confidence": confidence}),
            DigestEvent::DeepFailed { reason, .. } => json!({"error": reason}),
            DigestEvent::BacklogWarning {
                pending, threshold, ..
            } => json!({"pending_deep": pending, "threshold": threshold}),
            DigestEvent::PolicyPromoted { readiness, .. } => json!({"readiness": readiness}),
            DigestEvent::Lifecycle { details, .. } => details.clone(),
        }
    }
}

pub trait IngestInterceptor: Send + Sync {
    fn name(&self) -> &str;
    fn before(&self, _ctx: &mut IngestCtx) {}
    fn after(&self, _ctx: &IngestCtx, _event: &DigestEvent) {}
}

#[derive(Clone, Default)]
pub struct IngestPipeline {
    interceptors: Arc<RwLock<Vec<Box<dyn IngestInterceptor>>>>,
}

impl std::fmt::Debug for IngestPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IngestPipeline")
            .field("interceptors", &self.names())
            .finish()
    }
}

impl IngestPipeline {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register<I>(&self, interceptor: I)
    where
        I: IngestInterceptor + 'static,
    {
        if let Ok(mut guard) = self.interceptors.write() {
            guard.push(Box::new(interceptor));
        }
    }

    pub fn names(&self) -> Vec<String> {
        self.interceptors
            .read()
            .map(|items| items.iter().map(|item| item.name().to_string()).collect())
            .unwrap_or_default()
    }

    pub fn before(&self, ctx: &mut IngestCtx) {
        if let Ok(items) = self.interceptors.read() {
            for item in items.iter() {
                item.before(ctx);
            }
        }
    }

    pub fn after(&self, ctx: &IngestCtx, event: &DigestEvent) {
        if let Ok(items) = self.interceptors.read() {
            for item in items.iter() {
                item.after(ctx, event);
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct HadesQueueInterceptor {
    queue_path: PathBuf,
}

impl HadesQueueInterceptor {
    pub fn new(queue_path: impl Into<PathBuf>) -> Self {
        Self {
            queue_path: queue_path.into(),
        }
    }
}

impl IngestInterceptor for HadesQueueInterceptor {
    fn name(&self) -> &str {
        "hades_queue"
    }

    fn after(&self, _ctx: &IngestCtx, event: &DigestEvent) {
        if matches!(event, DigestEvent::ShallowSynced { .. }) {
            return;
        }
        let event_name = event.event_name();
        let source_id = event.source_id();
        let record = json!({
            "task_id": format!("ath_{source_id}"),
            "queued_at_utc": Utc::now().to_rfc3339(),
            "action": "investigate_orphan",
            "file": format!("books/{source_id}.jsonl"),
            "authorized_by": "athena",
            "reason": format!("athena lifecycle event: {event_name}"),
            "execute_after_utc": null
        });
        if let Err(err) = append_jsonl(&self.queue_path, &record) {
            tracing::warn!(error = %err, path = %self.queue_path.display(), "hades queue interceptor failed");
        }
    }
}

#[derive(Debug, Clone)]
pub struct WardenQueueInterceptor {
    queue_path: PathBuf,
}

impl WardenQueueInterceptor {
    pub fn new(queue_path: impl Into<PathBuf>) -> Self {
        Self {
            queue_path: queue_path.into(),
        }
    }
}

impl IngestInterceptor for WardenQueueInterceptor {
    fn name(&self) -> &str {
        "warden_queue"
    }

    fn after(&self, _ctx: &IngestCtx, event: &DigestEvent) {
        if matches!(event, DigestEvent::ShallowSynced { .. }) {
            return;
        }
        let event_name = event.event_name();
        let record = json!({
            "ts": Utc::now().to_rfc3339(),
            "event": event_name,
            "event_type": event_name,
            "source_id": event.source_id(),
            "details": event.details(),
            "crate_name": "athena",
            "source": "athena_lifecycle",
            "severity": match event {
                DigestEvent::DeepFailed { .. } | DigestEvent::BacklogWarning { .. } => "warning",
                _ => "info",
            },
            "status": match event {
                DigestEvent::DeepFailed { .. } => "attention_required",
                DigestEvent::BacklogWarning { .. } => "warning",
                _ => "observed",
            },
            "informant_id": format!("athena:{}:{}", event_name, event.source_id()),
            "origin": "athena",
            "synced": false
        });
        if let Err(err) = append_jsonl(&self.queue_path, &record) {
            tracing::warn!(error = %err, path = %self.queue_path.display(), "warden queue interceptor failed");
        }
    }
}

#[derive(Debug, Clone)]
pub struct MnemosyneInterceptor {
    semantic_root: PathBuf,
}

impl MnemosyneInterceptor {
    pub fn from_default() -> Self {
        let root = std::env::var("ANNUNIMAS_MNEMOSYNE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| layout::annunimas_root().join("data/mnemosyne"));
        Self {
            semantic_root: root.join("semantic").join("athena_policy_ready"),
        }
    }
}

impl Default for MnemosyneInterceptor {
    fn default() -> Self {
        Self::from_default()
    }
}

impl IngestInterceptor for MnemosyneInterceptor {
    fn name(&self) -> &str {
        "mnemosyne"
    }

    fn after(&self, ctx: &IngestCtx, event: &DigestEvent) {
        let DigestEvent::DeepSynced {
            source_id,
            policy_readiness,
            confidence,
        } = event
        else {
            return;
        };
        if policy_readiness != "policy_ready" {
            return;
        }

        let content = format!(
            "ATHENA marked {source_id} policy_ready with confidence {:.4}. Context: {}",
            confidence, ctx.task_context
        );
        let episodic_event = InformantEvent {
            informant_id: format!("athena:deep_synced:{source_id}"),
            crate_name: "athena".to_string(),
            event_type: "athena_policy_ready_memory".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: content.clone(),
            confidence_hint: Some(*confidence),
            tags: vec![
                "athena".to_string(),
                "deep_synced".to_string(),
                "policy_ready".to_string(),
                "semantic".to_string(),
            ],
        };
        match MnemosyneService::from_default_or_fallback() {
            Ok(service) => {
                if let Err(err) = service.encode(episodic_event) {
                    tracing::warn!(error = %err, source_id, "mnemosyne episodic encode failed");
                }
            }
            Err(err) => {
                tracing::warn!(error = %err, source_id, "mnemosyne service unavailable");
            }
        }

        let semantic = json!({
            "type": "semantic",
            "schema_version": "annunimas.athena-policy-ready-memory.v1",
            "memory_id": format!("athena_policy_ready_{source_id}"),
            "source_crate": "athena",
            "event_type": "athena_policy_ready_memory",
            "source_id": source_id,
            "policy_readiness": policy_readiness,
            "confidence": confidence,
            "content": content,
            "book_ref": format!("books/{source_id}.jsonl"),
            "created_at_utc": Utc::now().to_rfc3339(),
            "tags": ["athena", "policy_ready", "deep_synced", "semantic"]
        });
        let path = self
            .semantic_root
            .join(format!("athena_policy_ready_{source_id}.jsonl"));
        if let Err(err) = append_jsonl(&path, &semantic) {
            tracing::warn!(error = %err, path = %path.display(), "mnemosyne semantic write failed");
        }
    }
}

fn append_jsonl<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.lock_exclusive()?;
    let line = serde_json::to_string(value).map_err(std::io::Error::other)?;
    let write_result = writeln!(file, "{line}").and_then(|_| file.sync_data());
    let unlock_result = file.unlock();
    write_result?;
    unlock_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingInterceptor {
        before_seen: AtomicUsize,
        after_seen: AtomicUsize,
    }

    impl IngestInterceptor for CountingInterceptor {
        fn name(&self) -> &str {
            "counting"
        }

        fn before(&self, _ctx: &mut IngestCtx) {
            self.before_seen.fetch_add(1, Ordering::SeqCst);
        }

        fn after(&self, _ctx: &IngestCtx, _event: &DigestEvent) {
            self.after_seen.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn pipeline_runs_before_and_after_interceptors() {
        let pipeline = IngestPipeline::new();
        pipeline.register(CountingInterceptor {
            before_seen: AtomicUsize::new(0),
            after_seen: AtomicUsize::new(0),
        });
        let mut ctx = IngestCtx::new("test", "src_test", "raw", "canonical", "test", "test");
        pipeline.before(&mut ctx);
        pipeline.after(
            &ctx,
            &DigestEvent::DeepQueued {
                source_id: "src_test".into(),
                agent: "athena".into(),
                reason: "test".into(),
            },
        );
        assert_eq!(pipeline.names(), vec!["counting".to_string()]);
    }

    #[test]
    fn queue_interceptors_write_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        let hades = dir.path().join("hades.jsonl");
        let warden = dir.path().join("warden.jsonl");
        let pipeline = IngestPipeline::new();
        pipeline.register(HadesQueueInterceptor::new(&hades));
        pipeline.register(WardenQueueInterceptor::new(&warden));
        let ctx = IngestCtx::new("test", "src_test", "raw", "canonical", "test", "test");
        pipeline.after(
            &ctx,
            &DigestEvent::DeepFailed {
                source_id: "src_test".into(),
                reason: "boom".into(),
            },
        );
        assert!(fs::read_to_string(hades)
            .expect("hades")
            .contains("ath_src_test"));
        assert!(fs::read_to_string(warden)
            .expect("warden")
            .contains("attention_required"));
    }
}
