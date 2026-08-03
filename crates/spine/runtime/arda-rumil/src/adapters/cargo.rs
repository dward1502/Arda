use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use cargo_metadata::MetadataCommand;
use serde::{Deserialize, Serialize};

use crate::adapters::{outcome, provider_allowed, ProviderAdapter};
use crate::constants::{PROVIDER_COMPLETED, PROVIDER_SKIPPED_BY_POLICY, PROVIDER_UNAVAILABLE};
use crate::contracts::{AuditRequest, CapabilityOutcome};
use crate::error::{Result, RumilError};
use crate::policy::AuditPolicy;

pub const CAPABILITY: &str = "cargo_workspace";
pub const PROVIDER_ID: &str = "rumil.cargo_metadata.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoWorkspaceSnapshot {
    pub workspace_root_relative: String,
    pub packages: Vec<CargoPackageRecord>,
    pub dependency_edges: Vec<CargoDependencyEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoPackageRecord {
    pub name: String,
    pub version: String,
    pub manifest_path_relative: String,
    pub targets: Vec<CargoTargetRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoTargetRecord {
    pub name: String,
    pub kinds: Vec<String>,
    pub crate_types: Vec<String>,
    pub source_path_relative: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoDependencyEdge {
    pub package: String,
    pub dependency: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CargoAdapter;

impl CargoAdapter {
    pub fn inspect(&self, project_root: &Path) -> Result<CargoWorkspaceSnapshot> {
        let canonical_root = canonical_directory(project_root)?;
        let manifest = canonical_root.join("Cargo.toml");
        if !manifest.is_file() {
            return Err(RumilError::ProviderUnavailable(
                "Cargo.toml was not found at the selected project root".to_string(),
            ));
        }

        let metadata = MetadataCommand::new()
            .manifest_path(&manifest)
            .no_deps()
            .exec()
            .map_err(|error| RumilError::ProviderFailed(error.to_string()))?;
        let resolved_metadata = MetadataCommand::new()
            .manifest_path(&manifest)
            .exec()
            .map_err(|error| RumilError::ProviderFailed(error.to_string()))?;
        let workspace_ids: HashSet<_> = metadata.workspace_members.iter().cloned().collect();
        let package_names: HashMap<_, _> = metadata
            .packages
            .iter()
            .chain(resolved_metadata.packages.iter())
            .map(|package| (package.id.clone(), package.name.clone()))
            .collect();

        let mut packages = metadata
            .packages
            .iter()
            .filter(|package| workspace_ids.contains(&package.id))
            .map(|package| {
                let manifest_path_relative =
                    relative_path(&canonical_root, package.manifest_path.as_std_path())?;
                let mut targets = package
                    .targets
                    .iter()
                    .map(|target| {
                        Ok(CargoTargetRecord {
                            name: target.name.clone(),
                            kinds: target.kind.iter().map(ToString::to_string).collect(),
                            crate_types: target
                                .crate_types
                                .iter()
                                .map(ToString::to_string)
                                .collect(),
                            source_path_relative: relative_path(
                                &canonical_root,
                                target.src_path.as_std_path(),
                            )?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                targets.sort_by(|left, right| {
                    left.source_path_relative.cmp(&right.source_path_relative)
                });
                Ok(CargoPackageRecord {
                    name: package.name.clone(),
                    version: package.version.to_string(),
                    manifest_path_relative,
                    targets,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        packages.sort_by(|left, right| {
            left.manifest_path_relative
                .cmp(&right.manifest_path_relative)
        });

        let mut dependency_edges = Vec::new();
        if let Some(resolve) = resolved_metadata.resolve {
            for node in resolve.nodes {
                if !workspace_ids.contains(&node.id) {
                    continue;
                }
                let Some(package) = package_names.get(&node.id) else {
                    continue;
                };
                for dependency in node.dependencies {
                    if let Some(dependency_name) = package_names.get(&dependency) {
                        dependency_edges.push(CargoDependencyEdge {
                            package: package.clone(),
                            dependency: dependency_name.clone(),
                        });
                    }
                }
            }
        }
        dependency_edges.sort_by(|left, right| {
            (&left.package, &left.dependency).cmp(&(&right.package, &right.dependency))
        });
        dependency_edges.dedup();

        Ok(CargoWorkspaceSnapshot {
            workspace_root_relative: relative_path(
                &canonical_root,
                metadata.workspace_root.as_std_path(),
            )?,
            packages,
            dependency_edges,
        })
    }
}

impl ProviderAdapter for CargoAdapter {
    fn capability(&self) -> &str {
        CAPABILITY
    }

    fn provider_id(&self) -> &str {
        PROVIDER_ID
    }

    fn run(
        &self,
        _request: &AuditRequest,
        policy: &AuditPolicy,
        project_root: &Path,
    ) -> Result<(serde_json::Value, CapabilityOutcome)> {
        if !provider_allowed(policy, PROVIDER_ID) {
            return Ok((
                serde_json::Value::Null,
                outcome(
                    CAPABILITY,
                    PROVIDER_ID,
                    PROVIDER_SKIPPED_BY_POLICY,
                    Some("provider is not allowlisted".to_string()),
                ),
            ));
        }
        match self.inspect(project_root) {
            Ok(snapshot) => Ok((
                serde_json::to_value(snapshot)?,
                outcome(CAPABILITY, PROVIDER_ID, PROVIDER_COMPLETED, None),
            )),
            Err(RumilError::ProviderUnavailable(detail)) => Ok((
                serde_json::Value::Null,
                outcome(CAPABILITY, PROVIDER_ID, PROVIDER_UNAVAILABLE, Some(detail)),
            )),
            Err(error) => Err(error),
        }
    }
}

fn canonical_directory(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        return Err(RumilError::PathRejected(
            "adapter root is not a directory".to_string(),
        ));
    }
    path.canonicalize().map_err(RumilError::Io)
}

fn relative_path(root: &Path, path: &Path) -> Result<String> {
    let relative = path.strip_prefix(root).map_err(|_| {
        RumilError::PathRejected("Cargo metadata path escaped the project root".to_string())
    })?;
    let rendered = relative
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(if rendered.is_empty() {
        ".".to_string()
    } else {
        rendered
    })
}
