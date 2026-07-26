use anyhow::{Context, Result};
use arda_contract_registry::registry::ContractRegistry;
use serde::Serialize;
use std::path::{Path, PathBuf};

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

pub fn load_registry(root: &Path) -> Result<ContractRegistry> {
    let manifest_path = root.join("core/state/contract_registry.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {manifest_path:?}"))?;
    Ok(serde_json::from_str(&raw)?)
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
    fn registry_loads_from_workspace() {
        let root = Path::new(".");
        let registry = load_registry(root).expect("registry loads");
        assert_eq!(registry.schema_version, "arda.contract-registry.v1");
    }

    #[test]
    fn check_registry_returns_gate_status() {
        let root = Path::new(".");
        let result = check_registry(root).expect("registry check");
        assert!(["pass", "warn", "fail"].contains(&result.gate_status.as_str()));
        assert_eq!(result.track_count, 4);
    }
}
