//! Bounded generic inventory walker for Rúmil.
//!
//! Walks only approved roots with explicit maximum depth, file count, byte
//! count, and timeout. Excludes `.git`, `target`, `node_modules`, secrets,
//! credential files, and policy-defined private paths by default.
//! Uses the `ignore` crate behind the `walkdir` feature.

use std::path::{Path, PathBuf};

use crate::error::{Result, RumilError};
use crate::policy::AuditPolicy;
#[cfg(feature = "walkdir")]
use crate::policy::ExclusionKind;
use crate::tree::{TreeEntry, TreeEntryKind};

/// Configuration for a bounded inventory scan.
#[derive(Debug, Clone)]
pub struct InventoryConfig {
    /// Canonical project root used to make every emitted path relative.
    pub project_root: PathBuf,
    /// Canonical policy-selected subtree that traversal may inspect.
    pub scan_root: PathBuf,
    pub policy: AuditPolicy,
}

/// Result of a bounded inventory walk: entries plus truncation/exclusion metadata.
#[derive(Debug, Clone)]
pub struct InventoryReport {
    pub entries: Vec<TreeEntry>,
    pub total_bytes_seen: u64,
    pub truncation_reasons: Vec<String>,
    pub exclusion_summary: Vec<String>,
}

impl InventoryReport {
    /// Coverage is complete only when no traversal budget or timeout truncated it.
    pub fn is_complete(&self) -> bool {
        self.truncation_reasons.is_empty()
    }

    pub fn file_records(&self) -> Vec<crate::contracts::FileRecord> {
        self.entries.iter().map(TreeEntry::to_file_record).collect()
    }

    pub fn summary(&self) -> crate::contracts::InventorySummary {
        let mut summary = crate::contracts::InventorySummary::default();
        for entry in &self.entries {
            match entry.kind {
                TreeEntryKind::File => summary.total_files += 1,
                TreeEntryKind::Directory => summary.total_directories += 1,
                TreeEntryKind::Symlink => summary.total_symlinks += 1,
                TreeEntryKind::Unreadable | TreeEntryKind::Excluded => {}
            }
            if entry.content_sha256.is_some() {
                summary.sampled_files += 1;
            }
        }
        summary.total_bytes = self.total_bytes_seen;
        summary
    }
}

#[cfg(feature = "walkdir")]
const DEFAULT_SECRET_PATTERNS: &[&str] = &[
    ".env",
    "*.pem",
    "*.key",
    "credentials*",
    ".aws",
    ".ssh",
    "*.secrets",
];

#[cfg(feature = "walkdir")]
const DEFAULT_EXCLUDED_DIRS: &[&str] = &[".git", "target", "node_modules"];

/// A bounded tree walker that respects policy limits.
pub struct TreeWalker {
    config: InventoryConfig,
}

impl TreeWalker {
    pub fn new(config: InventoryConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &InventoryConfig {
        &self.config
    }

    /// Execute the bounded walk, producing `TreeEntry` records.
    fn walk(&self) -> Result<InventoryReport> {
        #[cfg(feature = "walkdir")]
        {
            self.walk_with_ignore()
        }
        #[cfg(not(feature = "walkdir"))]
        {
            let _ = self.config.policy;
            Err(RumilError::ProviderUnavailable(
                "walkdir feature is not enabled".to_string(),
            ))
        }
    }

    /// Walk implementation using the `walkdir` crate.
    #[cfg(feature = "walkdir")]
    fn walk_with_ignore(&self) -> Result<InventoryReport> {
        use walkdir::WalkDir;

        let budget = &self.config.policy.budget;
        let root = &self.config.scan_root;
        let mut entries = Vec::new();
        let mut truncation_reasons = Vec::new();
        let mut exclusion_summary = Vec::new();

        let mut total_bytes: u64 = 0;
        let mut file_count: u64 = 0;
        let max_files = budget.max_files;
        let max_bytes = budget.max_total_bytes;
        let started = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(budget.scan_timeout_seconds);

        let mut walker = WalkDir::new(root)
            .max_depth(budget.max_depth)
            .sort_by_file_name()
            .into_iter();

        while let Some(result) = walker.next() {
            if started.elapsed() >= timeout {
                truncation_reasons.push(format!(
                    "scan_timeout reached after {} seconds",
                    budget.scan_timeout_seconds
                ));
                break;
            }
            if file_count >= max_files {
                truncation_reasons.push(format!(
                    "file_count_budget reached at {} entries",
                    file_count
                ));
                break;
            }
            if total_bytes >= max_bytes {
                truncation_reasons.push(format!("byte_budget reached at {} bytes", total_bytes));
                break;
            }

            let entry = match result {
                Ok(entry) => entry,
                Err(err) => {
                    let path = err.path().unwrap_or(root);
                    let rel = self.relative_path_safe(path);
                    entries.push(TreeEntry {
                        relative_path: rel.clone(),
                        kind: TreeEntryKind::Unreadable,
                        size_bytes: None,
                        content_sha256: None,
                        mime_or_extension: None,
                        executable: None,
                        symlink_target_relative: None,
                        redaction_state: crate::contracts::RedactionState::Observed,
                        observed_at_utc: chrono::Utc::now(),
                    });
                    continue;
                }
            };

            let rel = self.relative_path_safe(entry.path());
            let file_type = entry.file_type();

            if file_type.is_dir() {
                if self.is_excluded_dir(entry.file_name().to_str().unwrap_or("")) {
                    exclusion_summary.push(rel.clone());
                    entries.push(TreeEntry {
                        relative_path: rel,
                        kind: TreeEntryKind::Excluded,
                        size_bytes: None,
                        content_sha256: None,
                        mime_or_extension: None,
                        executable: None,
                        symlink_target_relative: None,
                        redaction_state: crate::contracts::RedactionState::Observed,
                        observed_at_utc: chrono::Utc::now(),
                    });
                    walker.skip_current_dir();
                    continue;
                }
                entries.push(TreeEntry {
                    relative_path: rel,
                    kind: TreeEntryKind::Directory,
                    size_bytes: None,
                    content_sha256: None,
                    mime_or_extension: None,
                    executable: None,
                    symlink_target_relative: None,
                    redaction_state: crate::contracts::RedactionState::Observed,
                    observed_at_utc: chrono::Utc::now(),
                });
                continue;
            }

            if file_type.is_file() {
                file_count += 1;
                if self.is_secret_path(&rel) {
                    exclusion_summary.push(rel.clone());
                    entries.push(TreeEntry {
                        relative_path: rel,
                        kind: TreeEntryKind::Excluded,
                        size_bytes: None,
                        content_sha256: None,
                        mime_or_extension: None,
                        executable: None,
                        symlink_target_relative: None,
                        redaction_state: crate::contracts::RedactionState::Redacted,
                        observed_at_utc: chrono::Utc::now(),
                    });
                    continue;
                }

                let size = std::fs::metadata(entry.path())
                    .map(|m| m.len())
                    .unwrap_or(0);
                if total_bytes.saturating_add(size) > max_bytes {
                    truncation_reasons.push(format!(
                        "byte_budget would be exceeded by {} ({} bytes)",
                        rel, size
                    ));
                    break;
                }
                total_bytes += size;

                let digest = if size <= budget.max_excerpt_bytes {
                    #[cfg(feature = "crypto")]
                    {
                        crate::hash::hash_file(entry.path(), budget.max_excerpt_bytes).ok()
                    }
                    #[cfg(not(feature = "crypto"))]
                    {
                        None
                    }
                } else {
                    None
                };

                let ext = entry
                    .path()
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_string());
                let executable = executable_flag(entry.path());

                entries.push(TreeEntry {
                    relative_path: rel,
                    kind: TreeEntryKind::File,
                    size_bytes: Some(size),
                    content_sha256: digest,
                    mime_or_extension: ext,
                    executable: Some(executable),
                    symlink_target_relative: None,
                    redaction_state: crate::contracts::RedactionState::Observed,
                    observed_at_utc: chrono::Utc::now(),
                });
                continue;
            }

            if file_type.is_symlink() {
                let target = std::fs::read_link(entry.path())
                    .ok()
                    .and_then(|p| p.strip_prefix(root).ok().map(|s| s.to_path_buf()))
                    .map(|p| p.to_string_lossy().replace('\\', "/"));
                file_count += 1;
                entries.push(TreeEntry {
                    relative_path: rel,
                    kind: TreeEntryKind::Symlink,
                    size_bytes: None,
                    content_sha256: None,
                    mime_or_extension: None,
                    executable: None,
                    symlink_target_relative: target,
                    redaction_state: crate::contracts::RedactionState::Observed,
                    observed_at_utc: chrono::Utc::now(),
                });
            }
        }

        Ok(InventoryReport {
            entries,
            total_bytes_seen: total_bytes,
            truncation_reasons,
            exclusion_summary,
        })
    }
}

#[cfg(feature = "walkdir")]
impl TreeWalker {
    /// Convert an absolute path to a POSIX-relative path under the project root.
    /// If the path is outside the root (shouldn't happen with `ignore`), returns
    /// "unknown".
    fn relative_path_safe(&self, path: &Path) -> String {
        let relative = path
            .strip_prefix(&self.config.project_root)
            .unwrap_or(Path::new("unknown"))
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        if relative.is_empty() {
            ".".to_string()
        } else {
            relative
        }
    }

    /// Check if a directory name matches default or policy exclusions.
    fn is_excluded_dir(&self, name: &str) -> bool {
        if DEFAULT_EXCLUDED_DIRS.contains(&name) {
            return true;
        }
        let fname = name.to_string();
        for rule in &self.config.policy.exclusion_rules {
            if matches!(rule.kind, ExclusionKind::Directory) && rule.pattern == fname {
                return true;
            }
        }
        false
    }

    /// Check if a relative path matches secret/credential patterns.
    fn is_secret_path(&self, rel: &str) -> bool {
        let basename = Path::new(rel)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(rel);
        for pattern in DEFAULT_SECRET_PATTERNS {
            if glob_match(pattern, rel) || glob_match(pattern, basename) {
                return true;
            }
        }
        for rule in &self.config.policy.exclusion_rules {
            if matches!(rule.kind, ExclusionKind::File | ExclusionKind::Glob)
                && glob_match(&rule.pattern, rel)
            {
                return true;
            }
        }
        false
    }
}

/// Simple glob matcher supporting `*` and `?` wildcards.
#[cfg(feature = "walkdir")]
fn glob_match(pattern: &str, text: &str) -> bool {
    let pat: Vec<char> = pattern.chars().collect();
    let txt: Vec<char> = text.chars().collect();
    glob_match_inner(&pat, &txt, 0, 0)
}

#[cfg(feature = "walkdir")]
fn glob_match_inner(pat: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    // Collapse consecutive `**` into a single `*` for our purposes.
    let mut pi = pi;
    while pi + 1 < pat.len() && pat[pi] == '*' && pat[pi + 1] == '*' {
        pi += 1;
    }

    if pi >= pat.len() {
        return ti >= text.len();
    }

    if pat[pi] == '*' {
        // `*` matches any sequence of chars (including empty)
        if pi + 1 >= pat.len() {
            return true;
        }
        for k in ti..=text.len() {
            if glob_match_inner(pat, text, pi + 1, k) {
                return true;
            }
        }
        return false;
    }

    if pat[pi] == '?' {
        if ti < text.len() {
            return glob_match_inner(pat, text, pi + 1, ti + 1);
        }
        return false;
    }

    // Literal match
    if ti < text.len() && pat[pi] == text[ti] {
        return glob_match_inner(pat, text, pi + 1, ti + 1);
    }
    false
}

#[cfg(all(feature = "walkdir", unix))]
fn executable_flag(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(all(feature = "walkdir", not(unix)))]
fn executable_flag(_path: &Path) -> bool {
    false
}

/// Walk a project root bounded by policy and produce an `InventoryReport`.
pub fn inventory_repo(root: &Path, policy: &AuditPolicy) -> Result<InventoryReport> {
    if !root.is_dir() {
        return Err(RumilError::PathRejected(format!(
            "root is not a readable directory: {}",
            root.display()
        )));
    }
    let project_root = root.canonicalize().map_err(RumilError::Io)?;
    let relative_root = validate_relative_root(&policy.root_relative)?;
    let scan_root = project_root
        .join(relative_root)
        .canonicalize()
        .map_err(RumilError::Io)?;
    if !scan_root.starts_with(&project_root) || !scan_root.is_dir() {
        return Err(RumilError::PathRejected(
            "root policy escapes the canonical project root".to_string(),
        ));
    }
    let config = InventoryConfig {
        project_root,
        scan_root,
        policy: policy.clone(),
    };
    let walker = TreeWalker::new(config);
    walker.walk()
}

fn validate_relative_root(root_relative: &str) -> Result<&Path> {
    let path = Path::new(root_relative);
    if root_relative.is_empty() || path.is_absolute() {
        return Err(RumilError::PathRejected(
            "root_relative must be a non-empty relative path".to_string(),
        ));
    }
    if path.components().any(|component| {
        matches!(
            component,
            std::path::Component::ParentDir
                | std::path::Component::RootDir
                | std::path::Component::Prefix(_)
        )
    }) {
        return Err(RumilError::PathRejected(
            "root_relative may not escape the project root".to_string(),
        ));
    }
    Ok(path)
}
