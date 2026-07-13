// sigil: REPAIR
use crate::service::CharonService;
use crate::types::CharonRequestEnvelope;
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: CharonService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| AnnunimasError::Agent {
        agent: "charon".to_string(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;
    loop {
        let (stream, _) = listener.accept().await.map_err(|e| AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "charon_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                let _ = handle_connection(stream, service).await;
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_CHARON_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream =
        UnixStream::connect(&socket_path)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "charon".to_string(),
                message: format!(
                    "failed to connect to CHARON socket {}: {e}",
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
        .map_err(|e| AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!("failed to write IPC request: {e}"),
        })?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!("failed to read IPC response: {e}"),
        })?;
    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!("invalid IPC response: {e}"),
        })?;
    response.into_result("charon")
}

async fn handle_connection(stream: UnixStream, service: CharonService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| AnnunimasError::Agent {
        agent: "charon".to_string(),
        message: format!("IPC read error: {e}"),
    })? {
        let response = match serde_json::from_str::<CommandEnvelope>(&line) {
            Ok(cmd) => match execute_command(&service, cmd).await {
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
            .map_err(|e| AnnunimasError::Agent {
                agent: "charon".to_string(),
                message: format!("IPC write error: {e}"),
            })?;
    }
    Ok(())
}

async fn execute_command(service: &CharonService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(serde_json::to_value(service.status().await?)?),
        "state" => Ok(service.state().await?),
        "providers" => Ok(serde_json::to_value(service.providers().await)?),
        "operator_summary" => service.operator_route_summary().await,
        "observability" => service.route_observability_rollup().await,
        "eval" => {
            let dry_run = cmd
                .payload
                .get("dry_run")
                .and_then(Value::as_bool)
                .unwrap_or(true);
            service.charon_eval(dry_run).await
        }
        "route_history" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|value| value.as_u64())
                .map(|value| value as usize)
                .unwrap_or(100);
            Ok(json!({
                "ok": true,
                "routes": service.route_history(limit).await
            }))
        }
        "route" | "request" => {
            let req: CharonRequestEnvelope =
                serde_json::from_value(cmd.payload).map_err(|e| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: format!("invalid route payload: {e}"),
                })?;
            Ok(serde_json::to_value(service.route(req).await?)?)
        }
        "proxy" => {
            let req: CharonRequestEnvelope =
                serde_json::from_value(cmd.payload).map_err(|e| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: format!("invalid proxy payload: {e}"),
                })?;
            Ok(service.proxy_openai(req).await?)
        }
        "cooldown" => {
            let provider_id = cmd
                .payload
                .get("provider_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: "missing provider_id".to_string(),
                })?;
            let seconds = cmd
                .payload
                .get("seconds")
                .and_then(|v| v.as_i64())
                .unwrap_or(60);
            service.mark_provider_cooldown(provider_id, seconds).await?;
            Ok(json!({"ok": true}))
        }
        "provider_result" => {
            let provider_id = cmd
                .payload
                .get("provider_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: "missing provider_id".to_string(),
                })?;
            let ok = cmd
                .payload
                .get("ok")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let latency_ms = cmd.payload.get("latency_ms").and_then(|v| v.as_u64());
            let error = cmd
                .payload
                .get("error")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            service
                .mark_provider_result(provider_id, ok, latency_ms, error)
                .await?;
            Ok(json!({"ok": true}))
        }
        "model_streaming_validation" => {
            let provider_id = cmd
                .payload
                .get("provider_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: "missing provider_id".to_string(),
                })?;
            let model_id = cmd
                .payload
                .get("model_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| AnnunimasError::Agent {
                    agent: "charon".to_string(),
                    message: "missing model_id".to_string(),
                })?;
            let streaming_validated = cmd
                .payload
                .get("streaming_validated")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let error = cmd
                .payload
                .get("error")
                .and_then(|v| v.as_str())
                .map(|v| v.to_string());
            service
                .mark_model_streaming_validation(provider_id, model_id, streaming_validated, error)
                .await?;
            Ok(json!({"ok": true}))
        }
        "paths" => Ok(service.paths()),
        "reload_config" | "reload" => Ok(service.reload_provider_config().await?),
        other => Err(AnnunimasError::Agent {
            agent: "charon".to_string(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}
