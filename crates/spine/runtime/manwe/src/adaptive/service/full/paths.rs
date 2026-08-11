use std::path::{Path, PathBuf};

pub(super) fn arda_root() -> PathBuf {
    crate::config::arda_root()
}

pub(super) fn bacon_lite_base(service_root: &Path) -> PathBuf {
    // Governance route receipts are runtime state. Keep every projection under
    // the injected service root rather than mutating tracked workspace docs.
    service_root.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bacon_lite_output_uses_the_injected_service_root() {
        assert_eq!(
            bacon_lite_base(Path::new("/tmp/manwe-test")),
            PathBuf::from("/tmp/manwe-test")
        );
    }
}
