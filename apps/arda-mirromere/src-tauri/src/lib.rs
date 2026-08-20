mod display;

use arda_mirromere::{
    load_continuity_reference, project_mirromere_surface_at, LifecycleAggregateState,
    LifecycleProjectionReference, MirromereInteractionReceipt, MirromereInteractionReceiptState,
    MirromereInteractionRequest, MirromereProjectionInput, MirromereProjectionSourceMode,
};
use arda_outpost_protocol::{MirromereDisplayRole, MirromereSurfaceProjection};
use chrono::Utc;
use display::DisplayState;
use std::process::Command;
use tauri::Manager;

fn observe_lifecycle() -> LifecycleProjectionReference {
    let observed_at = Utc::now();
    let output = Command::new("systemctl")
        .args(["--user", "is-active", "arda-agent.target"])
        .output();
    let state = match output {
        Ok(output) => match String::from_utf8_lossy(&output.stdout).trim() {
            "active" => LifecycleAggregateState::Healthy,
            "activating" => LifecycleAggregateState::Starting,
            "deactivating" => LifecycleAggregateState::Stopping,
            "inactive" => LifecycleAggregateState::Stopped,
            "failed" => LifecycleAggregateState::Failed,
            _ => LifecycleAggregateState::Unknown,
        },
        Err(_) => LifecycleAggregateState::Unknown,
    };
    LifecycleProjectionReference {
        aggregate_state: state,
        observed_at,
        evidence_ref: "systemd-user://arda-agent.target".to_string(),
    }
}

#[tauri::command]
async fn get_mirromere_surface(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    display_role: MirromereDisplayRole,
) -> Result<MirromereSurfaceProjection, String> {
    let input = MirromereProjectionInput {
        display_role,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(observe_lifecycle()),
        continuity: Some(load_continuity_reference().await),
    };
    let surface =
        project_mirromere_surface_at(input, Utc::now()).map_err(|error| error.to_string())?;
    state.remember_surface(surface.clone())?;
    Ok(surface)
}

#[tauri::command]
fn request_mirromere_interaction(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    request: MirromereInteractionRequest,
) -> Result<MirromereInteractionReceipt, String> {
    state.record(request, Utc::now())
}

#[tauri::command]
fn get_display_state(app: tauri::AppHandle) -> Result<DisplayState, String> {
    let selection = display::load_selection()?;
    display::apply_selection(
        &app,
        selection
            .as_ref()
            .map(|selection| selection.display_id.as_str()),
    )
}

#[tauri::command]
fn select_mirromere_display(
    app: tauri::AppHandle,
    display_id: String,
) -> Result<DisplayState, String> {
    let (_, displays) = display::inventory(&app)?;
    display::resolve_selected_display(&displays, &display_id)?;
    display::save_selection(&display_id)?;
    display::apply_selection(&app, Some(&display_id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(MirromereInteractionReceiptState::default())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                window.set_decorations(true)?;
                window.set_resizable(true)?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_mirromere_surface,
            request_mirromere_interaction,
            get_display_state,
            select_mirromere_display
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mirromere");
}
