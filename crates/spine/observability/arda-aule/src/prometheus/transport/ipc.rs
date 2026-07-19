#![cfg(feature = "full-cli")]
// sigil: REPAIR
use crate::service::PrometheusService;
use annunimas_core::daemon::{CommandEnvelope, ResponseEnvelope};
use annunimas_core::error::{AnnunimasError, Result};
use annunimas_core::spawn_bounded_background;
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

#[inline]
fn agent_err(message: impl Into<String>) -> AnnunimasError {
    AnnunimasError::Agent {
        agent: "prometheus".to_owned(),
        message: message.into(),
    }
}

pub async fn run_ipc_server(service: PrometheusService, socket_path: PathBuf) -> Result<()> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path).map_err(|e| {
        agent_err(format!(
            "failed to bind unix socket {}: {e}",
            socket_path.display()
        ))
    })?;

    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(|e| agent_err(format!("IPC accept error: {e}")))?;

        let service = service.clone();
        let _ = spawn_bounded_background(
            "prometheus_ipc_connection",
            ipc_connection_limit(),
            move || async move {
                let _ = handle_connection(stream, service).await;
            },
        );
    }
}

fn ipc_connection_limit() -> usize {
    std::env::var("ANNUNIMAS_PROMETHEUS_IPC_MAX_CONCURRENCY")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(16)
}

pub async fn send_command(socket_path: PathBuf, cmd: &str, payload: Value) -> Result<Value> {
    let mut stream = UnixStream::connect(&socket_path).await.map_err(|e| {
        agent_err(format!(
            "failed to connect to PROMETHEUS socket {}: {e}",
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
        .map_err(|e| agent_err(format!("failed to write IPC request: {e}")))?;

    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|e| agent_err(format!("failed to read IPC response: {e}")))?;

    let response: ResponseEnvelope = serde_json::from_str(line.trim())
        .map_err(|e| agent_err(format!("invalid IPC response: {e}")))?;

    response.into_result("prometheus")
}

async fn handle_connection(stream: UnixStream, service: PrometheusService) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| agent_err(format!("IPC read error: {e}")))?
    {
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
            .map_err(|e| agent_err(format!("IPC write error: {e}")))?;
    }
    Ok(())
}

fn execute_command(service: &PrometheusService, cmd: CommandEnvelope) -> Result<Value> {
    match cmd.cmd.as_str() {
        "status" => Ok(serde_json::to_value(service.status()?)?),
        "roster" => Ok(serde_json::to_value(service.roster())?),
        "thoughts" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            Ok(serde_json::to_value(service.thoughts(limit)?)?)
        }
        "escalations" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            let include_resolved = cmd
                .payload
                .get("include_resolved")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(serde_json::to_value(
                service.escalations(limit, include_resolved)?,
            )?)
        }
        "resolve_escalation" => {
            let escalation_id = cmd
                .payload
                .get("escalation_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| agent_err("missing escalation_id"))?;
            let note = cmd
                .payload
                .get("note")
                .and_then(|v| v.as_str())
                .unwrap_or("resolved");
            Ok(serde_json::to_value(
                service.resolve_escalation(escalation_id, note)?,
            )?)
        }
        "reconcile_runtime" => {
            let before = cmd
                .payload
                .get("before")
                .and_then(|v| v.as_str())
                .ok_or_else(|| agent_err("missing before"))?;
            let cutoff = chrono::DateTime::parse_from_rfc3339(before)
                .map_err(|err| agent_err(format!("invalid before: {err}")))?
                .with_timezone(&chrono::Utc);
            let apply = cmd
                .payload
                .get("apply")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let note = cmd
                .payload
                .get("note")
                .and_then(|v| v.as_str())
                .unwrap_or("resolved by Prometheus runtime reconciliation");
            Ok(serde_json::to_value(
                service.reconcile_runtime(cutoff, apply, note)?,
            )?)
        }
        "council_fanout" => {
            let topic = cmd
                .payload
                .get("topic")
                .and_then(|v| v.as_str())
                .ok_or_else(|| agent_err("missing topic"))?;
            let participants = cmd
                .payload
                .get("participants")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let context = cmd.payload.get("context").cloned();
            Ok(service.council_fanout(topic, participants, context)?)
        }
        "interrupt_reroute" => Ok(service.interrupt_reroute(cmd.payload)?),
        "execution_intents" => {
            let limit = cmd
                .payload
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(50) as usize;
            let include_terminal = cmd
                .payload
                .get("include_terminal")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(serde_json::to_value(
                service.execution_intents(limit, include_terminal)?,
            )?)
        }
        "execution_intents_recovery" => Ok(service.execution_intents_recovery()?),
        "transition_execution_intent" => {
            let intent_id = cmd
                .payload
                .get("intent_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| agent_err("missing intent_id"))?;
            let status = cmd
                .payload
                .get("status")
                .and_then(|v| v.as_str())
                .ok_or_else(|| agent_err("missing status"))?;
            let note = cmd.payload.get("note").and_then(|v| v.as_str());
            Ok(service.transition_execution_intent(intent_id, status, note)?)
        }
        "compact_execution_intents" => {
            let retention_days = cmd
                .payload
                .get("retention_days")
                .and_then(|v| v.as_i64())
                .unwrap_or(14);
            let max_keep = cmd
                .payload
                .get("max_keep")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000) as usize;
            Ok(service.compact_execution_intents(retention_days, max_keep)?)
        }
        "drift_detect_reconcile" => {
            let auto_open = cmd
                .payload
                .get("auto_open")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            Ok(service.drift_detect_reconcile(auto_open)?)
        }
        other => Err(agent_err(format!("unknown IPC command: {other}"))),
    }
}
