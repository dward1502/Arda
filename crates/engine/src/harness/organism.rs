use super::projects::ApiError;
use super::HarnessState;
use arda_core::organism::OrganismManifest;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use serde::Serialize;

const RESPONSE_SCHEMA: &str = "arda.organism-manifest-response.v1";

#[derive(Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct OrganismManifestResponse {
    schema_version: &'static str,
    manifest: OrganismManifest,
    manifest_digest: String,
}

pub(super) async fn get_manifest(
    State(state): State<HarnessState>,
    headers: HeaderMap,
) -> Result<Json<OrganismManifestResponse>, ApiError> {
    let supplied = headers
        .get("x-arda-operator-id")
        .and_then(|value| value.to_str().ok())
        .map(str::trim);
    if supplied != Some(state.operator_id.as_str()) {
        return Err(ApiError::forbidden(
            "organism manifest requires configured operator identity",
        ));
    }
    let manifest = OrganismManifest::load_from_root(&state.workbench_root)
        .map_err(|error| ApiError::internal(format!("organism manifest unavailable: {error}")))?;
    let manifest_digest = manifest
        .digest()
        .map_err(|error| ApiError::internal(format!("organism manifest invalid: {error}")))?;
    Ok(Json(OrganismManifestResponse {
        schema_version: RESPONSE_SCHEMA,
        manifest,
        manifest_digest,
    }))
}
