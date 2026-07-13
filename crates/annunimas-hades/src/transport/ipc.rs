// sigil: REPAIR
use crate::service::HadesService;
use crate::types::{QuorumProof, SigilVacuumRule};
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

fn hades_agent_error(message: impl Into<String>) -> AnnunimasError {
    AnnunimasError::Agent {
        agent: "hades".to_owned(),
        message: message.into(),
    }
}

pub async fn run_ipc_server(service: HadesService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| {
        hades_agent_error(format!(
            "failed to bind unix socket {}: {e}",
            socket_path.display()
        ))
    })?;

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|e| hades_agent_error(format!("IPC accept error: {e}")))?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "hades_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                let _ = handle_connection(stream, service).await;
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_HADES_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream = UnixStream::connect(&socket_path).await.map_err(|e| {
        hades_agent_error(format!(
            "failed to connect to HADES socket {}: {e}",
            socket_path.display()
        ))
    })?;
    let request = json!({
        "cmd": cmd,
        "payload": payload,
    });
    let mut encoded = serde_json::to_vec(&request)?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .map_err(|e| hades_agent_error(format!("failed to write IPC request: {e}")))?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| hades_agent_error(format!("failed to read IPC response: {e}")))?;
    let response: ResponseEnvelope = serde_json::from_str(line.trim())
        .map_err(|e| hades_agent_error(format!("invalid IPC response: {e}")))?;
    response.into_result("hades")
}

async fn handle_connection(stream: UnixStream, service: HadesService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    serve_lines(BufReader::new(reader), &mut writer, service).await
}

async fn serve_lines<R, W>(reader: R, writer: &mut W, service: HadesService) -> Result<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut lines = reader.lines();
    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| hades_agent_error(format!("IPC read error: {e}")))?
    {
        let response = match serde_json::from_str::<CommandEnvelope>(&line) {
            Ok(cmd) => match execute_command(&service, cmd) {
                Ok(value) => serde_json::to_value(ResponseEnvelope::success(value)).unwrap_or_else(
                    |err| json!({"ok": false, "error": format!("serialization error: {err}")}),
                ),
                Err(err) => serde_json::to_value(ResponseEnvelope::failure(err.to_string()))
                    .unwrap_or_else(
                        |serr| json!({"ok": false, "error": format!("serialization error: {serr}")}),
                    ),
            },
            Err(err) => serde_json::to_value(ResponseEnvelope::failure(format!(
                "invalid command: {err}"
            )))
            .unwrap_or_else(
                |serr| json!({"ok": false, "error": format!("serialization error: {serr}")}),
            ),
        };
        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        writer
            .write_all(&encoded)
            .await
            .map_err(|e| hades_agent_error(format!("IPC write error: {e}")))?;
    }
    Ok(())
}

fn execute_command(service: &HadesService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(serde_json::to_value(service.status()?)?),
        "queue" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            Ok(serde_json::to_value(service.queue(limit)?)?)
        }
        "log" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            let event = cmd
                .payload
                .get("event_filter")
                .and_then(|v| v.as_str())
                .or_else(|| cmd.payload.get("event").and_then(|v| v.as_str()));
            let rule = sigil_rule_from_payload(&cmd.payload)?;
            Ok(serde_json::to_value(service.log(
                limit,
                event,
                rule.as_ref(),
            )?)?)
        }
        "sigil_match" => {
            let path = cmd
                .payload
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| hades_agent_error("missing sigil_match path"))?;
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            let rule = sigil_rule_from_payload(&cmd.payload)?.unwrap_or_default();
            Ok(serde_json::to_value(
                service.sigil_match(path, &rule, limit)?,
            )?)
        }
        "sweep" => {
            let sweep_type = cmd
                .payload
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("manual");
            let path = cmd.payload.get("path").and_then(|v| v.as_str());
            Ok(serde_json::to_value(service.sweep(sweep_type, path)?)?)
        }
        "remove" => {
            let file = cmd
                .payload
                .get("file")
                .and_then(|v| v.as_str())
                .ok_or_else(|| hades_agent_error("missing remove file path"))?;
            let authorized_by = cmd
                .payload
                .get("authorized_by")
                .and_then(|v| v.as_str())
                .unwrap_or("orchestrator");
            let quorum_proof = cmd
                .payload
                .get("quorum_proof")
                .cloned()
                .map(serde_json::from_value::<QuorumProof>)
                .transpose()
                .map_err(|e| {
                    hades_agent_error(format!("invalid remove quorum_proof payload: {e}"))
                })?;
            Ok(serde_json::to_value(service.queue_remove_with_proof(
                file,
                authorized_by,
                quorum_proof,
            )?)?)
        }
        "paths" => Ok(service.paths()),
        other => Err(hades_agent_error(format!("unknown IPC command: {other}"))),
    }
}

fn sigil_rule_from_payload(payload: &Value) -> Result<Option<SigilVacuumRule>> {
    let code_regex = payload
        .get("sigil_code_regex")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let retention = payload
        .get("sigil_retention")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let tag = payload
        .get("sigil_tag")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    let source = payload
        .get("sigil_source")
        .and_then(|v| v.as_str())
        .map(str::to_owned);
    if code_regex.is_none() && retention.is_none() && tag.is_none() && source.is_none() {
        return Ok(None);
    }
    Ok(Some(SigilVacuumRule {
        code_regex,
        retention,
        tag,
        source,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TaskItem;
    use annunimas_core::daemon::ResponseEnvelope;
    use tokio::io::{duplex, AsyncBufReadExt, AsyncWriteExt, BufReader};

    #[test]
    fn remove_command_requires_file_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        let cmd = CommandEnvelope::new("remove", json!({}));

        let err = execute_command(&service, cmd).expect_err("missing file should fail");
        assert!(err.to_string().contains("missing remove file path"));
    }

    #[test]
    fn remove_command_rejects_malformed_quorum_proof() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        let cmd = CommandEnvelope::new(
            "remove",
            json!({
                "file": "/tmp/demo.jsonl",
                "quorum_proof": {
                    "approvers": "aurelius"
                }
            }),
        );

        let err = execute_command(&service, cmd).expect_err("invalid quorum should fail");
        assert!(err
            .to_string()
            .contains("invalid remove quorum_proof payload"));
    }

    #[test]
    fn remove_command_queues_task_when_quorum_is_valid() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        let target = dir.path().join("artifact.jsonl");
        std::fs::write(&target, "{\"ok\":true}\n").expect("write target");
        let cmd = CommandEnvelope::new(
            "remove",
            json!({
                "file": target,
                "authorized_by": "orchestrator",
                "quorum_proof": {
                    "approvers": ["aurelius", "bacon"],
                    "evidence": ["ticket-123"],
                    "asserted_at_utc": "2026-04-21T00:00:00Z"
                }
            }),
        );

        let value = execute_command(&service, cmd).expect("remove queued");
        let task: TaskItem = serde_json::from_value(value).expect("task item");

        assert_eq!(task.file, target.to_string_lossy());
        assert!(task.quorum_proof.is_some());
        assert_eq!(task.authorized_by.as_deref(), Some("orchestrator"));

        let queued = service.queue(10).expect("queue");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].file, task.file);
    }

    #[tokio::test]
    async fn ipc_roundtrip_status_and_remove() {
        let dir = tempfile::tempdir().expect("tempdir");
        let service = HadesService::new(dir.path()).expect("service");
        let target = dir.path().join("artifact.jsonl");
        std::fs::write(&target, "{\"ok\":true}\n").expect("write target");

        let (client, server_stream) = duplex(4096);
        let server = tokio::spawn(async move {
            let (server_reader, mut server_writer) = tokio::io::split(server_stream);
            serve_lines(BufReader::new(server_reader), &mut server_writer, service).await
        });
        let (reader, mut writer) = tokio::io::split(client);
        let mut reader = BufReader::new(reader);

        let mut encoded =
            serde_json::to_vec(&CommandEnvelope::new("status", json!({}))).expect("status encode");
        encoded.push(b'\n');
        writer.write_all(&encoded).await.expect("status write");

        let mut line = String::new();
        reader.read_line(&mut line).await.expect("status read");
        let status: ResponseEnvelope = serde_json::from_str(line.trim()).expect("status envelope");
        let status = status.into_result("hades").expect("status result");
        assert_eq!(status["warden_connected"], true);

        let mut encoded = serde_json::to_vec(&CommandEnvelope::new(
            "remove",
            json!({
                "file": target,
                "authorized_by": "orchestrator",
                "quorum_proof": {
                    "approvers": ["aurelius", "bacon"],
                    "evidence": ["ticket:ipc-1"],
                    "asserted_at_utc": "2026-04-21T00:00:00Z"
                }
            }),
        ))
        .expect("remove encode");
        encoded.push(b'\n');
        writer.write_all(&encoded).await.expect("remove write");

        line.clear();
        reader.read_line(&mut line).await.expect("remove read");
        let removed: ResponseEnvelope = serde_json::from_str(line.trim()).expect("remove envelope");
        let removed = removed.into_result("hades").expect("remove result");
        let task: TaskItem = serde_json::from_value(removed).expect("task");
        assert_eq!(task.file, target.to_string_lossy());

        let mut encoded = serde_json::to_vec(&CommandEnvelope::new("queue", json!({"limit": 5})))
            .expect("queue encode");
        encoded.push(b'\n');
        writer.write_all(&encoded).await.expect("queue write");

        line.clear();
        reader.read_line(&mut line).await.expect("queue read");
        let queue: ResponseEnvelope = serde_json::from_str(line.trim()).expect("queue envelope");
        let queue = queue.into_result("hades").expect("queue result");
        let queued: Vec<TaskItem> = serde_json::from_value(queue).expect("queue items");
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].file, task.file);

        drop(writer);
        server.abort();
        let _ = server.await;
    }
}
