#![cfg(feature = "full-cli")]
use super::*;

pub(crate) async fn athena_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match athena_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "ATHENA IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn athena_ingest_batch_chunk(
    socket_path: &std::path::Path,
    store: &AthenaStore,
    inputs: &[String],
    submitted_by: &str,
    task_context: &str,
) -> anyhow::Result<BatchIngestReport> {
    let value = athena_call_or_local(
        socket_path,
        "ingest_batch",
        serde_json::json!({
            "inputs": inputs,
            "submitted_by": submitted_by,
            "task_context": task_context
        }),
        || {
            Ok(serde_json::to_value(store.ingest_batch(
                inputs,
                submitted_by,
                task_context,
            )?)?)
        },
    )
    .await?;

    let report: BatchIngestReport = serde_json::from_value(value)
        .map_err(|err| anyhow::anyhow!("invalid ATHENA ingest_batch response shape: {err}"))?;
    Ok(report)
}

pub(crate) fn merge_batch_report(
    aggregate: &mut BatchIngestReport,
    mut report: BatchIngestReport,
    max_receipts: usize,
) {
    aggregate.total_inputs += report.total_inputs;
    aggregate.accepted_inputs += report.accepted_inputs;
    aggregate.deduplicated_inputs += report.deduplicated_inputs;
    aggregate.invalid_inputs += report.invalid_inputs;
    if aggregate.receipts.len() < max_receipts {
        let remaining = max_receipts.saturating_sub(aggregate.receipts.len());
        aggregate.receipts.extend(
            report
                .receipts
                .drain(..remaining.min(report.receipts.len())),
        );
    }
}

pub(crate) fn socket_path_from_env(env_key: &str, default_socket: &str) -> std::path::PathBuf {
    std::env::var(env_key)
        .map(|v| expand_home(&v))
        .unwrap_or_else(|_| {
            let file = std::path::Path::new(default_socket)
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or(default_socket);
            std::path::PathBuf::from(default_runtime_socket(file))
        })
}

pub(crate) async fn prometheus_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match prometheus_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "PROMETHEUS IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn manwe_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match manwe_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "MANWE IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn manwe_call_or_local_async<F, Fut>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>>,
{
    match manwe_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "MANWE IPC unavailable, using local fallback");
            local_fallback().await
        }
    }
}

pub(crate) async fn mnemosyne_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match mnemosyne_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "MNEMOSYNE IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn hades_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match hades_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "HADES IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn hermes_call_or_local<F>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> anyhow::Result<serde_json::Value>,
{
    match hermes_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "HERMES IPC unavailable, using local fallback");
            local_fallback()
        }
    }
}

pub(crate) async fn hermes_call_or_local_async<F, Fut>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>>,
{
    match hermes_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "HERMES IPC unavailable, using local fallback");
            local_fallback().await
        }
    }
}

pub(crate) async fn apollo_call_or_local<F, Fut>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>>,
{
    match apollo_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "APOLLO IPC unavailable, using local fallback");
            local_fallback().await
        }
    }
}

pub(crate) async fn plutus_call_or_local<F, Fut>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>>,
{
    match plutus_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "PLUTUS IPC unavailable, using local fallback");
            local_fallback().await
        }
    }
}

pub(crate) async fn oracle_call_or_local<F, Fut>(
    socket_path: &std::path::Path,
    cmd: &str,
    payload: serde_json::Value,
    local_fallback: F,
) -> anyhow::Result<serde_json::Value>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<serde_json::Value>>,
{
    match oracle_ipc_send_command(socket_path.to_path_buf(), cmd, payload).await {
        Ok(value) => Ok(value),
        Err(err) => {
            tracing::info!(error = %err, cmd, "ORACLE IPC unavailable, using local fallback");
            local_fallback().await
        }
    }
}

pub(crate) fn parse_json_input(input: &str) -> anyhow::Result<serde_json::Value> {
    Ok(serde_json::from_str(input)?)
}

pub(crate) fn parse_execution_priority(priority: &str) -> ExecutionPriority {
    match priority.to_ascii_lowercase().as_str() {
        "low" => ExecutionPriority::Low,
        "high" => ExecutionPriority::High,
        "critical" => ExecutionPriority::Critical,
        _ => ExecutionPriority::Normal,
    }
}

pub(crate) fn parse_joulework_unit(unit: &str) -> JouleWorkUnit {
    match unit.to_ascii_lowercase().as_str() {
        "compute" => JouleWorkUnit::Compute,
        "network" => JouleWorkUnit::Network,
        "storage" => JouleWorkUnit::Storage,
        "attention" => JouleWorkUnit::Attention,
        _ => JouleWorkUnit::Reasoning,
    }
}

pub(crate) fn load_aipkg_manifest(path: &str) -> anyhow::Result<AipkgManifest> {
    let path = expand_home(path);
    let manifest = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&manifest)?)
}

pub(crate) fn build_aipkg_preflight_receipt(
    manifest_path: &str,
    manifest: &AipkgManifest,
    requested_runtime_profile: Option<&str>,
) -> anyhow::Result<serde_json::Value> {
    let requested_profile = requested_runtime_profile
        .unwrap_or(&manifest.runtime_profile)
        .to_string();
    let compatible = requested_profile == manifest.runtime_profile;
    let status = if compatible {
        "compatible_zero_work"
    } else {
        "profile_mismatch"
    };
    Ok(json!({
        "schema_version": "arda.aipkg.preflight-receipt.v1",
        "generated_at_utc": Utc::now().to_rfc3339(),
        "authority": "aipkg_cli_preflight",
        "manifest_path": expand_home(manifest_path),
        "package_id": manifest.package_id,
        "version": manifest.version,
        "package_digest": manifest.package_digest,
        "requested_runtime_profile": requested_profile,
        "manifest_runtime_profile": manifest.runtime_profile,
        "compatibility": {
            "compatible": compatible,
            "required_runtime_profile_match": true,
            "zero_work_required": manifest.preflight.zero_work_required,
        },
        "quote": {
            "required": manifest.preflight.quote_required,
            "estimated_joulework": 0.0,
            "currency": "joulework",
        },
        "governance": {
            "triad_required": manifest.governance.triad_required,
            "bacon_lite_required": manifest.governance.bacon_lite_required,
            "joulework_budget_required": manifest.governance.joulework_budget_required,
            "love_eq_guard_required": manifest.governance.love_eq_guard_required,
            "soterion_trace_required": manifest.governance.soterion_trace_required,
        },
        "receipt_policy": {
            "preflight_required": manifest.receipts.preflight_required,
            "execution_required": manifest.receipts.execution_required,
            "validation_required": manifest.receipts.validation_required,
            "settlement_optional": manifest.receipts.settlement_optional,
            "signatures_required": manifest.receipts.signatures_required,
        },
        "status": status,
    }))
}
