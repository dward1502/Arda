//! Arda launcher backend.
//!
//! Thin Tauri + app harness surface over the Arda spine.
//!
//! The onboarding flow is implemented under `onboarding/`, and exposed to
//! the frontend via Tauri commands below.

pub mod lifecycle;
pub mod onboarding;

use arda_contract_registry::registry::ContractRegistry;
use serde::Serialize;
use std::path::PathBuf;

fn resolve_root(root: Option<String>) -> PathBuf {
    let seed = match root {
        Some(r) if !r.trim().is_empty() => PathBuf::from(r.trim()),
        _ => std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    };

    if seed
        .join("core")
        .join("state")
        .join("contract_registry.json")
        .exists()
    {
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

    let registry = match ContractRegistry::load_from_root(&root) {
        Ok(registry) => registry,
        Err(err) => {
            payload.error = Some(err.to_string());
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
    payload.gate_status = if tracks.iter().all(|t| t.status == "active")
        && tracks
            .iter()
            .all(|t| !t.source_modules.is_empty() || !t.receipt_stores.is_empty())
    {
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
        &profile, &root_path,
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

#[tauri::command]
fn first_run_status(root: Option<String>) -> Result<crate::onboarding::FirstRunProjection, String> {
    let root_path = resolve_root(root);
    crate::onboarding::build_first_run_projection(&root_path).map_err(|error| error.to_string())
}

#[derive(Debug, Serialize)]
struct ReleaseIdentity {
    contract: &'static str,
    version: &'static str,
    supported_profile: &'static str,
}

#[tauri::command]
fn release_identity() -> ReleaseIdentity {
    ReleaseIdentity {
        contract: "arda.launcher-release-identity.v1",
        version: env!("CARGO_PKG_VERSION"),
        supported_profile: "bluefin-lts-10-x86_64",
    }
}

#[cfg(target_os = "linux")]
fn needs_nvidia_wayland_explicit_sync_guard(
    wayland_display_present: bool,
    nvidia_driver_present: bool,
    override_present: bool,
) -> bool {
    wayland_display_present && nvidia_driver_present && !override_present
}

#[cfg(target_os = "linux")]
fn configure_linux_graphics_environment() {
    let wayland_display_present = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let nvidia_driver_present = std::path::Path::new("/proc/driver/nvidia/version").is_file()
        || std::path::Path::new("/sys/module/nvidia").exists();
    let override_present = std::env::var_os("__NV_DISABLE_EXPLICIT_SYNC").is_some();

    if needs_nvidia_wayland_explicit_sync_guard(
        wayland_display_present,
        nvidia_driver_present,
        override_present,
    ) {
        // WebKitGTK can terminate with Wayland protocol error 71 on NVIDIA's
        // explicit-sync path. Limit the guard to that host combination and
        // preserve any operator-provided override.
        std::env::set_var("__NV_DISABLE_EXPLICIT_SYNC", "1");
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    configure_linux_graphics_environment();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            crate::lifecycle::commands::lifecycle_status,
            crate::lifecycle::commands::start_arda_session,
            crate::lifecycle::commands::stop_arda_session,
            crate::lifecycle::commands::recover_component,
            crate::lifecycle::commands::launch_native_hud,
            crate::lifecycle::commands::hud_status,
            registry_status,
            readiness_status,
            service_plan_status,
            first_run_status,
            release_identity
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod release_identity_tests {
    use super::*;

    #[test]
    fn release_identity_reports_compiled_package_version() {
        let identity = release_identity();
        assert_eq!(identity.contract, "arda.launcher-release-identity.v1");
        assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(identity.supported_profile, "bluefin-lts-10-x86_64");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn graphics_guard_is_limited_to_unoverridden_nvidia_wayland_sessions() {
        assert!(needs_nvidia_wayland_explicit_sync_guard(true, true, false));
        assert!(!needs_nvidia_wayland_explicit_sync_guard(
            false, true, false
        ));
        assert!(!needs_nvidia_wayland_explicit_sync_guard(
            true, false, false
        ));
        assert!(!needs_nvidia_wayland_explicit_sync_guard(true, true, true));
    }
}
