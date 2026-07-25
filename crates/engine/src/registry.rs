//! Data-driven service registry for the Arda daemon.
//!
//! Services are declared as `[[service]]` records in the workspace
//! `services.toml`. Each record names the command, arguments, working
//! directory, UI classification, and health contract used by operators.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use tracing::{debug, warn};

use crate::supervisor::Service;

/// Process command declared for one service.
#[derive(Debug, Clone, Deserialize)]
pub struct StartSpec {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub cwd: Option<String>,
}

/// Operator-facing health contract declared for one service.
#[derive(Debug, Clone, Deserialize)]
pub struct HealthSpec {
    pub url: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub interval_secs: Option<u64>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

impl HealthSpec {
    /// Return the complete probe URL, accepting either a complete URL in
    /// `url` or the older split `url` + `path` representation.
    pub fn probe_url(&self) -> String {
        match self.path.as_deref() {
            Some(path) => format!("{}{}", self.url.trim_end_matches('/'), path),
            None => self.url.clone(),
        }
    }
}

/// One `[[service]]` entry in `services.toml`.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceSpec {
    /// Stable identifier (for example `manwe`).
    pub name: String,
    /// Source/build location retained as operator metadata.
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub tags: Vec<String>,
    pub start: StartSpec,
    #[serde(default)]
    pub health: Option<HealthSpec>,
}

impl ServiceSpec {
    fn is_ui(&self) -> bool {
        self.tags.iter().any(|tag| tag == "ui")
    }
}

/// The parsed `services.toml` document.
#[derive(Debug, Clone, Deserialize)]
pub struct Registry {
    /// Keep the public plural field used by engine callers while mapping it to
    /// the manifest's canonical singular TOML array name.
    #[serde(rename = "service", alias = "services", default)]
    pub services: Vec<ServiceSpec>,
}

impl Registry {
    /// Load, parse, and validate a registry from a TOML file path.
    pub fn load(path: &Path) -> anyhow::Result<Registry> {
        let text = std::fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("failed to read services file {path:?}: {e}"))?;
        let reg: Registry =
            toml::from_str(&text).map_err(|e| anyhow::anyhow!("failed to parse {path:?}: {e}"))?;
        reg.validate(path)?;
        Ok(reg)
    }

    fn validate(&self, path: &Path) -> anyhow::Result<()> {
        if self.services.is_empty() {
            anyhow::bail!("services registry {path:?} contains no [[service]] entries");
        }
        let mut names = std::collections::HashSet::new();
        for spec in &self.services {
            if !names.insert(spec.name.as_str()) {
                anyhow::bail!(
                    "services registry {path:?} contains duplicate service '{}'",
                    spec.name
                );
            }
            if spec.required && spec.optional {
                anyhow::bail!(
                    "service '{}' cannot be both required and optional",
                    spec.name
                );
            }
            if spec.start.command.trim().is_empty() {
                anyhow::bail!("service '{}' has an empty start command", spec.name);
            }
        }
        Ok(())
    }

    /// Resolve manifest commands and working directories into concrete
    /// processes for the supervisor.
    pub fn resolve(&self, root: &Path, no_ui: bool) -> (Vec<Service>, Vec<anyhow::Error>) {
        let mut services = Vec::new();
        let mut errors = Vec::new();

        for spec in &self.services {
            if no_ui && spec.is_ui() {
                debug!("registry: dropping UI service '{}' (--no-ui)", spec.name);
                continue;
            }

            let exe = resolve_command(root, &spec.start.command);
            let Some(exe) = exe else {
                let error = anyhow::anyhow!(
                    "registry: service '{}' command '{}' was not found",
                    spec.name,
                    spec.start.command
                );
                if spec.required {
                    errors.push(error);
                } else {
                    warn!("{error}");
                }
                continue;
            };

            let cwd = spec.start.cwd.as_deref().map(|cwd| root.join(cwd));
            if let Some(missing) = cwd.as_ref().filter(|cwd| !cwd.is_dir()) {
                let error = anyhow::anyhow!(
                    "registry: service '{}' working directory does not exist: {}",
                    spec.name,
                    missing.display()
                );
                if spec.required {
                    errors.push(error);
                } else {
                    warn!("{error}");
                }
                continue;
            }

            services.push(Service {
                name: Box::leak(spec.name.clone().into_boxed_str()),
                exe,
                args: spec.start.args.clone(),
                cwd,
                required: spec.required,
            });
        }

        (services, errors)
    }
}

fn resolve_command(root: &Path, command: &str) -> Option<PathBuf> {
    let command_path = Path::new(command);
    if command_path.is_absolute() {
        return command_path.is_file().then(|| command_path.to_path_buf());
    }
    if command_path.components().count() > 1 {
        let path = root.join(command_path);
        return path.is_file().then_some(path);
    }
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(command))
            .find(|candidate| candidate.is_file())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_registry_declares_canonical_manwe_process() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../services.toml");
        let registry = Registry::load(&manifest).expect("workspace services.toml loads");

        assert_eq!(registry.services.len(), 3);
        let manwe = registry
            .services
            .iter()
            .find(|service| service.name == "manwe")
            .expect("manwe service is registered");
        assert!(manwe.required);
        assert_eq!(manwe.start.command, "cargo");
        assert_eq!(
            manwe.start.args,
            ["run", "-p", "manwe", "--", "--config", "manwe.toml"]
        );
        assert_eq!(
            manwe.health.as_ref().map(HealthSpec::probe_url).as_deref(),
            Some("http://127.0.0.1:7171/healthz")
        );
    }

    #[test]
    fn no_ui_keeps_manwe_and_drops_ui_services() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let registry =
            Registry::load(&root.join("services.toml")).expect("workspace services.toml loads");
        let (services, errors) = registry.resolve(&root, true);

        assert!(
            errors.is_empty(),
            "unexpected resolution errors: {errors:?}"
        );
        assert_eq!(services.len(), 1);
        assert_eq!(services[0].name, "manwe");
        assert_eq!(services[0].cwd.as_deref(), Some(root.as_path()));
    }

    #[test]
    fn empty_registry_is_rejected_instead_of_silently_supervising_nothing() {
        let registry: Registry = toml::from_str("services = []").expect("empty registry parses");
        let error = registry
            .validate(Path::new("services.toml"))
            .expect_err("empty registry must fail validation");
        assert!(
            error
                .to_string()
                .contains("contains no [[service]] entries")
        );
    }

    #[test]
    fn missing_command_for_required_service_is_reported_as_error() {
        let registry: Registry = toml::from_str(
            r#"[[service]]
name = "ghost"
required = true
start.command = "definitely-missing-binary"
"#,
        )
        .expect("registry parses");

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let (services, errors) = registry.resolve(root, false);

        assert!(services.is_empty());
        assert_eq!(errors.len(), 1);
        let message = errors[0].to_string();
        assert!(message.contains("ghost"));
        assert!(message.contains("command 'definitely-missing-binary' was not found"));
    }

    #[test]
    fn missing_command_for_optional_service_drops_service_with_no_error() {
        let registry: Registry = toml::from_str(
            r#"[[service]]
name = "ghost"
command = "definitely-missing-binary"
start.command = "definitely-missing-binary"
optional = true
"#,
        )
        .expect("registry parses");

        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let (services, errors) = registry.resolve(root, false);

        assert!(errors.is_empty());
        assert!(services.is_empty());
    }
}
