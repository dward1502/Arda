use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::error::{Result, RumilError};
use crate::hash::sha256_bytes;

/// Profile-selected bound for targeted source inspection. Rúmil has no
/// repository-dump provider; callers must name paths and cap each excerpt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceInspectionPolicy {
    #[serde(default)]
    pub relative_paths: Vec<String>,
    pub max_excerpt_bytes_per_file: u64,
    pub max_total_excerpt_bytes: u64,
    #[serde(default)]
    pub redaction_patterns: Vec<String>,
}

impl SourceInspectionPolicy {
    pub fn disabled() -> Self {
        Self {
            relative_paths: Vec::new(),
            max_excerpt_bytes_per_file: 0,
            max_total_excerpt_bytes: 0,
            redaction_patterns: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceExcerpt {
    pub excerpt_id: String,
    pub relative_path: String,
    pub content: String,
    pub content_digest: String,
    pub redacted: bool,
    pub truncated: bool,
}

/// Inspect only profile-selected project-relative files. Paths that escape via
/// `..`, absolute syntax, or symlinks are rejected before reading.
pub fn inspect_sources(
    project_root: &Path,
    policy: &SourceInspectionPolicy,
) -> Result<Vec<SourceExcerpt>> {
    let root = std::fs::canonicalize(project_root).map_err(RumilError::Io)?;
    let mut excerpts = Vec::new();
    let mut remaining_total = policy.max_total_excerpt_bytes;

    for relative_text in &policy.relative_paths {
        if remaining_total == 0 {
            break;
        }
        let relative = safe_relative(relative_text)?;
        let selected = std::fs::canonicalize(root.join(relative)).map_err(RumilError::Io)?;
        if !selected.starts_with(&root) || !selected.is_file() {
            return Err(RumilError::PathRejected(
                "selected source path escapes project root or is not a file".to_string(),
            ));
        }
        let file_bytes = std::fs::read(&selected).map_err(RumilError::Io)?;
        let limit = policy
            .max_excerpt_bytes_per_file
            .min(remaining_total)
            .min(file_bytes.len() as u64) as usize;
        let raw_excerpt = &file_bytes[..limit];
        let source_digest = sha256_bytes(raw_excerpt);
        let mut content = String::from_utf8_lossy(raw_excerpt).into_owned();
        let mut redacted = false;
        for pattern in policy
            .redaction_patterns
            .iter()
            .filter(|pattern| !pattern.is_empty())
        {
            if content.contains(pattern) {
                content = content.replace(pattern, "[REDACTED]");
                redacted = true;
            }
        }
        let content = bounded_string(content, limit);
        let excerpt_id = sha256_bytes(
            format!("{}:{source_digest}", relative_text.replace('\\', "/")).as_bytes(),
        );
        excerpts.push(SourceExcerpt {
            excerpt_id,
            relative_path: relative_text.replace('\\', "/"),
            content,
            content_digest: source_digest,
            redacted,
            truncated: limit < file_bytes.len(),
        });
        remaining_total = remaining_total.saturating_sub(limit as u64);
    }
    Ok(excerpts)
}

fn safe_relative(path: &str) -> Result<&Path> {
    let path = Path::new(path);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(RumilError::PathRejected(
            "source excerpt path must be project-relative".to_string(),
        ));
    }
    Ok(path)
}

fn bounded_string(content: String, max_bytes: usize) -> String {
    if content.len() <= max_bytes {
        return content;
    }
    let mut end = max_bytes;
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    content[..end].to_string()
}
