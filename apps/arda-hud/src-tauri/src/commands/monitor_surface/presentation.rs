//! Local execution-adapter boundary. This is presentation, not knowledge retention
//! or an approval authority. Callers must already be authorized by their ingress.
use super::registry::{is_session_active, MonitorSessionRecord, WorkstationHandoff};
use super::typed::TypedMonitorSurfaceState;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationRequest {
    pub request_id: String,
    pub ambient_allowed: bool,
    pub operation: Operation,
    pub source: PresentationSource,
    pub slot_id: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Show,
    Play,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PresentationSource {
    Web { url: String },
    // Existing Arda-root-relative assets only; the acquisition adapter stages
    // authenticated attachments here instead of broadening the asset protocol.
    Asset { path: String, mime: String },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresentationResult {
    pub state: &'static str,
    pub session: MonitorSessionRecord,
}

fn content(source: &PresentationSource, operation: Operation) -> Result<Value, String> {
    match source {
        PresentationSource::Web { url } => {
            let parsed = tauri::Url::parse(url).map_err(|_| "invalid web URL")?;
            let host = parsed.host_str().ok_or("web URL requires a host")?;
            if !matches!(parsed.scheme(), "https" | "http")
                || !parsed.username().is_empty()
                || parsed.password().is_some()
                || host == "localhost"
                || host.ends_with(".localhost")
                || host.parse::<std::net::IpAddr>().is_ok()
                || !host.contains('.')
            {
                return Err(
                    "unsafe web source; use a public HTTP(S) hostname without credentials".into(),
                );
            }
            // Keep URLs verbatim. Native browser machinery owns navigation and
            // playback; publication alone is never proof of either outcome.
            if operation == Operation::Play {
                return Err("unsupported: web playback requires the existing browser playback capability; show is available".into());
            }
            Ok(
                json!({"kind":"web","url":url,"display":"capture_stream","sandboxProfile":"capture_stream"}),
            )
        }
        PresentationSource::Asset { path, mime } => {
            if !path.starts_with("data/media/imports/")
                || path.contains('\\')
                || std::path::Path::new(path)
                    .components()
                    .any(|p| !matches!(p, std::path::Component::Normal(_)))
            {
                return Err("asset must be a bounded data/media/imports path".into());
            }
            let source = json!({"kind":"local","path":path});
            match (mime.as_str(), operation) {
                ("image/png" | "image/jpeg" | "image/gif" | "image/webp", Operation::Show) => {
                    Ok(json!({"kind":"image","source":source,"fit":"contain"}))
                }
                ("application/pdf", Operation::Show) => {
                    Ok(json!({"kind":"document","source":source,"documentKind":"pdf"}))
                }
                ("text/plain" | "text/markdown", Operation::Show) => Ok(
                    json!({"kind":"document","source":source,"documentKind":if mime == "text/markdown" { "markdown" } else { "text" }}),
                ),
                ("video/mp4" | "video/webm", _) => Ok(
                    json!({"kind":"video","source":source,"mime":mime,"fit":"contain","autoplay":operation == Operation::Play,"muted":true}),
                ),
                _ => Err(format!(
                    "unsupported media/operation: {mime}; no native outcome is claimed"
                )),
            }
        }
    }
}

/// Ingress serialization is the adapter's responsibility. The registry's own
/// claim lock still rejects concurrent claims by different owners.
pub fn present(
    state: &TypedMonitorSurfaceState,
    request: PresentationRequest,
) -> Result<PresentationResult, String> {
    if !request.ambient_allowed {
        return Err("ambient presentation requires explicit operator permission".into());
    }
    if request.request_id.is_empty()
        || request.request_id.len() > 128
        || !request
            .request_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
    {
        return Err("requestId must be 1-128 ASCII identifier characters".into());
    }
    let descriptor = content(&request.source, request.operation)?;
    let owner = format!("presentation:{}", request.request_id);
    let snapshot = state.snapshot();
    if let Some(record) = snapshot
        .sessions
        .values()
        .find(|s| s.owner == owner && is_session_active(s))
    {
        if record.content != descriptor
            || request
                .slot_id
                .as_ref()
                .is_some_and(|id| id != &record.slot_id)
        {
            return Err("requestId replay has different content or target".into());
        }
        return Ok(PresentationResult {
            state: "published",
            session: record.clone(),
        });
    }
    let slots: Vec<String> = (1..=5).map(|n| format!("monitor_{n}")).collect();
    if request
        .slot_id
        .as_ref()
        .is_some_and(|id| !slots.contains(id))
    {
        return Err("target is not a canonical upper monitor".into());
    }
    let candidates = request.slot_id.map(|id| vec![id]).unwrap_or(slots);
    let slot = candidates
        .into_iter()
        .find(|id| !snapshot.sessions.get(id).is_some_and(is_session_active))
        .ok_or("deferred: requested monitors are occupied; no session was replaced")?;
    let now = Utc::now();
    let id = format!("presentation-{}", request.request_id);
    let session = MonitorSessionRecord {
        slot_id: slot,
        session_id: id.clone(),
        surface_session_id: id.clone(),
        owner,
        kind: descriptor["kind"].as_str().unwrap().to_string(),
        revision: 1,
        opened_at_utc: now.to_rfc3339(),
        lease_expires_at_utc: (now + Duration::hours(1)).to_rfc3339(),
        content: descriptor,
        playback: None,
        workstation_handoff: WorkstationHandoff {
            session_id: id,
            mode: "same_live_session".into(),
        },
        created_at_utc: now.to_rfc3339(),
        updated_at_utc: now.to_rfc3339(),
    };
    state.claim_session(session.clone())?;
    Ok(PresentationResult {
        state: "published",
        session,
    })
}
