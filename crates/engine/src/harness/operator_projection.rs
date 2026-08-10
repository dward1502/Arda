use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tracing::{debug, warn};

use super::HarnessState;
use crate::operator_projection::{
    publish_operator_projection, OperatorProjectionPublishError, OPERATOR_PROJECTION_PATH,
};

#[derive(Debug, Serialize)]
struct ProjectionReadError {
    state: &'static str,
    error: String,
    path: &'static str,
}

const PUBLISH_INTERVAL: Duration = Duration::from_secs(2);

pub(super) async fn publish_continuously(root: PathBuf, shutdown: Arc<Notify>) {
    let mut interval = tokio::time::interval(PUBLISH_INTERVAL);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = interval.tick() => {
                let publish_root = root.clone();
                match tokio::task::spawn_blocking(move || {
                    publish_operator_projection(&publish_root, chrono::Utc::now())
                }).await {
                    Ok(Ok(_)) => {}
                    Ok(Err(OperatorProjectionPublishError::Io { source, .. }))
                        if source.kind() == std::io::ErrorKind::NotFound =>
                    {
                        debug!("operator projection inputs are not available yet");
                    }
                    Ok(Err(error)) => warn!(%error, "operator projection publication failed"),
                    Err(error) => warn!(%error, "operator projection publisher task failed"),
                }
            }
        }
    }
}

pub async fn get_projection(State(state): State<HarnessState>) -> Response {
    let root = state.workbench_root;
    let published =
        tokio::task::spawn_blocking(move || publish_operator_projection(&root, chrono::Utc::now()))
            .await;

    match published {
        Ok(Ok(projection)) => (StatusCode::OK, Json(projection)).into_response(),
        Ok(Err(OperatorProjectionPublishError::Io { source, .. }))
            if source.kind() == std::io::ErrorKind::NotFound =>
        {
            (
                StatusCode::NOT_FOUND,
                Json(ProjectionReadError {
                    state: "unavailable",
                    error: "canonical operator projection inputs are unavailable".to_string(),
                    path: OPERATOR_PROJECTION_PATH,
                }),
            )
                .into_response()
        }
        Ok(Err(error)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ProjectionReadError {
                state: "failed",
                error: error.to_string(),
                path: OPERATOR_PROJECTION_PATH,
            }),
        )
            .into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ProjectionReadError {
                state: "failed",
                error: format!("operator projection publisher task failed: {error}"),
                path: OPERATOR_PROJECTION_PATH,
            }),
        )
            .into_response(),
    }
}
