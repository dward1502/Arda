//! Bounded tree-entry types for the generic inventory inventory.
//!
//! These are the normalized records produced by the walker. They carry only
//! project-relative POSIX paths; absolute host paths never leave the local
//! operator log.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::contracts::{FileRecord, FileRecordKind, RedactionState};

/// A single entry discovered during a bounded walk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeEntry {
    pub relative_path: String,
    pub kind: TreeEntryKind,
    pub size_bytes: Option<u64>,
    pub content_sha256: Option<String>,
    pub mime_or_extension: Option<String>,
    pub executable: Option<bool>,
    pub symlink_target_relative: Option<String>,
    pub redaction_state: RedactionState,
    pub observed_at_utc: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TreeEntryKind {
    File,
    Directory,
    Symlink,
    Unreadable,
    Excluded,
}

impl TreeEntry {
    /// Convert a tree entry into a bounded `FileRecord` for the audit packet.
    /// No raw content is embedded — only metadata and digests.
    pub fn to_file_record(&self) -> FileRecord {
        FileRecord {
            path: self.relative_path.clone(),
            kind: match self.kind {
                TreeEntryKind::File => FileRecordKind::File,
                TreeEntryKind::Directory => FileRecordKind::Directory,
                TreeEntryKind::Symlink => FileRecordKind::Symlink,
                TreeEntryKind::Unreadable => FileRecordKind::Unreadable,
                TreeEntryKind::Excluded => FileRecordKind::Excluded,
            },
            size_bytes: self.size_bytes,
            content_sha256: self.content_sha256.clone(),
            mime_or_extension: self.mime_or_extension.clone(),
            executable: self.executable,
            symlink_target_digest: None,
            source_excerpt_ids: Vec::new(),
            redaction_state: self.redaction_state,
            observed_at_utc: self.observed_at_utc,
        }
    }
}
