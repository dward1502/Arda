use std::path::{Path, PathBuf};

pub(super) fn arda_root() -> PathBuf {
    crate::config::arda_root()
}

pub(super) fn bacon_lite_base(service_root: &Path) -> PathBuf {
    #[cfg(test)]
    {
        // Unit routes must keep generated governance evidence inside their
        // injected temporary service root.
        service_root.to_path_buf()
    }
    #[cfg(not(test))]
    {
        let _ = service_root;
        arda_root()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bacon_lite_test_output_uses_the_injected_service_root() {
        assert_eq!(
            bacon_lite_base(Path::new("/tmp/manwe-test")),
            PathBuf::from("/tmp/manwe-test")
        );
    }
}
