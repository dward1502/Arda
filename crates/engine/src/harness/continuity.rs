//! Governed Hermes continuity and surface-handoff HTTP boundary.

use arda_orome::operator_bridge::OperatorIdentity;
use arda_orome::{DataDomain, HandoffState, PrivacyClass, SurfaceHandoff, SurfaceHandoffError};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Mutex;

use super::{
    projects::{require_loopback, ApiError},
    HarnessState,
};

const CONTINUITY_EVENT_SCHEMA: &str = "arda.continuity-event.v1";
const MAX_ID: usize = 256;
const MAX_REFS: usize = 32;
static CONTINUITY_WRITE: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ContinuityEvent {
    pub schema_version: String,
    pub event_id: String,
    pub session_lineage_id: String,
    pub current_session_id: String,
    pub surface_id: String,
    pub platform: String,
    pub chat_id: String,
    #[serde(default)]
    pub thread_id: Option<String>,
    pub privacy_class: PrivacyClass,
    pub authorized_domains: Vec<DataDomain>,
    pub requested_domains: Vec<DataDomain>,
    #[serde(default)]
    pub topic_refs: Vec<String>,
    #[serde(default)]
    pub commitment_refs: Vec<String>,
    #[serde(default)]
    pub memory_scope_refs: Vec<String>,
    pub observed_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContinuityEventRequest {
    pub operator: OperatorIdentity,
    pub event: ContinuityEvent,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateHandoffRequest {
    pub operator: OperatorIdentity,
    pub handoff: SurfaceHandoff,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptHandoffRequest {
    pub operator_ref: String,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize)]
pub struct ContinuityEventResponse {
    schema_version: &'static str,
    status: &'static str,
    replayed: bool,
    receipt_ref: String,
}

#[derive(Debug, Serialize)]
pub struct HandoffResponse {
    schema_version: &'static str,
    handoff: SurfaceHandoff,
    replayed: bool,
    receipt_ref: String,
}

#[derive(Debug, Serialize)]
pub struct SessionContinuityProjection {
    schema_version: &'static str,
    session_lineage_id: String,
    current_session_id: Option<String>,
    active_surface_id: Option<String>,
    privacy_class: Option<PrivacyClass>,
    topic_refs: Vec<String>,
    commitment_refs: Vec<String>,
    memory_scope_refs: Vec<String>,
    handoff_refs: Vec<String>,
    observed_at: Option<DateTime<Utc>>,
    freshness: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HudContinuityProjection {
    schema_version: &'static str,
    generated_at: DateTime<Utc>,
    active: bool,
    session_lineage_id: Option<String>,
    current_session_id: Option<String>,
    surface_id: Option<String>,
    privacy_class: Option<PrivacyClass>,
    freshness: &'static str,
    handoff_id: Option<String>,
    handoff_state: Option<HandoffState>,
    action_ids: Vec<&'static str>,
    private_refs_withheld: bool,
    topic_refs: Vec<String>,
    commitment_refs: Vec<String>,
    memory_scope_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransitionReceipt {
    schema_version: String,
    receipt_id: String,
    kind: String,
    target_ref: String,
    operator_ref: String,
    session_lineage_id: String,
    idempotency_key: String,
    from_state: Option<HandoffState>,
    to_state: Option<HandoffState>,
    payload_sha256: String,
    recorded_at: DateTime<Utc>,
}

impl ContinuityEvent {
    fn validate(&self, now: DateTime<Utc>) -> Result<(), ApiError> {
        if self.schema_version != CONTINUITY_EVENT_SCHEMA {
            return Err(ApiError::bad_request("unsupported continuity event schema"));
        }
        for value in [
            &self.event_id,
            &self.session_lineage_id,
            &self.current_session_id,
            &self.surface_id,
            &self.platform,
            &self.chat_id,
        ] {
            if value.trim().is_empty() || value.len() > MAX_ID {
                return Err(ApiError::bad_request(
                    "continuity event identity is missing or out of bounds",
                ));
            }
        }
        if self.expires_at <= self.observed_at || self.expires_at <= now {
            return Err(ApiError::bad_request("continuity event is expired"));
        }
        if self.requested_domains.is_empty()
            || self.authorized_domains.is_empty()
            || self
                .requested_domains
                .iter()
                .any(|domain| !self.authorized_domains.contains(domain))
        {
            return Err(ApiError::forbidden(
                "continuity event requested a data-domain escalation",
            ));
        }
        for refs in [
            &self.topic_refs,
            &self.commitment_refs,
            &self.memory_scope_refs,
        ] {
            if refs.len() > MAX_REFS
                || refs
                    .iter()
                    .any(|value| value.trim().is_empty() || value.len() > MAX_ID)
            {
                return Err(ApiError::bad_request(
                    "continuity references are out of bounds",
                ));
            }
        }
        validate_idempotency_key(&self.idempotency_key)
    }
}

pub async fn ingest_event(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<ContinuityEventRequest>,
) -> Result<(StatusCode, Json<ContinuityEventResponse>), ApiError> {
    require_loopback(peer)?;
    require_gateway_operator(&state, &request.operator)?;
    request.event.validate(Utc::now())?;
    let root = continuity_root(&state);
    let _guard = CONTINUITY_WRITE
        .lock()
        .map_err(|_| ApiError::internal("continuity writer lock poisoned"))?;
    fs::create_dir_all(&root).map_err(io_error)?;
    let events = root.join("events.jsonl");
    let receipts = root.join("receipts.jsonl");
    let payload_hash = hash_json(&request.event)?;
    if let Some(existing) = find_event_by_key(&events, &request.event.idempotency_key)? {
        if hash_json(&existing)? != payload_hash {
            return Err(ApiError::conflict(
                "continuity idempotency key was replayed with altered payload",
            ));
        }
        match find_receipt_by_key(&receipts, &request.event.idempotency_key)? {
            Some(receipt)
                if receipt.kind == "continuity_event"
                    && receipt.target_ref == request.event.event_id
                    && receipt.payload_sha256 == payload_hash => {}
            Some(_) => {
                return Err(ApiError::conflict(
                    "continuity replay receipt does not match the persisted event",
                ));
            }
            None => append_jsonl(
                &receipts,
                &continuity_event_receipt(
                    &request.event,
                    &request.operator.operator_id,
                    payload_hash.clone(),
                ),
            )?,
        }
        return Ok((
            StatusCode::OK,
            Json(ContinuityEventResponse {
                schema_version: "arda.continuity-event-response.v1",
                status: "accepted",
                replayed: true,
                receipt_ref: receipt_ref(&request.event.idempotency_key),
            }),
        ));
    }
    append_jsonl(&events, &request.event)?;
    let receipt =
        continuity_event_receipt(&request.event, &request.operator.operator_id, payload_hash);
    append_jsonl(&receipts, &receipt)?;
    Ok((
        StatusCode::CREATED,
        Json(ContinuityEventResponse {
            schema_version: "arda.continuity-event-response.v1",
            status: "accepted",
            replayed: false,
            receipt_ref: format!("arda://continuity/receipts/{}", receipt.receipt_id),
        }),
    ))
}

pub async fn create_handoff(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<CreateHandoffRequest>,
) -> Result<(StatusCode, Json<HandoffResponse>), ApiError> {
    require_loopback(peer)?;
    require_gateway_operator(&state, &request.operator)?;
    if request.handoff.operator_ref != state.operator_id {
        return Err(ApiError::forbidden(
            "handoff operator does not match configured authority",
        ));
    }
    request
        .handoff
        .validate(Utc::now())
        .map_err(handoff_error)?;
    if request.handoff.state != HandoffState::Requested {
        return Err(ApiError::bad_request(
            "new handoff must begin in requested state",
        ));
    }
    let root = continuity_root(&state);
    let _guard = CONTINUITY_WRITE
        .lock()
        .map_err(|_| ApiError::internal("continuity writer lock poisoned"))?;
    fs::create_dir_all(root.join("handoffs")).map_err(io_error)?;
    let snapshot = handoff_path(&root, &request.handoff.handoff_id)?;
    if snapshot.exists() {
        let existing = read_handoff(&snapshot)?;
        request
            .handoff
            .validate_replay(&existing)
            .map_err(handoff_error)?;
        let mut candidate = request.handoff.clone();
        candidate.state = HandoffState::Prepared;
        if candidate != existing {
            return Err(ApiError::conflict(
                "surface handoff replay altered the prepared payload",
            ));
        }
        return Ok((
            StatusCode::OK,
            Json(HandoffResponse {
                schema_version: "arda.surface-handoff-response.v1",
                handoff: existing,
                replayed: true,
                receipt_ref: receipt_ref(&request.handoff.idempotency_key),
            }),
        ));
    }
    let mut prepared = request.handoff;
    let from = prepared.state;
    prepared.state = HandoffState::Prepared;
    let payload_hash = hash_json(&prepared)?;
    let receipt = TransitionReceipt {
        schema_version: "arda.continuity-receipt.v1".into(),
        receipt_id: receipt_id(&prepared.idempotency_key),
        kind: "handoff_prepared".into(),
        target_ref: prepared.handoff_id.clone(),
        operator_ref: prepared.operator_ref.clone(),
        session_lineage_id: prepared.session_lineage_id.clone(),
        idempotency_key: prepared.idempotency_key.clone(),
        from_state: Some(from),
        to_state: Some(prepared.state),
        payload_sha256: payload_hash,
        recorded_at: Utc::now(),
    };
    append_jsonl(&root.join("receipts.jsonl"), &receipt)?;
    atomic_json(&snapshot, &prepared)?;
    Ok((
        StatusCode::CREATED,
        Json(HandoffResponse {
            schema_version: "arda.surface-handoff-response.v1",
            handoff: prepared,
            replayed: false,
            receipt_ref: format!("arda://continuity/receipts/{}", receipt.receipt_id),
        }),
    ))
}

pub async fn accept_handoff(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Path(id): Path<String>,
    Json(request): Json<AcceptHandoffRequest>,
) -> Result<Json<HandoffResponse>, ApiError> {
    require_loopback(peer)?;
    if request.operator_ref != state.operator_id {
        return Err(ApiError::forbidden(
            "handoff acceptance requires configured operator authority",
        ));
    }
    validate_idempotency_key(&request.idempotency_key)?;
    let root = continuity_root(&state);
    let _guard = CONTINUITY_WRITE
        .lock()
        .map_err(|_| ApiError::internal("continuity writer lock poisoned"))?;
    let snapshot = handoff_path(&root, &id)?;
    if !snapshot.exists() {
        return Err(ApiError::not_found("surface handoff not found"));
    }
    let current = read_handoff(&snapshot)?;
    if current.state == HandoffState::Accepted {
        let receipt = find_receipt_by_key(&root.join("receipts.jsonl"), &request.idempotency_key)?;
        if !receipt.is_some_and(|receipt| {
            receipt.kind == "handoff_accepted" && receipt.target_ref == current.handoff_id
        }) {
            return Err(ApiError::conflict(
                "accepted handoff replay used an unknown idempotency key",
            ));
        }
        return Ok(Json(HandoffResponse {
            schema_version: "arda.surface-handoff-response.v1",
            handoff: current,
            replayed: true,
            receipt_ref: receipt_ref(&request.idempotency_key),
        }));
    }
    if current.expires_at <= Utc::now() {
        return Err(ApiError::conflict(
            "surface handoff expired before acceptance",
        ));
    }
    let from = current.state;
    let mut accepted = current.clone();
    accepted.state = HandoffState::Accepted;
    accepted.consent.state = arda_orome::ConsentState::Granted;
    accepted.accepted_at = Some(Utc::now());
    current
        .validate_transition(&accepted)
        .map_err(handoff_error)?;
    let payload_hash = hash_json(&accepted)?;
    let receipt = TransitionReceipt {
        schema_version: "arda.continuity-receipt.v1".into(),
        receipt_id: receipt_id(&request.idempotency_key),
        kind: "handoff_accepted".into(),
        target_ref: accepted.handoff_id.clone(),
        operator_ref: request.operator_ref,
        session_lineage_id: accepted.session_lineage_id.clone(),
        idempotency_key: request.idempotency_key.clone(),
        from_state: Some(from),
        to_state: Some(accepted.state),
        payload_sha256: payload_hash,
        recorded_at: Utc::now(),
    };
    append_jsonl(&root.join("receipts.jsonl"), &receipt)?;
    atomic_json(&snapshot, &accepted)?;
    Ok(Json(HandoffResponse {
        schema_version: "arda.surface-handoff-response.v1",
        handoff: accepted,
        replayed: false,
        receipt_ref: format!("arda://continuity/receipts/{}", receipt.receipt_id),
    }))
}

pub async fn get_handoff(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<HandoffResponse>, ApiError> {
    require_loopback(peer)?;
    require_operator_header(&state, &headers)?;
    let handoff = read_handoff(&handoff_path(&continuity_root(&state), &id)?)?;
    Ok(Json(HandoffResponse {
        schema_version: "arda.surface-handoff-response.v1",
        receipt_ref: receipt_ref(&handoff.idempotency_key),
        handoff,
        replayed: false,
    }))
}

pub async fn get_session(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(lineage): Path<String>,
) -> Result<Json<SessionContinuityProjection>, ApiError> {
    require_loopback(peer)?;
    require_operator_header(&state, &headers)?;
    validate_path_id(&lineage)?;
    let root = continuity_root(&state);
    let events = read_events(&root.join("events.jsonl"))?;
    let latest = events
        .iter()
        .filter(|event| event.session_lineage_id == lineage)
        .max_by_key(|event| event.observed_at);
    let handoffs = read_handoffs(&root.join("handoffs"))?
        .into_iter()
        .filter(|handoff| handoff.session_lineage_id == lineage)
        .collect::<Vec<_>>();
    if latest.is_none() && handoffs.is_empty() {
        return Err(ApiError::not_found("continuity session lineage not found"));
    }
    let now = Utc::now();
    Ok(Json(SessionContinuityProjection {
        schema_version: "arda.continuity-session.v1",
        session_lineage_id: lineage,
        current_session_id: latest.map(|event| event.current_session_id.clone()),
        active_surface_id: latest.map(|event| event.surface_id.clone()),
        privacy_class: latest.map(|event| event.privacy_class),
        topic_refs: latest
            .map(|event| event.topic_refs.clone())
            .unwrap_or_default(),
        commitment_refs: latest
            .map(|event| event.commitment_refs.clone())
            .unwrap_or_default(),
        memory_scope_refs: latest
            .map(|event| event.memory_scope_refs.clone())
            .unwrap_or_default(),
        handoff_refs: handoffs
            .iter()
            .map(|handoff| format!("arda://continuity/handoffs/{}", handoff.handoff_id))
            .collect(),
        observed_at: latest.map(|event| event.observed_at),
        freshness: if latest.is_some_and(|event| event.expires_at > now) {
            "fresh"
        } else {
            "stale"
        },
    }))
}

pub async fn get_projection(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Result<Json<HudContinuityProjection>, ApiError> {
    require_loopback(peer)?;
    require_operator_header(&state, &headers)?;
    let now = Utc::now();
    let root = continuity_root(&state);
    let events = read_events(&root.join("events.jsonl"))?;
    let latest = events.iter().max_by_key(|event| event.observed_at);
    let handoff = read_handoffs(&root.join("handoffs"))?
        .into_iter()
        .max_by_key(|handoff| handoff.issued_at);
    let Some(event) = latest else {
        return Ok(Json(HudContinuityProjection {
            schema_version: "arda.continuity-projection.v1",
            generated_at: now,
            active: false,
            session_lineage_id: None,
            current_session_id: None,
            surface_id: None,
            privacy_class: None,
            freshness: "unavailable",
            handoff_id: handoff.as_ref().map(|value| value.handoff_id.clone()),
            handoff_state: handoff.as_ref().map(|value| value.state),
            action_ids: Vec::new(),
            private_refs_withheld: false,
            topic_refs: Vec::new(),
            commitment_refs: Vec::new(),
            memory_scope_refs: Vec::new(),
        }));
    };
    let private_refs_withheld = matches!(
        event.privacy_class,
        PrivacyClass::PublicRoom | PrivacyClass::SharedRoom
    );
    let action_ids = if handoff
        .as_ref()
        .is_some_and(|value| value.state == HandoffState::Prepared && value.expires_at > now)
    {
        vec!["continue_here"]
    } else {
        Vec::new()
    };
    Ok(Json(HudContinuityProjection {
        schema_version: "arda.continuity-projection.v1",
        generated_at: now,
        active: event.expires_at > now,
        session_lineage_id: Some(event.session_lineage_id.clone()),
        current_session_id: Some(event.current_session_id.clone()),
        surface_id: Some(event.surface_id.clone()),
        privacy_class: Some(event.privacy_class),
        freshness: if event.expires_at > now {
            "fresh"
        } else {
            "stale"
        },
        handoff_id: handoff.as_ref().map(|value| value.handoff_id.clone()),
        handoff_state: handoff.as_ref().map(|value| value.state),
        action_ids,
        private_refs_withheld,
        topic_refs: if private_refs_withheld {
            Vec::new()
        } else {
            event.topic_refs.clone()
        },
        commitment_refs: if private_refs_withheld {
            Vec::new()
        } else {
            event.commitment_refs.clone()
        },
        memory_scope_refs: if private_refs_withheld {
            Vec::new()
        } else {
            event.memory_scope_refs.clone()
        },
    }))
}

fn require_gateway_operator(
    state: &HarnessState,
    operator: &OperatorIdentity,
) -> Result<(), ApiError> {
    if !operator.authenticated
        || operator.authentication_method != "gateway_identity"
        || operator.operator_id != state.operator_id
    {
        return Err(ApiError::forbidden(
            "continuity event requires configured Hermes Gateway operator identity",
        ));
    }
    Ok(())
}

fn require_operator_header(state: &HarnessState, headers: &HeaderMap) -> Result<(), ApiError> {
    let observed = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok());
    if observed != Some(state.operator_id.as_str()) {
        return Err(ApiError::forbidden(
            "continuity read requires configured operator identity",
        ));
    }
    Ok(())
}

fn continuity_root(state: &HarnessState) -> PathBuf {
    state.workbench_root.join("core/state/continuity")
}

fn validate_idempotency_key(key: &str) -> Result<(), ApiError> {
    let digest = key.strip_prefix("sha256:");
    if !digest.is_some_and(|value| {
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    }) {
        return Err(ApiError::bad_request("invalid continuity idempotency key"));
    }
    Ok(())
}

fn validate_path_id(id: &str) -> Result<(), ApiError> {
    if id.trim().is_empty()
        || id.len() > MAX_ID
        || id.contains('/')
        || id.contains('\\')
        || id.contains("..")
    {
        return Err(ApiError::bad_request("invalid continuity path identity"));
    }
    Ok(())
}

fn handoff_path(root: &FsPath, id: &str) -> Result<PathBuf, ApiError> {
    validate_path_id(id)?;
    Ok(root.join("handoffs").join(format!("{id}.json")))
}

fn hash_json<T: Serialize>(value: &T) -> Result<String, ApiError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| ApiError::internal(format!("continuity serialization failed: {error}")))?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn receipt_id(key: &str) -> String {
    format!(
        "continuity-{}",
        &format!("{:x}", Sha256::digest(key.as_bytes()))[..24]
    )
}

fn receipt_ref(key: &str) -> String {
    format!("arda://continuity/receipts/{}", receipt_id(key))
}

fn continuity_event_receipt(
    event: &ContinuityEvent,
    operator_ref: &str,
    payload_sha256: String,
) -> TransitionReceipt {
    TransitionReceipt {
        schema_version: "arda.continuity-receipt.v1".into(),
        receipt_id: receipt_id(&event.idempotency_key),
        kind: "continuity_event".into(),
        target_ref: event.event_id.clone(),
        operator_ref: operator_ref.to_owned(),
        session_lineage_id: event.session_lineage_id.clone(),
        idempotency_key: event.idempotency_key.clone(),
        from_state: None,
        to_state: None,
        payload_sha256,
        recorded_at: Utc::now(),
    }
}

fn append_jsonl<T: Serialize>(path: &FsPath, value: &T) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(io_error)?;
    serde_json::to_writer(&mut file, value)
        .map_err(|error| ApiError::internal(format!("continuity serialization failed: {error}")))?;
    file.write_all(b"\n").map_err(io_error)?;
    file.sync_data().map_err(io_error)
}

fn atomic_json<T: Serialize>(path: &FsPath, value: &T) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    let temporary = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| ApiError::internal(format!("continuity serialization failed: {error}")))?;
    fs::write(&temporary, bytes).map_err(io_error)?;
    fs::rename(&temporary, path).map_err(io_error)
}

fn find_event_by_key(path: &FsPath, key: &str) -> Result<Option<ContinuityEvent>, ApiError> {
    Ok(read_events(path)?
        .into_iter()
        .find(|event| event.idempotency_key == key))
}

fn read_events(path: &FsPath) -> Result<Vec<ContinuityEvent>, ApiError> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    fs::read_to_string(path)
        .map_err(io_error)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line).map_err(|error| {
                ApiError::internal(format!("invalid continuity event ledger: {error}"))
            })
        })
        .collect()
}

fn find_receipt_by_key(path: &FsPath, key: &str) -> Result<Option<TransitionReceipt>, ApiError> {
    if !path.exists() {
        return Ok(None);
    }
    for line in fs::read_to_string(path).map_err(io_error)?.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let receipt: TransitionReceipt = serde_json::from_str(line).map_err(|error| {
            ApiError::internal(format!("invalid continuity receipt ledger: {error}"))
        })?;
        if receipt.idempotency_key == key {
            return Ok(Some(receipt));
        }
    }
    Ok(None)
}

fn read_handoff(path: &FsPath) -> Result<SurfaceHandoff, ApiError> {
    let bytes = fs::read(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ApiError::not_found("surface handoff not found")
        } else {
            io_error(error)
        }
    })?;
    serde_json::from_slice(&bytes)
        .map_err(|error| ApiError::internal(format!("invalid surface handoff state: {error}")))
}

fn read_handoffs(root: &FsPath) -> Result<Vec<SurfaceHandoff>, ApiError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut handoffs = Vec::new();
    for entry in fs::read_dir(root).map_err(io_error)? {
        let path = entry.map_err(io_error)?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            handoffs.push(read_handoff(&path)?);
        }
    }
    Ok(handoffs)
}

fn handoff_error(error: SurfaceHandoffError) -> ApiError {
    match error {
        SurfaceHandoffError::DomainEscalation | SurfaceHandoffError::InvalidConsent => {
            ApiError::forbidden(error.to_string())
        }
        SurfaceHandoffError::ReplayMismatch | SurfaceHandoffError::IllegalTransition => {
            ApiError::conflict(error.to_string())
        }
        _ => ApiError::bad_request(error.to_string()),
    }
}

fn io_error(error: std::io::Error) -> ApiError {
    ApiError::internal(format!("continuity persistence failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_is_required_for_continuity() {
        let error = require_loopback("192.0.2.1:1234".parse().unwrap()).unwrap_err();
        assert!(format!("{error:?}").contains("loopback_required"));
    }
}
