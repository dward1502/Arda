//! Private retained-worker transport. A dropped caller closes its socket; the
//! worker owns cancellation and must finish cleanup before accepting another job.
use super::{AdapterCancellation, BoundedProcessOutput, HermesAdapterError};
use crate::objectives::{snapshot_protocol as wire, RetainedExecution};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
mod dispatch;
pub(super) use dispatch::invoke;

pub(super) struct ExecutionLimits {
    pub duration: Duration,
    pub grace_ms: u64,
    pub limit: usize,
}

#[cfg(test)]
#[path = "retained_tests.rs"]
mod tests;

impl super::HermesAdapter {
    /// Construct from private durable authority, without reopening the live
    /// workspace. The worker, not this host path, selects the execution tree.
    pub fn load_retained(
        config_path: impl AsRef<std::path::Path>,
        project_root: impl AsRef<std::path::Path>,
        host_environment: &BTreeMap<String, String>,
        binding: RetainedExecution,
    ) -> Result<Self, HermesAdapterError> {
        let raw =
            std::fs::read_to_string(config_path).map_err(|source| HermesAdapterError::Io {
                context: "read retained adapter config".into(),
                source,
            })?;
        let config = super::HermesAdapterConfig::from_toml_str(&raw)?;
        let project_root = project_root.as_ref().to_path_buf();
        if !project_root.is_absolute()
            || project_root.components().any(|c| {
                matches!(
                    c,
                    std::path::Component::ParentDir | std::path::Component::CurDir
                )
            })
            || config.max_timeout_ms > wire::MAX_TIMEOUT_MS
            || config.max_output_bytes > wire::MAX_OUTPUT_BYTES
        {
            return Err(HermesAdapterError::InvalidConfig(
                "invalid retained workspace or unsupported budgets".into(),
            ));
        }
        // Mode discovery is authenticated against the saved manifest at dispatch.
        // Configured workers need no host executable; only legacy mode resolves it.
        let executable = std::path::PathBuf::from(&config.executable);
        let mut environment: BTreeMap<_, _> = config
            .inherit_environment
            .iter()
            .filter_map(|key| {
                host_environment
                    .get(key)
                    .map(|value| (key.clone(), value.clone()))
            })
            .collect();
        if let Some(path) = host_environment.get("PATH") {
            environment
                .entry("PATH".into())
                .or_insert_with(|| path.clone());
        }
        Ok(Self {
            config,
            executable,
            cwd: project_root.clone(),
            project_root,
            environment,
            workspace: None,
            retained: Some(binding),
        })
    }
}

#[derive(Deserialize)]
struct Output {
    ok: bool,
    #[serde(default)]
    timed_out: bool,
    #[serde(default)]
    cancelled: bool,
    #[serde(default)]
    output_limit: bool,
    stdout: Option<String>,
    stderr: Option<String>,
    code: Option<i32>,
}

fn protocol_error() -> HermesAdapterError {
    // Never echo a remote payload: it may contain private transport material.
    HermesAdapterError::InvalidResult(
        "retained worker response invalid; reconciliation required".into(),
    )
}

pub(super) async fn execute(
    binding: &RetainedExecution,
    argv: Vec<String>,
    environment: BTreeMap<String, String>,
    duration: Duration,
    cancellation: &AdapterCancellation,
    grace_ms: u64,
    limit: usize,
) -> Result<BoundedProcessOutput, HermesAdapterError> {
    execute_inner(
        binding,
        argv,
        environment,
        cancellation,
        ExecutionLimits {
            duration,
            grace_ms,
            limit,
        },
        None,
    )
    .await
}

async fn execute_inner(
    binding: &RetainedExecution,
    argv: Vec<String>,
    environment: BTreeMap<String, String>,
    cancellation: &AdapterCancellation,
    limits: ExecutionLimits,
    operation: Option<crate::objectives::runtime_operation::RuntimeOperation>,
) -> Result<BoundedProcessOutput, HermesAdapterError> {
    let ExecutionLimits {
        duration,
        grace_ms,
        limit,
    } = limits;
    let timeout_ms = u64::try_from(duration.as_millis()).map_err(|_| protocol_error())?;
    if !(1..=wire::MAX_TIMEOUT_MS).contains(&timeout_ms)
        || !(1..=wire::MAX_OUTPUT_BYTES).contains(&limit)
    {
        return Err(HermesAdapterError::InvalidConfig(
            "unsupported retained-worker execution limits".into(),
        ));
    }
    let request = if let Some(operation) = operation {
        wire::Request::Runtime {
            capability: binding.snapshot.capability.clone(),
            lease: binding.lease.clone(),
            operation,
            timeout_ms,
            max_output_bytes: limit,
        }
    } else {
        wire::Request::Execute {
            capability: binding.snapshot.capability.clone(),
            lease: binding.lease.clone(),
            argv,
            environment,
            timeout_ms,
            max_output_bytes: limit,
        }
    };
    let encoded = serde_json::to_vec(&request)?;
    if encoded.len() >= wire::MAX_REQUEST_BYTES {
        return Err(HermesAdapterError::InvalidTask(
            "retained-worker request exceeds framing limit".into(),
        ));
    }
    let mut signal = cancellation.subscribe();
    if *signal.borrow() {
        return Err(HermesAdapterError::Cancelled);
    }
    let started = tokio::time::Instant::now();
    let stream = tokio::select! {
        biased;
        _ = signal.wait_for(|value| *value) => return Err(HermesAdapterError::Cancelled),
        _ = tokio::time::sleep_until(started + duration) => return Err(HermesAdapterError::Timeout),
        stream = UnixStream::connect(&binding.snapshot.endpoint) => stream.map_err(|_| protocol_error())?,
    };
    if stream.peer_cred().map_err(|_| protocol_error())?.uid() != unsafe { libc::geteuid() } {
        return Err(protocol_error());
    }
    let (reader, mut writer) = stream.into_split();
    // The worker rejects EOF without a newline. Until the final delimiter is
    // sent, cancellation can close the socket without dispatching any process.
    tokio::select! {
        biased;
        _ = signal.wait_for(|value| *value) => return Err(HermesAdapterError::Cancelled),
        _ = tokio::time::sleep_until(started + duration) => return Err(HermesAdapterError::Timeout),
        result = writer.write_all(&encoded) => result.map_err(|_| protocol_error())?,
    }
    // A single-byte write is cancellation-safe: Pending means no dispatch;
    // successful completion moves immediately into cleanup-acknowledged mode.
    let written = tokio::select! {
        biased;
        _ = signal.wait_for(|value| *value) => return Err(HermesAdapterError::Cancelled),
        _ = tokio::time::sleep_until(started + duration) => return Err(HermesAdapterError::Timeout),
        result = writer.write(b"\n") => result.map_err(|_| protocol_error())?,
    };
    if written != 1 {
        return Err(protocol_error());
    }
    let response = async {
        let mut framed = BufReader::new(reader.take(wire::MAX_RESPONSE_BYTES as u64 + 1));
        let mut bytes = Vec::new();
        framed
            .read_until(b'\n', &mut bytes)
            .await
            .map_err(|_| protocol_error())?;
        if bytes.len() > wire::MAX_RESPONSE_BYTES || bytes.last() != Some(&b'\n') {
            return Err(protocol_error());
        }
        let output: Output = serde_json::from_slice(&bytes).map_err(|_| protocol_error())?;
        // Only successful execution envelopes are emitted after proven cleanup.
        if !output.ok || output.stdout.is_none() || output.stderr.is_none() {
            return Err(protocol_error());
        }
        Ok(output)
    };
    tokio::pin!(response);
    let interrupt = tokio::select! {
        biased;
        _ = signal.wait_for(|value| *value) => HermesAdapterError::Cancelled,
        _ = tokio::time::sleep_until(started + duration) => HermesAdapterError::Timeout,
        output = &mut response => return finish(output?, limit),
    };
    // Half-close requests cancellation but retains the response side so cleanup
    // acknowledgement, rather than socket disappearance, bounds our return.
    writer
        .shutdown()
        .await
        .map_err(|_| HermesAdapterError::ReapTimeout)?;
    match tokio::time::timeout(Duration::from_millis(grace_ms), &mut response).await {
        Ok(Ok(_)) => Err(interrupt),
        _ => Err(HermesAdapterError::ReapTimeout),
    }
}

pub(super) async fn verify_artifacts(
    binding: &RetainedExecution,
    artifacts: &[super::HermesArtifactEvidence],
    duration: Duration,
    cancellation: &AdapterCancellation,
    grace_ms: u64,
) -> Result<(), HermesAdapterError> {
    if artifacts.is_empty() {
        return Ok(());
    }
    // Isolated Python is part of the explicitly mounted /usr runtime, not a
    // script or interpreter resolved through provider-controlled workspace/PATH.
    let output = invoke(
        binding,
        crate::objectives::runtime_operation::RuntimeOperation::VerifyArtifacts {
            paths: artifacts.iter().map(|a| a.path.clone()).collect(),
        },
        vec![
            "/usr/bin/python3".into(),
            "-I".into(),
            "-c".into(),
            include_str!("verify_retained_artifacts.py").into(),
            serde_json::to_string(artifacts)?,
        ],
        BTreeMap::new(),
        cancellation,
        ExecutionLimits {
            duration,
            grace_ms,
            limit: 65536,
        },
    )
    .await?;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Stamp {
        path: String,
        sha256: String,
    }
    let stamps: Vec<Stamp> =
        serde_json::from_slice(&output.stdout).map_err(|_| protocol_error())?;
    if stamps.len() != artifacts.len()
        || stamps.iter().zip(artifacts).any(|(stamp, expected)| {
            stamp.path != expected.path || format!("sha256:{}", stamp.sha256) != expected.digest
        })
    {
        return Err(HermesAdapterError::InvalidResult(
            "retained artifact digest mismatch".into(),
        ));
    }
    Ok(())
}

fn finish(output: Output, limit: usize) -> Result<BoundedProcessOutput, HermesAdapterError> {
    if output.output_limit {
        return Err(HermesAdapterError::OutputTooLarge { limit });
    }
    if output.cancelled {
        return Err(HermesAdapterError::Cancelled);
    }
    if output.timed_out {
        return Err(HermesAdapterError::Timeout);
    }
    let stdout = output.stdout.ok_or_else(protocol_error)?.into_bytes();
    let stderr = output.stderr.ok_or_else(protocol_error)?.into_bytes();
    if stdout.len() > limit || stderr.len() > limit {
        return Err(HermesAdapterError::OutputTooLarge { limit });
    }
    if output.code != Some(0) {
        return Err(HermesAdapterError::ProcessFailed {
            code: output.code,
            stderr: String::from_utf8_lossy(&stderr).trim().to_owned(),
        });
    }
    Ok(BoundedProcessOutput { stdout, stderr })
}
