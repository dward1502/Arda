// sigil: REPAIR
use std::path::PathBuf;

pub(super) fn default_provider_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_CHARON_PROVIDER_CONFIG") {
        return PathBuf::from(path);
    }
    crate::adaptive::service::paths::arda_root().join("config/governance/charon.providers.toml")
}

pub(super) fn default_bootstrap_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_FLEET_BOOTSTRAP_STATE") {
        return PathBuf::from(path);
    }
    super::paths::arda_root().join("core/state/fleet_bootstrap.json")
}