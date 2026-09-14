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
use serde::Serialize;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::Manager;

const HERMES_DASHBOARD_ADDR: &str = "127.0.0.1:9119";
const HERMES_DASHBOARD_URL: &str = "http://127.0.0.1:9119";

#[derive(Default)]
struct HermesDashboardState {
    child: Mutex<Option<Child>>,
}

impl Drop for HermesDashboardState {
    fn drop(&mut self) {
        let Ok(child) = self.child.get_mut() else {
            return;
        };
        let Some(mut child) = child.take() else {
            return;
        };
        if matches!(child.try_wait(), Ok(None)) {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[derive(Serialize)]
struct HermesDashboardConnection {
    url: &'static str,
    launched: bool,
}

fn hermes_dashboard_addr() -> Result<SocketAddr, String> {
    HERMES_DASHBOARD_ADDR
        .parse()
        .map_err(|error| format!("invalid Hermes dashboard address: {error}"))
}

fn hermes_dashboard_is_ready() -> bool {
    let Ok(addr) = hermes_dashboard_addr() else {
        return false;
    };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let request = b"GET / HTTP/1.1\r\nHost: 127.0.0.1:9119\r\nConnection: close\r\n\r\n";
    if stream.write_all(request).is_err() {
        return false;
    }
    let mut response = String::new();
    if stream.read_to_string(&mut response).is_err() {
        return false;
    }
    let lower = response.to_ascii_lowercase();
    (response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"))
        && lower.contains("hermes")
}

fn wait_for_hermes_dashboard() -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(8) {
        if hermes_dashboard_is_ready() {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

#[tauri::command]
fn ensure_hermes_dashboard(
    app: tauri::AppHandle,
    state: tauri::State<'_, HermesDashboardState>,
) -> Result<HermesDashboardConnection, String> {
    if hermes_dashboard_is_ready() {
        let url = tauri::Url::parse(HERMES_DASHBOARD_URL).map_err(|error| error.to_string())?;
        app.get_webview_window("main")
            .ok_or_else(|| "Mirromere main window is unavailable".to_string())?
            .navigate(url)
            .map_err(|error| format!("failed to open Hermes in Mirromere: {error}"))?;
        return Ok(HermesDashboardConnection {
            url: HERMES_DASHBOARD_URL,
            launched: false,
        });
    }

    let mut child = state
        .child
        .lock()
        .map_err(|_| "Hermes dashboard process lock failed".to_string())?;
    if let Some(existing) = child.as_mut() {
        if matches!(existing.try_wait(), Ok(Some(_)) | Err(_)) {
            *child = None;
        }
    }
    if child.is_none() {
        *child = Some(
            Command::new("hermes")
                .args([
                    "dashboard",
                    "--host",
                    "127.0.0.1",
                    "--port",
                    "9119",
                    "--no-open",
                    "--skip-build",
                    "--tui",
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| format!("failed to start Hermes dashboard: {error}"))?,
        );
    }
    drop(child);

    if !wait_for_hermes_dashboard() {
        return Err("Hermes dashboard did not become ready at http://127.0.0.1:9119".to_string());
    }
    let url = tauri::Url::parse(HERMES_DASHBOARD_URL).map_err(|error| error.to_string())?;
    app.get_webview_window("main")
        .ok_or_else(|| "Mirromere main window is unavailable".to_string())?
        .navigate(url)
        .map_err(|error| format!("failed to open Hermes in Mirromere: {error}"))?;
    Ok(HermesDashboardConnection {
        url: HERMES_DASHBOARD_URL,
        launched: true,
    })
}

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
        .manage(HermesDashboardState::default())
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
            select_mirromere_display,
            ensure_hermes_dashboard
        ])
        .run(tauri::generate_context!())
        .expect("error while running Mirromere");
}
