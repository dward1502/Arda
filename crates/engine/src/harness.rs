//! The `arda` harness surface — the ONE tap-in port Hermes (and any operator
//! tool) connects to. It is NOT the inference gateway (that is `manwe` @7171);
//! the harness is the daemon's own control/status surface.
//!
//! Bind address is configurable (default `127.0.0.1:7878`) and deliberately
//! distinct from `manwe`'s `7171` so the two never collide.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Json;
use serde::Serialize;
use tokio::sync::Notify;
use tracing::{info, warn};

/// Default harness bind address.
pub const DEFAULT_HARNESS_ADDR: &str = "127.0.0.1:7878";

/// Shared harness state, injected into the axum router.
#[derive(Clone)]
pub struct HarnessState {
    /// Live supervised child PIDs, refreshed by the supervisor.
    pub child_pids: Arc<tokio::sync::RwLock<Vec<u32>>>,
    /// Names of services the harness knows about.
    pub service_names: Arc<Vec<String>>,
    /// The `manwe` gateway base URL the harness proxies `/v1/models` to.
    pub manwe_url: String,
}

#[derive(Serialize)]
struct Status {
    daemon: &'static str,
    harness_addr: String,
    manwe_url: String,
    services: Vec<String>,
    child_pids: Vec<u32>,
}

/// Build the axum router for the harness surface.
fn router(state: HarnessState) -> axum::Router {
    axum::Router::new()
        .route("/health", get(health))
        .route("/v1/status", get(status))
        .route("/v1/models", get(models))
        .route("/v1/harness", get(harness_info))
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
        harness_addr: DEFAULT_HARNESS_ADDR.to_string(),
        manwe_url: st.manwe_url.clone(),
        services: (*st.service_names).clone(),
        child_pids: pids,
    };
    (StatusCode::OK, Json(body))
}

/// Thin proxy to `manwe`'s `/v1/models` so callers only ever talk to the
/// harness port (one tap-in surface), not the gateway's internal 7171.
async fn models(State(st): State<HarnessState>) -> impl IntoResponse {
    let url = format!("{}/v1/models", st.manwe_url.trim_end_matches('/'));
    match reqwest::get(&url).await {
        Ok(resp) => match resp.json::<serde_json::Value>().await {
            Ok(v) => (StatusCode::OK, Json(v)).into_response(),
            Err(e) => {
                warn!("harness: failed to parse manwe /v1/models: {e}");
                (StatusCode::BAD_GATEWAY, "manwe returned unparseable body").into_response()
            }
        },
        Err(e) => {
            warn!("harness: manwe /v1/models unreachable at {url}: {e}");
            (StatusCode::BAD_GATEWAY, "manwe unreachable").into_response()
        }
    }
}

/// Self-describing harness info: the single tap-in contract.
async fn harness_info() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "harness": "arda",
            "bind": DEFAULT_HARNESS_ADDR,
            "connect_here": true,
            "routes": ["/health", "/v1/status", "/v1/models", "/v1/harness"],
        })),
    )
}

/// Start the harness HTTP surface, binding `addr` (falls back to
/// `DEFAULT_HARNESS_ADDR` when `None`). Returns the bound `SocketAddr` and a
/// `JoinHandle` for the serving task. The `shutdown` notify stops it.
pub async fn serve(
    addr: Option<SocketAddr>,
    state: HarnessState,
    shutdown: Arc<Notify>,
) -> anyhow::Result<(SocketAddr, tokio::task::JoinHandle<()>)> {
    let addr = addr.unwrap_or_else(|| DEFAULT_HARNESS_ADDR.parse().unwrap());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound = listener.local_addr()?;
    info!("harness: listening on {bound}");
    let app = router(state);
    let handle = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async move { shutdown.notified().await })
            .await
            .ok();
        info!("harness: stopped");
    });
    Ok((bound, handle))
}
