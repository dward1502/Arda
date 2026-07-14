// sigil: REPAIR
use crate::service::{InformantEvent, MnemosyneService};
use arda_core::daemon::{CommandEnvelope, ResponseEnvelope};
use arda_core::error::{ArdaError, Result};
use arda_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

pub async fn run_ipc_server(service: MnemosyneService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path).map_err(|e| ArdaError::Agent {
        agent: "mnemosyne".to_owned(),
        message: format!("failed to bind unix socket {}: {e}", socket_path.display()),
    })?;

    tracing::info!(socket = %socket_path.display(), "MNEMOSYNE IPC server listening");

    loop {
        let (stream, _) = listener.accept().await.map_err(|e| ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("IPC accept error: {e}"),
        })?;
        let service = service.clone();
        let _ = spawn_bounded_background(
            "mnemosyne_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                if let Err(err) = handle_connection(stream, service).await {
                    tracing::warn!(error = %err, "MNEMOSYNE IPC client failed");
                }
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_MNEMOSYNE_IPC_MAX_CONCURRENCY")
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
                agent: "mnemosyne".to_owned(),
                message: format!(
                    "failed to connect to MNEMOSYNE socket {}: {e}",
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
            agent: "mnemosyne".to_owned(),
            message: format!("failed to write IPC request: {e}"),
        })?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("failed to read IPC response: {e}"),
        })?;

    let response: ResponseEnvelope =
        serde_json::from_str(line.trim()).map_err(|e| ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("invalid IPC response: {e}"),
        })?;

    response.into_result("mnemosyne")
}

async fn handle_connection(stream: UnixStream, service: MnemosyneService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines.next_line().await.map_err(|e| ArdaError::Agent {
        agent: "mnemosyne".to_owned(),
        message: format!("IPC read error: {e}"),
    })? {
        let response = match serde_json::from_str::<CommandEnvelope>(&line) {
            Ok(cmd) => match execute_command(&service, cmd) {
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
                agent: "mnemosyne".to_owned(),
                message: format!("IPC write error: {e}"),
            })?;
    }

    Ok(())
}

fn execute_command(service: &MnemosyneService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(service.status()?),
        "stats" => Ok(serde_json::to_value(service.stats()?)?),
        "identity_state" => Ok(serde_json::to_value(service.identity_state()?)?),
        "paths" => Ok(service.paths()),
        "encode" => {
            let event: InformantEvent =
                serde_json::from_value(cmd.payload).map_err(|e| ArdaError::Agent {
                    agent: "mnemosyne".to_owned(),
                    message: format!("invalid encode payload: {e}"),
                })?;
            Ok(serde_json::to_value(service.encode(event)?)?)
        }
        "recall_recent" => {
            let hours = cmd
                .payload
                .get("hours")
                .and_then(|v| v.as_i64())
                .unwrap_or(24);
            let crate_name = cmd
                .payload
                .get("crate")
                .and_then(|v| v.as_str())
                .or_else(|| cmd.payload.get("crate_name").and_then(|v| v.as_str()));
            let scope = cmd.payload.get("scope").and_then(|v| v.as_str());
            let query = cmd.payload.get("query").and_then(|v| v.as_str());
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(12) as usize;
            let memories = if let Some(query) = query {
                service.recall_relevant(query, hours, crate_name, scope, limit)?
            } else {
                service.recall_recent_scoped(hours, crate_name, scope)?
            };
            Ok(serde_json::to_value(memories)?)
        }
        "recall_knowledge_seeds" => {
            let query = cmd.payload.get("query").and_then(|v| v.as_str());
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(12) as usize;
            Ok(serde_json::to_value(
                service.recall_knowledge_seeds(query, limit)?,
            )?)
        }
        "consolidate" => {
            let hours = cmd
                .payload
                .get("hours")
                .and_then(|v| v.as_i64())
                .unwrap_or(24);
            Ok(serde_json::to_value(service.consolidate(hours)?)?)
        }
        "obsidian_sync" => {
            let vault_path = cmd
                .payload
                .get("vault_path")
                .and_then(|v| v.as_str())
                .unwrap_or("human/.obsidian");
            let max_files = cmd
                .payload
                .get("max_files")
                .and_then(|v| v.as_u64())
                .unwrap_or(200) as usize;
            Ok(serde_json::to_value(
                service.sync_obsidian(vault_path, max_files)?,
            )?)
        }
        other => Err(ArdaError::Agent {
            agent: "mnemosyne".to_owned(),
            message: format!("unknown IPC command: {other}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::{run_ipc_server, send_command};
    use crate::service::{InformantEvent, MnemosyneService};
    use chrono::Utc;
    use serde_json::json;
    use tempfile::tempdir;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn ipc_round_trip_encode_recall_stats() {
        let dir = tempdir().expect("tempdir");
        let service = MnemosyneService::new(dir.path()).expect("service");
        let socket_path = dir.path().join("mnemosyne.sock");

        let server = tokio::spawn(run_ipc_server(service, socket_path.clone()));
        sleep(Duration::from_millis(50)).await;

        let encode = send_command(
            socket_path.clone(),
            "encode",
            serde_json::to_value(InformantEvent {
                informant_id: "test".to_string(),
                crate_name: "prometheus".to_string(),
                event_type: "task_completed".to_string(),
                ts_utc: Utc::now().to_rfc3339(),
                content: "ARDA mission completion".to_string(),
                confidence_hint: Some(0.9),
                tags: vec!["arda".to_string()],
            })
            .expect("value"),
        )
        .await;
        if let Err(err) = encode {
            let msg = err.to_string();
            if msg.contains("Operation not permitted") || msg.contains("Permission denied") {
                server.abort();
                return;
            }
            panic!("encode: {msg}");
        }

        let recall = send_command(
            socket_path.clone(),
            "recall_recent",
            json!({"hours": 24, "crate": "prometheus"}),
        )
        .await
        .expect("recall");
        assert!(recall.as_array().map(|v| !v.is_empty()).unwrap_or(false));
        assert_eq!(
            recall[0]["memory_scope"].as_str(),
            Some("system_continuity")
        );

        let stats = send_command(socket_path, "stats", json!({}))
            .await
            .expect("stats");
        assert!(stats.get("memory_counts").is_some());

        server.abort();
    }
}
