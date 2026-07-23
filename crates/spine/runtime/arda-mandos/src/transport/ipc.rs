// sigil: REPAIR
use crate::{EvidenceRef, OracleQuery, OracleService, QueryType};
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;
use chrono::Utc;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: OracleService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "oracle".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;
    tracing::info!(socket = %socket_path.display(), "ORACLE IPC server listening");
    loop {
        let (stream, _) = listener.accept().await.map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "oracle_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                if let Err(err) = handle_connection(stream, service).await {
                    tracing::warn!(error = %err, "ORACLE IPC client connection failed");
                }
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ARDA_MANDOS_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn handle_connection(stream: UnixStream, service: OracleService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| ArdaError::Agent {
        agent: "oracle".to_string(),
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
                agent: "oracle".to_string(),
                message: format!("IPC write error: {e}"),
            })?;
    }
    Ok(())
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!(
                "failed to connect to ORACLE socket {}: {e}",
                socket_path.display()
            ),
        })?;
    let mut encoded = serde_json::to_vec(&json!({"cmd": cmd, "payload": payload}))?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("failed to write IPC request: {e}"),
        })?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("failed to read IPC response: {e}"),
        })?;
    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("invalid IPC response: {e}"),
        })?;
    response.into_result("oracle")
}

async fn execute_command(service: &OracleService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => service.status().await.map_err(service_error),
        "evaluate" => {
            let mut query = OracleQuery::new(
                payload_str_optional(&cmd.payload, "id")
                    .map(ToString::to_string)
                    .unwrap_or_else(|| format!("oracle_ipc::{}", uuid::Uuid::new_v4())),
                payload_str(&cmd.payload, "task")?,
                payload_str_optional(&cmd.payload, "requester").unwrap_or("operator"),
            );
            query.context = cmd
                .payload
                .get("context")
                .and_then(|v| v.as_array())
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            query.evidence = cmd
                .payload
                .get("evidence")
                .cloned()
                .map(serde_json::from_value::<Vec<EvidenceRef>>)
                .transpose()?
                .unwrap_or_default()
                .into_iter()
                .map(|evidence| evidence.with_sensitive_excerpt(true))
                .collect();
            query.query_type = cmd
                .payload
                .get("query_type")
                .cloned()
                .map(serde_json::from_value::<QueryType>)
                .transpose()?
                .unwrap_or_default();
            query.timestamp = cmd
                .payload
                .get("timestamp")
                .cloned()
                .map(serde_json::from_value)
                .transpose()?
                .unwrap_or_else(Utc::now);
            query.correlation_id =
                payload_str_optional(&cmd.payload, "correlation_id").map(ToString::to_string);
            query.causation_id =
                payload_str_optional(&cmd.payload, "causation_id").map(ToString::to_string);
            Ok(serde_json::to_value(
                service
                    .evaluate(query)
                    .await
                    .map_err(service_error)?
                    .redacted_for_export(),
            )?)
        }
        "verdicts" => Ok(serde_json::to_value(
            service
                .recent_verdicts(
                    cmd.payload
                        .get("limit")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(10) as usize,
                )
                .map_err(service_error)?,
        )?),
        "paths" => Ok(serde_json::to_value(service.runtime_paths())?),
        other => Err(ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}

fn service_error(err: anyhow::Error) -> ArdaError {
    ArdaError::Agent {
        agent: "oracle".to_string(),
        message: err.to_string(),
    }
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("missing required payload key '{key}'"),
        })
}
fn payload_str_optional<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str())
}

#[cfg(test)]
mod tests {
    use super::{run_ipc_server, send_command};
    use crate::OracleService;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn ipc_round_trip_evaluate_status() {
        let _env_guard = crate::PLUTUS_ENV_LOCK.lock().await;
        let dir = tempdir().expect("tempdir");
        let plutus_home = dir.path().join("plutus");
        std::env::set_var("ARDA_PLUTUS_HOME", &plutus_home);
        let service = OracleService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("oracle.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let transport_evidence = crate::EvidenceRef::supplied(
            "transport-report",
            "ipc://fixture/report",
            chrono::Utc::now(),
            "transport-sensitive excerpt",
        );
        let verdict = send_command(
            socket_path.clone(),
            "evaluate",
            json!({
                "task":"Should we proceed with evidence?",
                "evidence":[transport_evidence],
                "requester":"prometheus"
            }),
        )
        .await;
        let verdict = match verdict {
            Ok(verdict) => verdict,
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                    server.abort();
                    return;
                }
                panic!("evaluate: {msg}");
            }
        };
        assert_eq!(
            verdict["gates"]["bacon"]["evidence"][0]["evidence"]["source_id"],
            "transport-report"
        );
        let status = send_command(socket_path.clone(), "status", json!({}))
            .await
            .expect("status");
        assert_eq!(status["authority"], "oracle_service");
        server.abort();
        std::env::remove_var("ARDA_PLUTUS_HOME");
    }
}
