use chrono::{DateTime, Utc};
use reqwest::Method;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

const DEFAULT_HARNESS_URL: &str = "http://127.0.0.1:7878";
pub const PERSONAL_OPS_PROJECTION_SCHEMA: &str = "arda.hud.personal-ops-projection.v1";
static MUTATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);
static PENDING_MUTATIONS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PersonalOpsLoadState {
    Healthy,
    Stale,
    Degraded,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PersonalOpsSnapshot {
    pub schema_version: String,
    pub state: PersonalOpsLoadState,
    pub source_revision: String,
    pub source_time_utc: DateTime<Utc>,
    pub inbox: Value,
    pub resume: Value,
    pub today_brief: Value,
    pub failures: Vec<String>,
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptureIntent {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClassificationIntent {
    pub item_id: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderAcknowledgementIntent {
    pub reminder_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletePersonalDataIntent {
    pub confirmation: String,
}

fn harness_url() -> String {
    std::env::var("ARDA_HARNESS_URL")
        .unwrap_or_else(|_| DEFAULT_HARNESS_URL.to_string())
        .trim_end_matches('/')
        .to_string()
}

fn endpoint(path: &str) -> String {
    format!("{}{path}", harness_url())
}

fn operator_id() -> Result<String, String> {
    std::env::var("ARDA_OPERATOR_ID")
        .map_err(|_| "ARDA_OPERATOR_ID is required for Personal Operations".to_string())
        .and_then(|value| {
            let value = value.trim().to_string();
            if value.is_empty() {
                Err("ARDA_OPERATOR_ID cannot be empty".to_string())
            } else {
                Ok(value)
            }
        })
}

fn checked_id<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(format!("{label} contains unsupported characters"));
    }
    Ok(value)
}

fn stable_hash(parts: &[&str]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for part in parts {
        for byte in part.as_bytes().iter().chain(std::iter::once(&0_u8)) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    hash
}

fn idempotency_key(operator_id: &str, action: &str, resource: &str) -> String {
    let sequence = MUTATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let issued_at = Utc::now().timestamp_nanos_opt().unwrap_or_default();
    format!(
        "personal-ops-{action}-{:016x}",
        stable_hash(&[
            operator_id,
            action,
            resource,
            &issued_at.to_string(),
            &sequence.to_string(),
        ])
    )
}

fn pending_idempotency_key(operator_id: &str, action: &str, resource: &str) -> String {
    let reference = format!("{operator_id}\0{action}\0{resource}");
    let mut pending = PENDING_MUTATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("personal operations pending mutation lock poisoned");
    pending
        .entry(reference)
        .or_insert_with(|| idempotency_key(operator_id, action, resource))
        .clone()
}

fn complete_pending_mutation(operator_id: &str, action: &str, resource: &str) {
    let reference = format!("{operator_id}\0{action}\0{resource}");
    PENDING_MUTATIONS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("personal operations pending mutation lock poisoned")
        .remove(&reference);
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<T>()
            .await
            .map_err(|error| format!("Personal Operations returned an invalid response: {error}"));
    }
    let detail = response.text().await.unwrap_or_default();
    Err(format!(
        "Personal Operations harness request failed ({status}): {detail}"
    ))
}

async fn request<T: DeserializeOwned>(
    method: Method,
    path: &str,
    operator_id: &str,
    body: Option<Value>,
    idempotency: Option<&str>,
) -> Result<T, String> {
    let client = reqwest::Client::new();
    let mut request = client
        .request(method, endpoint(path))
        .header("x-arda-operator-id", operator_id);
    if let Some(key) = idempotency {
        request = request.header("idempotency-key", key);
    }
    if let Some(body) = body {
        request = request.json(&body);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Unable to reach Personal Operations: {error}"))?;
    decode(response).await
}

async fn mutate<T: DeserializeOwned>(
    method: Method,
    path: &str,
    operator_id: &str,
    action: &str,
    resource: &str,
    body: Value,
) -> Result<T, String> {
    let key = pending_idempotency_key(operator_id, action, resource);
    let result = request(method, path, operator_id, Some(body), Some(&key)).await;
    if result.is_ok() {
        complete_pending_mutation(operator_id, action, resource);
    }
    result
}

fn project_snapshot(
    projection_response: Value,
    now: DateTime<Utc>,
) -> Result<PersonalOpsSnapshot, String> {
    let projection = projection_response
        .get("projection")
        .ok_or_else(|| "Personal Operations projection is missing projection data".to_string())?;
    let generated_at = projection
        .get("generated_at")
        .and_then(Value::as_str)
        .ok_or_else(|| "Personal Operations projection is missing generated_at".to_string())?;
    let source_time_utc = DateTime::parse_from_rfc3339(generated_at)
        .map_err(|_| "Personal Operations projection has invalid generated_at".to_string())?
        .with_timezone(&Utc);
    let age_seconds = now.signed_duration_since(source_time_utc).num_seconds();
    let state = if age_seconds > 300 {
        PersonalOpsLoadState::Stale
    } else {
        PersonalOpsLoadState::Healthy
    };
    let inbox_items = projection
        .get("inbox")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let today = projection
        .get("today")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let waiting = projection
        .get("waiting")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let event_count = projection
        .get("event_count")
        .and_then(Value::as_u64)
        .unwrap_or_default();
    let active_count = today
        .as_array()
        .into_iter()
        .flatten()
        .chain(waiting.as_array().into_iter().flatten())
        .filter_map(|item| item.get("item_id").and_then(Value::as_str))
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let summary = if active_count == 0 {
        if inbox_items.as_array().is_none_or(Vec::is_empty) {
            "Nothing in progress. Check your captures or scheduled items.".to_string()
        } else {
            format!(
                "{} capture(s) in the inbox awaiting classification.",
                inbox_items.as_array().map_or(0, Vec::len)
            )
        }
    } else {
        format!(
            "{active_count} active item(s) need attention: {} due today, {} waiting on reminders.",
            today.as_array().map_or(0, Vec::len),
            waiting.as_array().map_or(0, Vec::len)
        )
    };
    let reminders_awaiting_ack = today
        .as_array()
        .into_iter()
        .flatten()
        .filter(|item| {
            item.pointer("/reminder_state/policy/acknowledgement_required")
                .and_then(Value::as_bool)
                == Some(true)
                && item
                    .get("reminder_acknowledged_at")
                    .is_none_or(Value::is_null)
        })
        .count();
    let mut revision_projection = projection.clone();
    if let Some(object) = revision_projection.as_object_mut() {
        object.remove("generated_at");
    }
    let canonical = serde_json::to_string(&revision_projection)
        .map_err(|error| format!("Unable to version Personal Operations projection: {error}"))?;
    Ok(PersonalOpsSnapshot {
        schema_version: PERSONAL_OPS_PROJECTION_SCHEMA.to_string(),
        state,
        source_revision: format!("personal-ops-{:016x}", stable_hash(&[&canonical])),
        source_time_utc,
        inbox: json!({
            "schema_version": "arda.harness.personal-ops.v1",
            "inbox": inbox_items,
        }),
        resume: json!({
            "schema_version": "arda.harness.personal-ops.v1",
            "resume": {
                "summary": summary,
                "active_count": active_count,
                "inbox_count": projection.get("inbox").and_then(Value::as_array).map_or(0, Vec::len),
                "today_count": today.as_array().map_or(0, Vec::len),
                "waiting_count": waiting.as_array().map_or(0, Vec::len),
                "generated_at": generated_at,
            }
        }),
        today_brief: json!({
            "schema_version": "arda.harness.personal-ops.v1",
            "brief": {
                "generated_at": generated_at,
                "today": today,
                "waiting": waiting,
                "reminders_awaiting_ack": reminders_awaiting_ack,
                "quiet_mode": false,
                "uncertainty_disclosure": format!("Brief reconstructed from one durable projection revision containing {event_count} event(s)."),
            }
        }),
        failures: Vec::new(),
        recovery_action: (state == PersonalOpsLoadState::Stale)
            .then(|| "Refresh Personal Operations after restoring the harness owner.".to_string()),
    })
}

#[tauri::command]
pub async fn get_personal_ops_projection() -> Result<PersonalOpsSnapshot, String> {
    let operator_id = operator_id()?;
    let projection = request::<Value>(
        Method::GET,
        "/v1/personal-ops/projection",
        &operator_id,
        None,
        None,
    )
    .await?;
    project_snapshot(projection, Utc::now())
}

#[tauri::command]
pub async fn create_personal_capture(intent: CaptureIntent) -> Result<Value, String> {
    let operator_id = operator_id()?;
    let text = intent.text.trim();
    if text.is_empty() {
        return Err("Personal capture text cannot be empty".to_string());
    }
    mutate(
        Method::POST,
        "/v1/personal/captures",
        &operator_id,
        "capture",
        text,
        json!({ "operator_id": operator_id, "text": text }),
    )
    .await
}

#[tauri::command]
pub async fn confirm_personal_classification(
    intent: ClassificationIntent,
) -> Result<Value, String> {
    let operator_id = operator_id()?;
    let item_id = checked_id(&intent.item_id, "item_id")?;
    let kind = match intent.kind.to_lowercase().as_str() {
        "task" | "reminder" | "note" | "appointment" | "contact" | "health" => {
            intent.kind.to_lowercase()
        }
        _ => return Err("unsupported Personal Operations classification kind".to_string()),
    };
    let resource = format!("{item_id}:{kind}");
    mutate(
        Method::POST,
        &format!("/v1/personal/items/{item_id}/classify"),
        &operator_id,
        "classify",
        &resource,
        json!({
            "operator_id": operator_id,
            "item_id": item_id,
            "kind": kind,
            "evidence_class": "operator_authored",
            "rationale": "Confirmed in bounded HUD review",
        }),
    )
    .await
}

#[tauri::command]
pub async fn acknowledge_personal_reminder(
    intent: ReminderAcknowledgementIntent,
) -> Result<Value, String> {
    let operator_id = operator_id()?;
    let reminder_id = checked_id(&intent.reminder_id, "reminder_id")?;
    mutate(
        Method::POST,
        &format!("/v1/personal/reminders/{reminder_id}/acknowledge"),
        &operator_id,
        "acknowledge",
        reminder_id,
        json!({ "operator_id": operator_id, "state": "acknowledged" }),
    )
    .await
}

#[tauri::command]
pub async fn export_personal_data() -> Result<Value, String> {
    let operator_id = operator_id()?;
    request(
        Method::GET,
        "/v1/personal/data/export",
        &operator_id,
        None,
        None,
    )
    .await
}

#[tauri::command]
pub async fn delete_personal_data(intent: DeletePersonalDataIntent) -> Result<Value, String> {
    if intent.confirmation != "delete-personal-data" {
        return Err("Personal data deletion requires an exact confirmation".to_string());
    }
    let operator_id = operator_id()?;
    mutate(
        Method::DELETE,
        "/v1/personal/data",
        &operator_id,
        "delete-personal-data",
        &intent.confirmation,
        json!({ "operator_id": operator_id }),
    )
    .await
}

#[cfg(test)]
#[path = "personal_ops_tests.rs"]
mod tests;
