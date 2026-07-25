use arda_contract_registry::registry::ContractRegistry;

fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .canonicalize()
        .expect("workspace root should be resolvable")
}

fn registry() -> ContractRegistry {
    let manifest =
        std::fs::read_to_string(workspace_root().join("core/state/contract_registry.json"))
            .expect("contract registry must exist after Phase A");
    serde_json::from_str(&manifest).expect("contract registry must be valid JSON after Phase A")
}

fn resolve_module_path(module: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(module);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_root().join(path)
    }
}

#[test]
fn registry_schema_version_is_pinned() {
    let registry = registry();
    assert_eq!(registry.schema_version, "arda.contract-registry.v1");
}

#[test]
fn every_track_has_source_modules() {
    let registry = registry();
    for track in &registry.tracks {
        assert!(
            !track.source_modules.is_empty(),
            "track {} must declare at least one source module",
            track.track_id
        );
        for module in &track.source_modules {
            assert!(
                resolve_module_path(module).exists(),
                "declared source module missing for track {}: {}",
                track.track_id,
                module
            );
        }
    }
}

#[test]
fn every_track_schema_version_is_present_in_a_source_or_surface_module() {
    let registry = registry();
    for track in &registry.tracks {
        assert!(
            !track.schema_versions.is_empty(),
            "track {} must declare at least one schema_version",
            track.track_id
        );
        let mut found = false;
        for module in &track.source_modules {
            let Ok(contents) = std::fs::read_to_string(resolve_module_path(module)) else {
                continue;
            };
            if track.schema_versions.iter().any(|sv| contents.contains(sv)) {
                found = true;
                break;
            }
        }
        if !found {
            for surface in &track.receipt_stores {
                let surface_path = workspace_root().join(surface);
                if !surface_path.exists() {
                    continue;
                }
                for entry in walkdir::WalkDir::new(&surface_path)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    let Ok(contents) = std::fs::read_to_string(entry.path()) else {
                        continue;
                    };
                    if track.schema_versions.iter().any(|sv| contents.contains(sv)) {
                        found = true;
                        break;
                    }
                }
            }
        }
        assert!(
            found,
            "no declared schema_version found in source modules or receipt surfaces for track {}: {:?}",
            track.track_id, track.schema_versions
        );
    }
}
