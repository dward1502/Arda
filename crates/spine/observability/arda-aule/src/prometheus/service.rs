#![cfg(feature = "full-cli")]
// sigil: REPAIR
mod drift;
mod execution_intents;
mod runtime;
mod status;
mod support;

use crate::ceo::CoreAutonomyProfile;
use crate::prometheus::heartbeat::HeartbeatState;
use crate::prometheus::orders::OrderStore;
use crate::prometheus::registry::AgentRosterSnapshot;
use crate::prometheus::thought::ThoughtLedger;
use arda_core::error::Result;
use arda_vaire::MnemosyneService;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusStatus {
    pub heartbeat_mode: String,
    pub heartbeat_interval_ms: u64,
    pub confidence_threshold: f64,
    pub agents_online: usize,
    pub agents_silent: usize,
    pub active_orders: usize,
    pub pending_escalations: usize,
    pub thought_count_today: usize,
    pub resource_state: String,
    pub retinue_game_theory_score: f64,
    pub continuity_events_48h: usize,
    pub identity_focus: Option<String>,
    pub context_engineering: ContextEngineeringPolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triad_philosopher: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub triad_philosopher_evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEngineeringPolicy {
    pub context_budget_chars: usize,
    pub compaction_target_ratio: f64,
    pub reminder_interval_messages: usize,
    pub required_sections: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PrometheusService {
    core_root: PathBuf,
    profile: Option<CoreAutonomyProfile>,
    heartbeat: HeartbeatState,
    roster: Option<AgentRosterSnapshot>,
    thought_ledger: ThoughtLedger,
    order_store: OrderStore,
    council_events_path: PathBuf,
    execution_intents_path: PathBuf,
    execution_intents_recovery_path: PathBuf,
    confidence_threshold: f64,
    mnemosyne: Option<MnemosyneService>,
}

impl PrometheusService {}

fn prometheus_home() -> PathBuf {
    support::prometheus_home()
}

fn append_jsonl(path: &Path, value: &serde_json::Value) -> Result<()> {
    support::append_jsonl(path, value)
}

fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<serde_json::Value> {
    support::read_recent_jsonl(path, limit)
}

fn sha256_file_if_exists(path: &Path) -> Result<String> {
    support::sha256_file_if_exists(path)
}

fn queue_contains_task(path: &Path, task_id: &str) -> Result<bool> {
    support::queue_contains_task(path, task_id)
}

#[cfg(test)]
mod tests {
    use super::PrometheusService;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::tempdir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn interrupt_reroute_creates_execution_intents() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let out = service
            .interrupt_reroute(serde_json::json!({
                "event_id": "int_demo_1",
                "source": "voice",
                "sender": "operator",
                "content": "switch to remediation queue",
                "triad_passed": true,
                "triad_score": 0.81,
                "policy_safe": true,
                "requires_operator_review": false,
                "context": {
                    "task_ids": ["task_alpha"]
                }
            }))
            .expect("interrupt_reroute");
        assert_eq!(out.get("queued").and_then(|v| v.as_u64()), Some(1));

        let intents_path = prometheus_home.join("execution_intents.jsonl");
        let content = fs::read_to_string(intents_path).expect("intents read");
        assert!(content.contains("\"target_task_id\":\"task_alpha\""));
        assert!(content.contains("\"action\":\"create\""));
    }

    #[test]
    fn interrupt_reroute_is_idempotent_on_replay() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let payload = serde_json::json!({
            "event_id": "int_idem_1",
            "source": "voice",
            "sender": "operator",
            "content": "switch to remediation queue",
            "triad_passed": true,
            "triad_score": 0.81,
            "policy_safe": true,
            "requires_operator_review": false,
            "context": {
                "task_ids": ["task_alpha"]
            }
        });
        let first = service
            .interrupt_reroute(payload.clone())
            .expect("first reroute");
        assert_eq!(first.get("queued").and_then(|v| v.as_u64()), Some(1));
        assert_eq!(
            first.get("duplicates_ignored").and_then(|v| v.as_u64()),
            Some(0)
        );

        let second = service.interrupt_reroute(payload).expect("second reroute");
        assert_eq!(second.get("queued").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(
            second.get("duplicates_ignored").and_then(|v| v.as_u64()),
            Some(1)
        );

        let intents_path = prometheus_home.join("execution_intents.jsonl");
        let lines = fs::read_to_string(intents_path)
            .expect("intents read")
            .lines()
            .count();
        assert_eq!(lines, 2);
    }

    #[test]
    fn execution_intent_transition_enforces_lifecycle() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let created = service
            .interrupt_reroute(serde_json::json!({
                "event_id":"int_state_1",
                "source":"voice",
                "sender":"operator",
                "content":"reroute queue",
                "triad_passed":true,
                "triad_score":0.8,
                "policy_safe":true,
                "requires_operator_review":false,
                "context":{"task_ids":["task_alpha"]}
            }))
            .expect("create");
        let intent_id = created["intents"][0]["intent_id"]
            .as_str()
            .expect("intent id")
            .to_string();

        let assigned = service
            .transition_execution_intent(&intent_id, "assigned", Some("worker accepted"))
            .expect("assigned");
        assert_eq!(
            assigned.get("status").and_then(|v| v.as_str()),
            Some("assigned")
        );

        let invalid = service.transition_execution_intent(&intent_id, "queued", None);
        assert!(invalid.is_err());
    }

    #[test]
    fn compact_execution_intents_prunes_terminal_and_limits_size() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let intents_path = prometheus_home.join("execution_intents.jsonl");
        let old = (chrono::Utc::now() - chrono::Duration::days(30)).to_rfc3339();
        fs::write(
            &intents_path,
            format!(
                "{{\"intent_id\":\"i1\",\"status\":\"queued\",\"ts_utc\":\"{}\"}}\n{{\"intent_id\":\"i2\",\"status\":\"expired\",\"ts_utc\":\"{}\"}}\n{{\"intent_id\":\"i3\",\"status\":\"superseded\",\"ts_utc\":\"{}\"}}\n",
                chrono::Utc::now().to_rfc3339(),
                old,
                chrono::Utc::now().to_rfc3339()
            ),
        )
        .expect("seed intents");

        let compacted = service.compact_execution_intents(14, 2).expect("compact");
        assert_eq!(compacted.get("kept").and_then(|v| v.as_u64()), Some(2));
        let lines = fs::read_to_string(&intents_path)
            .expect("read intents")
            .lines()
            .count();
        assert_eq!(lines, 2);
    }

    #[test]
    fn startup_recovery_summary_is_written_and_readable() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let recovery = service.execution_intents_recovery().expect("recovery");
        assert_eq!(
            recovery.get("replay_safe").and_then(|v| v.as_bool()),
            Some(true)
        );
        let path = prometheus_home.join("execution_intents_recovery_last.json");
        assert!(path.exists());
    }

    #[test]
    fn status_reports_context_engineering_policy() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let status = service.status().expect("status");
        assert!(status.context_engineering.context_budget_chars >= 14_000);
        assert!(status.context_engineering.compaction_target_ratio > 0.0);
        assert!(status
            .context_engineering
            .required_sections
            .iter()
            .any(|section| section == "verification_targets"));
    }

    #[test]
    fn status_surfaces_compact_triad_philosopher_evidence() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::create_dir_all(core_root.join("metrics/by_crate/governance")).expect("metrics mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");
        fs::write(
            core_root.join("metrics/by_crate/governance/signals.json"),
            r#"{
                "goal": {
                    "triad_philosopher": {"action":"hold","alignment_score":0.42},
                    "triad_philosopher_evidence": [
                        "triad_philosopher:hold:0.42",
                        "non_triad:evidence"
                    ]
                }
            }"#,
        )
        .expect("signals write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        let status = service.status().expect("status");
        let status_json = serde_json::to_value(status).expect("status json");

        assert_eq!(
            status_json
                .get("triad_philosopher")
                .and_then(|verdict| verdict.get("action"))
                .and_then(|action| action.as_str()),
            Some("hold")
        );
        assert_eq!(
            status_json.get("triad_philosopher_evidence"),
            Some(&serde_json::json!(["triad_philosopher:hold:0.42"]))
        );
    }

    #[test]
    fn support_helpers_read_hash_and_scan_jsonl() {
        let dir = tempdir().expect("tempdir");
        let jsonl = dir.path().join("events.jsonl");
        fs::write(
            &jsonl,
            "\nnot-json\n{\"id\":\"old\"}\n{\"id\":\"target\"}\n{\"id\":\"new\"}\n",
        )
        .expect("jsonl write");

        let recent = super::read_recent_jsonl(&jsonl, 2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].get("id").and_then(|v| v.as_str()), Some("target"));
        assert_eq!(recent[1].get("id").and_then(|v| v.as_str()), Some("new"));
        assert!(super::read_recent_jsonl(&dir.path().join("missing.jsonl"), 5).is_empty());

        let file = dir.path().join("hash.txt");
        fs::write(&file, "arda").expect("hash file");
        let hash = super::sha256_file_if_exists(&file).expect("hash");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(
            super::sha256_file_if_exists(&dir.path().join("absent.txt")).expect("missing hash"),
            "missing"
        );
    }

    #[test]
    fn support_queue_contains_task_skips_bad_lines() {
        let dir = tempdir().expect("tempdir");
        let queue = dir.path().join("queue.jsonl");
        fs::write(
            &queue,
            "bad-json\n{\"id\":\"tsk_one\"}\n{\"id\":\"tsk_two\",\"status\":\"queued\"}\n",
        )
        .expect("queue write");

        assert!(super::queue_contains_task(&queue, "tsk_two").expect("contains"));
        assert!(!super::queue_contains_task(&queue, "tsk_missing").expect("missing"));
        assert!(
            !super::queue_contains_task(&dir.path().join("missing_queue.jsonl"), "tsk_two")
                .expect("missing queue")
        );
    }

    #[test]
    fn drift_detect_creates_baseline_and_latest_report() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let dir = tempdir().expect("tempdir");
        let core_root = dir.path().join("core");
        fs::create_dir_all(core_root.join("state")).expect("state mkdir");
        fs::write(core_root.join("state/world.json"), "{\"agents\":[]}").expect("world write");

        let prometheus_home = dir.path().join("prometheus_home");
        let minds_home = dir.path().join("minds");
        std::env::set_var("ARDA_PROMETHEUS_HOME", &prometheus_home);
        std::env::set_var("ARDA_PROMETHEUS_MINDS", &minds_home);

        let service = PrometheusService::from_core(&core_root).expect("service");
        assert!(service.latest_drift_report().is_none());

        let first = service.drift_detect_reconcile(false).expect("first drift");
        assert_eq!(
            first.get("baseline_created").and_then(|v| v.as_bool()),
            Some(true)
        );
        assert_eq!(first.get("drift_count").and_then(|v| v.as_u64()), Some(0));
        assert!(prometheus_home.join("drift_baseline.json").exists());

        let second = service.drift_detect_reconcile(false).expect("second drift");
        assert_eq!(
            second.get("baseline_created").and_then(|v| v.as_bool()),
            Some(false)
        );
        let latest = service.latest_drift_report().expect("latest drift");
        assert_eq!(
            latest.get("baseline_created").and_then(|v| v.as_bool()),
            Some(false)
        );

        let history =
            fs::read_to_string(prometheus_home.join("drift_reports.jsonl")).expect("drift history");
        assert_eq!(history.lines().count(), 2);
    }
}
