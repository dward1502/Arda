// sigil: REPAIR
//
// Store layout and bootstrap helpers: default Athena home resolution,
// human/machine library roots, and the permission-aware fallback path
// used when the primary store root cannot be opened.

use annunimas_core::error::{AnnunimasError, Result};
use std::path::{Path, PathBuf};

use super::AthenaStore;

pub(crate) fn annunimas_root() -> PathBuf {
    if let Ok(path) = std::env::var("ANNUNIMAS_ROOT") {
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
        if let Ok(custom) = std::env::var("ANNUNIMAS_ATHENA_HOME") {
            return PathBuf::from(custom);
        }
        annunimas_root().join("data/athena")
    }

    pub fn from_default_or_workspace_fallback() -> Result<Self> {
        let primary = Self::default_path();
        match Self::new(&primary) {
            Ok(store) => Ok(store),
            Err(err) => {
                if !is_permission_error(&err) {
                    return Err(err);
                }
                let fallback = annunimas_root().join("data").join("athena");
                Self::new(fallback)
            }
        }
    }
}

pub(super) fn human_library_root() -> PathBuf {
    std::env::var("ANNUNIMAS_ATHENA_HUMAN_LIBRARY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| annunimas_root().join("human/library/athena"))
}

pub(super) fn machine_library_root() -> PathBuf {
    std::env::var("ANNUNIMAS_ATHENA_MACHINE_LIBRARY_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| annunimas_root().join("data/knowledge/athena"))
}

fn is_permission_error(err: &AnnunimasError) -> bool {
    matches!(
        err,
        AnnunimasError::Ledger(ioe) if ioe.kind() == std::io::ErrorKind::PermissionDenied
    )
}
