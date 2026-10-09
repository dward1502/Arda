use anyhow::Result;
use arda_contract_registry::registry::ContractRegistry;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[cfg(test)]
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("workspace root should be resoluble")
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackCheck {
    pub track_id: String,
    pub status: String,
    pub missing_source_modules: Vec<String>,
    pub missing_receipt_stores: Vec<String>,
    pub missing_cli_verbs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistryCheckResult {
    pub schema_version: String,
    pub track_count: usize,
    pub gate_status: String,
    pub track_checks: Vec<TrackCheck>,
    pub checked_at_utc: String,
}

/// Installed startup consumes metadata, not portfolio implementation or prior receipts.
pub fn startup_blockers(registry: &ContractRegistry) -> Vec<String> {
    let mut blockers = Vec::new();
    if registry.schema_version != "arda.contract-registry.v1" {
        blockers.push("Unsupported contract registry schema".into());
    }
    let required: Vec<_> = registry.tracks.iter()
        .filter(|t| t.track_id == "arda-ecosystem-standard-track-5-workbench").collect();
    if required.len() != 1 {
        blockers.push("Require exactly one Workbench track declaration".into());
        return blockers;
    }
    let track = required[0];
    if track.status != "active" {
        blockers.push("Required Workbench track is not active".into());
    }
    for contract in ["arda.project-contract.v1", "arda.run-graph.v1"] {
        if track.schema_versions.iter().filter(|v| v.as_str() == contract).count() != 1 {
            blockers.push(format!("Require exactly one Workbench declaration of {contract}"));
        }
    }
    blockers
}

pub fn load_registry(root: &Path) -> Result<ContractRegistry> {
    Ok(ContractRegistry::load_from_root(root)?)
}

pub fn check_registry(root: &Path) -> Result<RegistryCheckResult> {
    let registry = load_registry(root)?;
    let mut track_checks = Vec::new();

    for track in &registry.tracks {
        let missing_source_modules: Vec<String> = track
            .source_modules
            .iter()
            .filter(|module| !resolve_module(root, module).exists())
            .cloned()
            .collect();

        let mut missing_receipt_stores = Vec::new();
        for store in &track.receipt_stores {
            let resolved = resolve_store(root, store);
            if !resolved.exists() {
                missing_receipt_stores.push(store.clone());
            }
        }

        let status = if missing_source_modules.is_empty() && missing_receipt_stores.is_empty() {
            "pass".into()
        } else if missing_source_modules.is_empty() {
            "warn".into()
        } else {
            "fail".into()
        };

        track_checks.push(TrackCheck {
            track_id: track.track_id.clone(),
            status,
            missing_source_modules,
            missing_receipt_stores,
            missing_cli_verbs: Vec::new(),
        });
    }

    let gate_status = if track_checks.iter().all(|t| t.status == "pass") {
        "pass"
    } else if track_checks.iter().any(|t| t.status == "fail") {
        "fail"
    } else {
        "warn"
    };

    let checked_at_utc = chrono::Utc::now().to_rfc3339();

    Ok(RegistryCheckResult {
        schema_version: registry.schema_version,
        track_count: registry.tracks.len(),
        gate_status: gate_status.into(),
        track_checks,
        checked_at_utc,
    })
}

pub fn registry_track_ids(root: &Path) -> Result<Vec<String>> {
    Ok(load_registry(root)?
        .track_ids()
        .into_iter()
        .map(|s| s.into())
        .collect())
}

pub fn resolve_module(root: &Path, module: &str) -> PathBuf {
    let path = Path::new(module);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

pub fn resolve_store(root: &Path, store: &str) -> PathBuf {
    let path = Path::new(store);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn startup_metadata_is_distinct_from_portfolio_audit() {
        let mut registry = load_registry(&workspace_root()).unwrap();
        assert!(registry.tracks.iter().any(|t| t.status == "pending_review"));
        assert!(startup_blockers(&registry).is_empty());
        for track in &mut registry.tracks {
            track.source_modules = vec!["absent-source.rs".into()];
            track.receipt_stores = vec!["absent-history".into()];
        }
        assert!(startup_blockers(&registry).is_empty());
        let index = registry.tracks.iter().position(|t| t.track_id.ends_with("track-5-workbench")).unwrap();
        let valid = registry.clone();
        registry.tracks[index].status = "pending_review".into();
        assert!(!startup_blockers(&registry).is_empty());
        registry = valid.clone();
        registry.tracks[index].schema_versions.clear();
        assert!(!startup_blockers(&registry).is_empty());
        registry = valid.clone();
        registry.tracks.push(registry.tracks[index].clone());
        assert!(!startup_blockers(&registry).is_empty());
        registry = valid.clone();
        registry.tracks.remove(index);
        assert!(!startup_blockers(&registry).is_empty());
        registry = valid;
        registry.schema_version = "unknown".into();
        assert!(!startup_blockers(&registry).is_empty());
    }

    #[test]
    fn registry_loads_from_workspace() {
        let registry = load_registry(&workspace_root()).expect("registry loads");
        assert_eq!(registry.schema_version, "arda.contract-registry.v1");
    }

    #[test]
    fn check_registry_returns_gate_status() {
        let result = check_registry(&workspace_root()).expect("registry check");
        assert!(["pass", "warn", "fail"].contains(&result.gate_status.as_str()));
        assert_eq!(result.track_count, result.track_checks.len());
        assert!(result
            .track_checks
            .iter()
            .any(|track| track.track_id == "arda-ecosystem-standard-track-5-workbench"));
    }
}
