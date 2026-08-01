use std::path::{Path, PathBuf};

/// Resolve the canonical Arda workspace root.
///
/// `ARDA_ROOT` is authoritative. `ANNUNIMAS_ROOT` remains a compatibility
/// fallback while legacy deployments migrate. Without either variable, the
/// resolver walks upward from `manifest_dir` until it finds the workspace
/// `Cargo.toml` instead of relying on a fixed crate depth.
pub fn arda_root_from(manifest_dir: impl AsRef<Path>) -> PathBuf {
    if let Some(root) = configured_root() {
        return root;
    }

    let manifest_dir = manifest_dir.as_ref();
    for ancestor in manifest_dir.ancestors() {
        if is_workspace_root(ancestor) {
            return ancestor.to_path_buf();
        }
    }

    manifest_dir.to_path_buf()
}

fn configured_root() -> Option<PathBuf> {
    ["ARDA_ROOT", "ANNUNIMAS_ROOT"]
        .into_iter()
        .find_map(|name| std::env::var_os(name).filter(|value| !value.is_empty()))
        .map(PathBuf::from)
}

fn is_workspace_root(path: &Path) -> bool {
    let Ok(manifest) = std::fs::read_to_string(path.join("Cargo.toml")) else {
        return false;
    };
    manifest.lines().any(|line| line.trim() == "[workspace]")
}

#[cfg(test)]
mod tests {
    use super::arda_root_from;

    #[test]
    fn discovers_workspace_without_assuming_crate_depth() {
        let temp = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .expect("workspace manifest");
        let crate_dir = temp.path().join("crates/spine/runtime/example");
        std::fs::create_dir_all(&crate_dir).expect("crate directory");

        assert_eq!(arda_root_from(&crate_dir), temp.path());
    }

    #[test]
    fn falls_back_to_start_when_no_workspace_exists() {
        let temp = tempfile::tempdir().expect("tempdir");
        let crate_dir = temp.path().join("standalone");
        std::fs::create_dir_all(&crate_dir).expect("crate directory");

        assert_eq!(arda_root_from(&crate_dir), crate_dir);
    }
}
