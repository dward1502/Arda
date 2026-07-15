use std::path::{Path, PathBuf};

pub(super) fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
