// sigil: REPAIR
use crate::service::HermesService;
use crate::types::{BoardroomPost, InboundMessage, InterruptionMessage, OutboundMessage};
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: HermesService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "hermes".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;

    loop {
        let (stream, _) = listener.accept().await.map_err(|e| ArdaError::Agent {
            agent: "hermes".to_string(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "hermes_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                let _ = handle_connection(stream, service).await;
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_HERMES_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream =
        UnixStream::connect(&socket_path)
            .await
            .map_err(|e| ArdaError::Agent {
                agent: "hermes".to_string(),
                message: format!(
                    "failed to connect to HERMES socket {}: {e}",
                    socket_path.display()
                ),
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
        .map_err(|e| ArdaError::Agent {
            agent: "hermes".to_string(),
            message: format!("failed to write IPC request: {e}"),
        })?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "hermes".to_string(),
            message: format!("failed to read IPC response: {e}"),
        })?;
    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
            agent: "hermes".to_string(),
            message: format!("invalid IPC response: {e}"),
        })?;
    response.into_result("hermes")
}

async fn handle_connection(stream: UnixStream, service: HermesService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| ArdaError::Agent {
        agent: "hermes".to_string(),
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
                agent: "hermes".to_string(),
                message: format!("IPC write error: {e}"),
            })?;
    }
    Ok(())
}

async fn execute_command(service: &HermesService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(serde_json::to_value(service.status().await?)?),
        "providers" => Ok(service.providers_status().await),
        "subcomponents" => Ok(serde_json::to_value(service.subcomponents())?),
        "paths" => Ok(service.paths()),
        "l3_readiness" => Ok(service.l3_readiness_projection()?),
        "classify" => {
            let msg: InboundMessage =
                serde_json::from_value(cmd.payload).map_err(|e| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: format!("invalid classify payload: {e}"),
                })?;
            Ok(serde_json::to_value(service.classify(msg)?)?)
        }
        "send" => {
            let msg: OutboundMessage =
                serde_json::from_value(cmd.payload).map_err(|e| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: format!("invalid send payload: {e}"),
                })?;
            Ok(service.send(msg).await?)
        }
        "retry_outbound" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            Ok(service.retry_outbound_queue(limit).await?)
        }
        "retry_reroute_dlq" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(100) as usize;
            Ok(service.retry_reroute_dlq(limit)?)
        }
        "boardroom_post" => {
            let post: BoardroomPost =
                serde_json::from_value(cmd.payload).map_err(|e| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: format!("invalid boardroom payload: {e}"),
                })?;
            service.boardroom_post(post)?;
            Ok(json!({"posted": true}))
        }
        "boardroom_recent" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            Ok(serde_json::to_value(service.boardroom_recent(limit)?)?)
        }
        "calendar_sync" => Ok(service.calendar_sync()?),
        "poll_once" => Ok(serde_json::json!({
            "processed": service.poll_providers_once().await?
        })),
        "ingest_external" => {
            let provider = cmd
                .payload
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("webhook");
            let sender = cmd
                .payload
                .get("sender")
                .and_then(|v| v.as_str())
                .unwrap_or("external");
            let content = cmd
                .payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let channel = cmd
                .payload
                .get("channel")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let is_illuvatar = cmd
                .payload
                .get("is_illuvatar")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(serde_json::to_value(service.ingest_external(
                provider,
                sender,
                content,
                channel,
                is_illuvatar,
            )?)?)
        }
        "interrupt" => {
            let msg: InterruptionMessage =
                serde_json::from_value(cmd.payload).map_err(|e| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: format!("invalid interrupt payload: {e}"),
                })?;
            Ok(service.interrupt(msg)?)
        }
        "illuvatar_fanout" => {
            let source_provider = cmd
                .payload
                .get("source_provider")
                .and_then(|v| v.as_str())
                .unwrap_or("discord");
            let sender = cmd
                .payload
                .get("sender")
                .and_then(|v| v.as_str())
                .unwrap_or("illuvatar");
            let content = cmd
                .payload
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let channel = cmd
                .payload
                .get("channel")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let mut msg = InboundMessage::new(source_provider, sender, content);
            msg.channel = channel;
            msg.is_illuvatar = true;
            Ok(service
                .fanout_illuvatar_directive(source_provider, &msg)
                .await?)
        }
        "council_open" => {
            let topic = cmd
                .payload
                .get("topic")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: "missing council_open topic".to_string(),
                })?;
            let participants = cmd
                .payload
                .get("participants")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            Ok(service.council_open(topic, participants)?)
        }
        "council_report" => {
            let session_id = cmd
                .payload
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: "missing council_report session_id".to_string(),
                })?;
            let from_agent = cmd
                .payload
                .get("from_agent")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: "missing council_report from_agent".to_string(),
                })?;
            let body = cmd
                .payload
                .get("body")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: "missing council_report body".to_string(),
                })?;
            Ok(service.council_report(session_id, from_agent, body)?)
        }
        "council_close" => {
            let session_id = cmd
                .payload
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ArdaError::Agent {
                    agent: "hermes".to_string(),
                    message: "missing council_close session_id".to_string(),
                })?;
            let outcome = cmd
                .payload
                .get("outcome")
                .and_then(|v| v.as_str())
                .unwrap_or("closed");
            Ok(service.council_close(session_id, outcome)?)
        }
        other => Err(ArdaError::Agent {
            agent: "hermes".to_string(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}
