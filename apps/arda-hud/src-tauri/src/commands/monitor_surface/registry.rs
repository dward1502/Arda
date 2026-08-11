use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

pub const MONITOR_SESSION_REGISTRY_SCHEMA_VERSION: &str = "arda.monitor-session-registry.v2";
pub const MONITOR_SURFACE_SCHEMA_VERSION: &str = "arda.monitor-surface-session.v2";

#[derive(Debug, Default)]
pub struct MonitorSessionRegistryState {
    sessions: Mutex<HashMap<String, MonitorSessionRecord>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSessionRecord {
    pub slot_id: String,
    pub session_id: String,
    pub surface_session_id: String,
    pub owner: String,
    pub kind: String,
    pub revision: u64,
    pub opened_at_utc: String,
    pub lease_expires_at_utc: String,
    pub content: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback: Option<serde_json::Value>,
    pub workstation_handoff: WorkstationHandoff,
    pub created_at_utc: String,
    pub updated_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkstationHandoff {
    pub session_id: String,
    pub mode: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRegistryDocument {
    pub schema_version: String,
    pub updated_at_utc: String,
    pub sessions: HashMap<String, MonitorSessionRecord>,
}

impl MonitorSessionRegistryState {
    pub fn insert_session(&self, session: MonitorSessionRecord) {
        let mut guard = self.sessions.lock().unwrap();
        guard.insert(session.slot_id.clone(), session);
    }

    pub fn remove_session(&self, slot_id: &str) -> Option<MonitorSessionRecord> {
        let mut guard = self.sessions.lock().unwrap();
        guard.remove(slot_id)
    }

    pub fn active_session(&self, slot_id: &str) -> Option<MonitorSessionRecord> {
        let guard = self.sessions.lock().unwrap();
        let record = guard.get(slot_id)?;
        if is_session_active(record) {
            Some(record.clone())
        } else {
            None
        }
    }

    pub fn claim_snapshot(&self) -> SessionRegistryDocument {
        let guard = self.sessions.lock().unwrap();
        SessionRegistryDocument {
            schema_version: MONITOR_SESSION_REGISTRY_SCHEMA_VERSION.to_string(),
            updated_at_utc: chrono::Utc::now().to_rfc3339(),
            sessions: guard.clone(),
        }
    }

    pub fn restore(&self, document: SessionRegistryDocument) -> Result<(), String> {
        validate_registry_document(&document)?;
        let mut guard = self.sessions.lock().unwrap();
        *guard = document.sessions;
        Ok(())
    }
}

pub fn is_session_active(record: &MonitorSessionRecord) -> bool {
    chrono::DateTime::parse_from_rfc3339(&record.lease_expires_at_utc)
        .map(|expiry| chrono::Utc::now() < expiry)
        .unwrap_or(false)
}

pub fn validate_session_content(content: &serde_json::Value) -> Result<String, String> {
    let object = content
        .as_object()
        .ok_or_else(|| "monitor session content must be a JSON object".to_string())?;
    let kind = object
        .get("kind")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .ok_or_else(|| "monitor session content.kind must be a nonempty string".to_string())?;
    Ok(kind.to_string())
}

pub fn validate_playback_state(playback: &serde_json::Value) -> Result<(), String> {
    let object = playback
        .as_object()
        .ok_or_else(|| "monitor playback state must be a JSON object".to_string())?;
    if !object
        .get("playing")
        .is_some_and(serde_json::Value::is_boolean)
    {
        return Err("monitor playback state.playing must be a boolean".to_string());
    }
    for field in ["currentTime", "duration", "volume"] {
        if let Some(value) = object.get(field) {
            let number = value
                .as_f64()
                .ok_or_else(|| format!("monitor playback state.{field} must be a number"))?;
            if number < 0.0 || (field == "volume" && number > 1.0) {
                return Err(format!("monitor playback state.{field} is out of range"));
            }
        }
    }
    Ok(())
}

pub fn validate_registry_document(document: &SessionRegistryDocument) -> Result<(), String> {
    if document.schema_version != MONITOR_SESSION_REGISTRY_SCHEMA_VERSION {
        return Err(format!(
            "invalid session registry schema version '{}'; expected '{}'",
            document.schema_version, MONITOR_SESSION_REGISTRY_SCHEMA_VERSION
        ));
    }
    for (slot_id, record) in &document.sessions {
        if slot_id != &record.slot_id {
            return Err(format!(
                "registry key '{}' does not match record slot_id '{}'",
                slot_id, record.slot_id
            ));
        }
        let content_kind = validate_session_content(&record.content)?;
        if record.kind != content_kind {
            return Err(format!(
                "record kind '{}' does not match content.kind '{}' for slot '{}'",
                record.kind, content_kind, record.slot_id
            ));
        }
        if let Some(playback) = &record.playback {
            validate_playback_state(playback)?;
        }
    }
    Ok(())
}

pub fn session_registry_document_json(
    document: &SessionRegistryDocument,
) -> Result<String, String> {
    serde_json::to_string_pretty(document).map_err(|error| error.to_string())
}

pub fn parse_session_registry_document(value: Option<&str>) -> Option<SessionRegistryDocument> {
    let raw = value?.trim();
    if raw.is_empty() {
        return None;
    }
    match serde_json::from_str::<SessionRegistryDocument>(raw) {
        Ok(document) if validate_registry_document(&document).is_ok() => Some(document),
        _ => None,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSessionProjection {
    pub slot_id: String,
    pub session_id: String,
    pub surface_session_id: String,
    pub owner: String,
    pub kind: String,
    pub revision: u64,
    pub lease_expires_at_utc: String,
    pub content: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub playback: Option<serde_json::Value>,
    pub workstation_handoff: WorkstationHandoff,
}

impl From<MonitorSessionRecord> for ActiveSessionProjection {
    fn from(record: MonitorSessionRecord) -> Self {
        Self {
            slot_id: record.slot_id,
            session_id: record.session_id,
            surface_session_id: record.surface_session_id,
            owner: record.owner,
            kind: record.kind,
            revision: record.revision,
            lease_expires_at_utc: record.lease_expires_at_utc,
            content: record.content,
            playback: record.playback,
            workstation_handoff: record.workstation_handoff,
        }
    }
}
