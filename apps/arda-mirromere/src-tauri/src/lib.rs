mod display;

use arda_launcher::lifecycle::commands::lifecycle_status;
use arda_launcher::lifecycle::types::AggregateState;
use arda_mirromere::{
    load_continuity_reference, project_mirromere_surface_at, LifecycleAggregateState,
    LifecycleProjectionReference, MirromereInteractionReceipt, MirromereInteractionReceiptState,
    MirromereInteractionRequest, MirromereProjectionInput, MirromereProjectionSourceMode,
};
use arda_outpost_protocol::{MirromereDisplayRole, MirromereSurfaceProjection};
use chrono::Utc;
use display::DisplayState;
use tauri::Manager;

fn map_lifecycle_state(value: AggregateState) -> LifecycleAggregateState {
    match value {
        AggregateState::Stopped => LifecycleAggregateState::Stopped,
        AggregateState::Starting => LifecycleAggregateState::Starting,
        AggregateState::Healthy => LifecycleAggregateState::Healthy,
        AggregateState::Degraded => LifecycleAggregateState::Degraded,
        AggregateState::Failed => LifecycleAggregateState::Failed,
        AggregateState::Stopping => LifecycleAggregateState::Stopping,
        AggregateState::Unknown => LifecycleAggregateState::Unknown,
    }
}

fn observe_lifecycle() -> LifecycleProjectionReference {
    let lifecycle = lifecycle_status();
    LifecycleProjectionReference {
        aggregate_state: map_lifecycle_state(lifecycle.aggregate_state),
        observed_at: lifecycle.observed_at,
        evidence_ref: format!("system-lifecycle://{}", lifecycle.observed_at.to_rfc3339()),
    }
}

#[tauri::command]
async fn get_mirromere_surface(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    display_role: MirromereDisplayRole,
) -> Result<MirromereSurfaceProjection, String> {
    let lifecycle = tauri::async_runtime::spawn_blocking(observe_lifecycle)
        .await
        .map_err(|error| format!("lifecycle observation failed: {error}"))?;
    let input = MirromereProjectionInput {
        display_role,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(lifecycle),
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
