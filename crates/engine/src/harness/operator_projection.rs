use arda_core::operator_projection::OperatorProjection;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use super::HarnessState;

const OPERATOR_PROJECTION_PATH: &str = "core/state/operator_projection.json";

#[derive(Debug, Serialize)]
struct ProjectionReadError {
    state: &'static str,
    error: String,
    path: &'static str,
}

pub async fn get_projection(State(state): State<HarnessState>) -> Response {
    let path = state.workbench_root.join(OPERATOR_PROJECTION_PATH);
    let raw = match tokio::fs::read_to_string(&path).await {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (
                StatusCode::NOT_FOUND,
                Json(ProjectionReadError {
                    state: "unavailable",
                    error: "canonical operator projection is unavailable".to_string(),
                    path: OPERATOR_PROJECTION_PATH,
                }),
            )
                .into_response();
        }
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ProjectionReadError {
                    state: "failed",
                    error: error.to_string(),
                    path: OPERATOR_PROJECTION_PATH,
                }),
            )
                .into_response();
        }
    };

    match OperatorProjection::from_json_str(&raw) {
        Ok(projection) => (StatusCode::OK, Json(projection)).into_response(),
        Err(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(ProjectionReadError {
                state: "failed",
                error: error.to_string(),
                path: OPERATOR_PROJECTION_PATH,
            }),
        )
            .into_response(),
    }
}
