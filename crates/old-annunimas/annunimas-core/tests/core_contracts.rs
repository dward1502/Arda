use annunimas_core::agent::Agent;
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::AnnunimasError;
use annunimas_core::ledger::Ledger;
use annunimas_core::pipeline::Pipeline;
use annunimas_core::router::Router;
use annunimas_core::soterion::{
    load_default_soterion_registry, machine_sigil_from_registry, parse_header_from_content,
    SoterionMeta,
};
use annunimas_core::task::{JouleWorkMeasurementSource, Task, TaskStatus};
use async_trait::async_trait;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!("annunimas-core-{label}-{}", Uuid::new_v4()))
}

struct StubAgent {
    name: &'static str,
    capabilities: &'static [&'static str],
    start_execution: bool,
    result: serde_json::Value,
}

#[async_trait]
impl Agent for StubAgent {
    fn name(&self) -> &str {
        self.name
    }

    fn capabilities(&self) -> &[&str] {
        self.capabilities
    }

    async fn execute(&self, task: &mut Task) -> annunimas_core::Result<()> {
        if self.start_execution {
            task.start_execution();
        }
        task.complete(self.result.clone());
        Ok(())
    }
}

#[test]
fn task_lifecycle_helpers_track_assignment_and_completion() {
    let mut task = Task::new("verify lifecycle", "audit");

    assert!(matches!(task.status, TaskStatus::Pending));
    assert!(task.assigned_agent.is_none());

    task.assign("athena");
    assert!(matches!(task.status, TaskStatus::Running));
    assert_eq!(task.assigned_agent.as_deref(), Some("athena"));
    assert!(task.planning_started_at.is_some());

    task.start_execution();
    assert!(task.execution_started_at.is_some());

    let result = json!({"ok": true});
    task.complete(result.clone());

    assert!(matches!(task.status, TaskStatus::Complete));
    assert_eq!(task.result, Some(result));
    assert!(task.execution_duration_secs() >= 0.0);
    assert!(task.calculate_resonance() >= 0.0);
}

#[test]
fn task_joulework_measurement_metadata_defaults_are_legacy_compatible() {
    let legacy_payload = json!({
        "id": Uuid::new_v4(),
        "description": "legacy task without measurement metadata",
        "task_type": "audit",
        "status": "pending",
        "created_at": chrono::Utc::now(),
        "updated_at": chrono::Utc::now(),
        "assigned_agent": null,
        "result": null,
        "planning_started_at": null,
        "execution_started_at": null,
        "joule_cost_estimated": 1.0,
        "joule_cost_actual": 0.8,
        "clarifications_requested": 0,
        "clarifications_resolved": 0
    });

    let task: Task = serde_json::from_value(legacy_payload).expect("legacy task decode");
    assert_eq!(
        task.joulework_measurement_source,
        JouleWorkMeasurementSource::DefaultFallback
    );
    assert_eq!(task.joulework_measurement_confidence, 0.0);
    assert!(!task.joulework_measurement_source.is_observed());
    assert!(!task.joulework_measurement_source.is_autonomy_truth());
}

#[test]
fn task_joulework_measurement_source_serializes_as_snake_case_contract() {
    let mut task = Task::new("observed provider usage", "route");
    task.joulework_measurement_source = JouleWorkMeasurementSource::ProviderUsageReport;
    task.joulework_measurement_confidence = 0.82;

    let value = serde_json::to_value(task).expect("task json");
    assert_eq!(
        value["joulework_measurement_source"],
        "provider_usage_report"
    );
    assert_eq!(value["joulework_measurement_confidence"], 0.82);
}

#[test]
fn response_envelope_success_and_failure_preserve_contract() {
    let success = ResponseEnvelope::success(json!({"route": "athena"}))
        .into_result("athena")
        .expect("success result");
    assert_eq!(success["route"], "athena");

    let err = ResponseEnvelope::failure("transport unavailable")
        .into_result("charon")
        .expect_err("failure result");
    match err {
        AnnunimasError::Agent { agent, message } => {
            assert_eq!(agent, "charon");
            assert_eq!(message, "transport unavailable");
        }
        other => panic!("unexpected error variant: {other}"),
    }

    let default_payload = CommandEnvelope::new("status", serde_json::Value::Null);
    let encoded = serde_json::to_string(&default_payload).expect("command encode");
    let decoded: CommandEnvelope = serde_json::from_str(&encoded).expect("command decode");
    assert_eq!(decoded.cmd, "status");
    assert_eq!(
        decoded.schema_version,
        annunimas_core::daemon::DAEMON_SCHEMA_VERSION
    );
    assert!(decoded.payload.is_null());
}

#[test]
fn ledger_append_injects_soterion_metadata() {
    let dir = temp_path("ledger");
    let ledger = Ledger::new(&dir).expect("ledger");
    ledger
        .append(&json!({"event":"task_received","source":"ceo"}))
        .expect("append");

    let contents = fs::read_to_string(ledger.path()).expect("read ledger");
    let line = contents.lines().next().expect("ledger line");
    let value: serde_json::Value = serde_json::from_str(line).expect("json line");

    assert_eq!(value["event"], "task_received");
    assert_eq!(value["soterion"]["sigil"], "𓆣");
    assert_eq!(value["soterion"]["realm"], "ledger");
    assert!(value["soterion"]["timestamp"].as_str().is_some());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn soterion_header_parser_extracts_expected_fields() {
    let content = r#"---
soterion:
  sigil: "𓁿"
  realm: "knowledge"
  tags: ["athena", "book"]
  resonance: 0.88
  triad_gate: "pass"
  jw_cost: 3.5
  clearance: "council"
---
body
"#;

    let meta = parse_header_from_content(content)
        .expect("parse")
        .expect("header present");

    assert_eq!(meta.sigil.as_deref(), Some("𓁿"));
    assert_eq!(meta.realm.as_deref(), Some("knowledge"));
    assert_eq!(meta.tags, vec!["athena".to_string(), "book".to_string()]);
    assert_eq!(meta.triad_gate.as_deref(), Some("pass"));
    assert_eq!(meta.clearance.as_deref(), Some("council"));
    assert_eq!(meta.joule_cost, Some(3.5));
}

#[test]
fn router_returns_no_route_for_unsupported_tasks() {
    let mut router = Router::new();
    router.register(Box::new(StubAgent {
        name: "athena",
        capabilities: &["ingest"],
        start_execution: false,
        result: json!({"ok": true}),
    }));

    let task = Task::new("route this", "deep_reason");
    let err = match router.route(&task) {
        Ok(agent) => panic!("unexpected route to {}", agent.name()),
        Err(err) => err,
    };
    match err {
        AnnunimasError::NoRoute(task_type) => assert_eq!(task_type, "deep_reason"),
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn pipeline_submit_assigns_executes_and_records_audit_trail() {
    let dir = temp_path("pipeline");
    let mut router = Router::new();
    router.register(Box::new(StubAgent {
        name: "athena",
        capabilities: &["ingest"],
        start_execution: true,
        result: json!({"summary": "stored"}),
    }));
    let ledger = Ledger::new(&dir).expect("ledger");
    let pipeline = Pipeline::new(router, ledger, 100);

    let task = Task::new("ingest example", "ingest");
    let completed = pipeline.submit(task).await.expect("pipeline submit");

    assert!(matches!(completed.status, TaskStatus::Complete));
    assert_eq!(completed.assigned_agent.as_deref(), Some("athena"));
    assert_eq!(completed.result, Some(json!({"summary": "stored"})));

    let ledger_path = fs::read_dir(&dir)
        .expect("read dir")
        .find_map(|entry| {
            let path = entry.ok()?.path();
            path.extension()
                .is_some_and(|ext| ext == "jsonl")
                .then_some(path)
        })
        .expect("ledger path");
    let contents = fs::read_to_string(ledger_path).expect("read ledger");
    let rows: Vec<serde_json::Value> = contents
        .lines()
        .map(|line| serde_json::from_str(line).expect("json row"))
        .collect();

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0]["schema_version"], "annunimas.message.v1");
    assert_eq!(rows[0]["payload"]["event_type"], "task_received");
    assert_eq!(rows[1]["payload"]["type"], "task_assignment");
    assert_eq!(rows[2]["payload"]["type"], "task_complete");
    assert!(rows.iter().all(|row| row["soterion"]["sigil"] == "𓆣"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn soterion_meta_default_is_empty_but_extensible() {
    let meta = SoterionMeta::default();
    assert!(meta.sigil.is_none());
    assert!(meta.realm.is_none());
    assert!(meta.tags.is_empty());

    let shared = Arc::new(Mutex::new(meta));
    assert!(shared.lock().expect("lock").extra.is_empty());
}

#[test]
fn default_soterion_registry_loads_machine_and_render_contracts() {
    let registry = load_default_soterion_registry().expect("registry");
    assert_eq!(registry.version.as_deref(), Some("0.1.0"));
    assert!(registry
        .machine_sigils
        .contains_key("SG_HERMES_DELIVERY_OK"));
    assert_eq!(
        registry
            .agent_identity
            .get("HERMES")
            .and_then(|entry| entry.glyph.as_deref()),
        Some("🜁")
    );
    assert_eq!(
        registry
            .machine_sigils
            .get("SG_HADES_QUARANTINE")
            .and_then(|entry| entry.retention.as_deref()),
        Some("quarantine")
    );
}

#[test]
fn machine_sigil_lookup_uses_registry_defaults() {
    let sigil = machine_sigil_from_registry("SG_HERMES_DELIVERY_OK").expect("sigil");
    assert_eq!(sigil.sigil_source.as_deref(), Some("hermes"));
    assert_eq!(sigil.sigil_retention.as_deref(), Some("summarize"));
    assert_eq!(sigil.sigil_render.as_deref(), Some("🜁◆◀"));
}
