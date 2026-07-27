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

use crate::observation::build_observation;
use crate::{
    survey_repo, MemoryFallback, ObservationMemoryBridge, ResearchReport, ResearchRequest,
    ScoutRecallQuery, ScoutRecallReport, SearxngClient,
};

const MEMORY_SCOPE: &str = "outpost_scout";

#[derive(Debug, Clone)]
pub struct ScoutRuntimeState {
    source: String,
    memory_root: PathBuf,
    search: SearxngClient,
}

impl ScoutRuntimeState {
    pub fn new(
        memory_root: impl Into<PathBuf>,
        searxng_url: impl AsRef<str>,
        source: impl Into<String>,
    ) -> Result<Self, crate::ResearchError> {
        Ok(Self {
            source: source.into(),
            memory_root: memory_root.into(),
            search: SearxngClient::new(searxng_url)?,
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
        .route("/search", post(search))
        .route("/survey", post(survey))
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

async fn search(
    State(state): State<ScoutRuntimeState>,
    Json(request): Json<ResearchRequest>,
) -> Result<Json<SearchResponse>, (StatusCode, Json<Value>)> {
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
    Ok(Json(SearchResponse { report, memory }))
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
