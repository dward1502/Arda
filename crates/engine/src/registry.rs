//! Data-driven service registry for the Arda daemon.
//!
//! Services (launcher, HUD, the [`manwe`] gateway, later others) are described
//! declaratively in a `services.toml` file instead of being hardcoded in
//! `main.rs`. To add or remove an app, edit the toml in one place — no Rust
//! source changes required.
//!
//! [`manwe`]: note the gateway's reserved local port `7171` is pinned in
//! `services.toml`; per the frozen refactor contract nothing else may claim it.

use std::path::PathBuf;

use serde::Deserialize;
use tracing::{debug, warn};

use crate::supervisor::Service;

/// One entry in `services.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceSpec {
    /// Stable identifier (e.g. `"arda-launcher"`, `"manwe"`).
    pub name: String,
    /// Directory candidates (relative to the repo root) to search for the exe.
    #[serde(default)]
    pub dirs: Vec<String>,
    /// Executable name candidates (tried in order, first that exists wins).
    #[serde(default)]
    pub names: Vec<String>,
    /// Extra args passed to the spawned process.
    #[serde(default)]
    pub args: Vec<String>,
    /// If true, an absent exe is a hard error (stops the daemon).
    /// If false (default), the service is skipped with a warning.
    #[serde(default)]
    pub required: bool,
    /// If true, this is a UI surface and is excluded when `--no-ui` is passed.
    #[serde(default)]
    pub no_ui: bool,
}

/// The parsed `services.toml` document.
#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    #[serde(default)]
    pub services: Vec<ServiceSpec>,
}

impl Registry {
    /// Load and parse a registry from a toml file path.
    pub fn load(path: &std::path::Path) -> anyhow::Result<Registry> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read services file {path:?}: {e}"))?;
        let reg: Registry =
            toml::from_str(&text).map_err(|e| anyhow::anyhow!("failed to parse {path:?}: {e}"))?;
        Ok(reg)
    }

    /// Resolve the registry into concrete [`Service`]s to supervise.
    ///
    /// * `root` — repo root used to resolve relative `dirs`.
    /// * `no_ui` — when true, services flagged `no_ui = true` are dropped.
    ///
    /// Returns `(services, errors)`. A service whose exe is missing and
    /// `required = true` produces an error entry; optional missing services are
    /// skipped silently (after a warning). Callers decide whether to abort on
    /// the errors (e.g. `required` failures).
    pub fn resolve(
        &self,
        root: &std::path::Path,
        no_ui: bool,
    ) -> (Vec<Service>, Vec<anyhow::Error>) {
        let mut services = Vec::new();
        let mut errors = Vec::new();

        for spec in &self.services {
            if no_ui && spec.no_ui {
                debug!("registry: dropping UI service '{}' (--no-ui)", spec.name);
                continue;
            }

            let exe = find_exe(root, &spec.dirs, &spec.names);
            let Some(exe) = exe else {
                if spec.required {
                    errors.push(anyhow::anyhow!(
                        "registry: required service '{}' not found (searched dirs {dirs:?} names {names:?} under {root:?})",
                        spec.name,
                        dirs = spec.dirs,
                        names = spec.names,
                        root = root
                    ));
                } else {
                    warn!(
                        "registry: skipping optional service '{}' — exe not found",
                        spec.name
                    );
                }
                continue;
            };

            services.push(Service {
                name: Box::leak(spec.name.clone().into_boxed_str()),
                exe,
                args: spec.args.clone(),
                required: spec.required,
            });
        }

        (services, errors)
    }
}

/// Search `dirs` (relative to `root`) cross-joined with `names`, returning the
/// first existing path. Mirrors the launcher/HUD discovery logic that used to
/// live in `main.rs` but now reads from data.
fn find_exe(root: &std::path::Path, dirs: &[String], names: &[String]) -> Option<PathBuf> {
    for dir in dirs {
        for name in names {
            let p = root.join(dir).join(name);
            if p.exists() {
                return Some(p);
            }
        }
    }
    None
}
