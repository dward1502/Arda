use arda_launcher::lifecycle::commands::lifecycle_status;
use arda_launcher::lifecycle::types::AggregateState;
use arda_outpost_protocol::{MirromereDisplayRole, MirromereSurfaceProjection};
use chrono::Utc;

pub use arda_mirromere::*;

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

#[tauri::command]
pub async fn get_mirromere_surface(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    display_role: MirromereDisplayRole,
) -> Result<MirromereSurfaceProjection, String> {
    let lifecycle = tauri::async_runtime::spawn_blocking(lifecycle_status)
        .await
        .map_err(|error| format!("lifecycle observation failed: {error}"))?;
    let lifecycle_reference = LifecycleProjectionReference {
        aggregate_state: map_lifecycle_state(lifecycle.aggregate_state),
        observed_at: lifecycle.observed_at,
        evidence_ref: format!("system-lifecycle://{}", lifecycle.observed_at.to_rfc3339()),
    };
    let input = MirromereProjectionInput {
        display_role,
        source_mode: MirromereProjectionSourceMode::Runtime,
        lifecycle: Some(lifecycle_reference),
        continuity: Some(load_continuity_reference().await),
    };
    let surface =
        project_mirromere_surface_at(input, Utc::now()).map_err(|error| error.to_string())?;
    state.remember_surface(surface.clone())?;
    Ok(surface)
}

#[tauri::command]
pub fn request_mirromere_interaction(
    state: tauri::State<'_, MirromereInteractionReceiptState>,
    request: MirromereInteractionRequest,
) -> Result<MirromereInteractionReceipt, String> {
    state.record(request, Utc::now())
}
