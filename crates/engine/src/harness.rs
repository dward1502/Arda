//! The `arda` harness surface — the ONE tap-in port Hermes (and any operator
//! tool) connects to. It is NOT the inference gateway (that is `manwe` @7171);
//! the harness is the daemon's own control/status surface.
//!
//! Bind address is configurable (default `127.0.0.1:7878`) and deliberately
//! distinct from `manwe`'s `7171` so the two never collide.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::harness::presence::HarnessPresenceState;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Json;
use serde::Serialize;
use tokio::sync::Notify;
use tracing::{info, warn};

pub mod presence;
mod projects;
mod research;
mod runs;

/// Default harness bind address.
pub const DEFAULT_HARNESS_ADDR: &str = "127.0.0.1:7878";

/// Default timeout for proxy requests to `manwe`.
pub const DEFAULT_MANWE_PROXY_TIMEOUT: Duration = Duration::from_secs(5);

/// Default timeout for bounded Warden scout queries.
pub const DEFAULT_WARDEN_SCOUT_TIMEOUT: Duration = Duration::from_secs(30);

/// Shared harness state, injected into the axum router.
#[derive(Clone)]
pub struct HarnessState {
    /// Actual bound harness address, populated by `serve` after binding.
    pub harness_addr: String,
    /// Live supervised child PIDs, refreshed by the supervisor.
    pub child_pids: Arc<tokio::sync::RwLock<Vec<u32>>>,
    /// Names of services the harness knows about.
    pub service_names: Arc<Vec<String>>,
    /// The `manwe` gateway base URL the harness proxies `/v1/models` to.
    pub manwe_url: String,
    /// HTTP client used for outbound proxy requests.
    pub client: reqwest::Client,
    /// Per-request timeout applied when proxying `/v1/models`.
    pub manwe_proxy_timeout: Duration,
    /// Optional bearer token forwarded as `Authorization` on `/v1/models`
    /// proxy requests. Useful when `manwe` requires auth but callers only
    /// talk to the harness surface.
    pub manwe_proxy_bearer: Option<String>,
    /// Optional Tailscale URL for the Warden scout outpost.
    pub warden_scout_url: Option<String>,
    /// Per-request timeout for Warden search and recall operations.
    pub warden_scout_timeout: Duration,
    /// Presence projection inputs published through the harness.
    pub presence_inputs: presence::HarnessPresenceState,
    /// Repository root containing canonical project and run state.
    pub workbench_root: PathBuf,
}

#[derive(Serialize)]
struct Status {
    daemon: &'static str,
    harness_addr: String,
    manwe_url: String,
    warden_scout_url: Option<String>,
    services: Vec<String>,
    child_pids: Vec<u32>,
}

/// Build the axum router for the harness surface.
fn router(state: HarnessState) -> axum::Router {
    axum::Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/models", get(models))
        .route("/v1/scout/health", get(scout_health))
        .route("/v1/scout/search", post(scout_search))
        .route("/v1/scout/recall", post(scout_recall))
        .route("/v1/research/brief", post(research::create_brief))
        .route("/v1/harness", get(harness_info))
        .route("/v1/projects/validate", post(projects::validate_project))
        .route("/v1/projects/attach", post(projects::attach_project))
        .route("/v1/projects", get(projects::list_projects))
        .route("/v1/runs/plan", post(runs::plan_run))
        .route("/v1/runs/:id/approve", post(runs::approve_run))
        .route(
            "/v1/runs/:id/nodes/:node_id/complete",
            post(runs::complete_run_node),
        )
        .route(
            "/v1/runs/:id/nodes/:node_id/execute-provider",
            post(runs::execute_provider_node),
        )
        .route("/v1/runs/:id/cancel", post(runs::cancel_run))
        .route("/v1/runs/:id", get(runs::get_run))
        .route("/v1/runs/:id/events", get(runs::get_run_events))
        .route("/v1/runs/:id/events/stream", get(runs::stream_run_events))
        .merge(presence::PresenceRouter::router(
            HarnessPresenceState::default(),
        ))
        .with_state(state)
}

/// Liveness probe. Returns 200 once the harness is listening.
async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// Operator status surface: what the daemon is supervising and where the
/// gateway lives. This is the single place an external tool queries to learn
/// the system's shape.
async fn status(State(st): State<HarnessState>) -> impl IntoResponse {
    let pids = st.child_pids.read().await.clone();
    let body = Status {
        daemon: "arda",
        harness_addr: st.harness_addr.clone(),
        manwe_url: st.manwe_url.clone(),
        warden_scout_url: st.warden_scout_url.clone(),
        services: (*st.service_names).clone(),
        child_pids: pids,
    };
    (StatusCode::OK, Json(body))
}

/// Thin proxy to `manwe`'s `/v1/models` so callers only ever talk to the
/// harness port (one tap-in surface), not the gateway's internal 7171.
async fn models(State(st): State<HarnessState>) -> impl IntoResponse {
    let target = format!("{}/v1/models", st.manwe_url.trim_end_matches('/'));
    let mut request = st.client.get(&target).timeout(st.manwe_proxy_timeout);
    if let Some(bearer) = st.manwe_proxy_bearer.as_ref() {
        request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {bearer}"));
    }
    match request.send().await {
        Ok(resp) => match resp.bytes().await {
            Ok(bytes) => match serde_json::from_slice::<serde_json::Value>(&bytes) {
                Ok(v) => (StatusCode::OK, Json(v)).into_response(),
                Err(e) => {
                    warn!("harness: failed to parse manwe /v1/models: {e}");
                    (StatusCode::BAD_GATEWAY, "manwe returned unparseable body").into_response()
                }
            },
            Err(e) => {
                warn!("harness: failed to read manwe /v1/models body: {e}");
                (StatusCode::BAD_GATEWAY, "manwe returned unreadable body").into_response()
            }
        },
        Err(e) => {
            warn!("harness: manwe /v1/models unreachable at {target}: {e}");
            (
                StatusCode::BAD_GATEWAY,
                format!("manwe unreachable: {target}"),
            )
                .into_response()
        }
    }
}

async fn scout_health(State(st): State<HarnessState>) -> impl IntoResponse {
    proxy_scout_get(&st, "/health").await
}

async fn scout_search(
    State(st): State<HarnessState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    proxy_scout_post(&st, "/search", body).await
}

async fn scout_recall(
    State(st): State<HarnessState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    proxy_scout_post(&st, "/recall", body).await
}

async fn proxy_scout_get(st: &HarnessState, path: &str) -> axum::response::Response {
    let Some(base_url) = st.warden_scout_url.as_deref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Warden scout is not configured"})),
        )
            .into_response();
    };
    let target = format!("{}{path}", base_url.trim_end_matches('/'));
    proxy_scout_response(
        st.client.get(&target).timeout(st.warden_scout_timeout),
        &target,
    )
    .await
}

async fn proxy_scout_post(
    st: &HarnessState,
    path: &str,
    body: serde_json::Value,
) -> axum::response::Response {
    let Some(base_url) = st.warden_scout_url.as_deref() else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"error": "Warden scout is not configured"})),
        )
            .into_response();
    };
    let target = format!("{}{path}", base_url.trim_end_matches('/'));
    proxy_scout_response(
        st.client
            .post(&target)
            .timeout(st.warden_scout_timeout)
            .json(&body),
        &target,
    )
    .await
}

async fn proxy_scout_response(
    request: reqwest::RequestBuilder,
    target: &str,
) -> axum::response::Response {
    match request.send().await {
        Ok(response) => {
            let status = response.status();
            match response.json::<serde_json::Value>().await {
                Ok(body) => (status, Json(body)).into_response(),
                Err(error) => {
                    warn!("harness: Warden scout returned invalid JSON from {target}: {error}");
                    (
                        StatusCode::BAD_GATEWAY,
                        Json(serde_json::json!({"error": "Warden scout returned invalid JSON"})),
                    )
                        .into_response()
                }
            }
        }
        Err(error) => {
            warn!("harness: Warden scout unreachable at {target}: {error}");
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": "Warden scout is unreachable"})),
            )
                .into_response()
        }
    }
}

/// Self-describing harness info: the single tap-in contract.
async fn harness_info(State(st): State<HarnessState>) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "harness": "arda",
            "bind": st.harness_addr,
            "connect_here": true,
            "routes": [
                "/health",
                "/v1/status",
                "/v1/models",
                "/v1/scout/health",
                "/v1/scout/search",
                "/v1/scout/recall",
                "/v1/research/brief",
                "/v1/harness"
            ],
        })),
    )
}

/// Start the harness HTTP surface. Uses `addr` when provided, otherwise reads
/// `ARDA_HARNESS_BIND_ADDR` from the environment, falling back to
/// `DEFAULT_HARNESS_ADDR`. Returns the bound `SocketAddr` and a
/// `JoinHandle` for the serving task. The `shutdown` notify stops it.
pub async fn serve(
    addr: Option<SocketAddr>,
    mut state: HarnessState,
    shutdown: Arc<Notify>,
) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let addr = addr
        .or_else(|| std::env::var("ARDA_HARNESS_BIND_ADDR").ok()?.parse().ok())
        .unwrap_or_else(|| DEFAULT_HARNESS_ADDR.parse().unwrap());
    if !addr.ip().is_loopback() {
        anyhow::bail!(
            "harness bind address {addr} is not loopback; remote exposure requires inbound authentication"
        );
    }
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    state.harness_addr = bound.to_string();
    info!("harness: listening on {bound}");
    let app = router(state);
    let handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move { shutdown.notified().await })
        .await
        .ok();
        info!("harness: stopped");
    });
    Ok((bound, handle))
}

#[cfg(test)]
mod tests {
    use super::{serve, HarnessState, DEFAULT_HARNESS_ADDR, DEFAULT_MANWE_PROXY_TIMEOUT};
    use crate::harness::presence::HarnessPresenceState;

    use axum::{
        http::HeaderMap,
        routing::{get, post},
        Json, Router,
    };
    use reqwest::Client;
    use serde_json::{json, Value};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::{Notify, RwLock};

    #[tokio::test]
    async fn harness_rejects_non_loopback_bind_without_inbound_authentication() {
        let state = HarnessState {
            harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            manwe_url: "http://127.0.0.1:1".into(),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
            manwe_proxy_bearer: None,
            warden_scout_url: None,
            warden_scout_timeout: std::time::Duration::from_secs(2),
            presence_inputs: HarnessPresenceState::default(),
            workbench_root: std::env::temp_dir(),
        };

        let error = serve(
            Some("0.0.0.0:0".parse().expect("non-loopback address")),
            state,
            Arc::new(Notify::new()),
        )
        .await
        .expect_err("unauthenticated harness must remain loopback-only");

        assert!(error.to_string().contains("loopback"));
    }

    #[tokio::test]
    async fn harness_proxies_search_to_the_configured_warden_scout() {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("upstream bind");
        let upstream_addr = upstream.local_addr().expect("upstream address");
        let upstream_handle = tokio::spawn(async move {
            axum::serve(
                upstream,
                Router::new().route(
                    "/search",
                    post(|Json(body): Json<Value>| async move {
                        Json(json!({"ok": true, "request": body}))
                    }),
                ),
            )
            .await
            .expect("upstream server");
        });

        let shutdown = Arc::new(Notify::new());
        let state = HarnessState {
            harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            manwe_url: "http://127.0.0.1:1".into(),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
            manwe_proxy_bearer: None,
            warden_scout_url: Some(format!("http://{upstream_addr}")),
            warden_scout_timeout: std::time::Duration::from_secs(2),
            presence_inputs: HarnessPresenceState::default(),
            workbench_root: std::env::temp_dir(),
        };
        let (bound, harness_handle) = serve(
            Some("127.0.0.1:0".parse().expect("harness address")),
            state,
            shutdown.clone(),
        )
        .await
        .expect("start harness");

        let status: Value = reqwest::get(format!("http://{bound}/v1/status"))
            .await
            .expect("status request")
            .error_for_status()
            .expect("status code")
            .json()
            .await
            .expect("status body");
        assert_eq!(status["harness_addr"], bound.to_string());

        let response: Value = reqwest::Client::new()
            .post(format!("http://{bound}/v1/scout/search"))
            .json(&json!({
                "query": "governance",
                "limit": 3,
                "source_policy": "allowlisted_public_web",
                "expires_at": "2026-07-30T00:00:00Z"
            }))
            .send()
            .await
            .expect("proxy request")
            .error_for_status()
            .expect("proxy status")
            .json()
            .await
            .expect("proxy body");
        assert_eq!(response["request"]["query"], "governance");
        assert_eq!(response["request"]["limit"], 3);
        assert_eq!(
            response["request"]["source_policy"],
            "allowlisted_public_web"
        );
        assert_eq!(response["request"]["expires_at"], "2026-07-30T00:00:00Z");

        shutdown.notify_waiters();
        harness_handle.await.expect("harness join");
        upstream_handle.abort();
    }

    #[tokio::test]
    async fn models_proxy_forwards_the_configured_bearer() {
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("upstream bind");
        let upstream_addr = upstream.local_addr().expect("upstream address");
        let upstream_handle = tokio::spawn(async move {
            axum::serve(
                upstream,
                Router::new().route(
                    "/v1/models",
                    get(|headers: HeaderMap| async move {
                        assert_eq!(
                            headers
                                .get(reqwest::header::AUTHORIZATION)
                                .and_then(|value| value.to_str().ok()),
                            Some("Bearer harness-secret")
                        );
                        Json(json!({"data": []}))
                    }),
                ),
            )
            .await
            .expect("upstream server");
        });

        let shutdown = Arc::new(Notify::new());
        let state = HarnessState {
            harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            manwe_url: format!("http://{upstream_addr}"),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
            manwe_proxy_bearer: Some("harness-secret".into()),
            warden_scout_url: None,
            warden_scout_timeout: std::time::Duration::from_secs(2),
            presence_inputs: HarnessPresenceState::default(),
            workbench_root: std::env::temp_dir(),
        };
        let (bound, harness_handle) = serve(
            Some("127.0.0.1:0".parse().expect("harness address")),
            state,
            shutdown.clone(),
        )
        .await
        .expect("start harness");

        let response: Value = reqwest::get(format!("http://{bound}/v1/models"))
            .await
            .expect("models request")
            .error_for_status()
            .expect("models status")
            .json()
            .await
            .expect("models body");
        assert_eq!(response, json!({"data": []}));

        shutdown.notify_waiters();
        harness_handle.await.expect("harness join");
        upstream_handle.abort();
    }

    #[tokio::test]
    async fn models_proxy_reports_network_loss_without_false_completion() {
        let shutdown = Arc::new(Notify::new());
        let state = HarnessState {
            harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            manwe_url: "http://127.0.0.1:1".into(),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: Duration::from_millis(100),
            manwe_proxy_bearer: None,
            warden_scout_url: None,
            warden_scout_timeout: Duration::from_millis(100),
            presence_inputs: HarnessPresenceState::default(),
            workbench_root: std::env::temp_dir(),
        };
        let (bound, harness_handle) = serve(
            Some("127.0.0.1:0".parse().expect("harness address")),
            state,
            shutdown.clone(),
        )
        .await
        .expect("start harness");

        let response = reqwest::get(format!("http://{bound}/v1/models"))
            .await
            .expect("harness response");
        assert_eq!(response.status(), reqwest::StatusCode::BAD_GATEWAY);
        assert!(response
            .text()
            .await
            .expect("error body")
            .contains("manwe unreachable"));

        shutdown.notify_waiters();
        harness_handle.await.expect("harness join");
    }

    #[tokio::test]
    async fn presence_routes_are_published_through_the_harness() {
        let state = HarnessState {
            harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
            child_pids: Arc::new(RwLock::new(Vec::new())),
            service_names: Arc::new(Vec::new()),
            manwe_url: "http://127.0.0.1:1".into(),
            client: reqwest::Client::new(),
            manwe_proxy_timeout: DEFAULT_MANWE_PROXY_TIMEOUT,
            manwe_proxy_bearer: None,
            warden_scout_url: None,
            warden_scout_timeout: std::time::Duration::from_secs(2),
            presence_inputs: HarnessPresenceState::default(),
            workbench_root: std::env::temp_dir(),
        };
        let shutdown = Arc::new(Notify::new());
        let (bound, harness_handle) = serve(
            Some("127.0.0.1:0".parse().expect("harness address")),
            state,
            shutdown.clone(),
        )
        .await
        .expect("start harness");

        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("build client");
        let snapshot: Value = client
            .get(format!("http://{bound}/v1/presence/snapshot"))
            .send()
            .await
            .expect("snapshot request")
            .error_for_status()
            .expect("snapshot status")
            .json()
            .await
            .expect("snapshot body");
        assert_eq!(snapshot["schema_version"], "arda.harness.presence.v1");

        let response = client
            .get(format!("http://{bound}/v1/presence/events"))
            .send()
            .await
            .expect("events request");
        assert_eq!(response.status(), 200);
        assert_eq!(
            response
                .headers()
                .get("content-type")
                .map(|v| v.to_str().unwrap_or("")),
            Some("text/event-stream")
        );

        shutdown.notify_waiters();
        harness_handle.await.expect("harness join");
    }
}
