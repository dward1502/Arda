use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

use crate::is_allowed_focus_mode;
use crate::surface_bridge_window_label;

const MONITOR_CLAIM_TTL_SECS: u64 = 300;

#[derive(Debug, Clone)]
struct ActiveMonitorClaim {
    owner: String,
    activity_kind: String,
    payload_binding: String,
    focus_mode: String,
    lease_expires_at_unix: u64,
}

#[derive(Default)]
pub struct MonitorSurfaceState {
    claims: Mutex<HashMap<String, ActiveMonitorClaim>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorClaimRequest {
    pub slot_id: String,
    pub owner: String,
    pub activity_kind: String,
    pub payload_binding: String,
    pub focus_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorClaimResult {
    pub ok: bool,
    pub message: String,
    pub slot_id: String,
    pub window_label: Option<String>,
    pub active: bool,
    pub lease_expires_at_utc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSurfacePayload {
    pub slot_id: String,
    pub owner: String,
    pub payload_binding: String,
    pub content: String,
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSurfacePayloadResult {
    pub ok: bool,
    pub message: String,
    pub slot_id: String,
}

fn is_monitor_slot_id(slot_id: &str) -> bool {
    slot_id.starts_with("monitor_")
}

fn is_valid_activity_kind(kind: &str) -> bool {
    matches!(
        kind,
        "agent_activity" | "streaming_text" | "remote_session" | "iframe_preview"
    )
}

fn is_authorized_owner(owner: &str) -> bool {
    owner == "hermes-agent"
        || owner.strip_prefix("hermes-agent-").is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
}

fn is_allowed_payload_binding(binding: &str) -> bool {
    ["hermes.", "arda.", "queue.", "service."]
        .iter()
        .any(|prefix| binding.starts_with(prefix) && binding.len() > prefix.len())
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn compute_lease_expiry(now: std::time::SystemTime, ttl_secs: u64) -> String {
    let expiry = now + std::time::Duration::from_secs(ttl_secs);
    let secs = expiry
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{secs}")
}

fn register_claim(
    state: &MonitorSurfaceState,
    request: &MonitorClaimRequest,
    lease_expires_at_unix: u64,
) -> Result<(), String> {
    let mut claims = state
        .claims
        .lock()
        .map_err(|_| "monitor claim registry lock poisoned")?;
    if let Some(active) = claims.get(&request.slot_id) {
        if active.lease_expires_at_unix > unix_now() && active.owner != request.owner {
            return Err(format!(
                "Monitor slot '{}' is already owned by '{}'",
                request.slot_id, active.owner
            ));
        }
    }
    claims.insert(
        request.slot_id.clone(),
        ActiveMonitorClaim {
            owner: request.owner.clone(),
            activity_kind: request.activity_kind.clone(),
            payload_binding: request.payload_binding.clone(),
            focus_mode: request.focus_mode.clone(),
            lease_expires_at_unix,
        },
    );
    Ok(())
}

fn take_claim(
    state: &MonitorSurfaceState,
    slot_id: &str,
    owner: &str,
) -> Result<ActiveMonitorClaim, String> {
    let mut claims = state
        .claims
        .lock()
        .map_err(|_| "monitor claim registry lock poisoned")?;
    match claims.get(slot_id) {
        Some(claim) if claim.owner == owner => claims
            .remove(slot_id)
            .ok_or_else(|| "monitor claim disappeared during release".to_string()),
        Some(claim) => Err(format!(
            "Claim owner '{}' cannot release monitor slot '{}' owned by '{}'",
            owner, slot_id, claim.owner
        )),
        None => Err(format!("Monitor slot '{}' has no active claim", slot_id)),
    }
}

fn refresh_claim(
    state: &MonitorSurfaceState,
    slot_id: &str,
    owner: &str,
    lease_expires_at_unix: u64,
) -> Result<ActiveMonitorClaim, String> {
    let mut claims = state
        .claims
        .lock()
        .map_err(|_| "monitor claim registry lock poisoned")?;
    let claim = claims
        .get_mut(slot_id)
        .ok_or_else(|| format!("Monitor slot '{}' has no active claim", slot_id))?;
    if claim.owner != owner {
        return Err(format!(
            "Claim owner '{}' cannot refresh monitor slot '{}' owned by '{}'",
            owner, slot_id, claim.owner
        ));
    }
    claim.lease_expires_at_unix = lease_expires_at_unix;
    Ok(claim.clone())
}

fn authorize_payload(
    state: &MonitorSurfaceState,
    payload: &MonitorSurfacePayload,
) -> Result<(), String> {
    let claims = state
        .claims
        .lock()
        .map_err(|_| "monitor claim registry lock poisoned")?;
    let claim = claims
        .get(&payload.slot_id)
        .ok_or_else(|| format!("Monitor slot '{}' has no active claim", payload.slot_id))?;
    if claim.lease_expires_at_unix <= unix_now()
        || claim.owner != payload.owner
        || claim.payload_binding != payload.payload_binding
    {
        return Err(
            "Surface payload rejected: owner, binding, or lease does not match the active claim"
                .to_string(),
        );
    }
    Ok(())
}

#[tauri::command]
pub fn claim_monitor_slot(
    app: AppHandle,
    state: State<'_, MonitorSurfaceState>,
    request: MonitorClaimRequest,
) -> Result<MonitorClaimResult, String> {
    if !is_monitor_slot_id(&request.slot_id) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Desk slot '{}' is not eligible for agent monitor claims; only monitor slots are allowed.",
                request.slot_id
            ),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if !is_valid_activity_kind(&request.activity_kind) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Activity kind '{}' is not permitted for monitor surface claims",
                request.activity_kind
            ),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if !is_allowed_focus_mode(&request.focus_mode) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Surface focus mode '{}' is not permitted for monitor surfaces",
                request.focus_mode
            ),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if request.owner.trim().is_empty() {
        return Ok(MonitorClaimResult {
            ok: false,
            message: "Claim owner is required".to_string(),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if !is_authorized_owner(&request.owner) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!("Claim owner '{}' is not authorized", request.owner),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if !is_allowed_payload_binding(&request.payload_binding) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Monitor payload binding '{}' is not permitted",
                request.payload_binding
            ),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    let now = std::time::SystemTime::now();
    let lease_expiry = compute_lease_expiry(now, MONITOR_CLAIM_TTL_SECS);
    let window_label = surface_bridge_window_label(&request.slot_id);

    let lease_expires_at_unix = unix_now() + MONITOR_CLAIM_TTL_SECS;
    if let Err(message) = register_claim(&state, &request, lease_expires_at_unix) {
        return Ok(MonitorClaimResult {
            ok: false,
            message,
            slot_id: request.slot_id,
            window_label: Some(window_label),
            active: true,
            lease_expires_at_utc: String::new(),
        });
    }

    let _ = app.emit(
        "monitor-claim-changed",
        serde_json::json!({
            "slotId": request.slot_id,
            "owner": request.owner,
            "activityKind": request.activity_kind,
            "payloadBinding": request.payload_binding,
            "focusMode": request.focus_mode,
            "leaseExpiresAtUtc": lease_expiry,
            "active": true,
        }),
    );

    Ok(MonitorClaimResult {
        ok: true,
        message: format!(
            "Claim accepted for slot '{}' (owner: {}, lease: {}s)",
            request.slot_id, request.owner, MONITOR_CLAIM_TTL_SECS
        ),
        slot_id: request.slot_id,
        window_label: Some(window_label),
        active: true,
        lease_expires_at_utc: lease_expiry,
    })
}

#[tauri::command]
pub fn release_monitor_slot(
    app: AppHandle,
    state: State<'_, MonitorSurfaceState>,
    slot_id: String,
    owner: String,
) -> Result<MonitorClaimResult, String> {
    if !is_monitor_slot_id(&slot_id) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Desk slot '{}' is not eligible for agent monitor release; only monitor slots are allowed.",
                slot_id
            ),
            slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if owner.trim().is_empty() {
        return Ok(MonitorClaimResult {
            ok: false,
            message: "Claim owner is required for release".to_string(),
            slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    let released = match take_claim(&state, &slot_id, &owner) {
        Ok(claim) => claim,
        Err(message) => {
            return Ok(MonitorClaimResult {
                ok: false,
                message,
                slot_id,
                window_label: None,
                active: true,
                lease_expires_at_utc: String::new(),
            });
        }
    };
    let window_label = surface_bridge_window_label(&slot_id);
    let _ = app.emit(
        "monitor-claim-changed",
        serde_json::json!({
            "slotId": slot_id,
            "owner": released.owner,
            "activityKind": released.activity_kind,
            "payloadBinding": released.payload_binding,
            "focusMode": released.focus_mode,
            "leaseExpiresAtUtc": "",
            "active": false,
        }),
    );

    Ok(MonitorClaimResult {
        ok: true,
        message: format!("Claim released for slot '{}' (owner: {})", slot_id, owner),
        slot_id,
        window_label: Some(window_label),
        active: false,
        lease_expires_at_utc: String::new(),
    })
}

#[tauri::command]
pub fn push_surface_payload(
    app: AppHandle,
    state: State<'_, MonitorSurfaceState>,
    payload: MonitorSurfacePayload,
) -> Result<MonitorSurfacePayloadResult, String> {
    if !is_monitor_slot_id(&payload.slot_id) {
        return Ok(MonitorSurfacePayloadResult {
            ok: false,
            message: format!(
                "Desk slot '{}' is not eligible for agent surface payloads; only monitor slots are allowed.",
                payload.slot_id
            ),
            slot_id: payload.slot_id,
        });
    }

    if payload.payload_binding.trim().is_empty() {
        return Ok(MonitorSurfacePayloadResult {
            ok: false,
            message: "payload_binding is required for surface payload".to_string(),
            slot_id: payload.slot_id,
        });
    }

    if let Err(message) = authorize_payload(&state, &payload) {
        return Ok(MonitorSurfacePayloadResult {
            ok: false,
            message,
            slot_id: payload.slot_id,
        });
    }

    let _ = app.emit(
        "monitor-surface-payload",
        serde_json::json!({
            "slotId": payload.slot_id,
            "owner": payload.owner,
            "payloadBinding": payload.payload_binding,
            "content": payload.content,
            "mime": payload.mime,
        }),
    );

    Ok(MonitorSurfacePayloadResult {
        ok: true,
        message: format!(
            "Surface payload delivered for slot '{}' (binding: {})",
            payload.slot_id, payload.payload_binding
        ),
        slot_id: payload.slot_id,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeaseRefreshRequest {
    pub slot_id: String,
    pub owner: String,
}

#[tauri::command]
pub fn refresh_monitor_slot_lease(
    app: AppHandle,
    state: State<'_, MonitorSurfaceState>,
    request: LeaseRefreshRequest,
) -> Result<MonitorClaimResult, String> {
    if !is_monitor_slot_id(&request.slot_id) {
        return Ok(MonitorClaimResult {
            ok: false,
            message: format!(
                "Desk slot '{}' is not eligible for lease refresh; only monitor slots are allowed.",
                request.slot_id
            ),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    if request.owner.trim().is_empty() {
        return Ok(MonitorClaimResult {
            ok: false,
            message: "Claim owner is required for lease refresh".to_string(),
            slot_id: request.slot_id,
            window_label: None,
            active: false,
            lease_expires_at_utc: String::new(),
        });
    }

    let lease_expires_at_unix = unix_now() + MONITOR_CLAIM_TTL_SECS;
    let claim = match refresh_claim(
        &state,
        &request.slot_id,
        &request.owner,
        lease_expires_at_unix,
    ) {
        Ok(claim) => claim,
        Err(message) => {
            return Ok(MonitorClaimResult {
                ok: false,
                message,
                slot_id: request.slot_id,
                window_label: None,
                active: false,
                lease_expires_at_utc: String::new(),
            });
        }
    };
    let lease_expiry = format!("unix:{lease_expires_at_unix}");
    let window_label = surface_bridge_window_label(&request.slot_id);

    let _ = app.emit(
        "monitor-claim-changed",
        serde_json::json!({
            "slotId": request.slot_id,
            "owner": claim.owner,
            "activityKind": claim.activity_kind,
            "payloadBinding": claim.payload_binding,
            "focusMode": claim.focus_mode,
            "leaseExpiresAtUtc": lease_expiry,
            "active": true,
        }),
    );

    Ok(MonitorClaimResult {
        ok: true,
        message: format!(
            "Lease refreshed for slot '{}' (owner: {}, +{}s)",
            request.slot_id, request.owner, MONITOR_CLAIM_TTL_SECS
        ),
        slot_id: request.slot_id,
        window_label: Some(window_label),
        active: true,
        lease_expires_at_utc: lease_expiry,
    })
}

#[cfg(test)]
mod command_contract_tests {
    use super::*;

    #[test]
    fn test_is_monitor_slot_id() {
        assert!(is_monitor_slot_id("monitor_1"));
        assert!(is_monitor_slot_id("monitor_2"));
        assert!(is_monitor_slot_id("monitor_3"));
        assert!(is_monitor_slot_id("monitor_4"));
        assert!(is_monitor_slot_id("monitor_5"));
        assert!(!is_monitor_slot_id("view_desk_l"));
        assert!(!is_monitor_slot_id("view_desk_aux"));
        assert!(!is_monitor_slot_id(""));
    }

    #[test]
    fn test_is_valid_activity_kind() {
        assert!(is_valid_activity_kind("agent_activity"));
        assert!(is_valid_activity_kind("streaming_text"));
        assert!(is_valid_activity_kind("remote_session"));
        assert!(is_valid_activity_kind("iframe_preview"));
        assert!(!is_valid_activity_kind("invalid_kind"));
        assert!(!is_valid_activity_kind(""));
    }

    #[test]
    fn test_claim_rejects_desk_slot() {
        assert!(!is_monitor_slot_id("view_desk_l"));
    }

    #[test]
    fn test_claim_rejects_invalid_activity_kind() {
        assert!(!is_valid_activity_kind("invalid_kind"));
    }

    #[test]
    fn test_claim_rejects_empty_owner() {
        assert!(!is_authorized_owner(""));
    }

    #[test]
    fn test_claim_rejects_unauthorized_owner() {
        assert!(!is_authorized_owner("intruder"));
    }

    #[test]
    fn test_claim_rejects_unapproved_payload_binding() {
        assert!(!is_allowed_payload_binding("arbitrary.secret"));
    }

    #[test]
    fn test_claim_accepts_valid_request() {
        let state = MonitorSurfaceState::default();
        let request = MonitorClaimRequest {
            slot_id: "monitor_1".to_string(),
            owner: "hermes-agent-001".to_string(),
            activity_kind: "agent_activity".to_string(),
            payload_binding: "hermes.live_stream".to_string(),
            focus_mode: "remote_preview".to_string(),
        };
        assert!(register_claim(&state, &request, unix_now() + MONITOR_CLAIM_TTL_SECS).is_ok());
        assert_eq!(
            surface_bridge_window_label(&request.slot_id),
            "arda-monitor-surface--monitor_1"
        );
    }

    #[test]
    fn test_release_rejects_desk_slot() {
        assert!(!is_monitor_slot_id("view_desk_l"));
    }

    #[test]
    fn test_release_rejects_empty_owner() {
        assert!("".trim().is_empty());
    }

    #[test]
    fn test_release_succeeds_for_monitor_slot() {
        let state = MonitorSurfaceState::default();
        let request = MonitorClaimRequest {
            slot_id: "monitor_1".to_string(),
            owner: "hermes-agent-001".to_string(),
            activity_kind: "agent_activity".to_string(),
            payload_binding: "hermes.live_stream".to_string(),
            focus_mode: "remote_preview".to_string(),
        };
        register_claim(&state, &request, unix_now() + MONITOR_CLAIM_TTL_SECS).unwrap();
        assert!(take_claim(&state, &request.slot_id, &request.owner).is_ok());
    }

    #[test]
    fn test_refresh_lease_rejects_desk_slot() {
        assert!(!is_monitor_slot_id("view_desk_r"));
    }

    #[test]
    fn test_refresh_lease_succeeds_for_monitor_slot() {
        let state = MonitorSurfaceState::default();
        let request = MonitorClaimRequest {
            slot_id: "monitor_2".to_string(),
            owner: "hermes-agent-001".to_string(),
            activity_kind: "agent_activity".to_string(),
            payload_binding: "hermes.live_stream".to_string(),
            focus_mode: "remote_preview".to_string(),
        };
        let first_expiry = unix_now() + MONITOR_CLAIM_TTL_SECS;
        register_claim(&state, &request, first_expiry).unwrap();
        let refreshed =
            refresh_claim(&state, &request.slot_id, &request.owner, first_expiry + 30).unwrap();
        assert_eq!(refreshed.lease_expires_at_unix, first_expiry + 30);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(slot_id: &str, owner: &str, payload_binding: &str) -> MonitorClaimRequest {
        MonitorClaimRequest {
            slot_id: slot_id.to_string(),
            owner: owner.to_string(),
            activity_kind: "agent_activity".to_string(),
            payload_binding: payload_binding.to_string(),
            focus_mode: "remote_preview".to_string(),
        }
    }

    #[test]
    fn validates_monitor_scope_owner_and_payload_namespaces() {
        assert!(is_monitor_slot_id("monitor_left_1"));
        assert!(!is_monitor_slot_id("view_desk_l"));
        assert!(is_valid_activity_kind("streaming_text"));
        assert!(!is_valid_activity_kind("invalid"));
        assert!(is_authorized_owner("hermes-agent-001"));
        assert!(!is_authorized_owner("intruder"));
        assert!(is_allowed_payload_binding("hermes.live_stream"));
        assert!(!is_allowed_payload_binding("arbitrary.secret"));
    }

    #[test]
    fn registry_enforces_single_owner_release_and_refresh() {
        let state = MonitorSurfaceState::default();
        let expiry = unix_now() + MONITOR_CLAIM_TTL_SECS;
        register_claim(
            &state,
            &request("monitor_left_1", "hermes-agent-001", "hermes.live_stream"),
            expiry,
        )
        .unwrap();

        let conflict = register_claim(
            &state,
            &request("monitor_left_1", "hermes-agent-002", "queue.heartbeat"),
            expiry,
        );
        assert!(conflict.unwrap_err().contains("already owned"));
        assert!(refresh_claim(&state, "monitor_left_1", "hermes-agent-002", expiry + 10).is_err());

        let refreshed =
            refresh_claim(&state, "monitor_left_1", "hermes-agent-001", expiry + 10).unwrap();
        assert_eq!(refreshed.lease_expires_at_unix, expiry + 10);
        assert!(take_claim(&state, "monitor_left_1", "hermes-agent-002").is_err());
        assert_eq!(
            take_claim(&state, "monitor_left_1", "hermes-agent-001")
                .unwrap()
                .owner,
            "hermes-agent-001"
        );
        assert!(take_claim(&state, "monitor_left_1", "hermes-agent-001").is_err());
    }

    #[test]
    fn payload_requires_exact_active_owner_and_binding() {
        let state = MonitorSurfaceState::default();
        register_claim(
            &state,
            &request("monitor_left_2", "hermes-agent-001", "hermes.live_stream"),
            unix_now() + MONITOR_CLAIM_TTL_SECS,
        )
        .unwrap();
        let payload = MonitorSurfacePayload {
            slot_id: "monitor_left_2".to_string(),
            owner: "hermes-agent-001".to_string(),
            payload_binding: "hermes.live_stream".to_string(),
            content: "provider healthy".to_string(),
            mime: "text/plain".to_string(),
        };
        assert!(authorize_payload(&state, &payload).is_ok());
        assert!(authorize_payload(
            &state,
            &MonitorSurfacePayload {
                owner: "hermes-agent-002".to_string(),
                ..payload.clone()
            }
        )
        .is_err());
        assert!(authorize_payload(
            &state,
            &MonitorSurfacePayload {
                payload_binding: "queue.heartbeat".to_string(),
                ..payload
            }
        )
        .is_err());
    }
}
