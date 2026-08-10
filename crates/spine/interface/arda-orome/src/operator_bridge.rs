//! Transport-neutral bridge from Hermes Gateway `MessageEvent` values into
//! canonical `arda.operator-session.v1` events.
//!
//! Hermes remains the platform callback and credential owner. This module
//! accepts only the normalized, authenticated event fields Arda needs.

use chrono::{DateTime, Utc};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const OPERATOR_SESSION_SCHEMA: &str = "arda.operator-session.v1";
const OPERATOR_RESPONSE_SCHEMA: &str = "arda.operator-session-response.v1";
const REDACTED_SUMMARY: &str = "Sensitive content withheld from non-private projection.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HermesSessionSource {
    pub platform: String,
    pub chat_id: String,
    pub chat_type: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HermesPromptResponse {
    pub prompt_id: String,
    pub option_id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub prompt_message_id: Option<String>,
}

/// Stable subset of Hermes Agent's normalized `MessageEvent`.
///
/// Deliberately excludes `raw_message`, free-form metadata, and all platform
/// credentials. A connector must project those away before crossing into Arda.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HermesMessageEvent {
    pub text: String,
    pub message_type: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub user_name: Option<String>,
    pub source: HermesSessionSource,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub media_urls: Vec<String>,
    #[serde(default)]
    pub media_types: Vec<String>,
    pub timestamp: String,
    #[serde(default)]
    pub prompt_response: Option<HermesPromptResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OperatorIdentity {
    pub operator_id: String,
    pub authenticated: bool,
    pub authentication_method: String,
    pub authenticated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgeLineage {
    pub session_id: String,
    #[serde(default)]
    pub objective_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Audience {
    OperatorPrivate,
    Direct,
    Group,
    Public,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContentSensitivity {
    Public,
    Internal,
    Private,
    Health,
    Financial,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BridgeOperation {
    Capture,
    Query,
    Approve,
    Reject,
    Revise,
    Cancel,
    Acknowledge,
    Defer,
    Resume,
}

impl BridgeOperation {
    fn is_approval_response(self) -> bool {
        matches!(self, Self::Approve | Self::Reject | Self::Revise)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalSingleUseState {
    Available,
    Consumed,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgeApproval {
    pub scope: Vec<String>,
    pub action_digest: String,
    pub expires_at: String,
    pub single_use_state: ApprovalSingleUseState,
    #[serde(default)]
    pub consumed_by_event_id: Option<String>,
}

/// Canonical pending-approval state supplied by Arda, never by the platform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ApprovalBinding {
    pub prompt_id: String,
    pub operator_id: String,
    pub action_digest: String,
    pub scope: Vec<String>,
    pub session_id: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub conversation_id: String,
    #[serde(default)]
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AttachmentProvenance {
    pub transport_event_id: String,
    pub operator_supplied: bool,
    pub captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgeAttachment {
    pub attachment_id: String,
    pub media_type: String,
    pub byte_length: u64,
    pub content_digest: String,
    pub source_ref: String,
    pub provenance: AttachmentProvenance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgeRequest {
    pub operator: OperatorIdentity,
    pub lineage: BridgeLineage,
    pub adapter_id: String,
    pub audience: Audience,
    pub sensitivity: ContentSensitivity,
    pub operation: BridgeOperation,
    pub event: HermesMessageEvent,
    #[serde(default)]
    pub attachments: Vec<BridgeAttachment>,
    #[serde(default)]
    pub approval: Option<BridgeApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorProjection {
    pub transport: String,
    pub adapter_id: String,
    pub conversation_id: String,
    pub thread_id: Option<String>,
    pub audience: Audience,
    pub platform_message_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorIncoming {
    pub event_id: String,
    pub idempotency_key: String,
    pub received_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorContent {
    pub sensitivity: ContentSensitivity,
    pub text: Option<String>,
    pub attachments: Vec<BridgeAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorDelivery {
    pub state: String,
    pub attempt: u32,
    pub next_retry_at: Option<String>,
    pub acknowledgement_id: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorResult {
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorSessionEvent {
    pub schema_version: String,
    pub operator: OperatorIdentity,
    pub lineage: BridgeLineage,
    pub projection: OperatorProjection,
    pub incoming: OperatorIncoming,
    pub content: OperatorContent,
    pub operation: BridgeOperation,
    pub approval: Option<BridgeApproval>,
    pub delivery: OperatorDelivery,
    pub result: OperatorResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorBridgeResponse {
    pub schema_version: String,
    pub transport: String,
    pub adapter_id: String,
    pub conversation_id: String,
    pub thread_id: Option<String>,
    pub reply_to_platform_message_id: String,
    pub session_id: String,
    pub objective_id: Option<String>,
    pub project_id: Option<String>,
    pub task_id: Option<String>,
    pub run_id: Option<String>,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportHealthState {
    NotConfigured,
    Unavailable,
    Degraded,
    Stale,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorTransportHealth {
    pub schema_version: String,
    pub state: TransportHealthState,
    pub configured: bool,
    pub connected: bool,
    pub authenticated: bool,
    pub last_success_at: Option<String>,
    pub last_error_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TransportHealthInput {
    pub configured: bool,
    pub connected: bool,
    pub authenticated: bool,
    pub last_success_at: Option<String>,
    pub last_error_code: Option<String>,
    pub stale_after_seconds: i64,
}

impl Default for TransportHealthInput {
    fn default() -> Self {
        Self {
            configured: false,
            connected: false,
            authenticated: false,
            last_success_at: None,
            last_error_code: None,
            stale_after_seconds: 300,
        }
    }
}

impl OperatorTransportHealth {
    pub fn derive(input: TransportHealthInput, now: DateTime<Utc>) -> Self {
        let stale = input
            .last_success_at
            .as_deref()
            .and_then(parse_timestamp)
            .map(|last| now.signed_duration_since(last).num_seconds() > input.stale_after_seconds)
            .unwrap_or(false);
        let state = if !input.configured {
            TransportHealthState::NotConfigured
        } else if !input.connected || !input.authenticated {
            TransportHealthState::Unavailable
        } else if input.last_error_code.is_some() {
            TransportHealthState::Degraded
        } else if stale {
            TransportHealthState::Stale
        } else {
            TransportHealthState::Ready
        };
        Self {
            schema_version: "arda.operator-transport-health.v1".into(),
            state,
            configured: input.configured,
            connected: input.connected,
            authenticated: input.authenticated,
            last_success_at: input.last_success_at,
            last_error_code: input.last_error_code,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BridgeError {
    #[error("invalid bridge event: {0}")]
    Invalid(String),
    #[error("duplicate transport event: {0}")]
    DuplicateEvent(String),
    #[error("approval has expired")]
    ApprovalExpired,
    #[error("approval is not available")]
    ApprovalUnavailable,
    #[error("approval does not match canonical pending state: {0}")]
    ApprovalMismatch(String),
    #[error("approval action was already consumed: {0}")]
    ApprovalAlreadyConsumed(String),
    #[error("bridge persistence failed: {0}")]
    Persistence(String),
}

#[derive(Debug, Clone)]
pub struct OperatorBridge {
    sessions_path: PathBuf,
}

impl OperatorBridge {
    pub fn new(root: impl AsRef<Path>) -> Result<Self, BridgeError> {
        let root = root.as_ref();
        fs::create_dir_all(root).map_err(persistence_error)?;
        Ok(Self {
            sessions_path: root.join("operator_sessions.jsonl"),
        })
    }

    pub fn ingest(
        &self,
        request: BridgeRequest,
        now: DateTime<Utc>,
    ) -> Result<OperatorSessionEvent, BridgeError> {
        self.ingest_inner(request, None, now)
    }

    /// Ingest an approval response against pending state loaded by Arda.
    /// The binding is separate so transport JSON cannot forge canonical state.
    pub fn ingest_approval(
        &self,
        request: BridgeRequest,
        binding: &ApprovalBinding,
        now: DateTime<Utc>,
    ) -> Result<OperatorSessionEvent, BridgeError> {
        self.ingest_inner(request, Some(binding), now)
    }

    fn ingest_inner(
        &self,
        request: BridgeRequest,
        pending_approval: Option<&ApprovalBinding>,
        now: DateTime<Utc>,
    ) -> Result<OperatorSessionEvent, BridgeError> {
        validate_request(&request)?;
        let event_id = request
            .event
            .message_id
            .as_deref()
            .or(request.event.source.message_id.as_deref())
            .ok_or_else(|| {
                BridgeError::Invalid("Hermes MessageEvent is missing message_id".into())
            })?
            .to_string();

        let mut sessions = locked_file(&self.sessions_path)?;
        if jsonl_has_field(&mut sessions, "incoming", "event_id", &event_id)? {
            return Err(BridgeError::DuplicateEvent(event_id));
        }

        if request.operation.is_approval_response() {
            self.validate_approval(&request, pending_approval, &mut sessions, now)?;
        }

        let redacted = should_redact(request.sensitivity, request.audience);
        let transport = normalize_transport(&request.event.source.platform);
        let summary = if redacted {
            REDACTED_SUMMARY.to_string()
        } else {
            format!(
                "Authenticated {transport} {:?} normalized for canonical handling.",
                request.operation
            )
        };
        let mut approval = request.approval;
        if let Some(value) = approval
            .as_mut()
            .filter(|_| request.operation.is_approval_response())
        {
            value.single_use_state = ApprovalSingleUseState::Consumed;
            value.consumed_by_event_id = Some(event_id.clone());
        }
        let session = OperatorSessionEvent {
            schema_version: OPERATOR_SESSION_SCHEMA.into(),
            operator: request.operator,
            lineage: request.lineage,
            projection: OperatorProjection {
                transport: transport.clone(),
                adapter_id: request.adapter_id,
                conversation_id: request.event.source.chat_id,
                thread_id: request.event.source.thread_id,
                audience: request.audience,
                platform_message_id: event_id.clone(),
            },
            incoming: OperatorIncoming {
                event_id: event_id.clone(),
                idempotency_key: format!("operator-event:{transport}:{event_id}"),
                received_at: request.event.timestamp,
            },
            content: OperatorContent {
                sensitivity: request.sensitivity,
                text: (!redacted).then_some(request.event.text),
                attachments: if redacted {
                    Vec::new()
                } else {
                    request.attachments
                },
            },
            operation: request.operation,
            approval,
            delivery: OperatorDelivery {
                state: "pending".into(),
                attempt: 0,
                next_retry_at: None,
                acknowledgement_id: None,
                last_error_code: None,
            },
            result: OperatorResult {
                summary,
                evidence_refs: vec![format!("arda://operator-events/{event_id}")],
            },
        };
        append_locked_json(&mut sessions, &session)?;
        Ok(session)
    }

    pub fn correlate_response(
        &self,
        session: &OperatorSessionEvent,
        summary: impl Into<String>,
        evidence_refs: Vec<String>,
    ) -> OperatorBridgeResponse {
        OperatorBridgeResponse {
            schema_version: OPERATOR_RESPONSE_SCHEMA.into(),
            transport: session.projection.transport.clone(),
            adapter_id: session.projection.adapter_id.clone(),
            conversation_id: session.projection.conversation_id.clone(),
            thread_id: session.projection.thread_id.clone(),
            reply_to_platform_message_id: session.projection.platform_message_id.clone(),
            session_id: session.lineage.session_id.clone(),
            objective_id: session.lineage.objective_id.clone(),
            project_id: session.lineage.project_id.clone(),
            task_id: session.lineage.task_id.clone(),
            run_id: session.lineage.run_id.clone(),
            summary: summary.into(),
            evidence_refs,
        }
    }

    fn validate_approval(
        &self,
        request: &BridgeRequest,
        pending_approval: Option<&ApprovalBinding>,
        sessions: &mut File,
        now: DateTime<Utc>,
    ) -> Result<(), BridgeError> {
        let approval = request.approval.as_ref().ok_or_else(|| {
            BridgeError::ApprovalMismatch("approval operation omitted approval".into())
        })?;
        let binding = pending_approval.ok_or_else(|| {
            BridgeError::ApprovalMismatch("canonical pending approval was not supplied".into())
        })?;
        let prompt = request.event.prompt_response.as_ref().ok_or_else(|| {
            BridgeError::ApprovalMismatch("Hermes prompt_response was not supplied".into())
        })?;
        if approval.single_use_state != ApprovalSingleUseState::Available
            || approval.consumed_by_event_id.is_some()
        {
            return Err(BridgeError::ApprovalUnavailable);
        }
        let expires_at = parse_timestamp(&approval.expires_at)
            .ok_or_else(|| BridgeError::Invalid("approval expiry is not RFC3339".into()))?;
        if expires_at <= now {
            return Err(BridgeError::ApprovalExpired);
        }
        compare("prompt_id", &prompt.prompt_id, &binding.prompt_id)?;
        compare(
            "operator_id",
            &request.operator.operator_id,
            &binding.operator_id,
        )?;
        compare(
            "action_digest",
            &approval.action_digest,
            &binding.action_digest,
        )?;
        compare(
            "session_id",
            &request.lineage.session_id,
            &binding.session_id,
        )?;
        compare(
            "conversation_id",
            &request.event.source.chat_id,
            &binding.conversation_id,
        )?;
        compare_optional(
            "task_id",
            request.lineage.task_id.as_deref(),
            binding.task_id.as_deref(),
        )?;
        compare_optional(
            "run_id",
            request.lineage.run_id.as_deref(),
            binding.run_id.as_deref(),
        )?;
        compare_optional(
            "thread_id",
            request.event.source.thread_id.as_deref(),
            binding.thread_id.as_deref(),
        )?;
        if approval.scope != binding.scope {
            return Err(BridgeError::ApprovalMismatch("scope".into()));
        }

        if jsonl_has_field(
            sessions,
            "approval",
            "action_digest",
            &approval.action_digest,
        )? {
            return Err(BridgeError::ApprovalAlreadyConsumed(
                approval.action_digest.clone(),
            ));
        }
        Ok(())
    }
}

fn validate_request(request: &BridgeRequest) -> Result<(), BridgeError> {
    for (name, value) in [
        ("operator_id", request.operator.operator_id.as_str()),
        ("session_id", request.lineage.session_id.as_str()),
        ("adapter_id", request.adapter_id.as_str()),
        ("platform", request.event.source.platform.as_str()),
        ("conversation_id", request.event.source.chat_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(BridgeError::Invalid(format!("{name} is empty")));
        }
    }
    if !request.operator.authenticated {
        return Err(BridgeError::Invalid(
            "operator identity is not authenticated".into(),
        ));
    }
    if !matches!(
        request.operator.authentication_method.as_str(),
        "gateway_identity" | "device_enrollment" | "local_session" | "service_capability"
    ) {
        return Err(BridgeError::Invalid(
            "unsupported operator authentication method".into(),
        ));
    }
    if request.event.user_id.as_deref() != Some(request.operator.operator_id.as_str()) {
        return Err(BridgeError::Invalid(
            "authenticated operator does not match MessageEvent user_id".into(),
        ));
    }
    if parse_timestamp(&request.operator.authenticated_at).is_none()
        || parse_timestamp(&request.event.timestamp).is_none()
    {
        return Err(BridgeError::Invalid("timestamp is not RFC3339".into()));
    }
    let event_id = request
        .event
        .message_id
        .as_deref()
        .or(request.event.source.message_id.as_deref())
        .ok_or_else(|| BridgeError::Invalid("message_id is required".into()))?;
    if let (Some(event_message_id), Some(source_message_id)) = (
        request.event.message_id.as_deref(),
        request.event.source.message_id.as_deref(),
    ) {
        compare(
            "MessageEvent/source message_id",
            event_message_id,
            source_message_id,
        )?;
    }
    if request.event.media_urls.len() != request.event.media_types.len()
        || request.event.media_urls.len() != request.attachments.len()
    {
        return Err(BridgeError::Invalid(
            "MessageEvent media and attachment provenance counts differ".into(),
        ));
    }
    if request.event.text.len() > 16_384 || request.attachments.len() > 16 {
        return Err(BridgeError::Invalid(
            "operator content exceeds the v1 bounds".into(),
        ));
    }
    for (index, attachment) in request.attachments.iter().enumerate() {
        if attachment.provenance.transport_event_id != event_id
            || attachment.source_ref != request.event.media_urls[index]
            || attachment.media_type != request.event.media_types[index]
        {
            return Err(BridgeError::Invalid(format!(
                "attachment {index} does not match MessageEvent provenance"
            )));
        }
        if !valid_digest(&attachment.content_digest) {
            return Err(BridgeError::Invalid(format!(
                "attachment {index} has invalid digest"
            )));
        }
        if attachment.byte_length > 52_428_800 || unsafe_source_ref(&attachment.source_ref) {
            return Err(BridgeError::Invalid(format!(
                "attachment {index} exceeds bounds or has an unsafe source reference"
            )));
        }
    }
    if let Some(approval) = request.approval.as_ref() {
        if approval.scope.is_empty()
            || !valid_digest(&approval.action_digest)
            || approval.scope.iter().any(|scope| scope.trim().is_empty())
        {
            return Err(BridgeError::Invalid(
                "approval scope or action digest is invalid".into(),
            ));
        }
        let mut unique_scope = approval.scope.clone();
        unique_scope.sort();
        unique_scope.dedup();
        if unique_scope.len() != approval.scope.len() {
            return Err(BridgeError::Invalid(
                "approval scope contains duplicates".into(),
            ));
        }
    }
    if !request.operation.is_approval_response() && request.approval.is_some() {
        return Err(BridgeError::Invalid(
            "non-approval operation carried approval authority".into(),
        ));
    }
    Ok(())
}

fn should_redact(sensitivity: ContentSensitivity, audience: Audience) -> bool {
    matches!(audience, Audience::Group | Audience::Public)
        && matches!(
            sensitivity,
            ContentSensitivity::Private
                | ContentSensitivity::Health
                | ContentSensitivity::Financial
        )
}

fn normalize_transport(platform: &str) -> String {
    match platform.trim().to_ascii_lowercase().as_str() {
        "telegram" | "discord" | "matrix" | "web" | "local" => platform.trim().to_ascii_lowercase(),
        _ => "other".into(),
    }
}

fn valid_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase())
    })
}

fn unsafe_source_ref(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.is_empty() || lower.starts_with("data:") || lower.starts_with("javascript:")
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.with_timezone(&Utc))
}

fn compare(field: &str, actual: &str, expected: &str) -> Result<(), BridgeError> {
    if actual == expected {
        Ok(())
    } else {
        Err(BridgeError::ApprovalMismatch(field.into()))
    }
}

fn compare_optional(
    field: &str,
    actual: Option<&str>,
    expected: Option<&str>,
) -> Result<(), BridgeError> {
    if actual == expected {
        Ok(())
    } else {
        Err(BridgeError::ApprovalMismatch(field.into()))
    }
}

fn locked_file(path: &Path) -> Result<File, BridgeError> {
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(persistence_error)?;
    file.lock_exclusive().map_err(persistence_error)?;
    Ok(file)
}

fn jsonl_has_field(
    file: &mut File,
    object: &str,
    field: &str,
    expected: &str,
) -> Result<bool, BridgeError> {
    file.seek(SeekFrom::Start(0)).map_err(persistence_error)?;
    for line in BufReader::new(&mut *file).lines() {
        let line = line.map_err(persistence_error)?;
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        if value
            .pointer(&format!("/{object}/{field}"))
            .and_then(|value| value.as_str())
            == Some(expected)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn append_locked_json(file: &mut File, value: &impl Serialize) -> Result<(), BridgeError> {
    file.seek(SeekFrom::End(0)).map_err(persistence_error)?;
    serde_json::to_writer(&mut *file, value)
        .map_err(|error| BridgeError::Persistence(error.to_string()))?;
    file.write_all(b"\n").map_err(persistence_error)?;
    file.sync_data().map_err(persistence_error)
}

fn persistence_error(error: std::io::Error) -> BridgeError {
    BridgeError::Persistence(error.to_string())
}
