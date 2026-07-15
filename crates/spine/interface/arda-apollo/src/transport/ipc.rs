// sigil: REPAIR
use crate::{ApolloService, ExecutionPriority, ExecutionRequest, InterruptionAttachmentRequest};
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: ApolloService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "apollo".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;
    tracing::info!(socket = %socket_path.display(), "APOLLO IPC server listening");

    loop {
        let (stream, _) = listener.accept().await.map_err(|e| ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "apollo_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                if let Err(err) = handle_connection(stream, service).await {
                    tracing::warn!(error = %err, "APOLLO IPC client connection failed");
                }
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_APOLLO_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn handle_connection(stream: UnixStream, service: ApolloService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| ArdaError::Agent {
        agent: "apollo".to_string(),
        message: format!("IPC read error: {e}"),
    })? {
        let response = match serde_json::from_str::<CommandEnvelope>(&line) {
            Ok(cmd) => match execute_command(&service, cmd).await {
                Ok(value) => json!({"ok": true, "result": value}),
                Err(err) => json!({"ok": false, "error": err.to_string()}),
            },
            Err(err) => json!({"ok": false, "error": format!("invalid command: {err}")}),
        };
        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        writer
            .write_all(&encoded)
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "apollo".to_string(),
                message: format!("IPC write error: {e}"),
            })?;
    }
    Ok(())
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream =
        UnixStream::connect(&socket_path)
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "apollo".to_string(),
                message: format!(
                    "failed to connect to APOLLO socket {}: {e}",
                    socket_path.display()
                ),
            })?;
    let mut encoded = serde_json::to_vec(&json!({"cmd": cmd, "payload": payload}))?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("failed to write IPC request: {e}"),
        })?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("failed to read IPC response: {e}"),
        })?;
    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("invalid IPC response: {e}"),
        })?;
    response.into_result("apollo")
}

async fn execute_command(service: &ApolloService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => service.status().await.map_err(service_error),
        "submit" => {
            let request = ExecutionRequest {
                task_id: payload_str(&cmd.payload, "task_id")?.to_string(),
                agent_id: payload_str(&cmd.payload, "agent_id")?.to_string(),
                payload: cmd
                    .payload
                    .get("payload")
                    .cloned()
                    .unwrap_or_else(|| json!({})),
                priority: parse_priority(cmd.payload.get("priority").and_then(|v| v.as_str())),
                timeout_secs: cmd
                    .payload
                    .get("timeout_secs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(60),
            };
            Ok(json!({"task_id": service.submit(request).await.map_err(service_error)?}))
        }
        "execute" => {
            let task_id = payload_str(&cmd.payload, "task_id")?;
            Ok(json!({"result": service.execute(task_id).await.map_err(service_error)?}))
        }
        "interrupt" => {
            let task_id = payload_str(&cmd.payload, "task_id")?;
            Ok(json!({
                "interrupt": service.attach_interrupt(InterruptionAttachmentRequest {
                    task_id,
                    source: payload_str_optional(&cmd.payload, "source").unwrap_or("ipc"),
                    sender: payload_str_optional(&cmd.payload, "sender").unwrap_or("operator"),
                    content: payload_str_optional(&cmd.payload, "content").unwrap_or("interrupt"),
                    disposition: payload_str_optional(&cmd.payload, "disposition").unwrap_or("note"),
                    run_id: payload_str_optional(&cmd.payload, "run_id").map(|s| s.to_string()),
                    session_id: payload_str_optional(&cmd.payload, "session_id").map(|s| s.to_string()),
                }).await.map_err(service_error)?
            }))
        }
        "paths" => Ok(serde_json::to_value(service.runtime_paths())?),
        other => Err(ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}

fn service_error(err: anyhow::Error) -> ArdaError {
    ArdaError::Agent {
        agent: "apollo".to_string(),
        message: err.to_string(),
    }
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArdaError::Agent {
            agent: "apollo".to_string(),
            message: format!("missing required payload key '{key}'"),
        })
}

fn payload_str_optional<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str())
}

fn parse_priority(value: Option<&str>) -> ExecutionPriority {
    match value.unwrap_or("normal").to_ascii_lowercase().as_str() {
        "low" => ExecutionPriority::Low,
        "high" => ExecutionPriority::High,
        "critical" => ExecutionPriority::Critical,
        _ => ExecutionPriority::Normal,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_ipc_server, send_command};
    use crate::ApolloService;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn ipc_round_trip_submit_execute_status() {
        let dir = tempdir().expect("tempdir");
        let service = ApolloService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("apollo.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let submit = send_command(
            socket_path.clone(),
            "submit",
            json!({"task_id":"task_ipc","agent_id":"athena","payload":{"op":"ingest"},"priority":"high","timeout_secs":30}),
        )
        .await;
        if let Err(err) = submit {
            let msg = err.to_string();
            if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                server.abort();
                return;
            }
            panic!("submit: {msg}");
        }
        let _ = send_command(
            socket_path.clone(),
            "execute",
            json!({"task_id":"task_ipc"}),
        )
        .await
        .expect("execute");
        let status = send_command(socket_path.clone(), "status", json!({}))
            .await
            .expect("status");
        assert_eq!(status["authority"], "apollo_service");
        server.abort();
    }

    #[tokio::test]
    async fn ipc_reports_unknown_command_errors() {
        let dir = tempdir().expect("tempdir");
        let service = ApolloService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("apollo.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let err = send_command(socket_path.clone(), "unknown", json!({}))
            .await
            .expect_err("unknown command should fail");
        let message = err.to_string();
        if message.contains("Operation not permitted") || message.contains("Permission denied") {
            server.abort();
            return;
        }
        assert!(message.contains("unknown"));
        assert!(message.contains("IPC command"));

        server.abort();
    }

    #[tokio::test]
    async fn ipc_reports_malformed_json_frames() {
        let dir = tempdir().expect("tempdir");
        let service = ApolloService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("apollo.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let stream = UnixStream::connect(&socket_path).await;
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                server.abort();
                return;
            }
            Err(err) => panic!("connect: {err}"),
        };
        stream
            .write_all(b"{\"cmd\":\"status\",\"payload\":\n")
            .await
            .expect("write malformed frame");

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await.expect("read response");
        let response: serde_json::Value = serde_json::from_str(line.trim()).expect("json");

        assert_eq!(response["ok"], false);
        assert!(response["error"]
            .as_str()
            .unwrap_or_default()
            .contains("invalid command"));

        server.abort();
    }
}
