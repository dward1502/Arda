//! Source-cited morning and context-transition briefs.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use chrono::Utc;
use serde_json::{json, Value};

use super::HarnessState;
use crate::personal_ops::PersonalOpsLogStore;

pub(super) async fn get_morning_brief(State(state): State<HarnessState>) -> impl IntoResponse {
    build_context_brief(&state, "morning")
}

pub(super) async fn get_transition_brief(State(state): State<HarnessState>) -> impl IntoResponse {
    build_context_brief(&state, "transition")
}

fn build_context_brief(state: &HarnessState, kind: &str) -> axum::response::Response {
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    let events = match store.load_all() {
        Ok(events) => events,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": format!("failed to load event log: {error}") })),
            )
                .into_response();
        }
    };
    let now = Utc::now();
    let projection = crate::personal_ops::build_projection(&events, now, now.naive_local().date());
    let mut source_records = personal_event_sources(&events);
    let mut uncertainty = vec![
        "Schedule and inbox sections include only operator-authored personal event records."
            .to_string(),
        "Run receipts include only durable workbench journal entries with receipt digests."
            .to_string(),
        "Connected projects include only explicitly attached workbench project contracts."
            .to_string(),
    ];
    let (projects, project_sources) =
        connected_project_sources(&state.workbench_root, &mut uncertainty);
    let (receipts, receipt_sources) =
        recent_run_receipt_sources(&state.workbench_root, &mut uncertainty);
    source_records.extend(project_sources);
    source_records.extend(receipt_sources);

    (
        StatusCode::OK,
        Json(json!({
            "schema_version": "arda.harness.personal-brief.v1",
            "brief": {
                "kind": kind,
                "generated_at": now.to_rfc3339(),
                "operator_authored_schedule": projection.today,
                "unresolved_captures": projection.inbox,
                "waiting": projection.waiting,
                "recent_run_receipts": receipts,
                "explicitly_connected_projects": projects,
                "source_records": source_records,
                "uncertainty": uncertainty,
                "uncertainty_disclosure": "This brief cites explicit local records only; missing or stale sources are reported rather than inferred."
            }
        })),
    )
        .into_response()
}

pub(super) fn personal_event_sources(
    events: &[arda_core::personal_ops::PersonalOpsEnvelope<
        arda_core::personal_ops::PersonalOpsRecord,
    >],
) -> Vec<Value> {
    events
        .iter()
        .map(|event| {
            let event_id = event.record.event_id().to_string();
            json!({
                "source_record_id": format!("personal-event:{event_id}"),
                "source_type": "personal_event",
                "record_id": event_id,
                "recorded_at": event.record.occurred_at().to_rfc3339(),
                "source_path": "data/personal/events.jsonl"
            })
        })
        .collect()
}

fn connected_project_sources(
    root: &std::path::Path,
    uncertainty: &mut Vec<String>,
) -> (Vec<Value>, Vec<Value>) {
    let path = root.join("data/workbench/projects.json");
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            uncertainty.push("No attached-project registry was present.".to_string());
            return (Vec::new(), Vec::new());
        }
        Err(_) => {
            uncertainty.push("The attached-project registry could not be read.".to_string());
            return (Vec::new(), Vec::new());
        }
    };
    let registry: Value = match serde_json::from_str(&raw) {
        Ok(registry) => registry,
        Err(_) => {
            uncertainty.push("The attached-project registry was malformed.".to_string());
            return (Vec::new(), Vec::new());
        }
    };
    let mut projects = Vec::new();
    let mut sources = Vec::new();
    for project in registry
        .get("projects")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(project_id) = project
            .pointer("/contract/identity/project_id")
            .and_then(Value::as_str)
        else {
            uncertainty.push("An attached project omitted its project identity.".to_string());
            continue;
        };
        let source_record_id = format!("attached-project:{project_id}");
        projects.push(json!({
            "project_id": project_id,
            "source_record_id": source_record_id
        }));
        sources.push(json!({
            "source_record_id": source_record_id,
            "source_type": "attached_project",
            "record_id": project_id,
            "source_path": "data/workbench/projects.json"
        }));
    }
    (projects, sources)
}

fn recent_run_receipt_sources(
    root: &std::path::Path,
    uncertainty: &mut Vec<String>,
) -> (Vec<Value>, Vec<Value>) {
    let entries = match std::fs::read_dir(root.join("data/runs")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            uncertainty.push("No durable run journal directory was present.".to_string());
            return (Vec::new(), Vec::new());
        }
        Err(_) => {
            uncertainty.push("Durable run journals could not be enumerated.".to_string());
            return (Vec::new(), Vec::new());
        }
    };
    let mut receipts = Vec::new();
    for entry in entries.flatten() {
        let run_id = entry.file_name().to_string_lossy().into_owned();
        let Ok(raw) = std::fs::read_to_string(entry.path().join("events.jsonl")) else {
            continue;
        };
        for line in raw.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(event) = serde_json::from_str::<Value>(line) else {
                uncertainty.push(format!(
                    "Run {run_id} contained a malformed journal record."
                ));
                continue;
            };
            let Some(receipt_digest) = event.get("receipt_digest").and_then(Value::as_str) else {
                continue;
            };
            let sequence = event.get("sequence").and_then(Value::as_u64).unwrap_or(0);
            receipts.push(json!({
                "run_id": run_id,
                "sequence": sequence,
                "event_kind": event.get("kind").cloned().unwrap_or(Value::Null),
                "receipt_digest": receipt_digest,
                "recorded_at_unix_ms": event.get("recorded_at_unix_ms").cloned().unwrap_or(Value::Null),
                "source_record_id": format!("run-receipt:{run_id}:{sequence}")
            }));
        }
    }
    receipts.sort_by_key(|receipt| {
        std::cmp::Reverse(
            receipt
                .get("recorded_at_unix_ms")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        )
    });
    receipts.truncate(10);
    let sources = receipts
        .iter()
        .map(|receipt| {
            let run_id = receipt
                .get("run_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            let sequence = receipt.get("sequence").and_then(Value::as_u64).unwrap_or(0);
            json!({
                "source_record_id": receipt["source_record_id"],
                "source_type": "run_receipt",
                "record_id": format!("{run_id}:{sequence}"),
                "source_path": format!("data/runs/{run_id}/events.jsonl")
            })
        })
        .collect();
    (receipts, sources)
}
