//! HTTP runtime for the Warden scout outpost.

use std::path::PathBuf;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::observation::build_observation;
use crate::{
    survey_repo, AcknowledgementReceipt, AuditFollowupRequest, AuditFollowupResponse,
    ExternalObservationReceipt, MemoryFallback, ObservationMemoryBridge, PersistedResearchChain,
    ResearchDispatch, ResearchReceiptLedger, ResearchReport, ResearchRequest, ResearchSuggestion,
    ResearchSuggestionLedger, ScoutAuditError, ScoutAuditOutcome, ScoutAuditRequest,
    ScoutAuditService, ScoutRecallQuery, ScoutRecallReport, SearxngClient,
};

const MEMORY_SCOPE: &str = "outpost_scout";

#[derive(Debug, Clone)]
pub struct ScoutRuntimeState {
    source: String,
    memory_root: PathBuf,
    search: SearxngClient,
    ledger: ResearchReceiptLedger,
    suggestions: ResearchSuggestionLedger,
}

impl ScoutRuntimeState {
    pub fn new(
        memory_root: impl Into<PathBuf>,
        searxng_url: impl AsRef<str>,
        source: impl Into<String>,
    ) -> Result<Self, crate::ResearchError> {
        let memory_root = memory_root.into();
        let ledger =
            ResearchReceiptLedger::open(memory_root.join("data/warden/research_receipts.jsonl"))
                .map_err(|error| crate::ResearchError::InvalidEndpoint(error.to_string()))?;
        let suggestions = ResearchSuggestionLedger::open(
            memory_root.join("data/warden/research_suggestions.jsonl"),
        )
        .map_err(|error| crate::ResearchError::InvalidEndpoint(error.to_string()))?;
        Ok(Self {
            source: source.into(),
            memory_root,
            search: SearxngClient::new(searxng_url)?,
            ledger,
            suggestions,
        })
    }

    fn memory(&self) -> ObservationMemoryBridge {
        ObservationMemoryBridge::at_root(MEMORY_SCOPE, &self.memory_root)
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    source: String,
    authority: &'static str,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub report: ResearchReport,
    pub memory: MemoryFallback,
    pub research_chain: PersistedResearchChain,
}

#[derive(Debug, Serialize)]
pub struct DiscoveryResponse {
    pub report: ResearchReport,
    pub memory: MemoryFallback,
}

#[derive(Debug, Serialize)]
pub struct SuggestionResponse {
    pub suggestion: ResearchSuggestion,
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct AuditResponse {
    pub audit: ScoutAuditOutcome,
    /// Present only for the first execution. Idempotent replays do not append
    /// duplicate Vairë records.
    pub memory: Option<MemoryFallback>,
}

#[derive(Debug, Deserialize)]
struct SurveyRequest {
    root: PathBuf,
}

#[derive(Debug, Serialize)]
struct SurveyResponse {
    report: crate::SurveyReport,
    memory: MemoryFallback,
}

pub fn build_runtime_router(state: ScoutRuntimeState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/discover", post(discover))
        .route("/search", post(search))
        .route("/suggestions", post(enqueue_suggestion))
        .route("/dispatch", post(dispatch_next))
        .route("/survey", post(survey))
        .route("/audit", post(audit))
        .route("/audit/followup", post(audit_followup))
        .route("/recall", post(recall))
        .with_state(state)
}

async fn health(State(state): State<ScoutRuntimeState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        source: state.source,
        authority: "advisory",
    })
}

async fn discover(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<ResearchRequest>,
) -> Result<Json<DiscoveryResponse>, (StatusCode, Json<Value>)> {
    request
        .validate_at(chrono::Utc::now())
        .map_err(bad_request)?;
    let report = state
        .search
        .search(&request)
        .await
        .map_err(internal_error)?;
    let observation = report.clone().into_observation(&state.source);
    let memory_state = state.clone();
    let memory = tokio::task::spawn_blocking(move || {
        memory_state
            .memory()
            .encode_observation_to_memory(&observation)
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;
    if memory.memory_id.is_none() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": memory.failure_reason, "memory": memory})),
        ));
    }
    Ok(Json(DiscoveryResponse { report, memory }))
}

async fn search(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<ResearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<Value>)> {
    request
        .validate_at(chrono::Utc::now())
        .map_err(bad_request)?;
    let now = chrono::Utc::now();
    let suggestion = ResearchSuggestion::new(
        request.query.clone(),
        format!("{}:{}", state.source, request.query),
        now,
        request.expires_at.expect("validated expiry"),
        request.limit,
        64 * 1024,
    )
    .map_err(internal_error)?;
    state
        .suggestions
        .append(&suggestion)
        .map_err(internal_error)?;
    let response = execute_search(&state, request, suggestion.clone()).await?;
    let sequence = state
        .suggestions
        .suggestions()
        .map_err(internal_error)?
        .iter()
        .position(|item| item.suggestion_id == suggestion.suggestion_id)
        .map(|index| index as u64 + 1)
        .ok_or_else(|| internal_error("persisted suggestion disappeared"))?;
    let cursor = state
        .suggestions
        .read_cursor("suggestions")
        .map_err(internal_error)?;
    if sequence > cursor.sequence {
        state
            .suggestions
            .advance_cursor("suggestions", sequence, suggestion.suggestion_id)
            .map_err(internal_error)?;
    }
    Ok(response)
}

async fn enqueue_suggestion(
    State(state): State<ScoutRuntimeState>,
    Json(suggestion): Json<ResearchSuggestion>,
) -> Result<Json<SuggestionResponse>, (StatusCode, Json<Value>)> {
    suggestion
        .validate_at(chrono::Utc::now())
        .map_err(bad_request)?;
    let stored = state
        .suggestions
        .append(&suggestion)
        .map_err(internal_error)?;
    Ok(Json(SuggestionResponse {
        suggestion: stored,
        status: "accepted",
    }))
}

async fn dispatch_next(
    State(state): State<ScoutRuntimeState>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<Value>)> {
    let suggestions = state.suggestions.suggestions().map_err(internal_error)?;
    let cursor = state
        .suggestions
        .read_cursor("suggestions")
        .map_err(internal_error)?;
    let (sequence, suggestion) = suggestions
        .into_iter()
        .enumerate()
        .find(|(index, suggestion)| {
            (*index as u64) >= cursor.sequence && suggestion.expires_at_utc > chrono::Utc::now()
        })
        .map(|(index, suggestion)| (index as u64 + 1, suggestion))
        .ok_or_else(|| bad_request("no unresolved research suggestion"))?;
    let request = ResearchRequest {
        query: suggestion.query.clone(),
        limit: suggestion.max_results,
        source_policy: crate::ALLOWLISTED_PUBLIC_WEB_POLICY.to_owned(),
        expires_at: Some(suggestion.expires_at_utc),
    };
    let response = execute_search(&state, request, suggestion.clone()).await?;
    state
        .suggestions
        .advance_cursor("suggestions", sequence, suggestion.suggestion_id)
        .map_err(internal_error)?;
    Ok(response)
}

async fn execute_search(
    state: &ScoutRuntimeState,
    request: ResearchRequest,
    suggestion: ResearchSuggestion,
) -> Result<Json<SearchResponse>, (StatusCode, Json<Value>)> {
    let report = state
        .search
        .search(&request)
        .await
        .map_err(internal_error)?;
    let now = chrono::Utc::now();

    let dispatch = ResearchDispatch::accepted(
        &suggestion,
        format!("{}:{}:dispatch", state.source, report.query),
        now,
        1,
    )
    .map_err(internal_error)?;
    let observation = report.clone().into_observation(&state.source);
    let memory_state = state.clone();
    let memory = tokio::task::spawn_blocking(move || {
        memory_state
            .memory()
            .encode_observation_to_memory(&observation)
    })
    .await
    .map_err(internal_error)?
    .map_err(internal_error)?;
    if memory.memory_id.is_none() {
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"error": memory.failure_reason, "memory": memory})),
        ));
    }
    let result = report.results.first().ok_or_else(|| {
        internal_error("research provider returned no source results; no receipt was published")
    })?;
    let canonical_content = state
        .search
        .crawl_canonical_content(&result.url, "fit")
        .await
        .map_err(internal_error)?;
    let observation = ExternalObservationReceipt::completed(
        &suggestion,
        &dispatch,
        &result.url,
        hex_digest(canonical_content.as_bytes()),
        hex_digest(result.url.as_bytes()),
        now,
    )
    .map_err(internal_error)?;
    let acknowledgement =
        AcknowledgementReceipt::completed(&suggestion, &dispatch, &observation, now)
            .map_err(internal_error)?;
    let research_chain = state
        .ledger
        .append_complete_chain(&suggestion, &dispatch, &observation, &acknowledgement, now)
        .map_err(internal_error)?;
    Ok(Json(SearchResponse {
        report,
        memory,
        research_chain,
    }))
}

fn hex_digest(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

async fn survey(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<SurveyRequest>,
) -> Result<Json<SurveyResponse>, (StatusCode, Json<Value>)> {
    tokio::task::spawn_blocking(move || {
        let report = survey_repo(&request.root).map_err(internal_error)?;
        let observation = build_observation(&state.source, &report);
        let memory = state
            .memory()
            .encode_observation_to_memory(&observation)
            .map_err(internal_error)?;
        Ok(Json(SurveyResponse { report, memory }))
    })
    .await
    .map_err(internal_error)?
}

async fn audit(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<ScoutAuditRequest>,
) -> Result<Json<AuditResponse>, (StatusCode, Json<Value>)> {
    tokio::task::spawn_blocking(move || {
        let service = ScoutAuditService::new(&state.memory_root, &state.source);
        let outcome = service
            .execute(request, chrono::Utc::now())
            .map_err(audit_error)?;
        let memory = if outcome.replayed {
            None
        } else {
            Some(
                state
                    .memory()
                    .encode_observation_to_memory(&outcome.observation)
                    .map_err(internal_error)?,
            )
        };
        Ok(Json(AuditResponse {
            audit: outcome,
            memory,
        }))
    })
    .await
    .map_err(internal_error)?
}

async fn audit_followup(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<AuditFollowupRequest>,
) -> Result<Json<AuditFollowupResponse>, (StatusCode, Json<Value>)> {
    tokio::task::spawn_blocking(move || {
        ScoutAuditService::new(&state.memory_root, &state.source)
            .followup(request)
            .map(Json)
            .map_err(audit_error)
    })
    .await
    .map_err(internal_error)?
}

async fn recall(
    State(state): State<ScoutRuntimeState>,
    Json(query): Json<ScoutRecallQuery>,
) -> Json<ScoutRecallReport> {
    Json(state.memory().recall_observations(&query))
}

fn internal_error(error: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({"error": error.to_string()})),
    )
}

fn bad_request(error: impl std::fmt::Display) -> (StatusCode, Json<Value>) {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({"error": error.to_string()})),
    )
}

fn audit_error(error: ScoutAuditError) -> (StatusCode, Json<Value>) {
    let status = match error {
        ScoutAuditError::Rejected(_) => StatusCode::BAD_REQUEST,
        ScoutAuditError::NotFound(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (status, Json(json!({"error": error.to_string()})))
}
