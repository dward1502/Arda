#![cfg(feature = "full-cli")]
// sigil: REPAIR
//! Agent-to-Human (A2H) message writer using `arda-orome` comms.
//! Appends JSON-encoded `A2HMessage` records to `data/comm/a2h.jsonl`
//! when the autopilot needs human approval (Oracle escalation, budget
//! exhaustion, etc.).

use super::decomposer::Objective;
use super::oracle_gate::GateDecision;
use arda_orome::{A2HMessage, Priority as CommPriority, ResponseAction};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::Path;
use uuid::Uuid;

pub fn write_message(path: impl AsRef<Path>, msg: &A2HMessage) -> std::io::Result<()> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)?;
    let line = serde_json::to_string(msg).map_err(std::io::Error::other)?;
    writeln!(f, "{}", line)
}

pub fn authorize_for_escalation(objective: &Objective, decision: &GateDecision) -> A2HMessage {
    authorize_for_escalation_with_id(Uuid::new_v4(), objective, decision)
}

pub fn authorize_for_escalation_with_id(
    request_id: Uuid,
    objective: &Objective,
    decision: &GateDecision,
) -> A2HMessage {
    let (reason, urgency) = match decision {
        GateDecision::Rejected { concerns, .. } => (
            format!("Oracle rejected plan. Concerns: {}", concerns.join("; ")),
            CommPriority::Critical,
        ),
        GateDecision::Conditional { concerns, .. } => (
            format!("Oracle approved with conditions: {}", concerns.join("; ")),
            CommPriority::High,
        ),
        _ => (
            "Plan requires human authorization".into(),
            CommPriority::Normal,
        ),
    };
    A2HMessage::Authorize {
        task_id: request_id,
        description: objective.statement.clone(),
        reason,
        urgency,
        deadline: objective
            .deadline
            .as_deref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|t| t.with_timezone(&Utc)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingAuthorization {
    pub request_id: Uuid,
    pub objective: Objective,
    pub gate: GateDecision,
    pub status: PendingAuthorizationStatus,
    pub created_at_utc: String,
    #[serde(default)]
    pub responded_at_utc: Option<String>,
    #[serde(default)]
    pub approved: Option<bool>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PendingAuthorizationStatus {
    Pending,
    Resumed,
    Denied,
}

#[derive(Debug, Clone)]
pub struct HumanApprovedObjective {
    pub request_id: Uuid,
    pub objective: Objective,
    pub conditions: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct H2AProcessReport {
    pub responses_processed: usize,
    pub objectives_resumed: usize,
    pub denials_recorded: usize,
}

#[derive(Debug, Clone)]
struct ApprovalResponse {
    request_id: Uuid,
    approved: bool,
    reason: Option<String>,
    conditions: Vec<String>,
}

pub fn append_pending_authorization(
    path: impl AsRef<Path>,
    request_id: Uuid,
    objective: &Objective,
    gate: &GateDecision,
) -> std::io::Result<()> {
    let record = PendingAuthorization {
        request_id,
        objective: objective.clone(),
        gate: gate.clone(),
        status: PendingAuthorizationStatus::Pending,
        created_at_utc: Utc::now().to_rfc3339(),
        responded_at_utc: None,
        approved: None,
        reason: None,
        conditions: Vec::new(),
    };
    append_jsonl(path, &record)
}

pub fn process_h2a_responses(
    pending_path: impl AsRef<Path>,
    h2a_path: impl AsRef<Path>,
) -> std::io::Result<(Vec<HumanApprovedObjective>, H2AProcessReport)> {
    let pending_path = pending_path.as_ref();
    let h2a_path = h2a_path.as_ref();
    let mut latest = load_pending_authorizations(pending_path)?;
    let mut consumed = latest
        .values()
        .filter(|row| {
            matches!(
                row.status,
                PendingAuthorizationStatus::Resumed | PendingAuthorizationStatus::Denied
            )
        })
        .map(|row| row.request_id)
        .collect::<BTreeSet<_>>();
    let mut approved = Vec::new();
    let mut report = H2AProcessReport::default();

    let Ok(content) = std::fs::read_to_string(h2a_path) else {
        return Ok((approved, report));
    };
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Some(response) = parse_approval_response(line) else {
            continue;
        };
        if consumed.contains(&response.request_id) {
            continue;
        }
        let Some(current) = latest.get(&response.request_id).cloned() else {
            continue;
        };
        if !matches!(current.status, PendingAuthorizationStatus::Pending) {
            consumed.insert(response.request_id);
            continue;
        }

        report.responses_processed += 1;
        let status = if response.approved {
            report.objectives_resumed += 1;
            approved.push(HumanApprovedObjective {
                request_id: response.request_id,
                objective: current.objective.clone(),
                conditions: response.conditions.clone(),
            });
            PendingAuthorizationStatus::Resumed
        } else {
            report.denials_recorded += 1;
            PendingAuthorizationStatus::Denied
        };
        let update = PendingAuthorization {
            status,
            responded_at_utc: Some(Utc::now().to_rfc3339()),
            approved: Some(response.approved),
            reason: response.reason,
            conditions: response.conditions,
            ..current
        };
        append_jsonl(pending_path, &update)?;
        latest.insert(response.request_id, update);
        consumed.insert(response.request_id);
    }

    Ok((approved, report))
}

fn load_pending_authorizations(
    path: &Path,
) -> std::io::Result<BTreeMap<Uuid, PendingAuthorization>> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(BTreeMap::new());
    };
    let mut latest = BTreeMap::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        if let Ok(row) = serde_json::from_str::<PendingAuthorization>(line) {
            latest.insert(row.request_id, row);
        }
    }
    Ok(latest)
}

fn parse_approval_response(line: &str) -> Option<ApprovalResponse> {
    let value = serde_json::from_str::<Value>(line).ok()?;
    let request_id = value
        .get("request_id")
        .or_else(|| value.get("message_id"))
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())?;
    let approved = if let Some(raw) = value.get("approved").and_then(Value::as_bool) {
        raw
    } else {
        match value
            .get("action")
            .and_then(Value::as_str)
            .map(|raw| raw.to_ascii_lowercase())
            .as_deref()
        {
            Some("approve") | Some("approved") => true,
            Some("deny") | Some("denied") => false,
            _ => return None,
        }
    };
    let reason = value
        .get("reason")
        .or_else(|| value.get("content"))
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let conditions = value
        .get("conditions")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(ApprovalResponse {
        request_id,
        approved,
        reason,
        conditions,
    })
}

fn append_jsonl(path: impl AsRef<Path>, value: &impl Serialize) -> std::io::Result<()> {
    let p = path.as_ref();
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(p)?;
    let line = serde_json::to_string(value).map_err(std::io::Error::other)?;
    writeln!(f, "{}", line)
}

#[allow(dead_code)]
fn _response_action_is_terminal(action: ResponseAction) -> bool {
    matches!(action, ResponseAction::Approve | ResponseAction::Deny)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn writes_authorize_message() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a2h.jsonl");
        let obj = Objective {
            id: "o".into(),
            statement: "ship migration".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let dec = GateDecision::Rejected {
            resonance: 0.4,
            concerns: vec!["risk".into()],
        };
        let msg = authorize_for_escalation(&obj, &dec);
        write_message(&path, &msg).unwrap();
        let s = std::fs::read_to_string(&path).unwrap();
        assert!(s.contains("\"type\":\"authorize\""));
        assert!(s.contains("ship migration"));
    }

    #[test]
    fn process_h2a_response_resumes_pending_authorization() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join("pending.jsonl");
        let h2a = dir.path().join("h2a.jsonl");
        let request_id = Uuid::new_v4();
        let obj = Objective {
            id: "o".into(),
            statement: "ship migration".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let gate = GateDecision::Rejected {
            resonance: 0.4,
            concerns: vec!["risk".into()],
        };
        append_pending_authorization(&pending, request_id, &obj, &gate).unwrap();
        std::fs::write(
            &h2a,
            format!(
                r#"{{"request_id":"{request_id}","approved":true,"reason":"ok","conditions":["watch logs"]}}"#
            ),
        )
        .unwrap();

        let (approved, report) = process_h2a_responses(&pending, &h2a).unwrap();
        assert_eq!(approved.len(), 1);
        assert_eq!(approved[0].request_id, request_id);
        assert_eq!(approved[0].conditions, vec!["watch logs".to_string()]);
        assert_eq!(report.responses_processed, 1);
        assert_eq!(report.objectives_resumed, 1);
        assert!(std::fs::read_to_string(&pending)
            .unwrap()
            .contains("\"status\":\"resumed\""));
    }

    #[test]
    fn process_h2a_response_records_denial_once() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join("pending.jsonl");
        let h2a = dir.path().join("h2a.jsonl");
        let request_id = Uuid::new_v4();
        let obj = Objective {
            id: "o".into(),
            statement: "ship migration".into(),
            constraints: vec![],
            deadline: None,
            success_criteria: vec![],
            tags: vec![],
        };
        let gate = GateDecision::Rejected {
            resonance: 0.4,
            concerns: vec!["risk".into()],
        };
        append_pending_authorization(&pending, request_id, &obj, &gate).unwrap();
        std::fs::write(
            &h2a,
            format!(r#"{{"message_id":"{request_id}","action":"deny","content":"hold"}}"#),
        )
        .unwrap();

        let (approved, report) = process_h2a_responses(&pending, &h2a).unwrap();
        assert!(approved.is_empty());
        assert_eq!(report.denials_recorded, 1);
        let (_, second) = process_h2a_responses(&pending, &h2a).unwrap();
        assert_eq!(second.responses_processed, 0);
    }
}
