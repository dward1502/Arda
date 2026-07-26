//! Arda launcher backend.
//!
//! Thin Tauri + app harness surface over the Arda spine.
//!
//! The onboarding flow is implemented under `onboarding/`, and exposed to
//! the frontend via Tauri commands below.

pub mod onboarding;

use arda_contract_registry::registry::ContractRegistry;
use serde::Serialize;
use std::path::{Path, PathBuf};

fn resolve_root(root: Option<String>) -> PathBuf {
    let seed = match root {
        Some(r) if !r.trim().is_empty() => PathBuf::from(r.trim()),
        _ => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    };

    if seed.join("core").join("state").join("contract_registry.json").exists() {
        return seed;
    }

    if seed.extension().map(|e| e == "json").unwrap_or(false) {
        if let Some(parent) = seed.parent() {
            let candidate = parent.to_path_buf();
            if candidate
                .join("core")
                .join("state")
                .join("contract_registry.json")
                .exists()
            {
                return candidate;
            }
        }
    }

    let mut current = seed.clone();
    for _ in 0..12 {
        if current
            .join("core")
            .join("state")
            .join("contract_registry.json")
            .exists()
        {
            return current;
        }
        if !current.pop() {
            break;
        }
    }

    seed
}

#[derive(Debug, Serialize)]
struct RegistryTrackView {
    track_id: String,
    title: String,
    owner: String,
    status: String,
    source_modules: Vec<String>,
    receipt_stores: Vec<String>,
}

#[derive(Debug, Serialize)]
struct RegistryStatusPayload {
    loaded: bool,
    schema_version: String,
    authority: String,
    track_count: usize,
    gate_status: String,
    tracks: Vec<RegistryTrackView>,
    checked_at_utc: String,
    error: Option<String>,
}

#[tauri::command]
fn registry_status(root: Option<String>) -> RegistryStatusPayload {
    let root = resolve_root(root);
    let mut payload = RegistryStatusPayload {
        loaded: false,
        schema_version: String::new(),
        authority: String::new(),
        track_count: 0,
        gate_status: String::new(),
        tracks: Vec::new(),
        checked_at_utc: String::new(),
        error: None,
    };

    let path = root.join("core/state/contract_registry.json");
    let registry = match std::fs::read_to_string(&path) {
        Ok(raw) => match serde_json::from_str::<ContractRegistry>(&raw) {
            Ok(registry) => registry,
            Err(err) => {
                payload.error = Some(format!(
                    "invalid registry JSON: {} at {}",
                    err,
                    path.display()
                ));
                return payload;
            }
        },
        Err(err) => {
            payload.error = Some(format!(
                "missing registry manifest: {} at {}",
                err,
                path.display()
            ));
            return payload;
        }
    };

    let mut tracks = Vec::new();
    for track in &registry.tracks {
        tracks.push(RegistryTrackView {
            track_id: track.track_id.clone(),
            title: track.title.clone(),
            owner: track.owner.clone(),
            status: track.status.clone(),
            source_modules: track.source_modules.clone(),
            receipt_stores: track.receipt_stores.clone(),
        });
    }

    payload.loaded = true;
    payload.schema_version = registry.schema_version;
    payload.authority = registry.authority;
    payload.track_count = tracks.len();
    payload.gate_status = if tracks.iter().all(|t| t.status == "active") && tracks.iter().all(|t| !t.source_modules.is_empty() || !t.receipt_stores.is_empty()) {
        "pass".into()
    } else {
        "warn".into()
    };
    payload.tracks = tracks;
    payload.checked_at_utc = chrono::Utc::now().to_rfc3339();
    payload
}

#[tauri::command]
fn readiness_status(root: Option<String>) -> Option<crate::onboarding::ReadinessProjection> {
    let root_path = resolve_root(root);
    let profile = match crate::onboarding::build_environment_profile(Some(&root_path), None, None) {
        Ok(profile) => profile,
        Err(_) => return None,
    };
    Some(crate::onboarding::build_readiness_projection(
        &profile,
        &root_path,
    ))
}

#[tauri::command]
fn service_plan_status(root: Option<String>) -> Option<crate::onboarding::types::ServicePlan> {
    let root_path = resolve_root(root);
    let profile = match crate::onboarding::build_environment_profile(Some(&root_path), None, None) {
        Ok(profile) => profile,
        Err(_) => return None,
    };
    Some(crate::onboarding::build_service_plan(&profile, &root_path))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            registry_status,
            readiness_status,
            service_plan_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
