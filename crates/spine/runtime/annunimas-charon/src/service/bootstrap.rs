pub(super) use super::bootstrap_overlay::load_providers_from_config;
#[cfg(test)]
pub(super) use super::bootstrap_overlay::{
    fleet_bootstrap_is_fresh, merge_with_default_providers, FleetBootstrapFile,
};
pub(super) use super::bootstrap_runtime::collect_package_runtime_signals;
use std::path::PathBuf;

pub(super) fn default_provider_config_path() -> PathBuf {
    if let Ok(path) = std::env::var("ANNUNIMAS_CHARON_PROVIDER_CONFIG") {
        return PathBuf::from(path);
    }
    super::paths::annunimas_root().join("config/charon.providers.toml")
}

pub(super) fn default_bootstrap_state_path() -> PathBuf {
    if let Ok(path) = std::env::var("ANNUNIMAS_FLEET_BOOTSTRAP_STATE") {
        return PathBuf::from(path);
    }
    super::paths::annunimas_root().join("core/state/fleet_bootstrap.json")
}
