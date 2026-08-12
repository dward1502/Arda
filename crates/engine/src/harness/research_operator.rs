//! Operator-owned research questions, watchlists, and brief projections.
//!
//! This surface owns product lifecycle only. Recurring execution is never
//! implemented here: non-read-only question creation forwards one typed
//! suggestion to Warden's canonical suggestion ingress, while watchlists
//! remain product records until the governed scheduler consumes them.

use arda_outpost_protocol::{ResearchQuestion, ResearchSuggestion, ResearchWatchlist};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    net::SocketAddr,
    path::{Path as FsPath, PathBuf},
};

use super::{
    projects::{require_loopback, ApiError, MutationEnvelope, WORKBENCH_MUTATIONS},
    HarnessState,
};

const QUESTION_SCHEMA: &str = "arda.workbench.research-questions.v1";
const WATCHLIST_SCHEMA: &str = "arda.workbench.research-watchlists.v1";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateQuestionRequest {
    question: ResearchQuestion,
    #[serde(default)]
    read_only: bool,
    envelope: MutationEnvelope,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CreateWatchlistRequest {
    watchlist: ResearchWatchlist,
    envelope: MutationEnvelope,
}

#[derive(Debug, Serialize)]
pub(super) struct QuestionCreateResponse {
    question: ResearchQuestion,
    backend_suggestion: ResearchSuggestion,
    backend_status: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct QuestionListResponse {
    questions: Vec<ResearchQuestion>,
}

#[derive(Debug, Serialize)]
pub(super) struct WatchlistListResponse {
    watchlists: Vec<ResearchWatchlist>,
}

#[derive(Debug, Serialize)]
pub(super) struct BriefListResponse {
    briefs: Vec<serde_json::Value>,
}

pub(super) async fn create_question(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<CreateQuestionRequest>,
) -> Result<(StatusCode, Json<QuestionCreateResponse>), ApiError> {
    require_loopback(peer)?;
    require_operator(&state, &headers, &request.question.owner)?;
    request.envelope.validate()?;
    request
        .question
        .validate_at(Utc::now())
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;

    let mut registry = load_questions(&state.workbench_root)?;
    if let Some(existing) = registry
        .iter()
        .find(|question| question.question_id == request.question.question_id)
        .cloned()
    {
        let suggestion = intended_suggestion(&existing, &request.envelope.idempotency_key)?;
        return Ok((
            StatusCode::OK,
            Json(QuestionCreateResponse {
                question: existing,
                backend_suggestion: suggestion,
                backend_status: "already_registered",
            }),
        ));
    }

    let suggestion = intended_suggestion(&request.question, &request.envelope.idempotency_key)?;
    let backend_status = if request.read_only {
        "read_only_not_enqueued"
    } else {
        enqueue_suggestion(&state, &suggestion).await?;
        "enqueued_via_warden_ingress"
    };
    registry.push(request.question.clone());
    registry.sort_by(|left, right| left.question_id.cmp(&right.question_id));
    write_json_atomic(
        &questions_path(&state.workbench_root),
        QUESTION_SCHEMA,
        &registry,
    )?;
    Ok((
        StatusCode::CREATED,
        Json(QuestionCreateResponse {
            question: request.question,
            backend_suggestion: suggestion,
            backend_status,
        }),
    ))
}

pub(super) async fn list_questions(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<QuestionListResponse>, ApiError> {
    require_operator_header(&state, &headers)?;
    Ok(Json(QuestionListResponse {
        questions: load_questions(&state.workbench_root)?,
    }))
}

pub(super) async fn get_question(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ResearchQuestion>, ApiError> {
    require_operator_header(&state, &headers)?;
    load_questions(&state.workbench_root)?
        .into_iter()
        .find(|question| question.question_id == id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("research question `{id}` was not found")))
}

pub(super) async fn create_watchlist(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<CreateWatchlistRequest>,
) -> Result<(StatusCode, Json<ResearchWatchlist>), ApiError> {
    require_loopback(peer)?;
    require_operator_header(&state, &headers)?;
    request.envelope.validate()?;
    if request.watchlist.schema_version != arda_outpost_protocol::WATCHLIST_SCHEMA_VERSION
        || request.watchlist.question_ids.is_empty()
        || request.watchlist.name.trim().is_empty()
    {
        return Err(ApiError::bad_request("invalid research watchlist"));
    }
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    let mut registry = load_watchlists(&state.workbench_root)?;
    if let Some(existing) = registry
        .iter()
        .find(|watchlist| watchlist.watchlist_id == request.watchlist.watchlist_id)
        .cloned()
    {
        return Ok((StatusCode::OK, Json(existing)));
    }
    registry.push(request.watchlist.clone());
    registry.sort_by(|left, right| left.watchlist_id.cmp(&right.watchlist_id));
    write_json_atomic(
        &watchlists_path(&state.workbench_root),
        WATCHLIST_SCHEMA,
        &registry,
    )?;
    Ok((StatusCode::CREATED, Json(request.watchlist)))
}

pub(super) async fn list_watchlists(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<WatchlistListResponse>, ApiError> {
    require_operator_header(&state, &headers)?;
    Ok(Json(WatchlistListResponse {
        watchlists: load_watchlists(&state.workbench_root)?,
    }))
}

pub(super) async fn get_watchlist(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<ResearchWatchlist>, ApiError> {
    require_operator_header(&state, &headers)?;
    find_watchlist(&state, &id).map(Json)
}

pub(super) async fn pause_watchlist(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(envelope): Json<MutationEnvelope>,
) -> Result<Json<ResearchWatchlist>, ApiError> {
    require_operator_header(&state, &headers)?;
    transition_watchlist(&state, peer, id, envelope, |watchlist| {
        watchlist
            .pause()
            .map_err(|error| ApiError::conflict(error.to_string()))
    })
    .await
}

pub(super) async fn resume_watchlist(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(envelope): Json<MutationEnvelope>,
) -> Result<Json<ResearchWatchlist>, ApiError> {
    require_operator_header(&state, &headers)?;
    transition_watchlist(&state, peer, id, envelope, |watchlist| {
        watchlist
            .resume()
            .map_err(|error| ApiError::conflict(error.to_string()))
    })
    .await
}

pub(super) async fn retire_watchlist(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(envelope): Json<MutationEnvelope>,
) -> Result<Json<ResearchWatchlist>, ApiError> {
    require_operator_header(&state, &headers)?;
    transition_watchlist(&state, peer, id, envelope, |watchlist| {
        watchlist.retire();
        Ok(())
    })
    .await
}

async fn transition_watchlist<F>(
    state: &HarnessState,
    peer: SocketAddr,
    id: String,
    envelope: MutationEnvelope,
    transition: F,
) -> Result<Json<ResearchWatchlist>, ApiError>
where
    F: FnOnce(&mut ResearchWatchlist) -> Result<(), ApiError>,
{
    require_loopback(peer)?;
    envelope.validate()?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;
    let mut registry = load_watchlists(&state.workbench_root)?;
    let watchlist = registry
        .iter_mut()
        .find(|watchlist| watchlist.watchlist_id == id)
        .ok_or_else(|| ApiError::not_found(format!("research watchlist `{id}` was not found")))?;
    transition(watchlist)?;
    let response = watchlist.clone();
    write_json_atomic(
        &watchlists_path(&state.workbench_root),
        WATCHLIST_SCHEMA,
        &registry,
    )?;
    Ok(Json(response))
}

pub(super) async fn list_briefs(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<BriefListResponse>, ApiError> {
    require_operator_header(&state, &headers)?;
    let mut briefs = Vec::new();
    let runs = state.workbench_root.join("data/runs");
    if let Ok(entries) = fs::read_dir(runs) {
        for run in entries.flatten() {
            let evidence = run.path().join("evidence");
            if let Ok(files) = fs::read_dir(evidence) {
                for file in files
                    .flatten()
                    .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
                {
                    if let Ok(bytes) = fs::read(file.path()) {
                        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                            if value
                                .get("schema_version")
                                .and_then(serde_json::Value::as_str)
                                == Some("arda.workbench.research-brief.v1")
                            {
                                briefs.push(value);
                            }
                        }
                    }
                }
            }
        }
    }
    briefs.sort_by(|left, right| {
        left.get("generated_at_utc")
            .and_then(serde_json::Value::as_str)
            .cmp(
                &right
                    .get("generated_at_utc")
                    .and_then(serde_json::Value::as_str),
            )
    });
    Ok(Json(BriefListResponse { briefs }))
}

pub(super) async fn get_brief(
    State(state): State<HarnessState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    require_operator_header(&state, &headers)?;
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(ApiError::bad_request("invalid research brief id"));
    }
    let briefs = list_briefs(State(state), headers).await?.0.briefs;
    briefs
        .into_iter()
        .find(|brief| {
            brief.get("brief_id").and_then(serde_json::Value::as_str) == Some(id.as_str())
        })
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("research brief `{id}` was not found")))
}

async fn enqueue_suggestion(
    state: &HarnessState,
    suggestion: &ResearchSuggestion,
) -> Result<(), ApiError> {
    let scout_url = state
        .warden_scout_url
        .as_deref()
        .ok_or_else(|| ApiError::internal("Warden scout is not configured"))?;
    state
        .client
        .post(format!("{}/suggestions", scout_url.trim_end_matches('/')))
        .timeout(state.warden_scout_timeout)
        .json(suggestion)
        .send()
        .await
        .map_err(|error| ApiError::internal(format!("Warden suggestion ingress failed: {error}")))?
        .error_for_status()
        .map_err(|error| {
            ApiError::internal(format!(
                "Warden suggestion ingress rejected request: {error}"
            ))
        })?;
    Ok(())
}

fn intended_suggestion(
    question: &ResearchQuestion,
    idempotency_key: &str,
) -> Result<ResearchSuggestion, ApiError> {
    ResearchSuggestion::new(
        question.question.clone(),
        format!(
            "research-question:{}:{idempotency_key}",
            question.question_id
        ),
        Utc::now(),
        question.expires_at_utc,
        question.budgets.max_results,
        question.budgets.max_fetch_bytes,
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))
}

fn find_watchlist(state: &HarnessState, id: &str) -> Result<ResearchWatchlist, ApiError> {
    load_watchlists(&state.workbench_root)?
        .into_iter()
        .find(|watchlist| watchlist.watchlist_id == id)
        .ok_or_else(|| ApiError::not_found(format!("research watchlist `{id}` was not found")))
}

fn require_operator_header<'a>(
    state: &HarnessState,
    headers: &'a HeaderMap,
) -> Result<&'a str, ApiError> {
    let operator = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::forbidden("x-arda-operator-id header required"))?;
    if operator != state.operator_id {
        return Err(ApiError::forbidden(
            "operator identity is not authorized by daemon configuration",
        ));
    }
    Ok(operator)
}

fn require_operator(
    state: &HarnessState,
    headers: &HeaderMap,
    record_owner: &str,
) -> Result<(), ApiError> {
    let operator = require_operator_header(state, headers)?;
    if operator != record_owner.trim() {
        return Err(ApiError::forbidden(
            "research record owner does not match configured operator authority",
        ));
    }
    Ok(())
}

fn questions_path(root: &FsPath) -> PathBuf {
    root.join("data/workbench/research/questions.json")
}
fn watchlists_path(root: &FsPath) -> PathBuf {
    root.join("data/workbench/research/watchlists.json")
}

fn load_questions(root: &FsPath) -> Result<Vec<ResearchQuestion>, ApiError> {
    load_registry(&questions_path(root), QUESTION_SCHEMA)
}

fn load_watchlists(root: &FsPath) -> Result<Vec<ResearchWatchlist>, ApiError> {
    load_registry(&watchlists_path(root), WATCHLIST_SCHEMA)
}

fn load_registry<T: for<'de> Deserialize<'de>>(
    path: &FsPath,
    schema: &str,
) -> Result<Vec<T>, ApiError> {
    match fs::read(path) {
        Ok(bytes) => {
            let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
                ApiError::internal(format!("failed to parse research registry: {error}"))
            })?;
            if value
                .get("schema_version")
                .and_then(serde_json::Value::as_str)
                != Some(schema)
            {
                return Err(ApiError::internal("unsupported research registry schema"));
            }
            serde_json::from_value(
                value
                    .get("records")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!([])),
            )
            .map_err(|error| {
                ApiError::internal(format!("failed to decode research registry: {error}"))
            })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(ApiError::internal(format!(
            "failed to read research registry: {error}"
        ))),
    }
}

fn write_json_atomic<T: Serialize>(
    path: &FsPath,
    schema: &str,
    records: &[T],
) -> Result<(), ApiError> {
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal("research registry has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| {
        ApiError::internal(format!(
            "failed to create research registry directory: {error}"
        ))
    })?;
    let value = serde_json::json!({"schema_version": schema, "records": records});
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    fs::write(
        &temporary,
        serde_json::to_vec_pretty(&value).map_err(|error| ApiError::internal(error.to_string()))?,
    )
    .and_then(|_| fs::rename(&temporary, path))
    .map_err(|error| ApiError::internal(format!("failed to write research registry: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arda_orome::types::{InterruptionLedgerDecision, TaskApprovalEnvelope};
    use arda_outpost_protocol::{
        ContradictionPolicy, WatchlistBudgets, WatchlistCadence, WatchlistEvidenceRequirements,
        WatchlistNotificationPolicy, WatchlistSourcePolicy,
    };
    use axum::extract::ConnectInfo;
    use std::sync::Arc;
    use tempfile::tempdir;
    use tokio::sync::RwLock;

    fn question() -> ResearchQuestion {
        ResearchQuestion::new(
            "operator-0",
            "What changed in the Arda runtime?",
            "Keep the operator brief current.",
            vec!["runtime".to_string()],
            WatchlistCadence::Manual,
            Utc::now() + chrono::Duration::hours(1),
            WatchlistSourcePolicy {
                policy_id: "public".to_string(),
                allowed_sources: vec!["docs.rs".to_string()],
                max_sources_per_run: 2,
                allow_private_targets: false,
            },
            WatchlistEvidenceRequirements {
                minimum_canonical_sources: 1,
                require_canonical_fetch: true,
                max_source_age_seconds: 3600,
            },
            ContradictionPolicy::RequireDisclosure,
            WatchlistBudgets {
                max_results: 2,
                max_fetch_bytes: 4096,
                max_tokens: 512,
                max_attempts: 1,
            },
            WatchlistNotificationPolicy {
                enabled: false,
                destination: None,
            },
        )
        .expect("valid question")
    }

    fn envelope() -> MutationEnvelope {
        MutationEnvelope {
            approval: TaskApprovalEnvelope {
                schema_version: "arda.orome.task_approval.v1".to_string(),
                proposal_id: "proposal-1".to_string(),
                approval_id: "approval-1".to_string(),
                ledger_writes: Vec::new(),
                decision: InterruptionLedgerDecision::PolicySafe,
                created_at_utc: Utc::now().to_rfc3339(),
            },
            idempotency_key: "question-create-1".to_string(),
        }
    }

    fn state(root: PathBuf) -> HarnessState {
        HarnessState {
            harness_addr: "127.0.0.1:7878".to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            service_statuses: Arc::new(RwLock::new(Vec::new())),
            manwe_url: "http://127.0.0.1:1".to_string(),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: std::time::Duration::from_secs(1),
            manwe_proxy_bearer: None,
            warden_scout_url: None,
            warden_scout_timeout: std::time::Duration::from_secs(1),
            presence_inputs: super::super::presence::HarnessPresenceState::default(),
            workbench_root: root,
            operator_id: "operator-0".to_string(),
        }
    }

    #[test]
    fn intended_suggestion_is_bounded_by_question_budget() {
        let question = question();
        let suggestion = intended_suggestion(&question, "idempotency").expect("suggestion");
        assert_eq!(suggestion.query, question.question);
        assert_eq!(suggestion.max_results, question.budgets.max_results);
        assert_eq!(suggestion.budget_bytes, question.budgets.max_fetch_bytes);
        assert!(suggestion.idempotency_key.contains(&question.question_id));
    }

    #[tokio::test]
    async fn read_only_question_reports_without_writing_backend_queue() {
        let root = tempdir().expect("temp root");
        let root_path = root.path().to_path_buf();
        let request = CreateQuestionRequest {
            question: question(),
            read_only: true,
            envelope: envelope(),
        };
        let (status, Json(response)) = create_question(
            State(state(root_path.clone())),
            ConnectInfo("127.0.0.1:1234".parse().unwrap()),
            HeaderMap::from_iter([(
                "x-arda-operator-id".parse().unwrap(),
                "operator-0".parse().unwrap(),
            )]),
            Json(request),
        )
        .await
        .expect("read-only question");
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(response.backend_status, "read_only_not_enqueued");
        assert_eq!(load_questions(&root_path).expect("registry").len(), 1);
        assert!(!root_path
            .join("data/warden/research_suggestions.jsonl")
            .exists());
    }
}
