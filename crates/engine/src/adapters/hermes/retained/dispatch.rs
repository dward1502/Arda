//! Resolve execution mode from the exact saved manifest, never PATH or a fallback.
use super::*;
use crate::objectives::runtime_operation::RuntimeOperation;
use sha2::{Digest, Sha256};

async fn configured(binding: &RetainedExecution) -> Result<bool, HermesAdapterError> {
    let mut stream = UnixStream::connect(&binding.snapshot.endpoint)
        .await
        .map_err(|_| protocol_error())?;
    if stream.peer_cred().map_err(|_| protocol_error())?.uid() != unsafe { libc::geteuid() } {
        return Err(protocol_error());
    }
    let mut request = serde_json::to_vec(&wire::Request::Inspect)?;
    request.push(b'\n');
    stream
        .write_all(&request)
        .await
        .map_err(|_| protocol_error())?;
    let mut reader = BufReader::new(stream.take(wire::MAX_RESPONSE_BYTES as u64 + 1));
    let mut bytes = Vec::new();
    reader
        .read_until(b'\n', &mut bytes)
        .await
        .map_err(|_| protocol_error())?;
    if bytes.len() > wire::MAX_RESPONSE_BYTES || bytes.last() != Some(&b'\n') {
        return Err(protocol_error());
    }
    #[derive(Deserialize)]
    struct Inspection {
        ok: bool,
        manifest: wire::Manifest,
        manifest_digest: String,
    }
    let inspected: Inspection = serde_json::from_slice(&bytes).map_err(|_| protocol_error())?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&inspected.manifest)?)
    );
    if !inspected.ok
        || digest != binding.snapshot.manifest_digest
        || inspected.manifest_digest != digest
        || inspected.manifest.capability != binding.snapshot.capability
    {
        return Err(protocol_error());
    }
    match (
        inspected.manifest.version,
        inspected.manifest.runtime_bundle.as_ref(),
        inspected.manifest.admission_digest.as_ref(),
    ) {
        (1, None, None) => Ok(false),
        (2, Some(bundle), Some(admission))
            if bundle.version == 1
                && admission.len() == 64
                && admission
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) =>
        {
            Ok(true)
        }
        _ => Err(protocol_error()),
    }
}

pub(in crate::adapters::hermes) async fn invoke(
    binding: &RetainedExecution,
    operation: RuntimeOperation,
    legacy_argv: Vec<String>,
    environment: BTreeMap<String, String>,
    cancellation: &AdapterCancellation,
    limits: ExecutionLimits,
) -> Result<BoundedProcessOutput, HermesAdapterError> {
    let ExecutionLimits {
        duration,
        grace_ms,
        limit,
    } = limits;
    let started = tokio::time::Instant::now();
    let mut cancelled = cancellation.subscribe();
    if *cancelled.borrow() {
        return Err(HermesAdapterError::Cancelled);
    }
    let configured = tokio::select! {
        biased;
        _ = cancelled.wait_for(|value| *value) => return Err(HermesAdapterError::Cancelled),
        _ = tokio::time::sleep_until(started + duration) => return Err(HermesAdapterError::Timeout),
        result = configured(binding) => result?,
    };
    if configured && environment.keys().any(|key| key != "PATH") {
        return Err(HermesAdapterError::InvalidConfig(
            "configured runtime owns all executable/profile/environment selection".into(),
        ));
    }
    let remaining = duration
        .checked_sub(started.elapsed())
        .filter(|d| !d.is_zero())
        .ok_or(HermesAdapterError::Timeout)?;
    if configured {
        operation.validate().map_err(|_| {
            HermesAdapterError::InvalidTask("invalid typed runtime operation".into())
        })?;
        execute_inner(
            binding,
            legacy_argv,
            environment,
            cancellation,
            ExecutionLimits {
                duration: remaining,
                grace_ms,
                limit,
            },
            Some(operation),
        )
        .await
    } else {
        let mut legacy_argv = legacy_argv;
        let executable = legacy_argv
            .first_mut()
            .ok_or_else(|| HermesAdapterError::InvalidTask("missing legacy executable".into()))?;
        if std::path::Path::new(executable.as_str()).is_relative() {
            let resolved = super::super::resolve_executable(executable, &environment)?;
            *executable = resolved
                .to_str()
                .ok_or_else(|| {
                    HermesAdapterError::InvalidConfig("legacy executable is not UTF-8".into())
                })?
                .to_owned();
        }
        let remaining = duration
            .checked_sub(started.elapsed())
            .filter(|d| !d.is_zero())
            .ok_or(HermesAdapterError::Timeout)?;
        execute(
            binding,
            legacy_argv,
            environment,
            remaining,
            cancellation,
            grace_ms,
            limit,
        )
        .await
    }
}
