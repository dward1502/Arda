use std::sync::Mutex;

use crate::commands::monitor_surface::registry::{
    session_registry_document_json, validate_registry_document, validate_session_content,
    ActiveSessionProjection, MonitorSessionRecord, SessionRegistryDocument,
    MONITOR_SESSION_REGISTRY_SCHEMA_VERSION,
};

#[derive(Debug, Default)]
pub struct MonitorSurfaceContractState {
    registry: Mutex<SessionRegistryDocument>,
}

impl MonitorSurfaceContractState {
    pub fn new() -> Self {
        Self {
            registry: Mutex::new(SessionRegistryDocument {
                schema_version: MONITOR_SESSION_REGISTRY_SCHEMA_VERSION.to_string(),
                updated_at_utc: chrono::Utc::now().to_rfc3339(),
                sessions: Default::default(),
            }),
        }
    }

    pub fn claim_session(
        &self,
        request: MonitorSessionRecord,
    ) -> Result<ActiveSessionProjection, String> {
        if !is_canonical_slot(&request.slot_id) {
            return Err(format!(
                "slot_id '{}' is not a canonical monitor slot",
                request.slot_id
            ));
        }
        if request.revision == 0 {
            return Err("revision must be greater than 0".to_string());
        }
        let content_kind = validate_session_content(&request.content)?;
        if request.kind != content_kind {
            return Err(format!(
                "record kind '{}' does not match content.kind '{}' for slot '{}'",
                request.kind, content_kind, request.slot_id
            ));
        }

        let mut registry = self.registry.lock().unwrap();
        if let Some(existing) = registry.sessions.get(&request.slot_id) {
            if is_session_active(existing) && existing.owner != request.owner {
                return Err(format!(
                    "session for slot '{}' is already owned by '{}'",
                    request.slot_id, existing.owner
                ));
            }
        }
        registry.updated_at_utc = chrono::Utc::now().to_rfc3339();
        registry
            .sessions
            .insert(request.slot_id.clone(), request.clone());
        Ok(request.into())
    }

    pub fn refresh_session(
        &self,
        slot_id: &str,
        owner: &str,
        revision: u64,
        ttl_secs: u64,
    ) -> Result<ActiveSessionProjection, String> {
        let mut registry = self.registry.lock().unwrap();
        let record = registry
            .sessions
            .get_mut(slot_id)
            .ok_or_else(|| format!("no active session for slot '{}'", slot_id))?;
        if record.owner != owner {
            return Err(format!(
                "session for slot '{}' is owned by '{}'",
                slot_id, record.owner
            ));
        }
        if revision == 0 || revision < record.revision {
            return Err(format!("revision conflict for slot '{}'", slot_id));
        }
        validate_session_content(&record.content)?;
        record.revision = revision;
        record.lease_expires_at_utc =
            (chrono::Utc::now() + chrono::Duration::seconds(ttl_secs as i64)).to_rfc3339();
        record.updated_at_utc = chrono::Utc::now().to_rfc3339();
        let projection: ActiveSessionProjection = record.clone().into();
        registry.updated_at_utc = chrono::Utc::now().to_rfc3339();
        Ok(projection)
    }

    pub fn release_session(
        &self,
        slot_id: &str,
        owner: &str,
    ) -> Result<SessionRegistryDocument, String> {
        let mut registry = self.registry.lock().unwrap();
        let existing = registry
            .sessions
            .get(slot_id)
            .ok_or_else(|| format!("no active session for slot '{}'", slot_id))?;
        if existing.owner != owner {
            return Err(format!(
                "session for slot '{}' is owned by '{}'",
                slot_id, existing.owner
            ));
        }
        registry.sessions.remove(slot_id);
        registry.updated_at_utc = chrono::Utc::now().to_rfc3339();
        Ok(registry.clone())
    }

    pub fn active_session(&self, slot_id: &str) -> Option<MonitorSessionRecord> {
        let registry = self.registry.lock().unwrap();
        let record = registry.sessions.get(slot_id)?;
        if is_session_active(record) {
            Some(record.clone())
        } else {
            None
        }
    }

    pub fn active_snapshot(&self) -> SessionRegistryDocument {
        let registry = self.registry.lock().unwrap();
        SessionRegistryDocument {
            updated_at_utc: chrono::Utc::now().to_rfc3339(),
            ..registry.clone()
        }
    }

    pub fn session_json(&self) -> Result<String, String> {
        let registry = self.registry.lock().unwrap();
        session_registry_document_json(&registry)
    }

    pub fn session_registry(&self) -> SessionRegistryDocument {
        let registry = self.registry.lock().unwrap();
        registry.clone()
    }

    pub fn restore(&self, document: SessionRegistryDocument) -> Result<(), String> {
        validate_registry_document(&document)?;
        let mut registry = self.registry.lock().unwrap();
        *registry = document;
        Ok(())
    }
}

fn is_canonical_slot(slot_id: &str) -> bool {
    matches!(
        slot_id,
        "monitor_1" | "monitor_2" | "monitor_3" | "monitor_4" | "monitor_5"
    )
}

pub fn is_session_active(record: &MonitorSessionRecord) -> bool {
    crate::commands::monitor_surface::registry::is_session_active(record)
}
