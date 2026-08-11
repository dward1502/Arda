use chrono::Utc;
use serde::Serialize;
use serde_json::Value;

use crate::{charon_socket_addr, read_local_http_json};

pub const MANWE_RUNTIME_PROJECTION_SCHEMA_VERSION: &str = "arda.system-health.manwe.v1";

const MANWE_SOURCES: [(&str, &str); 3] = [
    ("health", "/healthz"),
    ("capabilities", "/providers/capabilities"),
    ("provider_candidates", "/provider_candidates"),
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManweProjectionSource {
    pub source_id: String,
    pub path: String,
    pub state: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ManweRuntimeProjection {
    pub schema_version: String,
    pub state: String,
    pub source_revision: String,
    pub source_time_utc: String,
    pub recovery_action: Option<String>,
    pub sources: Vec<ManweProjectionSource>,
    pub health: Option<Value>,
    pub capabilities: Option<Value>,
    pub provider_candidates: Option<Value>,
}

fn source_state(payload: &Value) -> &'static str {
    if payload.get("ok").and_then(Value::as_bool) == Some(false) {
        "degraded"
    } else {
        "observed"
    }
}

fn source_time(payload: &Value) -> Option<&str> {
    payload
        .pointer("/capabilities/generated_at_utc")
        .or_else(|| payload.pointer("/promotion_guard/generated_at_utc"))
        .or_else(|| payload.get("generated_at_utc"))
        .and_then(Value::as_str)
}

fn projection_revision(values: &[Option<Value>]) -> String {
    let mut revision = 0xcbf29ce484222325_u64;
    let mut update = |bytes: &[u8]| {
        for byte in bytes {
            revision ^= u64::from(*byte);
            revision = revision.wrapping_mul(0x100000001b3);
        }
    };
    update(MANWE_RUNTIME_PROJECTION_SCHEMA_VERSION.as_bytes());
    for value in values {
        let encoded = value
            .as_ref()
            .map(Value::to_string)
            .unwrap_or_else(|| "unavailable".to_string());
        update(encoded.as_bytes());
    }
    format!("manwe-{revision:016x}")
}

fn load_manwe_runtime_projection_with<F>(mut read: F) -> ManweRuntimeProjection
where
    F: FnMut(&str) -> Result<Value, String>,
{
    let observed_at = Utc::now().to_rfc3339();
    let mut values = Vec::with_capacity(MANWE_SOURCES.len());
    let mut sources = Vec::with_capacity(MANWE_SOURCES.len());

    for (source_id, path) in MANWE_SOURCES {
        match read(path) {
            Ok(payload) => {
                sources.push(ManweProjectionSource {
                    source_id: source_id.to_string(),
                    path: path.to_string(),
                    state: source_state(&payload).to_string(),
                    error: None,
                });
                values.push(Some(payload));
            }
            Err(error) => {
                sources.push(ManweProjectionSource {
                    source_id: source_id.to_string(),
                    path: path.to_string(),
                    state: "unavailable".to_string(),
                    error: Some(error),
                });
                values.push(None);
            }
        }
    }

    let available = values.iter().filter(|value| value.is_some()).count();
    let degraded = sources.iter().any(|source| source.state == "degraded");
    let state = match available {
        0 => "unavailable",
        count if count < MANWE_SOURCES.len() => "partial",
        _ if degraded => "degraded",
        _ => "healthy",
    };
    let source_time_utc = values
        .iter()
        .filter_map(Option::as_ref)
        .filter_map(source_time)
        .max()
        .unwrap_or(&observed_at)
        .to_string();
    let recovery_action = match state {
        "partial" => Some("Restore the unavailable Manwe projection source; observed sources remain authoritative.".to_string()),
        "degraded" => Some("Inspect Manwe source diagnostics before routing new work.".to_string()),
        "unavailable" => Some("Start or repair the configured Manwe runtime, then refresh system health.".to_string()),
        _ => None,
    };
    let source_revision = projection_revision(&values);
    let mut values = values.into_iter();

    ManweRuntimeProjection {
        schema_version: MANWE_RUNTIME_PROJECTION_SCHEMA_VERSION.to_string(),
        state: state.to_string(),
        source_revision,
        source_time_utc,
        recovery_action,
        sources,
        health: values.next().flatten(),
        capabilities: values.next().flatten(),
        provider_candidates: values.next().flatten(),
    }
}

#[tauri::command]
pub fn read_manwe_runtime_projection() -> ManweRuntimeProjection {
    let address = charon_socket_addr();
    load_manwe_runtime_projection_with(|path| match address {
        Ok(address) => read_local_http_json(address, path),
        Err(ref error) => Err(error.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn payload(path: &str) -> Value {
        match path {
            "/healthz" => json!({"ok": true, "providers_healthy": 4}),
            "/providers/capabilities" => json!({
                "ok": true,
                "capabilities": {"generated_at_utc": "2026-08-11T10:00:00Z"}
            }),
            "/provider_candidates" => json!({
                "ok": true,
                "promotion_guard": {"generated_at_utc": "2026-08-11T10:01:00Z"}
            }),
            _ => unreachable!(),
        }
    }

    #[test]
    fn projection_is_healthy_only_when_every_source_is_observed() {
        let projection = load_manwe_runtime_projection_with(|path| Ok(payload(path)));

        assert_eq!(
            projection.schema_version,
            MANWE_RUNTIME_PROJECTION_SCHEMA_VERSION
        );
        assert_eq!(projection.state, "healthy");
        assert_eq!(projection.source_time_utc, "2026-08-11T10:01:00Z");
        assert_eq!(
            projection.source_revision,
            load_manwe_runtime_projection_with(|path| Ok(payload(path))).source_revision
        );
        assert!(projection.recovery_action.is_none());
        assert!(projection.health.is_some());
        assert!(projection.capabilities.is_some());
        assert!(projection.provider_candidates.is_some());
    }

    #[test]
    fn projection_preserves_observed_sources_when_one_source_is_unavailable() {
        let projection = load_manwe_runtime_projection_with(|path| {
            if path == "/providers/capabilities" {
                Err("capability endpoint offline".to_string())
            } else {
                Ok(payload(path))
            }
        });

        assert_eq!(projection.state, "partial");
        assert!(projection.health.is_some());
        assert!(projection.capabilities.is_none());
        assert!(projection.provider_candidates.is_some());
        assert_eq!(projection.sources[1].state, "unavailable");
        assert!(projection.recovery_action.is_some());
    }

    #[test]
    fn projection_reports_degraded_authoritative_payload() {
        let projection = load_manwe_runtime_projection_with(|path| {
            let mut value = payload(path);
            if path == "/healthz" {
                value["ok"] = json!(false);
            }
            Ok(value)
        });

        assert_eq!(projection.state, "degraded");
        assert_eq!(projection.sources[0].state, "degraded");
    }

    #[test]
    fn projection_reports_total_runtime_unavailability() {
        let projection =
            load_manwe_runtime_projection_with(|path| Err(format!("{path} unavailable")));

        assert_eq!(projection.state, "unavailable");
        assert!(projection.health.is_none());
        assert!(projection.capabilities.is_none());
        assert!(projection.provider_candidates.is_none());
        assert!(projection
            .sources
            .iter()
            .all(|source| source.state == "unavailable"));
    }
}
