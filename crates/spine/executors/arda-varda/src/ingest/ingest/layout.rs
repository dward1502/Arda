// sigil: REPAIR
//
// Store layout and bootstrap helpers: default Athena home resolution,
// human/machine library roots, and the permission-aware fallback path
// used when the primary store root cannot be opened.

use arda_core::error::{ArdaError, Result};
use std::path::{Path, PathBuf};

use super::AthenaStore;

pub(crate) fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl AthenaStore {
    pub fn default_path() -> PathBuf {
        if let Ok(custom) = std::env::var("ARDA_ATHENA_HOME") {
            return PathBuf::from(custom);
        }
        arda_root().join("data/athena")
    }

    pub fn from_default_or_workspace_fallback() -> Result<Self> {
        let primary = Self::default_path();
        match Self::new(&primary) {
            Ok(store) => Ok(store),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                let fallback = arda_root().join("data").join("athena");
                Self::new(fallback)
            }
        }
    }
}

pub(super) fn human_library_root() -> PathBuf {
    std::env::var("ARDA_ATHENA_HUMAN_LIBRARY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| arda_root().join("docs/operator/library/athena"))
}

pub(super) fn machine_library_root() -> PathBuf {
    std::env::var("ARDA_ATHENA_MACHINE_LIBRARY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| arda_root().join("data/knowledge/athena"))
}

fn is_permission_error(err: &ArdaError) -> bool {
    matches!(
        err,
        ArdaError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}
