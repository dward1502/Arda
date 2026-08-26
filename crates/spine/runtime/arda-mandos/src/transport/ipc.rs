// sigil: REPAIR
use super::dispatch::{
    dispatch, DispatchError, DispatchRequest, EvaluateRequest, ExportLedgerRequest,
};
use crate::OracleService;
use arda_core::daemon::CommandEnvelope;
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;

use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

const IPC_MAX_LINE_BYTES: usize = 1024 * 1024;

struct SocketPathGuard(PathBuf);

impl Drop for SocketPathGuard {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_file(&self.0) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(socket = %self.0.display(), %error, "failed to remove ORACLE IPC socket");
            }
        }
    }
}

pub async fn run_ipc_server(service: OracleService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        match UnixStream::connect(&socket_path).await {
            Ok(_) => {
                return Err(ArdaError::Agent {
                    agent: "oracle".to_string(),
                    message: format!(
                        "refusing to replace active unix socket {}",
                        socket_path.display()
                    ),
                })
            }
            Err(_) => std::fs::remove_file(&socket_path)?,
        }
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "oracle".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;
    let _socket_guard = SocketPathGuard(socket_path.clone());
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
    let mut reader = BufReader::new(reader);
    loop {
        let line = match read_bounded_line(&mut reader).await {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                write_response(&mut writer, &error.body()).await?;
                break;
            }
        };
        let response = match serde_json::from_str::<CommandEnvelope>(&line) {
            Ok(cmd) => match execute_command(&service, cmd).await {
                Ok(value) => json!({"ok": true, "result": value}),
                Err(error) => error.body(),
            },
            Err(err) => DispatchError::invalid_request(format!("invalid command: {err}")).body(),
        };
        write_response(&mut writer, &response).await?;
    }
    Ok(())
}

async fn read_bounded_line<R>(reader: &mut R) -> std::result::Result<Option<String>, DispatchError>
where
    R: tokio::io::AsyncBufRead + Unpin,
{
    let mut bytes = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .await
            .map_err(|err| DispatchError::invalid_request(format!("IPC read error: {err}")))?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            break;
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(available.len());
        if bytes.len().saturating_add(take) > IPC_MAX_LINE_BYTES {
            return Err(DispatchError::payload_too_large(IPC_MAX_LINE_BYTES));
        }
        bytes.extend_from_slice(&available[..take]);
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            break;
        }
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|err| DispatchError::invalid_request(format!("IPC request is not UTF-8: {err}")))
}

async fn write_response<W>(writer: &mut W, response: &Value) -> Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut encoded = serde_json::to_vec(response)?;
    encoded.push(b'\n');
    writer
        .write_all(&encoded)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("IPC write error: {e}"),
        })
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
    let response: Value = serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
        agent: "oracle".to_string(),
        message: format!("invalid IPC response: {e}"),
    })?;
    if response.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    } else {
        let code = response
            .pointer("/error/code")
            .and_then(Value::as_str)
            .unwrap_or("IPC_ERROR");
        let message = response
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("unknown IPC error");
        Err(ArdaError::Agent {
            agent: "oracle".to_string(),
            message: format!("{code}: {message}"),
        })
    }
}

async fn execute_command(
    service: &OracleService,
    cmd: CommandEnvelope,
) -> std::result::Result<Value, DispatchError> {
    match cmd.cmd.as_str() {
        "status" => dispatch(service, DispatchRequest::Status).await,
        "evaluate" => {
            let request = EvaluateRequest::from_payload(cmd.payload)?;
            dispatch(
                service,
                DispatchRequest::Evaluate {
                    request,
                    id_prefix: "oracle_ipc",
                },
            )
            .await
        }
        "verdicts" => {
            dispatch(
                service,
                DispatchRequest::Verdicts {
                    limit: cmd
                        .payload
                        .get("limit")
                        .and_then(Value::as_u64)
                        .unwrap_or(10) as usize,
                },
            )
            .await
        }
        "paths" => dispatch(service, DispatchRequest::Paths).await,
        "verify_ledger" => dispatch(service, DispatchRequest::VerifyLedger).await,
        "export_ledger" => {
            let request = ExportLedgerRequest::from_payload(cmd.payload)?;
            dispatch(
                service,
                DispatchRequest::ExportLedger {
                    destination: request.destination,
                },
            )
            .await
        }
        other => Err(DispatchError::unknown_command(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        execute_command, read_bounded_line, run_ipc_server, send_command, IPC_MAX_LINE_BYTES,
    };
    use crate::OracleService;
    use arda_core::daemon::CommandEnvelope;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn ipc_rejects_oversized_lines_before_json_parsing() {
        let payload = vec![b'x'; IPC_MAX_LINE_BYTES + 1];
        let mut reader = tokio::io::BufReader::new(payload.as_slice());
        let error = read_bounded_line(&mut reader)
            .await
            .expect_err("oversized line must fail");
        assert_eq!(error.code, "PAYLOAD_TOO_LARGE");
    }

    #[tokio::test]
    async fn ipc_dispatch_reports_structured_invalid_payload_errors() {
        let dir = tempdir().expect("tempdir");
        let service = OracleService::from_home(dir.path()).await.expect("service");
        let command: CommandEnvelope = serde_json::from_value(json!({
            "cmd": "evaluate",
            "payload": {"task": "review", "context": "not-an-array"}
        }))
        .expect("command");
        let error = execute_command(&service, command)
            .await
            .expect_err("invalid context must fail");
        assert_eq!(error.code, "INVALID_REQUEST");
    }

    #[tokio::test]
    async fn ipc_round_trip_evaluate_status() {
        let dir = tempdir().expect("tempdir");
        let service = OracleService::from_home(dir.path()).await.expect("service");
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
        .await
        .expect("evaluate");
        assert_eq!(
            verdict["gates"]["bacon"]["evidence"][0]["evidence"]["source_id"],
            "transport-report"
        );
        let status = send_command(socket_path.clone(), "status", json!({}))
            .await
            .expect("status");
        assert_eq!(status["authority"], "oracle_service");
        server.abort();
    }

    #[tokio::test]
    async fn ipc_dispatch_verifies_and_exports_the_authoritative_ledger() {
        let dir = tempdir().expect("tempdir");
        let service = OracleService::from_home(dir.path()).await.expect("service");
        execute_command(
            &service,
            serde_json::from_value(json!({
                "cmd": "evaluate",
                "payload": {"id": "ipc-export", "task": "review export evidence"}
            }))
            .expect("evaluate command"),
        )
        .await
        .expect("evaluate");

        let report = execute_command(
            &service,
            serde_json::from_value(json!({"cmd": "verify_ledger", "payload": {}}))
                .expect("verify command"),
        )
        .await
        .expect("verify ledger");
        assert_eq!(report["valid"], true);
        assert_eq!(report["valid_records"], 1);

        let destination = dir.path().join("exports/ipc-export.jsonl");
        let exported = execute_command(
            &service,
            serde_json::from_value(json!({
                "cmd": "export_ledger",
                "payload": {"destination": "ipc-export.jsonl"}
            }))
            .expect("export command"),
        )
        .await
        .expect("export ledger");
        assert_eq!(exported["valid"], true);
        assert!(destination.exists());
    }
}
