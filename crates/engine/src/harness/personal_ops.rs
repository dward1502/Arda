//! Personal operations HTTP handlers for the harness surface.
//!
//! Exposes capture, classification, scheduling, completion, and
//! projection endpoints over the append-only personal-ops event log.

use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::HarnessState;
use crate::personal_ops::PersonalOpsLogStore;

pub const PERSONAL_OPS_LOG_VERSION: &str = "arda.personal-ops.v1.log";

#[derive(Debug, Serialize)]
pub struct ProjectionResponse {
    pub schema_version: &'static str,
    pub projection: crate::personal_ops::PersonalOpsProjection,
}

#[derive(Debug, Serialize)]
pub struct CaptureResponse {
    pub event_id: String,
    pub capture_id: String,
}

#[derive(Debug, Deserialize)]
pub struct CaptureRequest {
    pub operator_id: String,
    pub text: Option<String>,
    pub audio_reference: Option<String>,
    pub project_id: Option<String>,
    pub priority: Option<u8>,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ClassifyRequest {
    pub operator_id: String,
    pub item_id: String,
    pub kind: String,
    pub evidence_class: Option<String>,
    pub confidence: Option<f32>,
    pub rationale: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ScheduleRequest {
    pub operator_id: String,
    pub scheduled_at: Option<String>,
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompleteRequest {
    pub operator_id: String,
}

fn require_loopback(peer: &std::net::SocketAddr) -> bool {
    peer.ip().is_loopback()
}

fn not_loopback() -> (StatusCode, axum::Json<serde_json::Value>) {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({"error": "mutations are loopback-only"})),
    )
}

type MutationError = (StatusCode, Json<serde_json::Value>);

fn mutation_event_id(
    headers: &HeaderMap,
    operator_id: &str,
    operation: &str,
) -> Result<uuid::Uuid, MutationError> {
    if operator_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "operator_id cannot be empty"})),
        ));
    }
    let header_operator = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error": "x-arda-operator-id header required"})),
            )
        })?;
    if header_operator != operator_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"error": "operator identity mismatch"})),
        ));
    }
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty() && value.len() <= 256)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": "bounded Idempotency-Key header required"})),
            )
        })?;

    Ok(deterministic_uuid(&format!(
        "{operator_id}\0{operation}\0{key}"
    )))
}

fn deterministic_uuid(input: &str) -> uuid::Uuid {
    let digest = Sha256::digest(input.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x50;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes)
}

fn derived_capture_id(event_id: uuid::Uuid) -> uuid::Uuid {
    deterministic_uuid(&format!("arda.personal.capture\0{event_id}"))
}

fn to_utc(input: &str) -> Result<chrono::DateTime<Utc>, String> {
    chrono::DateTime::parse_from_rfc3339(input)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| format!("invalid timestamp: {e}"))
}

/// Create a capture from a text or audio-reference input.
pub async fn create_capture(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<CaptureRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    if req.text.as_deref().is_none_or(|s| s.trim().is_empty())
        && req
            .audio_reference
            .as_deref()
            .is_none_or(|s| s.trim().is_empty())
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "text or audio_reference required" })),
        )
            .into_response();
    }

    let event_id = match mutation_event_id(&headers, &req.operator_id, "captures.create") {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let capture_id = derived_capture_id(event_id);
    let capture = arda_core::personal_ops::InboxCapture {
        capture_id,
        captured_at: Utc::now(),
        source: if req.audio_reference.is_some() {
            arda_core::personal_ops::CaptureSource::Audio
        } else {
            arda_core::personal_ops::CaptureSource::Text
        },
        content: arda_core::personal_ops::CaptureContent {
            text: req.text.clone(),
            audio_reference: req.audio_reference.clone(),
        },
        attachments: Vec::new(),
        project_id: req.project_id.and_then(|s| s.parse().ok()),
        priority: req.priority,
        due_at: req.due_at.as_ref().and_then(|s| to_utc(s).ok()),
    };

    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::CaptureRecorded(
            arda_core::personal_ops::CaptureRecordedEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                capture,
            },
        ),
    );

    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist capture: {e}") })),
        )
            .into_response();
    }

    (
        StatusCode::CREATED,
        Json(CaptureResponse {
            event_id: envelope.record.event_id().to_string(),
            capture_id: capture_id.to_string(),
        }),
    )
        .into_response()
}

/// Classify an inbox capture as a PersonalItem (Task, Reminder, Note, etc.).
pub async fn classify_item(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ClassifyRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    let item_uuid = match item_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid item_id" })),
            )
                .into_response();
        }
    };
    if req.item_id != item_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "body item_id must match path" })),
        )
            .into_response();
    }
    let event_id = match mutation_event_id(
        &headers,
        &req.operator_id,
        &format!("items.{item_id}.classify"),
    ) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let kind = parse_kind(&req.kind);
    let evidence = parse_evidence(req.evidence_class.as_deref());
    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::ItemClassified(
            arda_core::personal_ops::ItemClassifiedEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                item_id: item_uuid,
                kind,
                evidence_class: evidence,
                confidence: req.confidence,
                rationale: req.rationale.clone(),
            },
        ),
    );
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist: {e}") })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "event_id": envelope.record.event_id().to_string() })),
    )
        .into_response()
}

fn parse_kind(s: &str) -> arda_core::personal_ops::PersonalItemKind {
    match s.to_lowercase().as_str() {
        "task" => arda_core::personal_ops::PersonalItemKind::Task,
        "reminder" => arda_core::personal_ops::PersonalItemKind::Reminder,
        "note" => arda_core::personal_ops::PersonalItemKind::Note,
        "appointment" => arda_core::personal_ops::PersonalItemKind::Appointment,
        "contact" => arda_core::personal_ops::PersonalItemKind::Contact,
        "health" => arda_core::personal_ops::PersonalItemKind::Health,
        _ => arda_core::personal_ops::PersonalItemKind::Note,
    }
}

fn parse_evidence(s: Option<&str>) -> arda_core::personal_ops::EvidenceClass {
    match s.map(|v| v.to_lowercase()).as_deref() {
        Some("operator_authored") => arda_core::personal_ops::EvidenceClass::OperatorAuthored,
        Some("imported") => arda_core::personal_ops::EvidenceClass::Imported,
        Some("inferred") => arda_core::personal_ops::EvidenceClass::Inferred,
        Some("device_measured") => arda_core::personal_ops::EvidenceClass::DeviceMeasured,
        Some("self_reported") => arda_core::personal_ops::EvidenceClass::SelfReported,
        _ => arda_core::personal_ops::EvidenceClass::Unavailable,
    }
}

/// Schedule or reschedule a personal item.
pub async fn schedule_item(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ScheduleRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    let item_uuid = match item_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid item_id" })),
            )
                .into_response();
        }
    };
    let event_id = match mutation_event_id(
        &headers,
        &req.operator_id,
        &format!("items.{item_id}.schedule"),
    ) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let scheduled_at = req.scheduled_at.as_ref().and_then(|s| to_utc(s).ok());
    let due_at = req.due_at.as_ref().and_then(|s| to_utc(s).ok());
    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::ItemScheduled(
            arda_core::personal_ops::ItemScheduledEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                item_id: item_uuid,
                scheduled_at,
                due_at,
            },
        ),
    );
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist: {e}") })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "event_id": envelope.record.event_id().to_string() })),
    )
        .into_response()
}

/// Mark a personal item as completed.
pub async fn complete_item(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    Path(item_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<CompleteRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    let item_uuid = match item_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid item_id" })),
            )
                .into_response();
        }
    };
    let event_id = match mutation_event_id(
        &headers,
        &req.operator_id,
        &format!("items.{item_id}.complete"),
    ) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::ItemCompleted(
            arda_core::personal_ops::ItemCompletedEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                item_id: item_uuid,
                completed_at: Utc::now(),
            },
        ),
    );
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist: {e}") })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "event_id": envelope.record.event_id().to_string() })),
    )
        .into_response()
}

/// Get the current personal-ops projection (inbox, today, waiting, scheduled,
/// completed) by replaying the append-only event log.
pub async fn get_projection(State(state): State<HarnessState>) -> impl IntoResponse {
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    let events = match store.load_all() {
        Ok(events) => events,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("failed to load event log: {error}") })),
            )
                .into_response();
        }
    };

    let now = Utc::now();
    let local_date = now.naive_local().date();
    let projection = crate::personal_ops::build_projection(&events, now, local_date);

    (
        StatusCode::OK,
        Json(ProjectionResponse {
            schema_version: "arda.harness.personal-ops.v1",
            projection,
        }),
    )
        .into_response()
}

/// Get the inbox: unclassified captures awaiting organization.
pub async fn get_inbox(State(state): State<HarnessState>) -> impl IntoResponse {
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    let events = match store.load_all() {
        Ok(events) => events,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("failed to load event log: {error}") })),
            )
                .into_response();
        }
    };

    let now = Utc::now();
    let local_date = now.naive_local().date();
    let projection = crate::personal_ops::build_projection(&events, now, local_date);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "schema_version": "arda.harness.personal-ops.v1",
            "inbox": projection.inbox,
        })),
    )
        .into_response()
}

/// Get a "What was I doing?" resume card based on recent activity.
#[derive(Debug, Serialize)]
pub struct ResumeResponse {
    pub schema_version: &'static str,
    pub resume: ResumeCard,
}

#[derive(Debug, Serialize)]
pub struct ResumeCard {
    pub summary: String,
    pub active_count: usize,
    pub inbox_count: usize,
    pub today_count: usize,
    pub waiting_count: usize,
    pub generated_at: String,
}

pub async fn get_resume(State(state): State<HarnessState>) -> impl IntoResponse {
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    let events = match store.load_all() {
        Ok(events) => events,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("failed to load event log: {error}") })),
            )
                .into_response();
        }
    };

    let now = Utc::now();
    let local_date = now.naive_local().date();
    let projection = crate::personal_ops::build_projection(&events, now, local_date);

    let active_count = projection.today.len() + projection.waiting.len();
    let summary = if active_count == 0 {
        if projection.inbox.is_empty() {
            "Nothing in progress. Check your captures or scheduled items.".to_owned()
        } else {
            format!(
                "{} capture(s) in the inbox awaiting classification.",
                projection.inbox.len()
            )
        }
    } else {
        format!(
            "{} active item(s) need attention: {} due today, {} waiting on reminders.",
            active_count,
            projection.today.len(),
            projection.waiting.len()
        )
    };

    (
        StatusCode::OK,
        Json(ResumeResponse {
            schema_version: "arda.harness.personal-ops.v1",
            resume: ResumeCard {
                summary,
                active_count,
                inbox_count: projection.inbox.len(),
                today_count: projection.today.len(),
                waiting_count: projection.waiting.len(),
                generated_at: now.to_rfc3339(),
            },
        }),
    )
        .into_response()
}

/// Record a reminder attempt event. The Oromë adapter calls this
/// after attempting delivery; the receipt determines whether the
/// state transitions to Delivered, Acknowledged, Deferred, Dismissed,
/// or Failed.
#[derive(Debug, Deserialize)]
pub struct ReminderAttemptRequest {
    pub operator_id: String,
    pub item_id: String,
    pub reminder_id: String,
    pub state: String,
    pub provider_message_id: Option<String>,
    pub error: Option<String>,
}

pub async fn record_reminder_attempt(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ReminderAttemptRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    let item_uuid = match req.item_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid item_id" })),
            )
                .into_response();
        }
    };
    let reminder_uuid = match req.reminder_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid reminder_id" })),
            )
                .into_response();
        }
    };
    let event_id = match mutation_event_id(
        &headers,
        &req.operator_id,
        &format!("reminders.{reminder_uuid}.attempt"),
    ) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let delivery_state = parse_delivery_state(&req.state);
    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::ReminderAttempted(
            arda_core::personal_ops::ReminderAttemptedEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                item_id: item_uuid,
                receipt: arda_core::personal_ops::ReminderReceipt {
                    reminder_id: reminder_uuid,
                    item_id: item_uuid,
                    attempted_at: Utc::now(),
                    state: delivery_state,
                    channel: format!("{}:{}", req.item_id, req.reminder_id),
                    receipt_reference: req.provider_message_id,
                },
            },
        ),
    );
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist: {e}") })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "event_id": envelope.record.event_id().to_string() })),
    )
        .into_response()
}

/// Record a reminder acknowledgement (ack, defer, or dismiss).
#[derive(Debug, Deserialize)]
pub struct ReminderAcknowledgeRequest {
    pub operator_id: String,
    pub state: String,
    pub receipt_reference: Option<String>,
}

pub async fn acknowledge_reminder(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<std::net::SocketAddr>,
    Path(reminder_id): Path<String>,
    headers: HeaderMap,
    Json(req): Json<ReminderAcknowledgeRequest>,
) -> impl IntoResponse {
    if !require_loopback(&peer) {
        return not_loopback().into_response();
    }
    let reminder_uuid = match reminder_id.parse::<uuid::Uuid>() {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid reminder_id" })),
            )
                .into_response();
        }
    };
    let event_id = match mutation_event_id(
        &headers,
        &req.operator_id,
        &format!("reminders.{reminder_id}.acknowledge"),
    ) {
        Ok(id) => id,
        Err(error) => return error.into_response(),
    };
    let delivery_state = parse_delivery_state(&req.state);
    let envelope = arda_core::personal_ops::PersonalOpsEnvelope::new(
        arda_core::personal_ops::PersonalOpsRecord::ReminderAcknowledged(
            arda_core::personal_ops::ReminderAcknowledgedEvent {
                event_id,
                occurred_at: Utc::now(),
                operator_id: req.operator_id.clone(),
                reminder_id: reminder_uuid,
                state: delivery_state,
                receipt_reference: req.receipt_reference,
            },
        ),
    );
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    if let Err(e) = store.append(&envelope) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("failed to persist: {e}") })),
        )
            .into_response();
    }
    (
        StatusCode::CREATED,
        Json(serde_json::json!({ "event_id": envelope.record.event_id().to_string() })),
    )
        .into_response()
}

fn parse_delivery_state(s: &str) -> arda_core::personal_ops::ReminderDeliveryState {
    match s.to_lowercase().as_str() {
        "delivered" => arda_core::personal_ops::ReminderDeliveryState::Delivered,
        "acknowledged" => arda_core::personal_ops::ReminderDeliveryState::Acknowledged,
        "deferred" => arda_core::personal_ops::ReminderDeliveryState::Deferred,
        "dismissed" => arda_core::personal_ops::ReminderDeliveryState::Dismissed,
        "failed" => arda_core::personal_ops::ReminderDeliveryState::Failed,
        _ => arda_core::personal_ops::ReminderDeliveryState::Attempted,
    }
}

/// Get the today brief: items due today plus reminder state.
pub async fn get_today_brief(State(state): State<HarnessState>) -> impl IntoResponse {
    let store = PersonalOpsLogStore::new(&state.workbench_root);
    let events = match store.load_all() {
        Ok(events) => events,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("failed to load event log: {error}") })),
            )
                .into_response();
        }
    };

    let now = Utc::now();
    let local_date = now.naive_local().date();
    let projection = crate::personal_ops::build_projection(&events, now, local_date);

    let reminders_awaiting = projection
        .today
        .iter()
        .filter(|item| {
            item.reminder_state
                .as_ref()
                .map(|r| {
                    matches!(
                        r.delivery_state,
                        arda_core::personal_ops::ReminderDeliveryState::Attempted
                            | arda_core::personal_ops::ReminderDeliveryState::Deferred
                    )
                })
                .unwrap_or(false)
        })
        .count();

    let quiet_mode = false;
    let source_records = super::personal_briefs::personal_event_sources(&events);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "schema_version": "arda.harness.personal-ops.v1",
            "brief": {
                "generated_at": now.to_rfc3339(),
                "today": projection.today,
                "waiting": projection.waiting,
                "reminders_awaiting_ack": reminders_awaiting,
                "quiet_mode": quiet_mode,
                "uncertainty_disclosure": "Brief reconstructed from local event log; items may change as captures are reclassified.",
                "source_records": source_records,
            },
        })),
    )
        .into_response()
}
