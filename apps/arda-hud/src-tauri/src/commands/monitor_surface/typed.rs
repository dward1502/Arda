use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::commands::monitor_surface::contract::MonitorSurfaceContractState;
use crate::commands::monitor_surface::registry::{
    validate_session_content, ActiveSessionProjection, MonitorSessionRecord,
    SessionRegistryDocument, WorkstationHandoff,
};

const REGISTRY_CHANGED_EVENT: &str = "monitor-surface-registry-changed";

#[derive(Debug)]
pub struct TypedMonitorSurfaceState {
    contract: MonitorSurfaceContractState,
}

impl Default for TypedMonitorSurfaceState {
    fn default() -> Self {
        Self::new()
    }
}

impl TypedMonitorSurfaceState {
    pub fn new() -> Self {
        Self {
            contract: MonitorSurfaceContractState::new(),
        }
    }

    pub fn claim_session(
        &self,
        request: MonitorSessionRecord,
    ) -> Result<ActiveSessionProjection, String> {
        self.contract.claim_session(request)
    }

    pub fn release_session(
        &self,
        slot_id: &str,
        owner: &str,
    ) -> Result<SessionRegistryDocument, String> {
        self.contract.release_session(slot_id, owner)
    }

    pub fn refresh_session(
        &self,
        slot_id: &str,
        owner: &str,
        revision: u64,
        ttl_secs: u64,
    ) -> Result<ActiveSessionProjection, String> {
        self.contract
            .refresh_session(slot_id, owner, revision, ttl_secs)
    }

    pub fn snapshot(&self) -> SessionRegistryDocument {
        self.contract.session_registry()
    }

    pub fn restore(&self, document: SessionRegistryDocument) -> Result<(), String> {
        self.contract.restore(document)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedMonitorClaimRequest {
    pub slot_id: String,
    pub owner: String,
    pub content: serde_json::Value,
    pub workstation_handoff: WorkstationHandoff,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedMonitorClaimResult {
    pub ok: bool,
    pub message: String,
    pub slot_id: String,
    pub registry: SessionRegistryDocument,
    pub session: Option<MonitorSessionRecord>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSurfaceRegistryChangedEvent {
    pub operation: String,
    pub slot_id: String,
    pub registry: SessionRegistryDocument,
    pub session: Option<MonitorSessionRecord>,
}

fn emit_registry_changed(
    app: &AppHandle,
    operation: &str,
    slot_id: &str,
    registry: &SessionRegistryDocument,
    session: Option<MonitorSessionRecord>,
) {
    let _ = app.emit(
        REGISTRY_CHANGED_EVENT,
        MonitorSurfaceRegistryChangedEvent {
            operation: operation.to_string(),
            slot_id: slot_id.to_string(),
            registry: registry.clone(),
            session,
        },
    );
}

fn find_session_by_surface_id(
    registry: &SessionRegistryDocument,
    surface_session_id: &str,
) -> Result<MonitorSessionRecord, String> {
    registry
        .sessions
        .values()
        .find(|record| record.surface_session_id == surface_session_id)
        .cloned()
        .ok_or_else(|| format!("monitor session '{}' not found", surface_session_id))
}

#[tauri::command]
pub fn claim_monitor_surface(
    app: AppHandle,
    state: State<'_, TypedMonitorSurfaceState>,
    request: TypedMonitorClaimRequest,
) -> Result<TypedMonitorClaimResult, String> {
    if request.ttl_secs == 0 {
        return Err("monitor session ttl_secs must be greater than 0".to_string());
    }
    let kind = validate_session_content(&request.content)?;
    let now = Utc::now();
    let surface_session_id = format!("surface-{}", now.timestamp_micros());
    let session = MonitorSessionRecord {
        slot_id: request.slot_id.clone(),
        session_id: surface_session_id.clone(),
        surface_session_id: surface_session_id.clone(),
        owner: request.owner,
        kind,
        revision: 1,
        opened_at_utc: now.to_rfc3339(),
        lease_expires_at_utc: (now + chrono::Duration::seconds(request.ttl_secs as i64))
            .to_rfc3339(),
        content: request.content,
        workstation_handoff: WorkstationHandoff {
            session_id: surface_session_id,
            mode: request.workstation_handoff.mode,
        },
        created_at_utc: now.to_rfc3339(),
        updated_at_utc: now.to_rfc3339(),
    };

    state.claim_session(session.clone())?;
    let registry = state.snapshot();
    emit_registry_changed(
        &app,
        "claim",
        &request.slot_id,
        &registry,
        Some(session.clone()),
    );

    Ok(TypedMonitorClaimResult {
        ok: true,
        message: format!("Claim accepted for slot '{}'", request.slot_id),
        slot_id: request.slot_id,
        registry,
        session: Some(session),
    })
}

#[tauri::command]
pub fn release_monitor_surface(
    app: AppHandle,
    state: State<'_, TypedMonitorSurfaceState>,
    surface_session_id: String,
    owner: String,
) -> Result<TypedMonitorClaimResult, String> {
    let record = find_session_by_surface_id(&state.snapshot(), &surface_session_id)?;
    if record.owner != owner {
        return Err(format!(
            "session for slot '{}' is owned by '{}'",
            record.slot_id, record.owner
        ));
    }
    let registry = state.release_session(&record.slot_id, &owner)?;
    emit_registry_changed(&app, "release", &record.slot_id, &registry, None);

    Ok(TypedMonitorClaimResult {
        ok: true,
        message: format!("Released session for slot '{}'", record.slot_id),
        slot_id: record.slot_id,
        registry,
        session: None,
    })
}

#[tauri::command]
pub fn refresh_monitor_surface_lease(
    app: AppHandle,
    state: State<'_, TypedMonitorSurfaceState>,
    surface_session_id: String,
    owner: String,
    ttl_secs: u64,
) -> Result<TypedMonitorClaimResult, String> {
    if ttl_secs == 0 {
        return Err("monitor session ttl_secs must be greater than 0".to_string());
    }
    let record = find_session_by_surface_id(&state.snapshot(), &surface_session_id)?;
    if record.owner != owner {
        return Err(format!(
            "session for slot '{}' is owned by '{}'",
            record.slot_id, record.owner
        ));
    }
    state.refresh_session(&record.slot_id, &owner, record.revision + 1, ttl_secs)?;
    let registry = state.snapshot();
    let session = registry.sessions.get(&record.slot_id).cloned();
    emit_registry_changed(&app, "refresh", &record.slot_id, &registry, session.clone());

    Ok(TypedMonitorClaimResult {
        ok: true,
        message: format!("Lease refreshed for slot '{}'", record.slot_id),
        slot_id: record.slot_id,
        registry,
        session,
    })
}

#[tauri::command]
pub fn get_monitor_surface_registry(
    state: State<'_, TypedMonitorSurfaceState>,
) -> Result<SessionRegistryDocument, String> {
    Ok(state.snapshot())
}

#[tauri::command]
pub fn restore_monitor_surface_registry(
    app: AppHandle,
    state: State<'_, TypedMonitorSurfaceState>,
    document: SessionRegistryDocument,
) -> Result<(), String> {
    state.restore(document)?;
    let registry = state.snapshot();
    emit_registry_changed(&app, "restore", "", &registry, None);
    Ok(())
}
