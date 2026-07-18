// sigil: REPAIR
use std::path::PathBuf;

pub fn arda_root() -> PathBuf {
    std::env::var("ARDA_HOME")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}