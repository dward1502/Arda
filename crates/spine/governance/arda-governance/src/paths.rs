//! Explicit filesystem roots for governance configuration and evidence.
//!
//! The crate never infers the repository root from its build-time manifest path.
//! Applications choose a base directory and may override individual paths when
//! needed.

use std::path::{Path, PathBuf};

/// Repository/application-relative paths used by governance loaders and writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GovernancePaths {
    base_dir: PathBuf,
}

impl GovernancePaths {
    pub const CHAIN_CONFIG: &'static str = "config/governance/chains.toml";
    pub const PHILOSOPHER_PROFILES: &'static str = "config/governance/philosophers.toml";
    pub const BACON_LITE_MACHINE_LOG: &'static str = "data/governance/bacon_lite.jsonl";
    pub const BACON_LITE_HUMAN_LOG: &'static str = "docs/operator/library/governance/bacon_lite.md";

    /// Resolve governance paths relative to an application-supplied base directory.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    /// Resolve governance paths relative to the process working directory.
    pub fn from_current_dir() -> std::io::Result<Self> {
        std::env::current_dir().map(Self::new)
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.base_dir.join(path)
        }
    }

    pub fn chain_config(&self) -> PathBuf {
        self.resolve(Self::CHAIN_CONFIG)
    }

    pub fn philosopher_profiles(&self) -> PathBuf {
        self.resolve(Self::PHILOSOPHER_PROFILES)
    }

    pub fn bacon_lite_machine_log(&self) -> PathBuf {
        self.resolve(Self::BACON_LITE_MACHINE_LOG)
    }

    pub fn bacon_lite_human_log(&self) -> PathBuf {
        self.resolve(Self::BACON_LITE_HUMAN_LOG)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_every_default_from_the_injected_base() {
        let paths = GovernancePaths::new("/tmp/arda-instance");

        assert_eq!(
            paths.chain_config(),
            PathBuf::from("/tmp/arda-instance/config/governance/chains.toml")
        );
        assert_eq!(
            paths.philosopher_profiles(),
            PathBuf::from("/tmp/arda-instance/config/governance/philosophers.toml")
        );
        assert_eq!(
            paths.bacon_lite_machine_log(),
            PathBuf::from("/tmp/arda-instance/data/governance/bacon_lite.jsonl")
        );
    }

    #[test]
    fn preserves_an_explicit_absolute_override() {
        let paths = GovernancePaths::new("/unused");
        assert_eq!(
            paths.resolve("/var/lib/arda/governance.jsonl"),
            PathBuf::from("/var/lib/arda/governance.jsonl")
        );
    }
}
