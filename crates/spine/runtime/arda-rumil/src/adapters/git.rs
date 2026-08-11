use std::path::{Component, Path};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::adapters::{outcome, provider_allowed, ProviderAdapter};
use crate::constants::{PROVIDER_COMPLETED, PROVIDER_SKIPPED_BY_POLICY, PROVIDER_UNAVAILABLE};
use crate::contracts::{AuditRequest, CapabilityOutcome};
use crate::error::{Result, RumilError};
use crate::policy::AuditPolicy;

pub const CAPABILITY: &str = "git_state";
pub const PROVIDER_ID: &str = "rumil.git_readonly.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStateSnapshot {
    pub revision: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub status_entries: Vec<GitStatusEntry>,
    pub truncated: bool,
    pub truncation_reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub status: String,
    pub relative_path: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GitAdapter;

impl GitAdapter {
    pub fn inspect(&self, project_root: &Path, policy: &AuditPolicy) -> Result<GitStateSnapshot> {
        if !project_root.is_dir() {
            return Err(RumilError::PathRejected(
                "adapter root is not a directory".to_string(),
            ));
        }
        let inside = git_output(project_root, &["rev-parse", "--is-inside-work-tree"])?;
        if trim_ascii(&inside) != "true" {
            return Err(RumilError::ProviderUnavailable(
                "selected project root is not a Git work tree".to_string(),
            ));
        }

        let revision = git_output_optional(project_root, &["rev-parse", "--verify", "HEAD"])?
            .map(|output| trim_ascii(&output));
        let branch_text = trim_ascii(&git_output(project_root, &["branch", "--show-current"])?);
        let branch = (!branch_text.is_empty()).then_some(branch_text);
        let raw_status = git_output(
            project_root,
            &["status", "--porcelain=v1", "-z", "--untracked-files=normal"],
        )?;
        let (status_entries, truncation_reasons) = parse_status(
            &raw_status,
            policy.budget.max_files,
            policy.budget.max_total_bytes,
        );

        Ok(GitStateSnapshot {
            revision,
            branch,
            dirty: !raw_status.is_empty(),
            status_entries,
            truncated: !truncation_reasons.is_empty(),
            truncation_reasons,
        })
    }
}

impl ProviderAdapter for GitAdapter {
    fn capability(&self) -> &str {
        CAPABILITY
    }

    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn run(
        &self,
        _request: &AuditRequest,
        policy: &AuditPolicy,
        project_root: &Path,
    ) -> Result<(serde_json::Value, CapabilityOutcome)> {
        if !provider_allowed(policy, PROVIDER_ID) {
            return Ok((
                serde_json::Value::Null,
                outcome(
                    CAPABILITY,
                    PROVIDER_ID,
                    PROVIDER_SKIPPED_BY_POLICY,
                    Some("provider is not allowlisted".to_string()),
                ),
            ));
        }
        match self.inspect(project_root, policy) {
            Ok(snapshot) => {
                let detail = snapshot
                    .truncated
                    .then(|| snapshot.truncation_reasons.join("; "));
                Ok((
                    serde_json::to_value(snapshot)?,
                    outcome(CAPABILITY, PROVIDER_ID, PROVIDER_COMPLETED, detail),
                ))
            }
            Err(RumilError::ProviderUnavailable(detail)) => Ok((
                serde_json::Value::Null,
                outcome(CAPABILITY, PROVIDER_ID, PROVIDER_UNAVAILABLE, Some(detail)),
            )),
            Err(error) => Err(error),
        }
    }
}

fn git_output(project_root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| RumilError::ProviderUnavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(RumilError::ProviderUnavailable(format!(
            "read-only Git query failed with status {}",
            output.status
        )));
    }
    Ok(output.stdout)
}

fn git_output_optional(project_root: &Path, args: &[&str]) -> Result<Option<Vec<u8>>> {
    let output = Command::new("git")
        .args(args)
        .current_dir(project_root)
        .output()
        .map_err(|error| RumilError::ProviderUnavailable(error.to_string()))?;
    Ok(output.status.success().then_some(output.stdout))
}

fn parse_status(raw: &[u8], max_files: u64, max_bytes: u64) -> (Vec<GitStatusEntry>, Vec<String>) {
    let mut entries = Vec::new();
    let mut reasons = Vec::new();
    let mut consumed_bytes = 0_u64;
    let records: Vec<&[u8]> = raw.split(|byte| *byte == 0).collect();
    let mut index = 0;
    while index < records.len() {
        let record = records[index];
        index += 1;
        if record.is_empty() {
            continue;
        }
        if entries.len() as u64 >= max_files {
            reasons.push(format!("git_status_file_budget reached at {max_files}"));
            break;
        }
        if consumed_bytes.saturating_add(record.len() as u64) > max_bytes {
            reasons.push(format!("git_status_byte_budget reached at {max_bytes}"));
            break;
        }
        consumed_bytes += record.len() as u64;
        if record.len() < 4 || record[2] != b' ' {
            reasons.push("malformed Git status record omitted".to_string());
            continue;
        }
        let status = String::from_utf8_lossy(&record[..2]).into_owned();
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        if safe_relative_path(&path) {
            entries.push(GitStatusEntry {
                status: status.clone(),
                relative_path: path,
            });
        } else {
            reasons.push("unsafe Git status path omitted".to_string());
        }
        if status.contains('R') || status.contains('C') {
            index += 1;
        }
    }
    (entries, reasons)
}

fn safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn trim_ascii(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).trim().to_string()
}
