use crate::significance::{classify_significance, evaluate_significance, SignificanceResult};
use arda_core::error::{ArdaError, Result};
use arda_economics::{JouleWorkUnit, PlutusService};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration as StdDuration;

// Ownership boundary: this module defines the public memory contracts and
// coordinates scoring, persistence, retrieval, and consolidation. `store`
// owns the on-disk JSONL layout; `retrieval` owns ranking; `promotion` owns
// derived semantic/procedural records and their promotion receipts.
pub mod continuity;
pub mod governance;
pub mod governed;
mod persona_derive;
mod promotion;
pub mod retention;
mod retrieval;
pub mod scope_policy;
mod status;
mod store;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformantEvent {
    pub informant_id: String,
    pub crate_name: String,
    pub event_type: String,
    pub ts_utc: String,
    pub content: String,
    pub confidence_hint: Option<f64>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallRecentEntry {
    pub schema_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub migrated_from_schema: Option<String>,
    pub memory_id: String,
    pub source_crate: String,
    pub event_type: String,
    pub memory_scope: String,
    pub significance: f64,
    /// Source confidence supplied at encode time (or the deterministic
    /// Bacon-lite fallback for legacy records).
    pub confidence: f64,
    /// Bacon-lite confidence, discounted when the governance triad did not pass.
    pub trust: f64,
    pub sigil: String,
    pub content: String,
    pub ts_utc: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSeedRecallEntry {
    pub path: String,
    pub title: String,
    pub classification: String,
    pub canonical_home: String,
    pub domain: String,
    pub authority: String,
    pub recommended_action: String,
    pub rationale: String,
    pub soterion_glyph: String,
    pub soterion_sigil: String,
    pub triaged_at_utc: String,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityState {
    pub generated_at_utc: String,
    pub core_memory_count: usize,
    pub active_memory_count: usize,
    pub peripheral_memory_count: usize,
    pub transient_memory_count: usize,
    pub recent_events: Vec<RecallRecentEntry>,
    pub current_mission_focus: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCounts {
    pub core: usize,
    pub active: usize,
    pub peripheral: usize,
    pub transient: usize,
    pub consolidated: usize,
    pub archived: usize,
    pub released: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MnemosyneStats {
    pub schema_version: String,
    pub generated_at_utc: String,
    pub memory_counts: MemoryCounts,
    pub last_consolidation_utc: Option<String>,
    pub next_consolidation_utc: String,
    pub chain_integrity: String,
    pub informants_connected: usize,
    pub checkpoint_policy: MemoryCheckpointPolicy,
    pub malformed_noise_records: usize,
    pub malformed_obsidian_records: usize,
    pub malformed_archive_records: usize,
    pub malformed_episodic_records: usize,
    pub legacy_episodic_records: usize,
    pub unsupported_episodic_records: usize,
    pub observability: MemoryObservabilitySnapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryObservabilitySnapshot {
    pub recall_requests_total: u64,
    pub recall_results_total: u64,
    pub last_recall_fidelity: Option<f64>,
    pub last_recall_latency_ms: Option<u64>,
    pub queue_observations_total: u64,
    pub last_queue_latency_ms: Option<u64>,
    pub last_consolidation_depth: usize,
    pub promotion_receipts_total: u64,
    pub retention_runs_total: u64,
    pub retention_decayed_total: u64,
    pub compression_runs_total: u64,
    pub compression_source_records_total: u64,
    pub revocation_receipts_total: u64,
    pub quarantine_records_total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCheckpointPolicy {
    pub checkpoint_interval_events: usize,
    pub recall_window_hours: i64,
    pub priority_tags: Vec<String>,
    pub consolidation_bias: String,
    pub memory_pressure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub consolidated_at_utc: String,
    pub window_hours: i64,
    pub episodic_scanned: usize,
    pub semantic_patterns_written: usize,
    pub procedural_patterns_written: usize,
    pub archived_records_written: usize,
    pub promotion_receipts_written: usize,
    pub consolidation_depth: usize,
    pub observed_count: usize,
    pub eligible_count: usize,
    pub promoted_count: usize,
    pub blocked_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsidianSyncReport {
    pub synced_at_utc: String,
    pub vault_path: String,
    pub files_scanned: usize,
    pub notes_indexed: usize,
    pub memories_encoded: usize,
    pub index_path: String,
}

#[derive(Debug, Clone)]
struct EpisodicRecord {
    schema_version: String,
    migrated_from_schema: Option<String>,
    sigil: String,
    memory_id: String,
    source_crate: String,
    event_type: String,
    memory_scope: String,
    significance: f64,
    confidence: f64,
    trust: f64,
    content: String,
    ts_utc: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MnemosyneService {
    root: PathBuf,
    episodic_root: PathBuf,
    semantic_root: PathBuf,
    procedural_root: PathBuf,
    archive_root: PathBuf,
    persona_root: PathBuf,
    chain_head_path: PathBuf,
    noise_ledger_path: PathBuf,
    obsidian_index_path: PathBuf,
    last_consolidation_path: PathBuf,
    human_projection_root: Option<PathBuf>,
    /// When set, every successful encode() also writes a v0.1
    /// `MemoryRecord` to `root/episodic/id.json`. The dual-write path is
    /// opt-in so existing tests and offline runs are unchanged.
    pub(crate) contract_memory_root: Option<PathBuf>,
    metrics_root: Option<PathBuf>,
    observability: Arc<Mutex<MemoryObservabilitySnapshot>>,
}

impl MnemosyneService {
    pub fn observability_snapshot(&self) -> MemoryObservabilitySnapshot {
        self.observability
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn observe_recall(&self, result_count: usize, fidelity: Option<f64>, elapsed: StdDuration) {
        let snapshot = {
            let mut metrics = self
                .observability
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            metrics.recall_requests_total += 1;
            metrics.recall_results_total += result_count as u64;
            metrics.last_recall_fidelity = fidelity;
            metrics.last_recall_latency_ms =
                Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
            metrics.clone()
        };
        self.persist_observability_best_effort(&snapshot);
    }

    pub(crate) fn observe_queue_latency(&self, elapsed: StdDuration) {
        let snapshot = {
            let mut metrics = self
                .observability
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            metrics.queue_observations_total += 1;
            metrics.last_queue_latency_ms =
                Some(elapsed.as_millis().min(u128::from(u64::MAX)) as u64);
            metrics.clone()
        };
        self.persist_observability_best_effort(&snapshot);
    }

    fn observe_consolidation(&self, depth: usize, receipts: usize) {
        let snapshot = {
            let mut metrics = self
                .observability
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            metrics.last_consolidation_depth = depth;
            metrics.promotion_receipts_total += receipts as u64;
            metrics.clone()
        };
        self.persist_observability_best_effort(&snapshot);
    }

    pub(super) fn observe_governance(
        &self,
        retention_decayed: Option<usize>,
        compression_sources: Option<usize>,
        revocation_receipts: usize,
        quarantined_records: usize,
    ) {
        let snapshot = {
            let mut metrics = self
                .observability
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(decayed) = retention_decayed {
                metrics.retention_runs_total += 1;
                metrics.retention_decayed_total += decayed as u64;
            }
            if let Some(sources) = compression_sources {
                metrics.compression_runs_total += 1;
                metrics.compression_source_records_total += sources as u64;
            }
            metrics.revocation_receipts_total += revocation_receipts as u64;
            metrics.quarantine_records_total += quarantined_records as u64;
            metrics.clone()
        };
        self.persist_observability_best_effort(&snapshot);
    }

    fn persist_observability_best_effort(&self, snapshot: &MemoryObservabilitySnapshot) {
        if let Err(error) = self.persist_observability(snapshot) {
            tracing::warn!(%error, "failed to persist Mnemosyne observability snapshot");
        }
    }

    fn persist_observability(&self, snapshot: &MemoryObservabilitySnapshot) -> Result<()> {
        let Some(metrics_root) = &self.metrics_root else {
            return Ok(());
        };
        store::write_atomic_json(
            &metrics_root.join("observability.json"),
            &serde_json::json!({
                "schema_version": "arda.mnemosyne.observability.v1",
                "generated_at_utc": Utc::now().to_rfc3339(),
                "metrics": snapshot,
            }),
        )
    }

    /// Export continuity and observability projections for `arda-aule`.
    pub fn export_runtime_snapshots(&self) -> Result<()> {
        let Some(metrics_root) = &self.metrics_root else {
            return Ok(());
        };
        let stats = self.stats()?;
        let status = serde_json::json!({
            "schema_version": crate::schema::CONTINUITY_SCHEMA_VERSION,
            "ok": true,
            "status": &stats,
        });
        store::write_atomic_json(&metrics_root.join("stats.json"), &stats)?;
        store::write_atomic_json(&metrics_root.join("status.json"), &status)?;
        self.persist_observability(&stats.observability)
    }

    fn apply_adaptive_significance(&self, event: &InformantEvent) -> SignificanceResult {
        let mut out = evaluate_significance(
            &event.content,
            Some(&event.event_type),
            &event.tags,
            event.confidence_hint,
        );
        let records = self.read_episodic_records().unwrap_or_default();
        let now = Utc::now();

        let mut crate_events_24h = 0usize;
        let mut repeated_content_48h = 0usize;
        let mut tag_hits_7d = 0usize;
        let checkpoint_worthy = event.tags.iter().any(|tag| {
            matches!(
                tag.to_ascii_lowercase().as_str(),
                "checkpoint"
                    | "decision"
                    | "boardroom"
                    | "interrupt"
                    | "delegation"
                    | "completion"
                    | "failure"
                    | "routing"
                    | "continuity"
                    | "governance"
            )
        }) || matches!(
            event.event_type.as_str(),
            "boardroom_posted"
                | "interruption_captured"
                | "task_delegated"
                | "task_completed"
                | "task_failed"
                | "routing_failure"
                | "decision_completed"
                | "illuvatar_fanout"
        );
        for rec in records {
            let ts = DateTime::parse_from_rfc3339(&rec.ts_utc)
                .map(|v| v.with_timezone(&Utc))
                .unwrap_or(now - Duration::days(365));
            if rec.source_crate == event.crate_name && ts >= now - Duration::hours(24) {
                crate_events_24h += 1;
            }
            if rec.content == event.content && ts >= now - Duration::hours(48) {
                repeated_content_48h += 1;
            }
            if ts >= now - Duration::days(7)
                && !event.tags.is_empty()
                && rec.tags.iter().any(|tag| {
                    event
                        .tags
                        .iter()
                        .any(|incoming| incoming.eq_ignore_ascii_case(tag))
                })
            {
                tag_hits_7d += 1;
            }
        }

        let overload_penalty = if checkpoint_worthy {
            ((crate_events_24h as f64 - 18.0).max(0.0) / 90.0).min(0.08)
        } else {
            ((crate_events_24h as f64 - 12.0).max(0.0) / 60.0).min(0.18)
        };
        let repetition_penalty = if checkpoint_worthy {
            (repeated_content_48h as f64 * 0.03).min(0.10)
        } else {
            (repeated_content_48h as f64 * 0.05).min(0.20)
        };
        let novelty_bonus = if checkpoint_worthy && tag_hits_7d <= 2 {
            0.12
        } else if !event.tags.is_empty() && tag_hits_7d <= 1 {
            0.08
        } else {
            0.0
        };
        let adjusted = (out.significance + novelty_bonus - overload_penalty - repetition_penalty)
            .clamp(0.0, 1.0);
        let (class, sigil) = classify_significance(adjusted);
        out.significance = adjusted;
        out.class = class.to_owned();
        out.sigil = sigil.to_owned();
        out
    }

    async fn record_work_signal_async(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) -> Result<()> {
        let plutus = PlutusService::from_default_or_workspace_fallback().map_err(|err| {
            ArdaError::Agent {
                agent: "mnemosyne".to_owned(),
                message: format!("plutus work signal init failed: {err}"),
            }
        })?;
        plutus
            .track_work(agent_id, amount, unit, task_id)
            .await
            .map_err(|err| ArdaError::Agent {
                agent: "mnemosyne".to_owned(),
                message: format!("plutus work signal failed: {err}"),
            })?;
        Ok(())
    }

    fn emit_work_signal_background(
        &self,
        agent_id: &str,
        amount: f64,
        unit: JouleWorkUnit,
        task_id: Option<String>,
    ) {
        let service = self.clone();
        let agent_id = agent_id.to_owned();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                if let Err(err) = service
                    .record_work_signal_async(&agent_id, amount, unit, task_id)
                    .await
                {
                    tracing::debug!(error = %err, "MNEMOSYNE plutus work signal failed");
                }
            });
        } else {
            static WORK_SIGNAL_RUNTIME: OnceLock<
                std::result::Result<tokio::runtime::Runtime, String>,
            > = OnceLock::new();
            match WORK_SIGNAL_RUNTIME.get_or_init(|| {
                tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(1)
                    .thread_name("vaire-work-signal")
                    .enable_all()
                    .build()
                    .map_err(|err| err.to_string())
            }) {
                Ok(runtime) => {
                    runtime.spawn(async move {
                        if let Err(err) = service
                            .record_work_signal_async(&agent_id, amount, unit, task_id)
                            .await
                        {
                            tracing::debug!(error = %err, "MNEMOSYNE plutus work signal failed");
                        }
                    });
                }
                Err(err) => tracing::warn!(
                    error = %err,
                    "MNEMOSYNE failed to create fallback runtime for work signal"
                ),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::governed;
    use super::{store::append_jsonl, InformantEvent, MnemosyneService};
    use arda_economics::PlutusService;
    use chrono::Utc;
    use std::fs;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::thread;
    use tempfile::tempdir;

    static ARDA_ROOT_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn arda_root_test_guard() -> MutexGuard<'static, ()> {
        ARDA_ROOT_TEST_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("ARDA_ROOT test lock")
    }

    #[test]
    fn encode_and_recall_recent() {
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let event = InformantEvent {
            informant_id: "prometheus_mneme".to_string(),
            crate_name: "prometheus".to_string(),
            event_type: "decision_made".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "Illuvatar mission ARDA decision".to_string(),
            confidence_hint: Some(0.9),
            tags: vec!["arda".to_string(), "decision".to_string()],
        };
        let encoded = svc.encode(event).expect("encode");
        assert!(encoded.is_some());

        let recent = svc.recall_recent(24, Some("prometheus")).expect("recall");
        assert!(!recent.is_empty());

        let identity = svc.identity_state().expect("identity");
        assert!(identity.core_memory_count + identity.active_memory_count >= 1);
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let plutus = PlutusService::from_home(&plutus_home).expect("plutus");
        let mut total = 0.0;
        for _ in 0..20 {
            total = rt.block_on(plutus.status()).expect("plutus status")["joulework"]["total"]
                .as_f64()
                .unwrap_or(0.0);
            if total > 0.0 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(total > 0.0);
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }

    #[test]
    fn consolidate_and_stats() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");

        for i in 0..3 {
            let event = InformantEvent {
                informant_id: "prometheus_mneme".to_string(),
                crate_name: "prometheus".to_string(),
                event_type: if i % 2 == 0 {
                    "task_completed".to_string()
                } else {
                    "task_delegated".to_string()
                },
                ts_utc: Utc::now().to_rfc3339(),
                content: format!("ARDA VR iteration {i} mission update"),
                confidence_hint: Some(0.8),
                tags: vec!["arda".to_string(), "vr".to_string()],
            };
            let _ = svc.encode(event).expect("encode");
        }

        let report = svc.consolidate(24).expect("consolidate");
        assert!(report.semantic_patterns_written >= 1);

        let stats = svc.stats().expect("stats");
        assert!(stats.memory_counts.active + stats.memory_counts.core >= 1);
        assert!(stats.last_consolidation_utc.is_some());
        assert!(!stats.checkpoint_policy.priority_tags.is_empty());
        assert!(stats.checkpoint_policy.checkpoint_interval_events >= 4);
        assert!(report.promotion_receipts_written >= 1);
        assert!(report.consolidation_depth >= 2);
        assert_eq!(
            stats.observability.promotion_receipts_total,
            report.promotion_receipts_written as u64
        );
    }

    #[test]
    fn sync_obsidian_indexes_notes() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let vault = dir.path().join("human").join(".obsidian");
        fs::create_dir_all(vault.join("notes")).expect("mkdir");
        fs::write(
            vault.join("notes").join("mission.md"),
            "# Mission\nIntegrate ARDA + Hermes + Mnemosyne",
        )
        .expect("write");

        let report = svc.sync_obsidian(&vault, 50).expect("sync");
        assert!(report.notes_indexed >= 1);
        assert!(report.memories_encoded >= 1);
    }

    #[test]
    fn append_jsonl_serializes_concurrent_writers() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("noise.jsonl");
        let mut threads = Vec::new();
        for idx in 0..8usize {
            let path = path.clone();
            threads.push(thread::spawn(move || {
                for seq in 0..25usize {
                    append_jsonl(&path, &serde_json::json!({"idx": idx, "seq": seq}))
                        .expect("append");
                }
            }));
        }
        for handle in threads {
            handle.join().expect("join");
        }
        let content = fs::read_to_string(&path).expect("read");
        assert_eq!(content.lines().count(), 200);
        assert!(content
            .lines()
            .all(|line| serde_json::from_str::<serde_json::Value>(line).is_ok()));
    }

    #[test]
    fn encode_dual_writes_contract_memory_record_when_root_set() {
        // Don't touch ARDA_PLUTUS_HOME — other tests in this
        // file race on it. The dual-write happens before the plutus
        // background call, so a missing plutus just logs and is fine.
        let dir = tempdir().expect("tempdir");
        let contract_root = dir.path().join("contract_memory");
        let svc = MnemosyneService::new(dir.path().join("mnemosyne"))
            .expect("svc")
            .with_contract_memory_root(contract_root.clone());
        let event = InformantEvent {
            informant_id: "prometheus_mneme".to_string(),
            crate_name: "prometheus".to_string(),
            event_type: "decision_made".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "Phase 1 dual-write smoke test".to_string(),
            confidence_hint: Some(0.9),
            tags: vec!["arda".to_string(), "decision".to_string()],
        };
        let encoded = svc.encode(event).expect("encode");
        assert!(encoded.is_some());

        // Exactly one .json record under contract_root/episodic
        let episodic_dir = contract_root.join("episodic");
        let mut count = 0usize;
        for entry in fs::read_dir(&episodic_dir).expect("read contract dir") {
            let entry = entry.expect("entry");
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
                && !entry.file_name().to_string_lossy().starts_with('.')
            {
                count += 1;
                let content = fs::read_to_string(entry.path()).expect("read record");
                let v: serde_json::Value = serde_json::from_str(&content).expect("parse");
                assert_eq!(v["contract_version"], "0.1.0");
                assert_eq!(v["kind"], "episodic");
                assert_eq!(v["agent"], "prometheus");
            }
        }
        assert_eq!(count, 1, "expected one contract record");
    }

    #[test]
    fn encode_without_contract_root_writes_no_contract_record() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path().join("mnemosyne")).expect("svc");
        let event = InformantEvent {
            informant_id: "prometheus_mneme".to_string(),
            crate_name: "prometheus".to_string(),
            event_type: "decision_made".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "no contract root".to_string(),
            confidence_hint: Some(0.9),
            tags: vec!["arda".to_string()],
        };
        svc.encode(event).expect("encode");
        // No contract_memory directory should have appeared.
        assert!(!dir.path().join("contract_memory").exists());
    }

    #[test]
    fn stats_report_malformed_record_counts_and_skip_bad_episodic() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        fs::write(dir.path().join("noise.jsonl"), "{\"ok\":true}\n[\n").expect("noise");
        fs::write(
            dir.path().join("obsidian_index.jsonl"),
            "{\"ok\":true}\ninvalid\n",
        )
        .expect("obsidian");
        fs::write(
            dir.path().join("archive").join("consolidation.jsonl"),
            "{\"ok\":true}\n{\"broken\"\n",
        )
        .expect("archive");
        let month = dir
            .path()
            .join("episodic")
            .join(Utc::now().format("%Y-%m").to_string());
        fs::create_dir_all(&month).expect("month");
        fs::write(
            month.join("mem_bad.jsonl"),
            "{\"sigil\":\"MNEME_ACTIVE\"}\n{\"content\":\n",
        )
        .expect("episodic");

        let stats = svc.stats().expect("stats");
        assert_eq!(stats.malformed_noise_records, 1);
        assert_eq!(stats.malformed_obsidian_records, 1);
        assert_eq!(stats.malformed_archive_records, 1);
        assert_eq!(stats.malformed_episodic_records, 1);
        assert_eq!(svc.recall_recent(24, None).expect("recall").len(), 0);
    }

    #[test]
    fn recall_relevant_prioritizes_query_matches_in_protected_scopes() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");

        let boardroom = InformantEvent {
            informant_id: "prometheus_mneme".to_string(),
            crate_name: "prometheus".to_string(),
            event_type: "decision_completed".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "Boardroom routing decision for ARDA runtime promotion".to_string(),
            confidence_hint: Some(0.92),
            tags: vec!["boardroom".to_string(), "routing".to_string()],
        };
        let generic = InformantEvent {
            informant_id: "hades_mneme".to_string(),
            crate_name: "hades".to_string(),
            event_type: "cleanup_completed".to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: "General maintenance sweep completed without routing changes".to_string(),
            confidence_hint: Some(0.6),
            tags: vec!["maintenance".to_string()],
        };

        let _ = svc.encode(generic).expect("encode generic");
        let _ = svc.encode(boardroom).expect("encode boardroom");

        let relevant = svc
            .recall_relevant("boardroom routing", 24, None, None, 5)
            .expect("recall relevant");

        assert!(!relevant.is_empty());
        assert_eq!(relevant[0].source_crate, "prometheus");
        assert_eq!(relevant[0].memory_scope, "boardroom_council");
        assert!((0.0..=1.0).contains(&relevant[0].confidence));
        assert!((0.0..=1.0).contains(&relevant[0].trust));
        let metrics = svc.observability_snapshot();
        assert_eq!(metrics.recall_requests_total, 1);
        assert!(metrics
            .last_recall_fidelity
            .is_some_and(|value| value > 0.0));
    }

    #[test]
    fn recall_knowledge_seeds_filters_delete_candidates_and_missing_paths() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join("root");
        let registry_path = root.join("core/state/knowledge_triage_registry.jsonl");
        let memory_path = root.join("docs/plans/MEMORY.md");
        fs::create_dir_all(registry_path.parent().expect("registry parent")).expect("mkdir");
        fs::create_dir_all(memory_path.parent().expect("memory parent")).expect("mkdir");
        fs::write(&memory_path, "Imported memory conclusion").expect("memory file");

        let registry = [
            serde_json::json!({
                "schema_version": "arda.knowledge_triage.v1",
                "path": "docs/plans/MEMORY.md",
                "title": "Memory Plan",
                "classification": "memory_seed",
                "canonical_home": "data/mnemosyne",
                "domain": "imported_corpus",
                "authority": "curated_memory",
                "recommended_action": "encode",
                "rationale": "Imported memory",
                "soterion": {
                    "glyph": "◈",
                    "sigil": "MNEMOSYNE",
                    "retention": "encode_or_link"
                },
                "triaged_at_utc": "2026-04-30T00:00:00Z"
            }),
            serde_json::json!({
                "schema_version": "arda.knowledge_triage.v1",
                "path": "human/Keys.md",
                "title": "Keys",
                "classification": "delete_candidate",
                "canonical_home": "archive/contaminated",
                "domain": "credential_hygiene",
                "authority": "revoked_secret",
                "recommended_action": "delete_after_rotation",
                "rationale": "Stale credentials",
                "soterion": {
                    "glyph": "↝",
                    "sigil": "DELETE_CANDIDATE",
                    "retention": "remove_after_rotation"
                },
                "triaged_at_utc": "2026-04-30T00:00:00Z"
            }),
            serde_json::json!({
                "schema_version": "arda.knowledge_triage.v1",
                "path": "docs/plans/MISSING.md",
                "title": "Missing Memory",
                "classification": "memory_seed",
                "canonical_home": "data/mnemosyne",
                "domain": "imported_corpus",
                "authority": "curated_memory",
                "recommended_action": "encode",
                "rationale": "Missing file should not recall",
                "soterion": {
                    "glyph": "◈",
                    "sigil": "MNEMOSYNE",
                    "retention": "encode_or_link"
                },
                "triaged_at_utc": "2026-04-30T00:00:00Z"
            }),
        ]
        .into_iter()
        .map(|entry| entry.to_string())
        .collect::<Vec<_>>()
        .join("\n");
        fs::write(&registry_path, format!("{registry}\n")).expect("registry");

        let _arda_root_guard = arda_root_test_guard();
        let previous_root = std::env::var_os("ARDA_ROOT");
        // SAFETY: guarded by `ARDA_ROOT_TEST_LOCK` so no sibling test in this module
        // mutates ARDA_ROOT while this test reads it.
        unsafe {
            std::env::set_var("ARDA_ROOT", &root);
        }

        let svc = MnemosyneService::new(dir.path().join("mnemosyne")).expect("svc");
        let seeds = svc
            .recall_knowledge_seeds(Some("imported"), 10)
            .expect("knowledge seeds");

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].path, "docs/plans/MEMORY.md");
        assert_eq!(seeds[0].classification, "memory_seed");

        // SAFETY: warden-owned by `arda-vaire` test scaffolding — single-threaded
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
    fn recall_knowledge_seeds_bridges_deep_athena_github_records() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join("root");
        let registry_path = root.join("core/state/knowledge_triage_registry.jsonl");
        let book_path = root.join("data/athena/books/src_github.jsonl");
        fs::create_dir_all(registry_path.parent().expect("registry parent")).expect("mkdir");
        fs::create_dir_all(book_path.parent().expect("book parent")).expect("mkdir");

        let registry = serde_json::json!({
            "schema_version": "arda.knowledge_triage.v1",
            "path": "data/athena/books/src_github.jsonl",
            "title": "https://github.com/example/project",
            "classification": "reference",
            "canonical_home": "data/athena",
            "domain": "athena_ingest",
            "authority": "canonical",
            "recommended_action": "keep as ATHENA reference evidence",
            "rationale": "ATHENA ingest produced reference evidence",
            "soterion": {
                "glyph": "📜",
                "sigil": "SCROLL",
                "retention": "keep"
            },
            "triaged_at_utc": "2026-05-21T00:00:00Z"
        });
        fs::write(&registry_path, format!("{registry}\n")).expect("registry");

        let shallow = serde_json::json!({
            "version": 1,
            "stage": "shallow",
            "data": {
                "title": "https://github.com/example/project",
                "relevance_tags": ["githubrepo"]
            }
        });
        let deep = serde_json::json!({
            "version": 2,
            "stage": "deep",
            "data": {
                "title": "example/project",
                "full_summary": "Deep synthesis for a GitHub repo.",
                "relevance_tags": ["githubrepo", "implementation"],
                "policy_readiness": "implementation_ready",
                "implementation_brief": {
                    "method_summary": "Repo-backed implementation candidate",
                    "source_url": "https://github.com/example/project"
                }
            }
        });
        fs::write(&book_path, format!("{shallow}\n{deep}\n")).expect("book");

        let _arda_root_guard = arda_root_test_guard();
        let previous_root = std::env::var_os("ARDA_ROOT");
        // SAFETY: guarded by `ARDA_ROOT_TEST_LOCK` so no sibling test in this module
        // mutates ARDA_ROOT while this test reads it.
        unsafe {
            std::env::set_var("ARDA_ROOT", &root);
        }

        let svc = MnemosyneService::new(dir.path().join("mnemosyne")).expect("svc");
        let seeds = svc
            .recall_knowledge_seeds(Some("github"), 10)
            .expect("knowledge seeds");

        assert_eq!(seeds.len(), 1);
        assert_eq!(seeds[0].path, "data/athena/books/src_github.jsonl");
        assert_eq!(seeds[0].classification, "memory_seed");
        assert_eq!(seeds[0].authority, "curated_memory");

        // SAFETY: warden-owned by `arda-vaire` test scaffolding — single-threaded
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
    fn sync_obsidian_rejects_missing_vault_path() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let missing = dir.path().join("missing_vault");

        let err = svc.sync_obsidian(&missing, 10).expect_err("missing vault");
        assert!(err.to_string().contains("obsidian path not found"));
    }

    #[test]
    fn encode_honors_explicit_scope_tag() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");

        let encoded = svc
            .encode(InformantEvent {
                informant_id: "athena_mneme".to_string(),
                crate_name: "athena".to_string(),
                event_type: "research_note".to_string(),
                ts_utc: Utc::now().to_rfc3339(),
                content: "Operator note with explicit protected scope".to_string(),
                confidence_hint: Some(0.85),
                tags: vec!["scope:human_context".to_string(), "analysis".to_string()],
            })
            .expect("encode")
            .expect("memory");

        assert_eq!(encoded.memory_scope, "human_context");
    }

    #[test]
    fn status_surfaces_recent_noise_obsidian_and_paths() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let vault = dir.path().join("human").join(".obsidian");
        fs::create_dir_all(vault.join("notes")).expect("mkdir");
        fs::write(vault.join("notes").join("memo.md"), "quick human sync note").expect("write");

        append_jsonl(
            &dir.path().join("noise.jsonl"),
            &serde_json::json!({
                "ts": Utc::now().to_rfc3339(),
                "event": {
                    "crate_name": "manwe"
                },
                "reason": "test noise surface"
            }),
        )
        .expect("noise append");
        let sync = svc.sync_obsidian(&vault, 10).expect("sync");
        assert!(sync.notes_indexed >= 1);

        let status = svc.status().expect("status");
        assert_eq!(status["ok"], true);
        assert!(status["status"]["malformed_noise_records"].is_number());

        let noise = svc.recent_noise_events(5);
        assert_eq!(noise.len(), 1);
        assert_eq!(noise[0]["event"]["crate_name"], "manwe");

        let obsidian = svc.recent_obsidian_entries(5);
        assert_eq!(obsidian.len(), 1);
        assert_eq!(obsidian[0]["sigil"], "SCROLL");

        let paths = svc.paths();
        assert_eq!(paths["root"], dir.path().to_string_lossy().as_ref());
        assert_eq!(
            paths["obsidian_index"],
            dir.path()
                .join("obsidian_index.jsonl")
                .to_string_lossy()
                .as_ref()
        );
    }

    fn make_event(
        crate_name: &str,
        event_type: &str,
        content: &str,
        tags: Vec<&str>,
    ) -> InformantEvent {
        InformantEvent {
            informant_id: format!("{crate_name}_mneme"),
            crate_name: crate_name.to_string(),
            event_type: event_type.to_string(),
            ts_utc: Utc::now().to_rfc3339(),
            content: content.to_string(),
            confidence_hint: Some(0.8),
            tags: tags.into_iter().map(str::to_string).collect(),
        }
    }

    #[test]
    fn consolidate_with_no_recent_events_writes_zero_patterns() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let report = svc.consolidate(24).expect("consolidate");
        assert_eq!(report.episodic_scanned, 0);
        assert_eq!(report.semantic_patterns_written, 0);
        assert_eq!(report.procedural_patterns_written, 0);
        assert_eq!(report.archived_records_written, 1);
        assert_eq!(report.promotion_receipts_written, 0);
        assert_eq!(report.consolidation_depth, 0);
    }

    #[test]
    fn consolidate_skips_single_entry_clusters() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        // Single event under tag "alpha" — should NOT promote (cluster.len() < 2).
        svc.encode(make_event(
            "prometheus",
            "task_completed",
            "alpha-only",
            vec!["alpha"],
        ))
        .expect("encode");
        let report = svc.consolidate(24).expect("consolidate");
        assert_eq!(report.semantic_patterns_written, 0);
    }

    #[test]
    fn consolidate_buckets_untagged_events_under_untagged_tag() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        // Two untagged high-significance events should land in the "untagged" cluster
        // and produce one semantic pattern.
        for i in 0..2 {
            svc.encode(make_event(
                "prometheus",
                "task_completed",
                &format!("ARDA boardroom interrupt mission {i}"),
                vec![],
            ))
            .expect("encode");
        }
        let report = svc.consolidate(24).expect("consolidate");
        assert!(
            report.semantic_patterns_written >= 1,
            "untagged cluster should promote"
        );
    }

    #[test]
    fn consolidate_excludes_non_procedural_event_types() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        // Two events with an event_type that does NOT contain
        // completed/delegated/ingest — should NOT produce a procedural pattern.
        for i in 0..2 {
            svc.encode(make_event(
                "prometheus",
                "noise_observed",
                &format!("noise {i}"),
                vec!["noise"],
            ))
            .expect("encode");
        }
        let report = svc.consolidate(24).expect("consolidate");
        assert_eq!(report.procedural_patterns_written, 0);
    }

    #[test]
    fn sync_obsidian_with_empty_vault_indexes_zero_files() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let vault = dir.path().join("vault");
        fs::create_dir_all(&vault).expect("mkdir vault");
        let report = svc.sync_obsidian(&vault, 50).expect("sync");
        assert_eq!(report.files_scanned, 0);
        assert_eq!(report.notes_indexed, 0);
        assert_eq!(report.memories_encoded, 0);
    }

    #[test]
    fn consolidation_promotions_are_receipt_backed() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        for index in 0..3 {
            svc.encode(make_event(
                "prometheus",
                "task_completed",
                &format!("ARDA governance checkpoint completed {index}"),
                vec!["governance", "checkpoint"],
            ))
            .expect("encode");
        }

        let report = svc.consolidate(24).expect("consolidate");
        assert_eq!(
            report.promotion_receipts_written,
            report.semantic_patterns_written + report.procedural_patterns_written
        );
        assert!(report.promotion_receipts_written >= 2);

        let receipts =
            fs::read_to_string(dir.path().join("archive").join("promotion_receipts.jsonl"))
                .expect("promotion receipts");
        let parsed = receipts
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("receipt json"))
            .collect::<Vec<_>>();
        assert_eq!(parsed.len(), report.promotion_receipts_written);
        assert!(parsed.iter().all(|receipt| {
            receipt["receipt_id"]
                .as_str()
                .is_some_and(|id| id.starts_with("promotion_"))
                && receipt["source_memory_ids"]
                    .as_array()
                    .is_some_and(|ids| ids.len() >= 2)
        }));
    }

    #[test]
    fn repeated_raw_warden_observations_are_blocked_from_promotion() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        for index in 0..4 {
            svc.encode(make_event(
                "warden",
                "external_observation",
                &format!("Repeated raw Warden observation {index}"),
                vec!["raw_warden_observation", "governance"],
            ))
            .expect("encode");
        }

        let report = svc.consolidate(24).expect("consolidate");
        assert_eq!(report.observed_count, 4);
        assert_eq!(report.eligible_count, 0);
        assert_eq!(report.promoted_count, 0);
        assert_eq!(report.blocked_count, 4);
        assert_eq!(report.promotion_receipts_written, 0);
    }

    #[test]
    fn governed_derived_receipts_preserve_approval_references() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        for index in 0..2 {
            svc.ingest_approved_delta(governed::ApprovedKnowledgeDelta {
                delta_id: format!("delta-{index}"),
                source_reference: format!("https://example.com/{index}"),
                warden_observation_id: format!("obs-{index}"),
                varda_evaluation_id: format!("eval-{index}"),
                approval_reference: format!("approval-{index}"),
                content: format!("Approved governed knowledge {index}"),
                correction_of: None,
            })
            .expect("approved intake");
        }

        let report = svc.consolidate(24).expect("consolidate");
        assert!(report.promoted_count > 0);
        let receipts =
            fs::read_to_string(dir.path().join("archive").join("promotion_receipts.jsonl"))
                .expect("promotion receipts");
        for line in receipts.lines() {
            let value: serde_json::Value = serde_json::from_str(line).expect("receipt json");
            assert!(value["approval_references"]
                .as_array()
                .is_some_and(|references| !references.is_empty()));
        }
    }

    #[test]
    fn adaptive_significance_handles_duplicates_overload_and_novel_checkpoints() {
        let dir = tempdir().expect("tempdir");
        let svc = MnemosyneService::new(dir.path()).expect("svc");
        let duplicate = make_event(
            "athena",
            "research_note",
            "Repeated implementation observation with enough detail for durable comparison",
            vec!["implementation"],
        );
        let fresh_score = svc.apply_adaptive_significance(&duplicate).significance;
        for _ in 0..4 {
            svc.encode(duplicate.clone()).expect("duplicate encode");
        }
        let duplicate_score = svc.apply_adaptive_significance(&duplicate).significance;
        assert!(duplicate_score < fresh_score);

        let ordinary = make_event(
            "athena",
            "research_note",
            "Ordinary corpus observation with sufficient detail for significance evaluation",
            vec!["corpus"],
        );
        let clean_dir = tempdir().expect("clean tempdir");
        let clean = MnemosyneService::new(clean_dir.path()).expect("clean svc");
        let baseline_score = clean.apply_adaptive_significance(&ordinary).significance;
        for index in 0..20 {
            svc.encode(make_event(
                "athena",
                "research_note",
                &format!(
                    "Distinct overload corpus observation number {index} with detailed context"
                ),
                vec!["load"],
            ))
            .expect("overload encode");
        }
        let overloaded_score = svc.apply_adaptive_significance(&ordinary).significance;
        assert!(overloaded_score < baseline_score);

        let novel_checkpoint = make_event(
            "athena",
            "decision_completed",
            "Novel governance decision completed because receipt evidence was verified",
            vec!["decision", "novel-receipt"],
        );
        let checkpoint = svc.apply_adaptive_significance(&novel_checkpoint);
        assert_ne!(checkpoint.class, "noise");
        assert!(checkpoint.significance > overloaded_score);
    }
}
