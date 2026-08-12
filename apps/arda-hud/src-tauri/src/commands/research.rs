use arda_outpost_protocol::{
    ContradictionPolicy, ResearchQuestion, ResearchWatchlist, WatchlistBudgets, WatchlistCadence,
    WatchlistEvidenceRequirements, WatchlistNotificationPolicy, WatchlistSourcePolicy,
};
use chrono::{DateTime, Duration, Utc};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_HARNESS_URL: &str = "http://127.0.0.1:7878";
pub const RESEARCH_PROJECTION_SCHEMA: &str = "arda.hud.research-projection.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskApproval {
    pub schema_version: String,
    pub proposal_id: String,
    pub approval_id: String,
    pub ledger_writes: Vec<String>,
    pub decision: String,
    pub created_at_utc: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MutationEnvelope {
    pub approval: TaskApproval,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchQuestionIntent {
    pub question: String,
    pub rationale: String,
    pub tags: Vec<String>,
    pub cadence: WatchlistCadence,
    pub source_policy: WatchlistSourcePolicy,
    pub evidence_requirements: WatchlistEvidenceRequirements,
    pub contradiction_policy: ContradictionPolicy,
    pub budgets: WatchlistBudgets,
    pub notification_policy: WatchlistNotificationPolicy,
    pub approval_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchWatchlistIntent {
    pub name: String,
    pub question_ids: Vec<String>,
    pub approval_reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResearchWatchlistActionIntent {
    pub watchlist_id: String,
    pub action: String,
    pub approval_reference: String,
}

#[derive(Debug, Clone, Serialize)]
struct CreateQuestionRequest<'a> {
    question: &'a ResearchQuestion,
    read_only: bool,
    envelope: &'a MutationEnvelope,
}

#[derive(Debug, Clone, Serialize)]
struct CreateWatchlistRequest<'a> {
    watchlist: &'a ResearchWatchlist,
    envelope: &'a MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionCreateResponse {
    pub question: ResearchQuestion,
    pub backend_suggestion: Value,
    pub backend_status: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResearchLoadState {
    Loading,
    Healthy,
    Stale,
    Partial,
    Degraded,
    Unavailable,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResearchProjection {
    pub schema_version: String,
    pub state: ResearchLoadState,
    pub source_revision: String,
    pub source_time_utc: DateTime<Utc>,
    pub questions: Vec<Value>,
    pub watchlists: Vec<Value>,
    pub briefs: Vec<Value>,
    pub failures: Vec<String>,
    pub recovery_action: Option<String>,
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

fn resolve_research_intent(
    approval_reference: &str,
    action: &str,
    resource: &str,
) -> Result<MutationEnvelope, String> {
    let operator_id = std::env::var("ARDA_OPERATOR_ID")
        .map_err(|_| "ARDA_OPERATOR_ID is required for Research mutations".to_string())?;
    let raw = std::env::var("ARDA_RESEARCH_APPROVAL_ENVELOPE_JSON").map_err(|_| {
        "ARDA_RESEARCH_APPROVAL_ENVELOPE_JSON is required; Research will not mint approval"
            .to_string()
    })?;
    let approval: TaskApproval = serde_json::from_str(&raw)
        .map_err(|error| format!("configured Research approval envelope is invalid: {error}"))?;
    let max_age_seconds = std::env::var("ARDA_RESEARCH_APPROVAL_MAX_AGE_SECONDS")
        .ok()
        .map(|value| {
            value.parse::<i64>().map_err(|_| {
                "ARDA_RESEARCH_APPROVAL_MAX_AGE_SECONDS must be an integer".to_string()
            })
        })
        .transpose()?
        .unwrap_or(3_600);
    resolve_research_intent_from(
        approval_reference,
        action,
        resource,
        &operator_id,
        approval,
        Utc::now(),
        max_age_seconds,
    )
}

fn resolve_research_intent_from(
    approval_reference: &str,
    action: &str,
    resource: &str,
    operator_id: &str,
    approval: TaskApproval,
    now: DateTime<Utc>,
    max_age_seconds: i64,
) -> Result<MutationEnvelope, String> {
    let approval_reference = approval_reference.trim();
    if approval_reference.is_empty() {
        return Err("approval reference is required for Research mutations".to_string());
    }
    if operator_id.trim().is_empty() {
        return Err("ARDA_OPERATOR_ID cannot be empty".to_string());
    }
    if approval.schema_version != "arda.orome.task_approval.v1" {
        return Err("configured Research approval has an unsupported schema".to_string());
    }
    if approval.approval_id != approval_reference {
        return Err(
            "approval reference does not match the configured approval envelope".to_string(),
        );
    }
    if approval.decision != "policy_safe" {
        return Err("configured Research approval does not authorize mutation".to_string());
    }
    if approval.proposal_id.trim().is_empty() || approval.created_at_utc.trim().is_empty() {
        return Err("configured Research approval is missing required lineage".to_string());
    }
    if max_age_seconds <= 0 {
        return Err("Research approval maximum age must be positive".to_string());
    }
    let created = DateTime::parse_from_rfc3339(&approval.created_at_utc)
        .map_err(|_| "configured Research approval timestamp is invalid".to_string())?
        .with_timezone(&Utc);
    let age = now.signed_duration_since(created).num_seconds();
    if age < -300 {
        return Err("configured Research approval timestamp is in the future".to_string());
    }
    if age > max_age_seconds {
        return Err("configured Research approval has expired".to_string());
    }
    let digest = stable_hash(&[action, resource, approval_reference, operator_id]);
    Ok(MutationEnvelope {
        approval,
        idempotency_key: format!("research-{action}-{digest:016x}"),
    })
}

fn canonical_question(
    intent: ResearchQuestionIntent,
    operator_id: &str,
    now: DateTime<Utc>,
) -> Result<ResearchQuestion, String> {
    let mut question = ResearchQuestion::new(
        operator_id,
        intent.question,
        intent.rationale,
        intent.tags,
        intent.cadence,
        now + Duration::days(7),
        intent.source_policy,
        intent.evidence_requirements,
        intent.contradiction_policy,
        intent.budgets,
        intent.notification_policy,
    )
    .map_err(|error| error.to_string())?;
    question.question_id = format!(
        "research-question-{:016x}",
        stable_hash(&[
            operator_id,
            &question.question,
            &question.rationale,
            &intent.approval_reference,
        ])
    );
    Ok(question)
}

fn canonical_watchlist(intent: ResearchWatchlistIntent) -> Result<ResearchWatchlist, String> {
    let approval_reference = intent.approval_reference;
    let mut watchlist = ResearchWatchlist::new(intent.name, intent.question_ids)
        .map_err(|error| error.to_string())?;
    watchlist.watchlist_id = format!(
        "research-watchlist-{:016x}",
        stable_hash(&[
            &watchlist.name,
            &watchlist.question_ids.join(","),
            &approval_reference,
        ])
    );
    Ok(watchlist)
}

async fn decode<T: DeserializeOwned>(response: reqwest::Response) -> Result<T, String> {
    let status = response.status();
    if status.is_success() {
        return response
            .json::<T>()
            .await
            .map_err(|error| format!("Harness returned an invalid Research response: {error}"));
    }
    let detail = response.text().await.unwrap_or_default();
    Err(format!(
        "Research harness request failed ({status}): {detail}"
    ))
}

async fn get_value(path: &str) -> Result<Value, String> {
    let response = reqwest::Client::new()
        .get(endpoint(path))
        .send()
        .await
        .map_err(|error| format!("Unable to reach the Research harness: {error}"))?;
    decode(response).await
}

async fn post_json<B: Serialize + ?Sized, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let response = reqwest::Client::new()
        .post(endpoint(path))
        .json(body)
        .send()
        .await
        .map_err(|error| format!("Unable to reach the Research harness: {error}"))?;
    decode(response).await
}

fn records(value: Result<Value, String>, key: &str, failures: &mut Vec<String>) -> Vec<Value> {
    match value {
        Ok(value) => value
            .get(key)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default(),
        Err(error) => {
            failures.push(format!("{key}: {error}"));
            Vec::new()
        }
    }
}

fn project_research_snapshot(
    questions: Result<Value, String>,
    watchlists: Result<Value, String>,
    briefs: Result<Value, String>,
    source_time_utc: DateTime<Utc>,
) -> ResearchProjection {
    let mut failures = Vec::new();
    let questions = records(questions, "questions", &mut failures);
    let watchlists = records(watchlists, "watchlists", &mut failures);
    let briefs = records(briefs, "briefs", &mut failures);
    let stale = briefs
        .iter()
        .any(|brief| brief.get("stale").and_then(Value::as_bool) == Some(true));
    let state = match failures.len() {
        0 if stale => ResearchLoadState::Stale,
        0 => ResearchLoadState::Healthy,
        1 | 2 => ResearchLoadState::Partial,
        _ => ResearchLoadState::Unavailable,
    };
    let canonical = serde_json::to_string(&(&questions, &watchlists, &briefs, &failures))
        .unwrap_or_else(|_| "projection-serialization-failed".to_string());
    ResearchProjection {
        schema_version: RESEARCH_PROJECTION_SCHEMA.to_string(),
        state,
        source_revision: format!("research-{:016x}", stable_hash(&[&canonical])),
        source_time_utc,
        questions,
        watchlists,
        briefs,
        recovery_action: (!failures.is_empty())
            .then(|| "Refresh Research after restoring the reported harness owner.".to_string()),
        failures,
    }
}

#[tauri::command]
pub async fn get_research_projection() -> Result<ResearchProjection, String> {
    let (questions, watchlists, briefs) = tokio::join!(
        get_value("/v1/research/questions"),
        get_value("/v1/research/watchlists"),
        get_value("/v1/research/briefs")
    );
    Ok(project_research_snapshot(
        questions,
        watchlists,
        briefs,
        Utc::now(),
    ))
}

#[tauri::command]
pub async fn create_research_question(
    intent: ResearchQuestionIntent,
) -> Result<QuestionCreateResponse, String> {
    let operator_id = std::env::var("ARDA_OPERATOR_ID")
        .map_err(|_| "ARDA_OPERATOR_ID is required to create a Research question".to_string())?;
    let approval_reference = intent.approval_reference.clone();
    let question = canonical_question(intent, &operator_id, Utc::now())?;
    let envelope = resolve_research_intent(
        &approval_reference,
        "create-question",
        &format!("{}:{}", question.question, question.rationale),
    )?;
    post_json(
        "/v1/research/questions",
        &CreateQuestionRequest {
            question: &question,
            read_only: false,
            envelope: &envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn create_research_watchlist(
    intent: ResearchWatchlistIntent,
) -> Result<ResearchWatchlist, String> {
    let approval_reference = intent.approval_reference.clone();
    let watchlist = canonical_watchlist(intent)?;
    let envelope = resolve_research_intent(
        &approval_reference,
        "create-watchlist",
        &format!("{}:{}", watchlist.name, watchlist.question_ids.join(",")),
    )?;
    post_json(
        "/v1/research/watchlists",
        &CreateWatchlistRequest {
            watchlist: &watchlist,
            envelope: &envelope,
        },
    )
    .await
}

#[tauri::command]
pub async fn change_research_watchlist_state(
    intent: ResearchWatchlistActionIntent,
) -> Result<ResearchWatchlist, String> {
    let watchlist_id = checked_id(&intent.watchlist_id, "watchlist_id")?;
    if !matches!(intent.action.as_str(), "pause" | "resume" | "retire") {
        return Err("Research watchlist action must be pause, resume, or retire".to_string());
    }
    let envelope =
        resolve_research_intent(&intent.approval_reference, &intent.action, watchlist_id)?;
    post_json(
        &format!("/v1/research/watchlists/{watchlist_id}/{}", intent.action),
        &envelope,
    )
    .await
}

#[cfg(test)]
#[path = "research_tests.rs"]
mod tests;
