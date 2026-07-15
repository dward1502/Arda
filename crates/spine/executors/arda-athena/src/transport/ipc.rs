// sigil: REPAIR
use crate::ingest::AthenaStore;
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::time::timeout;

pub async fn run_ipc_server(store: AthenaStore, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "athena".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;

    tracing::info!(socket = %socket_path.display(), "ATHENA IPC server listening");

    loop {
        let (stream, _) = listener.accept().await.map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("IPC accept error: {e}"),
        })?;

        let store = store.clone();
        let _ = spawn_bounded_background(
            "athena_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                if let Err(err) = handle_connection(stream, store).await {
                    tracing::warn!(error = %err, "IPC client connection failed");
                }
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ARDA_ATHENA_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

fn ipc_connect_timeout() -> Duration {
    let secs = std::env::var("ARDA_ATHENA_IPC_CONNECT_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5);
    Duration::from_secs(secs)
}

fn ipc_idle_timeout() -> Duration {
    let secs = std::env::var("ARDA_ATHENA_IPC_IDLE_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30);
    Duration::from_secs(secs)
}

fn ipc_io_timeout() -> Duration {
    // Default 120s: ATHENA deep_analyze runs LLM-driven knowledge extraction
    // through Charon's model router, which can take 30-90s on large models.
    // Override via ARDA_ATHENA_IPC_IO_TIMEOUT_SECS for fast-path tools
    // that need tighter timeouts.
    let secs = std::env::var("ARDA_ATHENA_IPC_IO_TIMEOUT_SECS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(120);
    Duration::from_secs(secs)
}

async fn handle_connection(stream: UnixStream, store: AthenaStore) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    let idle = ipc_idle_timeout();
    let io_deadline = ipc_io_timeout();

    loop {
        let next = match timeout(idle, lines.next_line()).await {
            Ok(Ok(Some(line))) => line,
            Ok(Ok(None)) => break,
            Ok(Err(e)) => {
                return Err(ArdaError::Agent {
                    agent: "athena".to_string(),
                    message: format!("IPC read error: {e}"),
                });
            }
            Err(_) => {
                tracing::debug!(
                    idle_seconds = idle.as_secs(),
                    "ATHENA IPC idle timeout; closing connection"
                );
                break;
            }
        };

        let response = match serde_json::from_str::<CommandEnvelope>(&next) {
            Ok(cmd) => match execute_command(&store, cmd) {
                Ok(value) => json!({"ok": true, "result": value}),
                Err(err) => json!({"ok": false, "error": err.to_string()}),
            },
            Err(err) => json!({"ok": false, "error": format!("invalid command: {err}")}),
        };

        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        timeout(io_deadline, writer.write_all(&encoded))
            .await
            .map_err(|_| ArdaError::Agent {
                agent: "athena".to_string(),
                message: format!("IPC write timeout after {}s", io_deadline.as_secs()),
            })?
            .map_err(|e| ArdaError::Agent {
                agent: "athena".to_string(),
                message: format!("IPC write error: {e}"),
            })?;
    }

    Ok(())
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let connect_deadline = ipc_connect_timeout();
    let io_deadline = ipc_io_timeout();

    let mut stream = timeout(connect_deadline, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!(
                "connect timeout after {}s on ATHENA socket {}",
                connect_deadline.as_secs(),
                socket_path.display()
            ),
        })?
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!(
                "failed to connect to ATHENA socket {}: {e}",
                socket_path.display()
            ),
        })?;

    let request = json!({
        "cmd": cmd,
        "payload": payload,
    });
    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    timeout(io_deadline, stream.write_all(&encoded))
        .await
        .map_err(|_| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("IPC write timeout after {}s", io_deadline.as_secs()),
        })?
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("failed to write IPC request: {e}"),
        })?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    timeout(io_deadline, reader.read_line(&mut line))
        .await
        .map_err(|_| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("IPC read timeout after {}s", io_deadline.as_secs()),
        })?
        .map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("failed to read IPC response: {e}"),
        })?;

    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("invalid IPC response: {e}"),
        })?;

    response.into_result("athena")
}

fn execute_command(store: &AthenaStore, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(serde_json::to_value(store.status()?)?),
        "metrics" => {
            let _ = store.status()?;
            Ok(json!({"format": "prometheus", "text": store.metrics().render_prometheus()}))
        }
        "ingest" => {
            let input = payload_str(&cmd.payload, &["raw_input", "input", "url"])?;
            let submitted_by =
                payload_str_optional(&cmd.payload, "submitted_by").unwrap_or("orchestrator");
            let task_context =
                payload_str_optional(&cmd.payload, "task_context").unwrap_or("ipc ingest");
            Ok(serde_json::to_value(store.ingest(
                input,
                submitted_by,
                task_context,
            )?)?)
        }
        "ingest_batch" => {
            let inputs = payload_string_vec(&cmd.payload, "inputs")?;
            let submitted_by =
                payload_str_optional(&cmd.payload, "submitted_by").unwrap_or("orchestrator");
            let task_context =
                payload_str_optional(&cmd.payload, "task_context").unwrap_or("ipc batch ingest");
            Ok(serde_json::to_value(store.ingest_batch(
                &inputs,
                submitted_by,
                task_context,
            )?)?)
        }
        "query" => {
            let query = payload_str(&cmd.payload, &["query"])?;
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(8) as usize;
            Ok(serde_json::to_value(store.query(query, limit)?)?)
        }
        "deep_analyze" | "deep" => {
            let source_id = payload_str(&cmd.payload, &["source_id"])?;
            let reason = payload_str_optional(&cmd.payload, "reason").unwrap_or("ipc deep request");
            let queued = store.queue_deep_analysis(source_id, "orchestrator", reason)?;
            let deep = store.deep_analyze(source_id)?;
            Ok(json!({"queued": queued, "deep": deep}))
        }
        "deep_process" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(25) as usize;
            let retry_failed = cmd
                .payload
                .get("retry_failed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(store.process_deep_queue(limit, retry_failed)?)
        }
        "digest" => {
            let source_id = payload_str_optional(&cmd.payload, "source_id");
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(25) as usize;
            Ok(serde_json::to_value(store.read_digest(source_id, limit)?)?)
        }
        "policy_readiness" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(25) as usize;
            Ok(serde_json::to_value(store.policy_readiness(limit)?)?)
        }
        "policy_promote" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(25) as usize;
            let reevaluate = cmd
                .payload
                .get("reevaluate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(store.promote_policy_readiness(limit, reevaluate)?)
        }
        "harvest_opposition" => {
            let source_id = payload_str(&cmd.payload, &["source_id"])?;
            let topic = payload_str_optional(&cmd.payload, "topic");
            let submitted_by =
                payload_str_optional(&cmd.payload, "submitted_by").unwrap_or("orchestrator");
            Ok(store.harvest_opposition_evidence(source_id, topic, submitted_by)?)
        }
        "generate_planning_tasks" => {
            let source_id = payload_str(&cmd.payload, &["source_id"])?;
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as usize;
            Ok(store.generate_planning_tasks(source_id, limit)?)
        }
        other => Err(ArdaError::Agent {
            agent: "athena".to_string(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}

fn payload_str<'a>(payload: &'a Value, keys: &[&str]) -> Result<&'a str> {
    for key in keys {
        if let Some(value) = payload.get(*key).and_then(|v| v.as_str()) {
            return Ok(value);
        }
    }
    Err(ArdaError::Agent {
        agent: "athena".to_string(),
        message: format!("missing required payload key, expected one of {:?}", keys),
    })
}

fn payload_str_optional<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str())
}

fn payload_string_vec(payload: &Value, key: &str) -> Result<Vec<String>> {
    let items =
        payload
            .get(key)
            .and_then(|v| v.as_array())
            .ok_or_else(|| ArdaError::Agent {
                agent: "athena".to_string(),
                message: format!("missing required payload key: {key}"),
            })?;

    let values = items
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    Ok(values)
}

#[cfg(test)]
// Tests that mutate process environment variables must serialize across await
// points so no other test observes a partially configured runtime. This is
// test-scaffolding only; production code must not hold std mutex guards across
// async boundaries.
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::{handle_connection, run_ipc_server, send_command};
    use crate::ingest::AthenaStore;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::net::{UnixListener, UnixStream};
    use tokio::time::{sleep, Duration};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_support::env_guard()
    }

    #[tokio::test]
    async fn ipc_round_trip_ingest_query_status() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let socket_path = dir.path().join("athena.sock");

        let server = tokio::spawn(run_ipc_server(store, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let ingested = send_command(
            socket_path.clone(),
            "ingest",
            json!({
                "input": "https://github.com/example/rust-api",
                "submitted_by": "test",
                "task_context": "ipc test"
            }),
        )
        .await;
        if let Err(err) = ingested {
            let msg = err.to_string();
            if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                server.abort();
                return;
            }
            panic!("ingest: {msg}");
        }

        let queried = send_command(
            socket_path.clone(),
            "query",
            json!({"query": "rust", "limit": 5}),
        )
        .await
        .expect("query");
        assert!(
            queried
                .get("total_matches")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
                >= 1
        );

        let status = send_command(socket_path.clone(), "status", json!({}))
            .await
            .expect("status");
        assert_eq!(status.get("books_count").and_then(|v| v.as_u64()), Some(1));

        let metrics = send_command(socket_path, "metrics", json!({}))
            .await
            .expect("metrics");
        assert_eq!(
            metrics.get("format").and_then(|v| v.as_str()),
            Some("prometheus")
        );
        assert!(metrics
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("athena_ingest_documents_total"));

        server.abort();
    }

    #[tokio::test]
    async fn ipc_client_connect_times_out_on_dead_socket() {
        let _guard = env_guard();
        std::env::set_var("ARDA_ATHENA_IPC_CONNECT_TIMEOUT_SECS", "1");

        let dir = tempdir().expect("tempdir");
        let socket_path = dir.path().join("nonexistent.sock");

        let start = std::time::Instant::now();
        let err = send_command(socket_path, "status", json!({}))
            .await
            .expect_err("expected connect failure");
        let elapsed = start.elapsed();

        // Must bail fast (<5s). The connect to a nonexistent socket fails
        // immediately with ENOENT, so we only assert the error is surfaced
        // cleanly.
        assert!(
            elapsed < Duration::from_secs(5),
            "send_command took too long: {elapsed:?}"
        );
        let msg = err.to_string();
        assert!(
            msg.contains("failed to connect") || msg.contains("connect timeout"),
            "unexpected error: {msg}"
        );

        std::env::remove_var("ARDA_ATHENA_IPC_CONNECT_TIMEOUT_SECS");
    }

    #[tokio::test]
    async fn ipc_server_closes_idle_connection() {
        let _guard = env_guard();
        std::env::set_var("ARDA_ATHENA_IPC_IDLE_TIMEOUT_SECS", "1");

        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new(dir.path()).expect("store");
        let socket_path = dir.path().join("idle.sock");
        let listener = match UnixListener::bind(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                    std::env::remove_var("ARDA_ATHENA_IPC_IDLE_TIMEOUT_SECS");
                    return;
                }
                panic!("bind: {err}");
            }
        };

        let server_task = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            handle_connection(stream, store).await
        });

        // Connect a client and never send a command.
        let _client = UnixStream::connect(&socket_path).await.expect("connect");
        let start = std::time::Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(5), server_task)
            .await
            .expect("server finished")
            .expect("server join");
        let elapsed = start.elapsed();

        assert!(
            result.is_ok(),
            "handle_connection returned error: {result:?}"
        );
        assert!(
            elapsed >= Duration::from_millis(900),
            "server closed too fast: {elapsed:?}"
        );
        assert!(
            elapsed < Duration::from_secs(4),
            "server waited past the idle timeout: {elapsed:?}"
        );

        std::env::remove_var("ARDA_ATHENA_IPC_IDLE_TIMEOUT_SECS");
    }
}
