//! Bounded, read-only prerequisite observations, never execution qualification.
use super::HarnessState;
use axum::{extract::State, Extension, Json};
use serde::Serialize;
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Default)]
pub struct RuntimePrerequisites {
    /// Exact endpoint supplied to the resident store, including CLI precedence.
    pub keeper_socket: Option<PathBuf>,
}

#[derive(Serialize)]
pub(super) struct Observation {
    scope: &'static str,
    execution_ready: Option<bool>,
    provider_catalog: &'static str,
    keeper_transport: &'static str,
}

pub(super) async fn observe(
    State(state): State<HarnessState>,
    configured: Option<Extension<RuntimePrerequisites>>,
) -> Json<Observation> {
    let configured = configured.map(|Extension(value)| value).unwrap_or_default();
    let (provider_catalog, keeper_transport) = tokio::join!(
        catalog(&state),
        keeper_transport(configured.keeper_socket.as_deref()),
    );
    Json(Observation {
        scope: "prerequisite_observation_only",
        // A catalog response or connected socket cannot prove Hermes execution,
        // runtime grants, reconciliation, or per-objective admission authority.
        execution_ready: None,
        provider_catalog,
        keeper_transport,
    })
}

async fn catalog(state: &HarnessState) -> &'static str {
    tokio::time::timeout(Duration::from_secs(3), async {
        let target = format!("{}/v1/models", state.manwe_url.trim_end_matches('/'));
        let mut request = state.client.get(target).timeout(Duration::from_secs(3));
        if let Some(bearer) = &state.manwe_proxy_bearer {
            request = request.bearer_auth(bearer);
        }
        let Ok(mut response) = request.send().await else {
            return "unreachable";
        };
        if !response.status().is_success() {
            return "http_refused";
        }
        let mut body = Vec::new();
        loop {
            match response.chunk().await {
                Ok(Some(chunk)) if body.len().saturating_add(chunk.len()) <= 65_536 => {
                    body.extend_from_slice(&chunk)
                }
                Ok(Some(_)) => return "response_too_large",
                Ok(None) => break,
                Err(_) => return "unreadable",
            }
        }
        match serde_json::from_slice::<serde_json::Value>(&body) {
            Ok(value) => match value.get("data").and_then(|v| v.as_array()) {
                Some(models) if models.is_empty() => "empty",
                Some(models)
                    if models.iter().all(|model| {
                        model
                            .get("id")
                            .and_then(|v| v.as_str())
                            .is_some_and(|id| !id.trim().is_empty())
                    }) =>
                {
                    "available"
                }
                _ => "invalid_response",
            },
            Err(_) => "invalid_response",
        }
    })
    .await
    .unwrap_or("timed_out")
}

async fn keeper_transport(path: Option<&std::path::Path>) -> &'static str {
    let Some(path) = path else {
        return "not_configured";
    };
    if !path.is_absolute() {
        return "invalid_endpoint";
    }
    match tokio::time::timeout(
        Duration::from_secs(1),
        tokio::net::UnixStream::connect(path),
    )
    .await
    {
        Ok(Ok(stream)) => match stream.peer_cred() {
            Ok(peer) if peer.uid() == unsafe { libc::geteuid() } => "same_user_connected",
            _ => "peer_rejected",
        },
        Ok(Err(_)) => "unreachable",
        Err(_) => "timed_out",
    }
}
