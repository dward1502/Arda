use arda_core::project_contract::ProjectContract;
use arda_orome::types::{InterruptionLedgerDecision, TaskApprovalEnvelope};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, net::SocketAddr, path::Path};
use tokio::sync::Mutex;

use super::HarnessState;

const APPROVAL_ENVELOPE_VERSION: &str = "arda.orome.task_approval.v1";
const PROJECT_REGISTRY_VERSION: &str = "arda.workbench.project-registry.v1";

pub(super) static WORKBENCH_MUTATIONS: Mutex<()> = Mutex::const_new(());

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MutationEnvelope {
    pub approval: TaskApprovalEnvelope,
    pub idempotency_key: String,
}

impl MutationEnvelope {
    pub(super) fn validate(&self) -> Result<(), ApiError> {
        if self.approval.schema_version != APPROVAL_ENVELOPE_VERSION {
            return Err(ApiError::bad_request(format!(
                "unsupported approval envelope version `{}`",
                self.approval.schema_version
            )));
        }
        for (name, value) in [
            ("approval_id", self.approval.approval_id.as_str()),
            ("proposal_id", self.approval.proposal_id.as_str()),
            ("created_at_utc", self.approval.created_at_utc.as_str()),
            ("idempotency_key", self.idempotency_key.as_str()),
        ] {
            if value.trim().is_empty() {
                return Err(ApiError::bad_request(format!(
                    "mutation envelope `{name}` cannot be empty"
                )));
            }
        }
        if self.approval.decision != InterruptionLedgerDecision::PolicySafe {
            return Err(ApiError::forbidden(
                "approval envelope does not authorize mutation",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct ValidatedProjectContract(ProjectContract);

impl<'de> Deserialize<'de> for ValidatedProjectContract {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let raw = serde_json::to_string(&value).map_err(serde::de::Error::custom)?;
        ProjectContract::from_json_str(&raw)
            .map(Self)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateProjectRequest {
    contract: ValidatedProjectContract,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachProjectRequest {
    contract: ValidatedProjectContract,
    pub envelope: MutationEnvelope,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AttachedProject {
    pub contract: ProjectContract,
    pub approval_id: String,
    pub proposal_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectRegistry {
    schema_version: String,
    projects: Vec<AttachedProject>,
}

#[derive(Debug, Serialize)]
pub struct ProjectValidationResponse {
    valid: bool,
    project_id: String,
}

#[derive(Debug, Serialize)]
pub struct ProjectListResponse {
    projects: Vec<AttachedProject>,
}

#[derive(Debug)]
pub(super) struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
    recovery_action: &'static str,
}

impl ApiError {
    pub(super) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: "invalid_request",
            message: message.into(),
            recovery_action: "Correct the request using the published contract and retry.",
        }
    }

    pub(super) fn not_found(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: "not_found",
            message: message.into(),
            recovery_action: "Reload authoritative state and choose an existing target.",
        }
    }

    pub(super) fn conflict(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: "conflict",
            message: message.into(),
            recovery_action: "Reload authoritative state before retrying the intent.",
        }
    }

    pub(super) fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: "forbidden",
            message: message.into(),
            recovery_action: "Use the configured operator authority or request approval.",
        }
    }

    pub(super) fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "internal_error",
            message: message.into(),
            recovery_action: "Inspect the harness diagnostics and retry only after recovery.",
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({
                "schema_version": "arda.hud.error.v1",
                "status": "failed",
                "code": self.code,
                "message": self.message,
                "recovery_action": self.recovery_action,
            })),
        )
            .into_response()
    }
}

pub(super) fn require_loopback(peer: SocketAddr) -> Result<(), ApiError> {
    if peer.ip().is_loopback() {
        Ok(())
    } else {
        Err(ApiError {
            status: StatusCode::FORBIDDEN,
            code: "loopback_required",
            message: "Workbench mutations are loopback-only".to_string(),
            recovery_action: "Submit the mutation through the local Arda HUD.",
        })
    }
}

pub(super) async fn validate_project(
    Json(request): Json<ValidateProjectRequest>,
) -> Result<Json<ProjectValidationResponse>, ApiError> {
    Ok(Json(ProjectValidationResponse {
        valid: true,
        project_id: request.contract.0.identity.project_id.to_string(),
    }))
}

pub(super) async fn attach_project(
    State(state): State<HarnessState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(request): Json<AttachProjectRequest>,
) -> Result<(StatusCode, Json<AttachedProject>), ApiError> {
    require_loopback(peer)?;
    request.envelope.validate()?;
    let _guard = WORKBENCH_MUTATIONS.lock().await;

    let mut registry = load_registry(&state.workbench_root)?;
    if let Some(existing) = registry
        .projects
        .iter()
        .find(|project| project.idempotency_key == request.envelope.idempotency_key)
    {
        if existing.contract == request.contract.0 {
            return Ok((StatusCode::OK, Json(existing.clone())));
        }
        return Err(ApiError::conflict(
            "idempotency key was already applied to another project contract",
        ));
    }

    let attached = AttachedProject {
        contract: request.contract.0,
        approval_id: request.envelope.approval.approval_id,
        proposal_id: request.envelope.approval.proposal_id,
        idempotency_key: request.envelope.idempotency_key,
    };
    if let Some(existing) = registry.projects.iter_mut().find(|project| {
        project.contract.identity.project_id == attached.contract.identity.project_id
    }) {
        *existing = attached.clone();
    } else {
        registry.projects.push(attached.clone());
    }
    registry.projects.sort_by(|left, right| {
        left.contract
            .identity
            .project_id
            .cmp(&right.contract.identity.project_id)
    });
    write_registry(&state.workbench_root, &registry)?;
    Ok((StatusCode::CREATED, Json(attached)))
}

pub(super) async fn list_projects(
    State(state): State<HarnessState>,
) -> Result<Json<ProjectListResponse>, ApiError> {
    let registry = load_registry(&state.workbench_root)?;
    Ok(Json(ProjectListResponse {
        projects: registry.projects,
    }))
}

pub(super) fn find_attached_project(
    root: &Path,
    project_id: &str,
) -> Result<AttachedProject, ApiError> {
    load_registry(root)?
        .projects
        .into_iter()
        .find(|project| project.contract.identity.project_id.to_string() == project_id)
        .ok_or_else(|| ApiError::not_found(format!("project `{project_id}` is not attached")))
}

pub(super) fn contract_digest(contract: &ProjectContract) -> Result<String, ApiError> {
    let bytes = serde_json::to_vec(contract).map_err(|error| {
        ApiError::internal(format!("failed to serialize project contract: {error}"))
    })?;
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn registry_path(root: &Path) -> std::path::PathBuf {
    root.join("data/workbench/projects.json")
}

fn load_registry(root: &Path) -> Result<ProjectRegistry, ApiError> {
    let path = registry_path(root);
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProjectRegistry {
                schema_version: PROJECT_REGISTRY_VERSION.to_string(),
                projects: Vec::new(),
            });
        }
        Err(error) => {
            return Err(ApiError::internal(format!(
                "failed to read project registry at {}: {error}",
                path.display()
            )));
        }
    };
    let registry: ProjectRegistry = serde_json::from_str(&raw).map_err(|error| {
        ApiError::internal(format!(
            "failed to parse project registry at {}: {error}",
            path.display()
        ))
    })?;
    if registry.schema_version != PROJECT_REGISTRY_VERSION {
        return Err(ApiError::internal(format!(
            "unsupported project registry version `{}`",
            registry.schema_version
        )));
    }
    for project in &registry.projects {
        project.contract.validate().map_err(|error| {
            ApiError::internal(format!("stored project contract is invalid: {error}"))
        })?;
    }
    Ok(registry)
}

fn write_registry(root: &Path, registry: &ProjectRegistry) -> Result<(), ApiError> {
    let path = registry_path(root);
    let parent = path
        .parent()
        .ok_or_else(|| ApiError::internal("project registry path has no parent"))?;
    fs::create_dir_all(parent).map_err(|error| {
        ApiError::internal(format!(
            "failed to create project registry directory {}: {error}",
            parent.display()
        ))
    })?;
    let temporary = path.with_extension(format!("tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(registry).map_err(|error| {
        ApiError::internal(format!("failed to serialize project registry: {error}"))
    })?;
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &path)
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(ApiError::internal(format!(
            "failed to write project registry at {}: {error}",
            path.display()
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::require_loopback;

    #[test]
    fn mutation_guard_rejects_non_loopback_peers() {
        assert!(require_loopback("127.0.0.1:1234".parse().unwrap()).is_ok());
        assert!(require_loopback("[::1]:1234".parse().unwrap()).is_ok());
        assert!(require_loopback("192.0.2.10:1234".parse().unwrap()).is_err());
    }
}
