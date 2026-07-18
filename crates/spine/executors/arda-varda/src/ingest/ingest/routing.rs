// sigil: REPAIR
//
// Inference-router bridging: the sync snapshot entry point plus the IPC/HTTP
// transport paths it delegates to, and a sync-bridge that lets us block on
// futures from non-async call sites.

use arda_core::daemon::ResponseEnvelope;
use arda_core::error::{ArdaError, Result};
use arda_core::try_run_bounded;
use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::time::timeout;

use super::athena_error;

fn route_connect_timeout() -> Duration {
    let secs = std::env::var("ARDA_ATHENA_IPC_CONNECT_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    Duration::from_secs(secs)
}

fn route_io_timeout() -> Duration {
    let secs = std::env::var("ARDA_ATHENA_IPC_IO_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(15);
    Duration::from_secs(secs)
}

pub(super) fn resolve_inference_route_snapshot(
    source_id: &str,
    title: &str,
    tags: &[String],
) -> serde_json::Value {
    let origin_default = std::env::var("ARDA_ROUTE_ORIGIN_DEFAULT")
        .unwrap_or_else(|_| "auto".to_string())
        .to_ascii_lowercase();
    let origin_preference = if matches!(origin_default.as_str(), "local" | "cloud" | "auto") {
        origin_default
    } else {
        "auto".to_string()
    };
    let route_payload = serde_json::json!({
        "agent_id": "athena",
        "task_type": "research",
        "priority": "normal",
        "messages": [
            {
                "role": "system",
                "content": "ATHENA knowledge routing request."
            },
            {
                "role": "user",
                "content": format!(
                    "Route deep analysis for source_id={} title='{}' tags=[{}]",
                    source_id,
                    title,
                    tags.join(", ")
                )
            }
        ],
        "options": {
            "privacy_tier": "internal",
            "cost_tier": "balanced",
            "quality_tier": "high",
            "origin_preference": origin_preference,
            "latency_sla_ms": 5000
        }
    });

    let route_url = std::env::var("ARDA_INFERENCE_ROUTER_ROUTE_URL").ok();
    let socket_path = std::env::var("ARDA_INFERENCE_ROUTER_SOCKET")
        .or_else(|_| std::env::var("ARDA_CHARON_SOCKET"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data/charon/charon.sock"));

    if socket_path.exists() {
        if let Ok(result) =
            run_async_for_sync(route_via_ipc(socket_path.clone(), route_payload.clone()))
        {
            return serde_json::json!({
                "mode": "ipc",
                "task_type": "research",
                "router_socket": socket_path,
                "source_id": source_id,
                "decision": result
            });
        }
    }

    if let Some(url) = route_url {
        if let Ok(result) = run_async_for_sync(route_via_http(url.clone(), route_payload.clone())) {
            return serde_json::json!({
                "mode": "http",
                "task_type": "research",
                "router_url": url,
                "source_id": source_id,
                "decision": result
            });
        }
    }

    serde_json::json!({
        "mode": "unconfigured",
        "task_type": "research",
        "source_id": source_id,
        "reason": "no compatible inference router endpoint configured or reachable"
    })
}

async fn route_via_ipc(
    socket_path: PathBuf,
    route_payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let connect_deadline = route_connect_timeout();
    let io_deadline = route_io_timeout();

    let mut stream = timeout(connect_deadline, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| {
            athena_error(format!(
                "connect timeout after {}s on inference router socket {}",
                connect_deadline.as_secs(),
                socket_path.display()
            ))
        })?
        .map_err(|e| {
            athena_error(format!(
                "failed to connect to inference router socket {}: {e}",
                socket_path.display()
            ))
        })?;

    let request = serde_json::json!({
        "cmd": "route",
        "payload": route_payload,
    });
    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    timeout(io_deadline, stream.write_all(&encoded))
        .await
        .map_err(|_| {
            athena_error(format!(
                "inference router IPC write timeout after {}s",
                io_deadline.as_secs()
            ))
        })?
        .map_err(|e| athena_error(format!("failed to write inference router IPC request: {e}")))?;

    let mut reader = tokio::io::BufReader::new(stream);
    let mut line = String::new();
    timeout(io_deadline, reader.read_line(&mut line))
        .await
        .map_err(|_| {
            athena_error(format!(
                "inference router IPC read timeout after {}s",
                io_deadline.as_secs()
            ))
        })?
        .map_err(|e| athena_error(format!("failed to read inference router IPC response: {e}")))?;
    let response: ResponseEnvelope = serde_json::from_str(line.trim())
        .map_err(|e| athena_error(format!("invalid inference router IPC response: {e}")))?;
    response.into_result("athena")
}

async fn route_via_http(
    route_url: String,
    route_payload: serde_json::Value,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let response = client
        .post(&route_url)
        .json(&route_payload)
        .send()
        .await
        .map_err(|e| {
            athena_error(format!(
                "failed to POST inference router route request: {e}"
            ))
        })?
        .error_for_status()
        .map_err(|e| athena_error(format!("inference router HTTP route request failed: {e}")))?;
    let value: serde_json::Value = response
        .json()
        .await
        .map_err(|e| athena_error(format!("invalid inference router HTTP response JSON: {e}")))?;
    if value.get("ok").and_then(|v| v.as_bool()) == Some(true) {
        Ok(value
            .get("decision")
            .cloned()
            .or_else(|| value.get("result").cloned())
            .unwrap_or_else(|| serde_json::json!({})))
    } else {
        Ok(value)
    }
}

pub(super) fn run_async_for_sync<F, T>(fut: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_ok() {
        let Some(result) = try_run_bounded("athena_sync_bridge", 1, || {
            std::thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(ArdaError::Ledger)?;
                rt.block_on(fut)
            })
            .join()
            .map_err(|_| athena_error("failed to join inference router worker thread"))?
        }) else {
            return Err(athena_error("sync bridge concurrency gate saturated"));
        };
        result
    } else {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(ArdaError::Ledger)?;
        rt.block_on(fut)
    }
}
