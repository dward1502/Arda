use axum::{extract::State, http::HeaderMap, Json};

use super::{projects::ApiError, HarnessState};

pub(super) async fn get_next_action(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<arda_core::next_action::NextActionProjection>, ApiError> {
    let operator_id = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::forbidden("x-arda-operator-id header required"))?;
    if operator_id != state.operator_id {
        return Err(ApiError::forbidden(
            "operator identity is not authorized by daemon configuration",
        ));
    }
    let root = state.workbench_root;
    let operator_id = operator_id.to_string();
    let projection = tokio::task::spawn_blocking(move || {
        crate::next_action::publish_next_action_projection(&root, &operator_id, chrono::Utc::now())
    })
    .await
    .map_err(|error| ApiError::internal(format!("next-action publisher failed: {error}")))?
    .map_err(|error| ApiError::internal(format!("next-action projection failed: {error}")))?;
    Ok(Json(projection))
}
