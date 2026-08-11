use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use chrono::Utc;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::time::timeout;
use uuid::Uuid;

use super::{CommandProviderSpec, ProviderExecution};
use crate::contracts::{CommandReceipt, CommandReceiptStatus};
use crate::error::{Result, RumilError};
use crate::hash::sha256_bytes;
use crate::policy::AuditPolicy;

#[derive(Debug, Default, Clone, Copy)]
pub struct ProviderRunner;

impl ProviderRunner {
    pub async fn run(
        &self,
        spec: &CommandProviderSpec,
        policy: &AuditPolicy,
        project_root: &Path,
    ) -> Result<ProviderExecution> {
        let started_at_utc = Utc::now();
        let command_id = Uuid::new_v4();
        let argv_digest = digest_json(&(spec.program.as_str(), &spec.args))?;
        let configuration_digest = Some(digest_json(spec)?);

        if !policy
            .provider_allowlist
            .iter()
            .any(|allowed| allowed == &spec.provider_id)
        {
            return Ok(empty_execution(CommandReceipt {
                command_id,
                provider_id: spec.provider_id.clone(),
                argv_digest,
                working_directory_relative: spec.working_directory_relative.clone(),
                policy_id: policy.profile_id.clone(),
                started_at_utc,
                finished_at_utc: Some(Utc::now()),
                exit_code: None,
                stdout_digest: None,
                stderr_digest: None,
                stdout_bytes_retained: 0,
                stderr_bytes_retained: 0,
                truncated: false,
                timed_out: false,
                status: CommandReceiptStatus::Denied,
                tool_version: None,
                configuration_digest,
                authority: "review_only".to_string(),
            }));
        }

        let working_directory =
            match safe_working_directory(project_root, &spec.working_directory_relative) {
                Ok(path) => path,
                Err(_) => {
                    return Ok(empty_execution(CommandReceipt {
                        command_id,
                        provider_id: spec.provider_id.clone(),
                        argv_digest,
                        working_directory_relative: spec.working_directory_relative.clone(),
                        policy_id: policy.profile_id.clone(),
                        started_at_utc,
                        finished_at_utc: Some(Utc::now()),
                        exit_code: None,
                        stdout_digest: None,
                        stderr_digest: None,
                        stdout_bytes_retained: 0,
                        stderr_bytes_retained: 0,
                        truncated: false,
                        timed_out: false,
                        status: CommandReceiptStatus::Denied,
                        tool_version: None,
                        configuration_digest,
                        authority: "review_only".to_string(),
                    }));
                }
            };

        let effective_timeout = spec
            .timeout_seconds
            .min(policy.budget.command_timeout_seconds);
        let stdout_limit = spec.max_stdout_bytes.min(policy.budget.max_total_bytes);
        let stderr_limit = spec.max_stderr_bytes.min(policy.budget.max_total_bytes);
        let tool_version = probe_version(spec, &working_directory, effective_timeout).await;
        let mut command = Command::new(&spec.program);
        command
            .args(&spec.args)
            .current_dir(&working_directory)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(_) => {
                return Ok(empty_execution(CommandReceipt {
                    command_id,
                    provider_id: spec.provider_id.clone(),
                    argv_digest,
                    working_directory_relative: spec.working_directory_relative.clone(),
                    policy_id: policy.profile_id.clone(),
                    started_at_utc,
                    finished_at_utc: Some(Utc::now()),
                    exit_code: None,
                    stdout_digest: None,
                    stderr_digest: None,
                    stdout_bytes_retained: 0,
                    stderr_bytes_retained: 0,
                    truncated: false,
                    timed_out: false,
                    status: CommandReceiptStatus::Unavailable,
                    tool_version,
                    configuration_digest,
                    authority: "review_only".to_string(),
                }));
            }
        };

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| RumilError::ProviderFailed("provider stdout was not piped".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| RumilError::ProviderFailed("provider stderr was not piped".into()))?;
        let stdout_task = tokio::spawn(read_bounded(stdout, stdout_limit));
        let stderr_task = tokio::spawn(read_bounded(stderr, stderr_limit));

        let (exit_code, timed_out, status) =
            match timeout(Duration::from_secs(effective_timeout), child.wait()).await {
                Ok(Ok(exit)) if exit.success() => {
                    (exit.code(), false, CommandReceiptStatus::Completed)
                }
                Ok(Ok(exit)) => (exit.code(), false, CommandReceiptStatus::Failed),
                Ok(Err(_)) => (None, false, CommandReceiptStatus::Failed),
                Err(_) => {
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    (None, true, CommandReceiptStatus::TimedOut)
                }
            };
        let stdout_result = stdout_task
            .await
            .map_err(|error| RumilError::ProviderFailed(error.to_string()))??;
        let stderr_result = stderr_task
            .await
            .map_err(|error| RumilError::ProviderFailed(error.to_string()))??;
        let truncated = stdout_result.truncated || stderr_result.truncated;

        Ok(ProviderExecution {
            stdout: stdout_result.retained,
            stderr: stderr_result.retained,
            receipt: CommandReceipt {
                command_id,
                provider_id: spec.provider_id.clone(),
                argv_digest,
                working_directory_relative: spec.working_directory_relative.clone(),
                policy_id: policy.profile_id.clone(),
                started_at_utc,
                finished_at_utc: Some(Utc::now()),
                exit_code,
                stdout_digest: Some(stdout_result.digest),
                stderr_digest: Some(stderr_result.digest),
                stdout_bytes_retained: stdout_result.retained_len,
                stderr_bytes_retained: stderr_result.retained_len,
                truncated,
                timed_out,
                status,
                tool_version,
                configuration_digest,
                authority: "review_only".to_string(),
            },
        })
    }
}

struct BoundedRead {
    retained: Vec<u8>,
    retained_len: u64,
    digest: String,
    truncated: bool,
}

async fn read_bounded<R>(mut reader: R, max_bytes: u64) -> Result<BoundedRead>
where
    R: AsyncRead + Unpin,
{
    let capacity = usize::try_from(max_bytes.min(1024 * 1024)).unwrap_or(1024 * 1024);
    let mut retained = Vec::with_capacity(capacity);
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut buffer = [0_u8; 8192];
    loop {
        let read = reader.read(&mut buffer).await.map_err(RumilError::Io)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        total = total.saturating_add(read as u64);
        let remaining = max_bytes.saturating_sub(retained.len() as u64) as usize;
        retained.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    Ok(BoundedRead {
        retained_len: retained.len() as u64,
        retained,
        digest: format!("{:x}", hasher.finalize()),
        truncated: total > max_bytes,
    })
}

async fn probe_version(
    spec: &CommandProviderSpec,
    cwd: &Path,
    effective_timeout: u64,
) -> Option<String> {
    if spec.version_args.is_empty() {
        return None;
    }
    let mut command = Command::new(&spec.program);
    command
        .args(&spec.version_args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .kill_on_drop(true);
    let output = timeout(
        Duration::from_secs(effective_timeout.max(1)),
        command.output(),
    )
    .await
    .ok()?
    .ok()?;
    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!version.is_empty()).then_some(version.chars().take(256).collect())
}

fn safe_working_directory(project_root: &Path, relative: &str) -> Result<PathBuf> {
    let relative = Path::new(relative);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RumilError::PathRejected(
            "provider working directory escapes project root".to_string(),
        ));
    }
    let root = std::fs::canonicalize(project_root).map_err(RumilError::Io)?;
    let selected = std::fs::canonicalize(root.join(relative)).map_err(RumilError::Io)?;
    if !selected.starts_with(&root) || !selected.is_dir() {
        return Err(RumilError::PathRejected(
            "provider working directory is outside project root".to_string(),
        ));
    }
    Ok(selected)
}

fn digest_json<T: Serialize>(value: &T) -> Result<String> {
    Ok(sha256_bytes(&serde_json::to_vec(value)?))
}

fn empty_execution(receipt: CommandReceipt) -> ProviderExecution {
    ProviderExecution {
        stdout: Vec::new(),
        stderr: Vec::new(),
        receipt,
    }
}

use serde::Serialize;
