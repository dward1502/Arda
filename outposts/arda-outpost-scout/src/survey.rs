use arda_outpost_protocol::{ObservationClassification, ObservationScope, OutpostObservation};
use std::path::Path;
use walkdir::WalkDir;

use crate::observation::CratePackage;
use crate::{observation::CrateStatus, CrateObservation, Result, ScoutError, SurveyReport};

pub fn survey_repo(root: impl AsRef<Path>) -> Result<SurveyReport> {
    let root = root.as_ref();
    if !root.exists() {
        return Err(ScoutError::InvalidPath(root.display().to_string()));
    }

    let mut observations = Vec::new();
    for subtree in ["crates", "apps", "outposts"] {
        let scan_root = root.join(subtree);
        if !scan_root.exists() {
            continue;
        }
        for entry in WalkDir::new(scan_root)
            .max_depth(6)
            .into_iter()
            .filter_entry(|entry| !is_ignored_directory(entry.path()))
            .filter_map(|item| item.ok())
        {
            let path = entry.path();
            if entry.file_type().is_dir() && path.join("Cargo.toml").exists() {
                observations.push(inspect_path(path)?);
            }
        }
    }

    Ok(SurveyReport::new("node-pi5-warden", observations))
}

fn is_ignored_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| matches!(name, ".git" | "node_modules" | "target"))
        .unwrap_or(false)
}

fn parse_cargo_manifest(path: &Path) -> Result<Option<CratePackage>> {
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(cargo_toml)?;
    let doc: toml::Value = content.parse().map_err(|err| {
        ScoutError::Protocol(format!("invalid Cargo.toml at {}: {}", path.display(), err))
    })?;
    let package = doc.get("package").ok_or_else(|| {
        ScoutError::Protocol(format!("missing [package] section at {}", path.display()))
    })?;

    Ok(Some(CratePackage {
        name: package
            .get("name")
            .and_then(|value: &toml::Value| value.as_str())
            .unwrap_or("")
            .to_string(),
        version: package
            .get("version")
            .and_then(|value: &toml::Value| value.as_str())
            .unwrap_or("")
            .to_string(),
        description: package
            .get("description")
            .and_then(|value: &toml::Value| value.as_str())
            .map(|descriptor: &str| descriptor.to_string()),
    }))
}

fn inspect_path(path: &Path) -> Result<CrateObservation> {
    let manifest = parse_cargo_manifest(path)?;
    let package = manifest.unwrap_or(CratePackage {
        name: path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string(),
        version: String::new(),
        description: None,
    });
    let status = detect_status(path, &package);

    Ok(CrateObservation {
        path: path.display().to_string(),
        name: package.name,
        purpose: package.description,
        status,
        key_entrypoints: detect_entrypoints(path),
        test_surface: detect_tests(path),
        dependencies: detect_dependencies(path)?,
        dev_patterns: Vec::new(),
        observed_at: chrono::Utc::now(),
    })
}

fn detect_status(path: &Path, package: &CratePackage) -> CrateStatus {
    if package.name.trim().is_empty() && package.version.trim().is_empty() {
        return CrateStatus::Shell;
    }
    let has_src_lib = path.join("src/lib.rs").exists();
    let has_src_bin = path.join("src/main.rs").exists();
    if has_src_lib || has_src_bin {
        CrateStatus::Active
    } else {
        CrateStatus::Shell
    }
}

fn detect_entrypoints(path: &Path) -> Vec<String> {
    let mut entrypoints = Vec::new();
    if let Ok(entries) = std::fs::read_dir(path.join("src")) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "main.rs" {
                entrypoints.push("src/main.rs".to_string());
            } else if name == "lib.rs" {
                entrypoints.push("src/lib.rs".to_string());
            } else if name.ends_with(".rs") && !name.contains("test") {
                entrypoints.push(format!("src/{}", name));
            }
        }
    }
    entrypoints.truncate(8);
    entrypoints
}

fn detect_tests(path: &Path) -> Vec<String> {
    let mut tests = Vec::new();
    for entry in WalkDir::new(path)
        .max_depth(2)
        .into_iter()
        .filter_map(|item| item.ok())
    {
        let candidate = entry.path();
        if candidate.starts_with(path.join("tests"))
            || entry.file_name().to_string_lossy().contains("test")
            || entry
                .path()
                .extension()
                .map(|ext| ext == "py")
                .unwrap_or(false)
        {
            tests.push(candidate.display().to_string());
        }
    }
    tests.truncate(12);
    tests
}

fn detect_dependencies(path: &Path) -> Result<Vec<String>> {
    let cargo_toml = path.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(cargo_toml)?;
    let doc: toml::Value = content.parse().map_err(|err| {
        ScoutError::Protocol(format!("invalid Cargo.toml at {}: {}", path.display(), err))
    })?;
    let mut values = Vec::new();
    if let Some(table) = doc.get("dependencies").and_then(|table| table.as_table()) {
        for key in table.keys() {
            values.push(key.to_string());
        }
    }
    Ok(values)
}

pub fn build_observation(source: &str, payload: serde_json::Value) -> OutpostObservation {
    OutpostObservation::new(
        source,
        ObservationScope::Crates,
        ObservationClassification::DerivedEstimate,
        crate::AuthorityClass::Advisory,
        payload,
    )
    .with_confidence(0.8)
    .with_provenance("arda-outpost-scout://survey")
}
