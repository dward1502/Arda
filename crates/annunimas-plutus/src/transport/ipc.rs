// sigil: REPAIR
use crate::{CostModelConfig, JouleWorkUnit, PlutusService};
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: PlutusService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }
    let listener = UnixListener::bind(&socket_path).map_err(|e| AnnunimasError::Agent {
        agent: "plutus".to_owned(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;
    tracing::info!(socket = %socket_path.display(), "PLUTUS IPC server listening");
    loop {
        let (stream, _) = listener.accept().await.map_err(|e| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "plutus_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                if let Err(err) = handle_connection(stream, service).await {
                    tracing::warn!(error = %err, "PLUTUS IPC client connection failed");
                }
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_PLUTUS_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

async fn handle_connection(stream: UnixStream, service: PlutusService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await.map_err(|e| AnnunimasError::Agent {
        agent: "plutus".to_owned(),
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
            .map_err(|e| AnnunimasError::Agent {
                agent: "plutus".to_owned(),
                message: format!("IPC write error: {e}"),
            })?;
    }
    Ok(())
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream =
        UnixStream::connect(&socket_path)
            .await
            .map_err(|e| AnnunimasError::Agent {
                agent: "plutus".to_owned(),
                message: format!(
                    "failed to connect to PLUTUS socket {}: {e}",
                    socket_path.display()
                ),
            })?;
    let mut encoded = serde_json::to_vec(&json!({"cmd": cmd, "payload": payload}))?;
    encoded.push(b'\n');
    stream
        .write_all(&encoded)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("failed to write IPC request: {e}"),
        })?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("failed to read IPC response: {e}"),
        })?;
    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("invalid IPC response: {e}"),
        })?;
    response.into_result("plutus")
}

async fn execute_command(service: &PlutusService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => service.status().await.map_err(service_error),
        "register_model" => {
            service
                .register_model(CostModelConfig {
                    provider: payload_str(&cmd.payload, "provider")?.to_owned(),
                    input_rate: payload_f64(&cmd.payload, "input_rate")?,
                    output_rate: payload_f64(&cmd.payload, "output_rate")?,
                    batch_size: cmd
                        .payload
                        .get("batch_size")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1000) as usize,
                })
                .await
                .map_err(service_error)?;
            Ok(json!({"registered": true}))
        }
        "record_spend" => Ok(json!({
            "cost": service.record_spend(
                payload_str(&cmd.payload, "provider")?,
                cmd.payload.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                cmd.payload.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
            ).await.map_err(service_error)?
        })),
        "track_work" => {
            service
                .track_work(
                    payload_str(&cmd.payload, "agent_id")?,
                    payload_f64(&cmd.payload, "amount")?,
                    parse_unit(cmd.payload.get("unit").and_then(|v| v.as_str())),
                    payload_str_optional(&cmd.payload, "task_id").map(str::to_owned),
                )
                .await
                .map_err(service_error)?;
            Ok(json!({"tracked": true}))
        }
        "credit" => {
            service
                .credit(
                    payload_str(&cmd.payload, "account")?,
                    payload_f64(&cmd.payload, "amount")?,
                )
                .await
                .map_err(service_error)?;
            Ok(json!({"credited": true}))
        }
        "relationship" => Ok(json!({
            "score": service.record_relationship(
                payload_str(&cmd.payload, "from")?,
                payload_str(&cmd.payload, "to")?,
                payload_f64(&cmd.payload, "trust")?,
                payload_f64(&cmd.payload, "attention")?,
                payload_f64(&cmd.payload, "reciprocity")?,
            ).await.map_err(service_error)?
        })),
        "paths" => Ok(serde_json::to_value(service.runtime_paths())?),
        other => Err(AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}

fn service_error(err: anyhow::Error) -> AnnunimasError {
    AnnunimasError::Agent {
        agent: "plutus".to_owned(),
        message: err.to_string(),
    }
}

fn payload_str<'a>(payload: &'a Value, key: &str) -> Result<&'a str> {
    payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("missing required payload key '{key}'"),
        })
}
fn payload_str_optional<'a>(payload: &'a Value, key: &str) -> Option<&'a str> {
    payload.get(key).and_then(|v| v.as_str())
}
fn payload_f64(payload: &Value, key: &str) -> Result<f64> {
    payload
        .get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| AnnunimasError::Agent {
            agent: "plutus".to_owned(),
            message: format!("missing required numeric payload key '{key}'"),
        })
}
fn parse_unit(value: Option<&str>) -> JouleWorkUnit {
    match value.unwrap_or("reasoning").to_ascii_lowercase().as_str() {
        "compute" => JouleWorkUnit::Compute,
        "network" => JouleWorkUnit::Network,
        "storage" => JouleWorkUnit::Storage,
        "attention" => JouleWorkUnit::Attention,
        _ => JouleWorkUnit::Reasoning,
    }
}

#[cfg(test)]
mod tests {
    use super::{run_ipc_server, send_command};
    use crate::PlutusService;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn ipc_round_trip_register_and_status() {
        let dir = tempdir().expect("tempdir");
        let service = PlutusService::from_home(dir.path()).expect("service");
        let socket_path = dir.path().join("plutus.sock");
        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let registered = send_command(
            socket_path.clone(),
            "register_model",
            json!({
                "provider":"openai","input_rate":0.001,"output_rate":0.002,"batch_size":1000
            }),
        )
        .await;
        if let Err(err) = registered {
            let msg = err.to_string();
            if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                server.abort();
                return;
            }
            panic!("register_model: {msg}");
        }
        let status = send_command(socket_path.clone(), "status", json!({}))
            .await
            .expect("status");
        assert_eq!(status["authority"], "plutus_service");
        server.abort();
    }
}
