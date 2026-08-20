// sigil: REPAIR
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
mod commands;
pub mod mirromere;

use base64::{engine::general_purpose, Engine as _};
use commands::monitor_surface::{
    claim_monitor_slot, claim_monitor_surface, click_browser_capture, get_browser_capture_frame,
    get_browser_capture_status, get_monitor_surface_registry, get_pty_capture_status,
    key_browser_capture, navigate_browser_capture, patch_monitor_surface_playback,
    push_surface_payload, refresh_monitor_slot_lease, refresh_monitor_surface_lease,
    release_monitor_slot, release_monitor_surface, restore_monitor_surface_registry,
    scroll_browser_capture, start_browser_capture, start_pty_capture, stop_browser_capture,
    stop_pty_capture, type_browser_capture, write_pty_capture, BrowserCaptureState,
    MonitorSurfaceState, PtyCaptureState, TypedMonitorSurfaceState,
};
use commands::workbench::{
    approve_workbench_run, attach_project_contract, cancel_workbench_run,
    complete_workbench_run_node, execute_workbench_provider_node, get_workbench_run,
    get_workbench_run_events, plan_workbench_run, start_workbench_run_event_stream,
    validate_project_contract, WorkbenchEventStreamState,
};
use portable_pty::CommandBuilder;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tauri::async_runtime::Mutex as AsyncMutex;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, Position, Size, State, WebviewUrl,
    WebviewWindowBuilder,
};
use tauri_plugin_opener::OpenerExt;

const IMAGE_PREVIEW_MAX_BYTES: u64 = 2 * 1024 * 1024;
const VIDEO_PREVIEW_MAX_BYTES: u64 = 12 * 1024 * 1024;
const PDF_PREVIEW_MAX_BYTES: u64 = 8 * 1024 * 1024;
// const DEFAULT_WINDOWED_WIDTH: f64 = 1280.0;
// const DEFAULT_WINDOWED_HEIGHT: f64 = 720.0;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileReadResult {
    pub success: bool,
    pub content: Option<String>,
    pub error: Option<String>,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub num_cpus: usize,
    pub memory_total_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryTreeNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub relative_path: String,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
    pub modified_unix: Option<i64>,
    pub children: Vec<InventoryTreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HudPulseEvent {
    pub ts_unix_ms: u64,
    pub status: String,
    pub source: String,
    pub sequence: u64,
}

#[derive(Default)]
struct HudPulseStreamState {
    running: Arc<AtomicBool>,
    handle: Mutex<Option<thread::JoinHandle<()>>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkstationWindowRequest {
    window_label: String,
    title: String,
    subtitle: Option<String>,
    workstation_id: String,
    source_zone_id: Option<String>,
    origin_anchor_id: Option<String>,
    presentation_mode: String,
    width: f64,
    height: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HermesRuntimeWindowResult {
    window_label: String,
    url: String,
    port: u16,
    launched_process: bool,
    already_listening: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct HermesRuntimeStatus {
    url: String,
    host: String,
    port: u16,
    port_open: bool,
    identity_verified: bool,
    owned_process_running: bool,
    state: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct LocalOperatorActionResult {
    ok: bool,
    message: String,
    receipt_path: String,
    result_path: String,
    generated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceRevealResult {
    ok: bool,
    source_path: String,
    resolved_path: String,
    message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceImagePreviewResult {
    ok: bool,
    source_path: String,
    resolved_path: String,
    mime_type: String,
    size_bytes: u64,
    data_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceVideoPreviewResult {
    ok: bool,
    source_path: String,
    resolved_path: String,
    mime_type: String,
    size_bytes: u64,
    data_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SourcePdfPreviewResult {
    ok: bool,
    source_path: String,
    resolved_path: String,
    mime_type: String,
    size_bytes: u64,
    data_url: String,
}

#[derive(Default, Clone)]
struct HermesRuntimeState {
    child: Arc<Mutex<Option<Child>>>,
}

impl HermesRuntimeState {
    fn cleanup_owned_child(&self) {
        let Ok(mut guard) = self.child.lock() else {
            return;
        };
        let Some(mut child) = guard.take() else {
            return;
        };

        match child.try_wait() {
            Ok(Some(_status)) => {}
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
            }
            Err(_error) => {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl Drop for HermesRuntimeState {
    fn drop(&mut self) {
        self.cleanup_owned_child();
    }
}

const DEFAULT_HERMES_RUNTIME_HOST: &str = "127.0.0.1";
const DEFAULT_HERMES_RUNTIME_PORT: u16 = 9119;
const DEFAULT_CHARON_HOST: &str = "127.0.0.1";
const DEFAULT_CHARON_PORT: u16 = 7171;
const HERMES_RUNTIME_WINDOW_LABEL: &str = "arda-workstation-hermes_runtime_workstation";
const HERMES_TERMINAL_WINDOW_LABEL: &str = "arda-hermes-terminal";
const HERMES_TERMINAL_WINDOW_PATH: &str = "terminal.html";
const HERMES_RUNTIME_HOST_ENV: &str = "ARDA_HERMES_RUNTIME_HOST";
const HERMES_RUNTIME_PORT_ENV: &str = "ARDA_HERMES_RUNTIME_PORT";

#[derive(Debug, Clone, PartialEq, Eq)]
struct HermesRuntimeConfig {
    host: String,
    port: u16,
}

impl Default for HermesRuntimeConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HERMES_RUNTIME_HOST.to_string(),
            port: DEFAULT_HERMES_RUNTIME_PORT,
        }
    }
}

impl HermesRuntimeConfig {
    fn from_env() -> Self {
        Self {
            host: dashboard_env_value(HERMES_RUNTIME_HOST_ENV)
                .unwrap_or_else(|| DEFAULT_HERMES_RUNTIME_HOST.to_string()),
            port: dashboard_env_value(HERMES_RUNTIME_PORT_ENV)
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(DEFAULT_HERMES_RUNTIME_PORT),
        }
    }

    fn url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    fn socket_addr(&self) -> Result<SocketAddr, String> {
        format!("{}:{}", self.host, self.port)
            .to_socket_addrs()
            .map_err(|error| format!("invalid Hermes runtime socket address: {}", error))?
            .next()
            .ok_or_else(|| "Hermes runtime socket address did not resolve".to_string())
    }
}

fn dashboard_env_value(key: &str) -> Option<String> {
    std::env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn charon_socket_addr() -> Result<SocketAddr, String> {
    let host = std::env::var("ARDA_MANWE_HOST")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CHARON_HOST.to_string());
    let port = std::env::var("ARDA_MANWE_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(DEFAULT_CHARON_PORT);
    format!("{host}:{port}")
        .to_socket_addrs()
        .map_err(|error| format!("invalid Charon socket address: {error}"))?
        .next()
        .ok_or_else(|| "Charon socket address did not resolve".to_string())
}

fn allowed_charon_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/status" | "/providers/capabilities" | "/provider_candidates"
    )
}

fn read_local_http_json(addr: SocketAddr, path: &str) -> Result<serde_json::Value, String> {
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(2))
        .map_err(|error| format!("failed to connect to Charon at {addr}: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(4)))
        .map_err(|error| format!("failed to set Charon read timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set Charon write timeout: {error}"))?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to send Charon request: {error}"))?;
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|error| format!("failed to read Charon response: {error}"))?;
    let Some((header, body)) = response.split_once("\r\n\r\n") else {
        return Err("Charon response was not a valid HTTP response".to_string());
    };
    if !header.starts_with("HTTP/1.1 2") && !header.starts_with("HTTP/1.0 2") {
        let status_line = header.lines().next().unwrap_or("HTTP status unknown");
        return Err(format!("Charon returned {status_line}"));
    }
    serde_json::from_str(body).map_err(|error| format!("invalid Charon JSON: {error}"))
}

fn is_safe_relative_path(relative: &str) -> bool {
    let path = Path::new(relative);
    !path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    })
}

fn scoped_join(root: &str, relative: &str) -> Result<PathBuf, String> {
    if !is_safe_relative_path(relative) {
        return Err("Unsafe relative path".to_string());
    }
    Ok(Path::new(root).join(relative))
}

fn resolve_scoped_source_path(root: &str, source_path: &str) -> Result<PathBuf, String> {
    let source_path = source_path.trim();
    if source_path.is_empty() {
        return Err("Source path is empty".to_string());
    }

    let root_path = Path::new(root)
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize Annunimas root: {error}"))?;
    let candidate = if Path::new(source_path).is_absolute() {
        PathBuf::from(source_path)
    } else {
        scoped_join(root, source_path)?
    };
    let resolved = candidate
        .canonicalize()
        .map_err(|error| format!("source path does not exist or cannot be resolved: {error}"))?;

    if !resolved.starts_with(&root_path) {
        return Err("Source path is outside the Annunimas workspace".to_string());
    }

    Ok(resolved)
}

fn image_mime_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "avif" => Some("image/avif"),
        _ => None,
    }
}

fn video_mime_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "ogv" | "ogg" => Some("video/ogg"),
        "mov" => Some("video/quicktime"),
        _ => None,
    }
}

fn pdf_mime_for_path(path: &Path) -> Option<&'static str> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    match extension.as_str() {
        "pdf" => Some("application/pdf"),
        _ => None,
    }
}

fn home_arda_root() -> String {
    Path::new(&std::env::var("HOME").unwrap_or_else(|_| "/var/home/mythos".to_string()))
        .join("ARDA")
        .to_string_lossy()
        .into_owned()
}

fn resolve_existing_env(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .filter(|value| !value.trim().is_empty())
            .filter(|value| Path::new(value).exists())
    })
}

fn resolve_ardas_root() -> String {
    if let Some(path) = resolve_existing_env(&[
        "ARDA_ROOT",
        "ARDA_NUMENOR_ROOT",
        "NUMENOR_ROOT",
        "NUMENOR_PRIME_ROOT",
        "ARDA_ROOT",
        "ANNUNIMAS_NUMENOR_ROOT",
    ]) {
        return path;
    }
    if Path::new("/var/home/mythos/Eregion/Arda/core/state").exists() {
        return "/var/home/mythos/Eregion/Arda".to_string();
    }
    home_arda_root()
}

fn write_system_paths_file(root: &str, state_dir: &Path) -> Result<(), String> {
    let path = state_dir.join("system_paths.json");
    let payload = serde_json::json!({
        "arda_root": root,
        "arda_state_dir": state_dir.to_string_lossy().into_owned(),
        "resolved_at": chrono::Utc::now().to_rfc3339(),
    });
    fs::create_dir_all(state_dir).map_err(|e| format!("failed to create state dir: {e}"))?;
    let content = serde_json::to_string_pretty(&payload)
        .map_err(|e| format!("serialize system paths failed: {e}"))?;
    fs::write(&path, content).map_err(|e| format!("write system paths failed: {e}"))?;
    Ok(())
}
fn resolve_ardas_state_dir(root: &str) -> PathBuf {
    if let Some(path) = std::env::var_os("ARDA_STATE_DIR").and_then(|value| {
        let trimmed = value.to_string_lossy().trim().to_string();
        (!trimmed.is_empty() && Path::new(&trimmed).exists()).then_some(trimmed)
    }) {
        return PathBuf::from(path);
    }
    Path::new(root).join("core/state")
}

fn _resolve_voice_root() -> String {
    if let Some(path) = resolve_existing_env(&[
        "ANNUNIMAS_VALINOR_ROOT",
        "ANNUNIMAS_VOICE_ROOT",
        "ARDA_VALINOR_ROOT",
        "ARDA_VOICE_ROOT",
    ]) {
        return path;
    }

    let arda_root = resolve_ardas_root();
    let arda_valinor = Path::new(&arda_root).join("Valinor");
    if arda_valinor.exists() {
        return arda_valinor.to_string_lossy().to_string();
    }

    Path::new(&arda_root)
        .join("Valinor")
        .to_string_lossy()
        .to_string()
}

fn modified_unix(meta: &fs::Metadata) -> Option<i64> {
    meta.modified()
        .ok()
        .and_then(|m| m.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
}

fn build_inventory_tree(root: &Path, path: &Path, depth: u8, max_depth: u8) -> InventoryTreeNode {
    let meta = fs::metadata(path).ok();
    let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
    let rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let rel_norm = if rel.is_empty() { ".".to_string() } else { rel };

    let mut children: Vec<InventoryTreeNode> = Vec::new();
    if is_dir && depth < max_depth {
        if let Ok(entries) = fs::read_dir(path) {
            let mut paths: Vec<PathBuf> =
                entries.filter_map(|e| e.ok().map(|x| x.path())).collect();
            paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
            for entry_path in paths {
                children.push(build_inventory_tree(
                    root,
                    &entry_path,
                    depth + 1,
                    max_depth,
                ));
            }
        }
    }

    InventoryTreeNode {
        id: rel_norm.clone(),
        name: path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Numenor_Prime".to_string()),
        path: path.to_string_lossy().to_string(),
        relative_path: rel_norm,
        is_dir,
        size_bytes: meta
            .as_ref()
            .and_then(|m| if m.is_file() { Some(m.len()) } else { None }),
        modified_unix: meta.as_ref().and_then(modified_unix),
        children,
    }
}

#[tauri::command]
fn read_file(path: String) -> FileReadResult {
    let full_path = PathBuf::from(&path);

    match fs::read_to_string(&full_path) {
        Ok(content) => FileReadResult {
            success: true,
            content: Some(content),
            error: None,
            path: path.clone(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: path.clone(),
        },
    }
}

#[tauri::command]
fn get_arda_root() -> String {
    resolve_ardas_root()
}

#[derive(Serialize)]
struct HudRenderContext {
    software_renderer: bool,
}

#[tauri::command]
fn get_hud_render_context() -> HudRenderContext {
    HudRenderContext {
        software_renderer: std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER")
            .is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true")),
    }
}

#[tauri::command]
fn get_numenor_path() -> String {
    resolve_ardas_root()
}

#[tauri::command]
fn _get_voice_root() -> String {
    _resolve_voice_root()
}

#[tauri::command]
fn _get_system_info() -> SystemInfo {
    SystemInfo {
        hostname: hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string()),
        os: std::env::consts::OS.to_string(),
        num_cpus: num_cpus::get(),
        memory_total_gb: _get_memory_gb(),
    }
}

fn json_string_field(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|item| item.as_str())
        .filter(|item| !item.trim().is_empty())
        .map(ToString::to_string)
}

fn unix_timestamp_fallback() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_else(|_| 0);
    format!("unix:{seconds}")
}

fn run_bounded_command(
    mut command: Command,
    timeout: Duration,
    label: &str,
) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("failed to launch {label}: {error}"))?;
    let started = Instant::now();

    loop {
        match child.try_wait() {
            Ok(Some(_status)) => {
                return child
                    .wait_with_output()
                    .map_err(|error| format!("failed to collect {label} output: {error}"));
            }
            Ok(None) => {
                if started.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!(
                        "{label} timed out after {} seconds",
                        timeout.as_secs()
                    ));
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed while waiting for {label}: {error}"));
            }
        }
    }
}

fn local_action_result_from_state(
    state_json: &serde_json::Value,
    message: String,
    receipt_path: String,
    result_path: String,
) -> LocalOperatorActionResult {
    let generated_at = json_string_field(state_json, "generated_at_utc")
        .or_else(|| json_string_field(state_json, "generated_at"))
        .unwrap_or_else(unix_timestamp_fallback);

    LocalOperatorActionResult {
        ok: true,
        message,
        receipt_path,
        result_path,
        generated_at,
    }
}

#[tauri::command]
fn run_chronos_provider_checks(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "chronos.run_provider_checks" {
        return Err(format!("unsupported CHRONOS provider action: {action_id}"));
    }

    let arda_root = resolve_ardas_root();
    let state_path = "core/state/chronos_runtime.json";
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-chronos")
        .env("ARDA_ROOT", &arda_root)
        .current_dir(&arda_root);
    let output = run_bounded_command(command, Duration::from_secs(45), "CHRONOS provider checks")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "CHRONOS provider checks failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let result_path = state_path.to_string();
    let state_json = fs::read_to_string(Path::new(&arda_root).join(&result_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::Value::Null);
    let status = json_string_field(&state_json, "status").unwrap_or_else(|| "unknown".to_string());

    Ok(local_action_result_from_state(
        &state_json,
        format!("CHRONOS provider checks refreshed ({status})"),
        state_path.to_string(),
        result_path,
    ))
}

#[tauri::command]
fn run_queue_cleanup_preview(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "queue.preview_cleanup" {
        return Err(format!(
            "unsupported queue cleanup preview action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let state_path = "core/state/queue_summary.json";
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-cli")
        .arg("--")
        .arg("export")
        .arg("autonomy-resume")
        .env("ARDA_ROOT", &arda_root)
        .current_dir(&arda_root);
    let output = run_bounded_command(command, Duration::from_secs(45), "queue cleanup preview")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "queue cleanup preview failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let result_path = state_path.to_string();
    let state_json = fs::read_to_string(Path::new(&arda_root).join(&result_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::Value::Null);
    let queued_total = state_json
        .get("project_tasks")
        .and_then(|project_tasks| project_tasks.get("counts_by_status"))
        .and_then(|counts| counts.get("queued"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);

    Ok(local_action_result_from_state(
        &state_json,
        format!("queue cleanup preview refreshed ({queued_total} queued tasks)"),
        state_path.to_string(),
        result_path,
    ))
}

#[tauri::command]
fn run_hades_recurring_maintenance(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "hades.run_nightly" {
        return Err(format!(
            "unsupported HADES recurring maintenance action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let receipt_path = "core/state/hades_nightly_operations.json";
    let output = run_bounded_command(
        {
            let mut command = Command::new("python3");
            command
                .arg("scripts/hades_nightly_operations.py")
                .arg("--root")
                .arg(&arda_root)
                .current_dir(&arda_root);
            command
        },
        Duration::from_secs(120),
        &format!("HADES recurring maintenance for {source}"),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "HADES recurring maintenance failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let summary = serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap_or_else(|_| serde_json::Value::Null);
    let state_json = fs::read_to_string(Path::new(&arda_root).join(receipt_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::Value::Null);
    let status = json_string_field(&state_json, "status")
        .or_else(|| json_string_field(&summary, "status"))
        .unwrap_or_else(|| "unknown".to_string());
    let result_path = state_json
        .get("artifacts")
        .and_then(|artifacts| artifacts.get("organization_plan"))
        .and_then(serde_json::Value::as_str)
        .filter(|path| !path.trim().is_empty())
        .unwrap_or("data/hades/organization_plan_last.json")
        .to_string();

    Ok(local_action_result_from_state(
        &state_json,
        format!("HADES recurring maintenance refreshed ({status})"),
        receipt_path.to_string(),
        result_path,
    ))
}

fn run_setup_console_audit_receipt(
    arda_root: &str,
    out_dir: &str,
    state_path: &str,
    label: &str,
    source: &str,
) -> Result<(serde_json::Value, String, String, String), String> {
    let mut rust_command = Command::new("cargo");
    rust_command
        .arg("run")
        .arg("-p")
        .arg("arda-cli")
        .arg("--")
        .arg("onboarding")
        .arg("readiness")
        .arg("--out-dir")
        .arg(out_dir)
        .arg("--output")
        .arg(state_path)
        .current_dir(arda_root);
    let output = match run_bounded_command(
        rust_command,
        Duration::from_secs(120),
        &format!("{label} for {source} via arda-cli onboarding readiness"),
    ) {
        Ok(output) if output.status.success() => output,
        Ok(primary_output) => {
            let stdout = String::from_utf8_lossy(&primary_output.stdout);
            let stderr = String::from_utf8_lossy(&primary_output.stderr);
            return Err(format!(
                "{label} failed for {source}: {}",
                if stderr.trim().is_empty() {
                    stdout.trim()
                } else {
                    stderr.trim()
                }
            ));
        }
        Err(primary_error) => {
            return Err(format!("{label} errored for {source}: {primary_error}"));
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "{label} failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let summary = serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap_or_else(|_| serde_json::Value::Null);
    let receipt_path = json_string_field(&summary, "receipt")
        .unwrap_or_else(|| format!("{out_dir}/setup_console_readiness_receipt.json"));
    let result_path =
        json_string_field(&summary, "state").unwrap_or_else(|| state_path.to_string());
    let state_json = fs::read_to_string(Path::new(arda_root).join(&result_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::Value::Null);
    let gate_status =
        json_string_field(&state_json, "gate_status").unwrap_or_else(|| "unknown".to_string());

    Ok((state_json, receipt_path, result_path, gate_status))
}

#[tauri::command]
fn run_setup_readiness_check(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "setup.run_readiness_check" {
        return Err(format!("unsupported setup readiness action: {action_id}"));
    }

    let arda_root = resolve_ardas_root();
    let state_dir = resolve_ardas_state_dir(&arda_root);
    let _ = write_system_paths_file(&arda_root, &state_dir);
    let (state_json, receipt_path, result_path, gate_status) = run_setup_console_audit_receipt(
        &arda_root,
        "audit/setup-console-runs/arda-hud-readiness-last",
        "core/state/setup_console_readiness.json",
        "setup readiness audit",
        &source,
    )?;

    Ok(local_action_result_from_state(
        &state_json,
        format!("setup readiness refreshed ({gate_status})"),
        receipt_path,
        result_path,
    ))
}

#[tauri::command]
fn read_charon_json(path: String) -> Result<serde_json::Value, String> {
    if !allowed_charon_path(&path) {
        return Err(format!("unsupported Charon HUD path: {path}"));
    }
    read_local_http_json(charon_socket_addr()?, &path)
}

#[tauri::command]
fn run_setup_repair_preflight(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "setup.run_repair_flow" {
        return Err(format!(
            "unsupported setup repair preflight action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let state_dir = resolve_ardas_state_dir(&arda_root);
    let _ = write_system_paths_file(&arda_root, &state_dir);
    let (state_json, receipt_path, result_path, gate_status) = run_setup_console_audit_receipt(
        &arda_root,
        "audit/setup-console-runs/arda-hud-repair-preflight-last",
        "core/state/setup_repair_preflight.json",
        "setup repair preflight",
        &source,
    )?;

    Ok(local_action_result_from_state(
        &state_json,
        format!("setup repair preflight refreshed ({gate_status}; no repair mutations applied)"),
        receipt_path,
        result_path,
    ))
}

#[tauri::command]
fn run_setup_repair_execution_gate(
    action_id: String,
    source: String,
    confirmation_phrase: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "setup.run_repair_flow" {
        return Err(format!(
            "unsupported setup repair execution gate action: {action_id}"
        ));
    }
    if confirmation_phrase != "RUN_SETUP_REPAIR_FLOW" {
        return Err(
            "setup repair execution gate requires confirmationPhrase=RUN_SETUP_REPAIR_FLOW"
                .to_string(),
        );
    }

    let arda_root = resolve_ardas_root();
    let state_dir = resolve_ardas_state_dir(&arda_root);
    let _ = write_system_paths_file(&arda_root, &state_dir);
    let (state_json, receipt_path, result_path, gate_status) = run_setup_console_audit_receipt(
        &arda_root,
        "audit/setup-console-runs/arda-hud-repair-execution-gate-last",
        "core/state/setup_repair_execution_gate.json",
        "operator-confirmed setup repair execution gate",
        &source,
    )?;

    Ok(local_action_result_from_state(
        &state_json,
        format!("operator-confirmed setup repair execution gate opened ({gate_status}; repair mutations still disabled)"),
        receipt_path,
        result_path,
    ))
}

#[tauri::command]
fn run_repeated_audit_preview(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "audit.run_repeated_audit" {
        return Err(format!("unsupported repeated audit action: {action_id}"));
    }

    let arda_root = resolve_ardas_root();
    let state_dir = resolve_ardas_state_dir(&arda_root);
    let _ = write_system_paths_file(&arda_root, &state_dir);
    let out_dir = "audit/repeated-audit-runs/arda-hud-preview-last";
    let state_path = "core/state/repeated_audit_status.json";
    let mut command = Command::new("python3");
    command
        .arg("scripts/audit/repeated_audit.py")
        .arg("--root")
        .arg(&arda_root)
        .arg("--out")
        .arg(out_dir)
        .arg("--state-path")
        .arg(state_path)
        .arg("--run-id")
        .arg("arda-hud-preview-last")
        .current_dir(&arda_root);
    let output = run_bounded_command(
        command,
        Duration::from_secs(120),
        &format!("repeated audit preview for {source}"),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "repeated audit preview failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let cli_summary = serde_json::from_str::<serde_json::Value>(&stdout)
        .unwrap_or_else(|_| serde_json::Value::Null);
    let state_json = fs::read_to_string(Path::new(&arda_root).join(state_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::Value::Null);
    let gate_status = json_string_field(&state_json, "gate_status")
        .or_else(|| json_string_field(&cli_summary, "gate_status"))
        .unwrap_or_else(|| "unknown".to_string());
    let regression_count = state_json
        .get("regression_count")
        .or_else(|| cli_summary.get("regression_count"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let result_path = state_json
        .get("outputs")
        .and_then(|outputs| outputs.get("summary_json"))
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            cli_summary
                .get("summary")
                .and_then(serde_json::Value::as_str)
        })
        .filter(|path| !path.trim().is_empty())
        .unwrap_or("audit/repeated-audit-runs/arda-hud-preview-last/summary.json")
        .to_string();

    Ok(local_action_result_from_state(
        &state_json,
        format!("repeated audit preview refreshed ({gate_status}, {regression_count} regressions)"),
        state_path.to_string(),
        result_path,
    ))
}

fn arda_cli_command(root: &str, args: &[&str]) -> Command {
    let build_root = Path::new(root).join(".cache").join("arda-build");
    let target_dir = build_root.join("target");
    let tmp_dir = build_root.join("tmp");
    let _ = fs::create_dir_all(&target_dir);
    let _ = fs::create_dir_all(&tmp_dir);

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-cli")
        .arg("--")
        .args(args)
        .env("ARDA_ROOT", root)
        .env("ARDA_BUILD_CACHE_ROOT", build_root.as_os_str())
        .env("CARGO_TARGET_DIR", target_dir.as_os_str())
        .env("TMPDIR", tmp_dir.as_os_str())
        .current_dir(root);
    command
}

fn run_arda_cli(
    root: &str,
    args: &[&str],
    timeout: Duration,
    label: &str,
) -> Result<Output, String> {
    let command = arda_cli_command(root, args);
    let output = run_bounded_command(command, timeout, label)?;
    if output.status.success() {
        return Ok(output);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "{label} failed: {}",
        if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        }
    ))
}

fn count_valid_jsonl_records(path: &Path, label: &str) -> Result<usize, String> {
    let content = fs::read_to_string(path).map_err(|error| {
        format!(
            "failed to read {label} JSONL at {}: {error}",
            path.display()
        )
    })?;
    let mut count = 0;
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        serde_json::from_str::<serde_json::Value>(trimmed).map_err(|error| {
            format!(
                "failed to parse {label} JSONL record {} at {}: {error}",
                index + 1,
                path.display()
            )
        })?;
        count += 1;
    }
    Ok(count)
}

fn count_json_array_records(value: &serde_json::Value) -> usize {
    value.as_array().map(Vec::len).unwrap_or(0)
}

fn count_provider_intelligence_records(value: &serde_json::Value) -> usize {
    value
        .get("providers")
        .and_then(serde_json::Value::as_object)
        .map(|providers| providers.len())
        .unwrap_or(0)
}

#[tauri::command]
fn run_charon_provider_intelligence_refresh(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "charon.refresh_provider_intelligence" {
        return Err(format!(
            "unsupported CHARON provider intelligence action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let state_path = "core/state/provider_intelligence.json";
    let mut command = Command::new("python3");
    command
        .arg("scripts/refresh_provider_intelligence.py")
        .current_dir(&arda_root)
        .env("ARDA_ROOT", &arda_root);
    let output = run_bounded_command(
        command,
        Duration::from_secs(90),
        &format!("CHARON provider intelligence refresh for {source}"),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        return Err(format!(
            "CHARON provider intelligence refresh failed for {source}: {}",
            if stderr.trim().is_empty() {
                stdout.trim()
            } else {
                stderr.trim()
            }
        ));
    }

    let state_json = fs::read_to_string(Path::new(&arda_root).join(state_path))
        .map_err(|error| format!("failed to read CHARON provider intelligence state: {error}"))
        .and_then(|content| {
            serde_json::from_str::<serde_json::Value>(&content).map_err(|error| {
                format!("failed to parse CHARON provider intelligence state: {error}")
            })
        })?;
    let provider_total = count_provider_intelligence_records(&state_json);

    Ok(local_action_result_from_state(
        &state_json,
        format!("CHARON provider intelligence refreshed ({provider_total} providers)"),
        state_path.to_string(),
        state_path.to_string(),
    ))
}

#[tauri::command]
fn run_athena_knowledge_ingestion(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "athena.ingest_knowledge" {
        return Err(format!(
            "unsupported ATHENA knowledge ingestion action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let receipt_path = "core/state/knowledge_triage_registry.jsonl";
    let result_path = "data/athena/human_ingestion_results.jsonl";
    run_arda_cli(
        &arda_root,
        &[
            "athena",
            "human-scan",
            "--human-root",
            "human",
            "--output",
            result_path,
            "--contradictions",
            "data/athena/human_contradiction_candidates.jsonl",
            "--limit",
            "25",
        ],
        Duration::from_secs(90),
        &format!("ATHENA knowledge ingestion for {source}"),
    )?;

    let scanned_total = count_valid_jsonl_records(
        &Path::new(&arda_root).join(result_path),
        "ATHENA knowledge ingestion result",
    )?;
    let state_json = serde_json::json!({
        "generated_at_utc": unix_timestamp_fallback(),
        "status": "scan_complete",
        "scanned_total": scanned_total,
    });

    Ok(local_action_result_from_state(
        &state_json,
        format!("ATHENA knowledge ingestion refreshed ({scanned_total} files scanned)"),
        receipt_path.to_string(),
        result_path.to_string(),
    ))
}

#[tauri::command]
fn run_athena_digest_refresh(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "athena.refresh_digest" {
        return Err(format!(
            "unsupported ATHENA digest refresh action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let receipt_path = "core/state/athena_runtime.json";
    let result_path = "data/athena/digest.jsonl";
    run_arda_cli(
        &arda_root,
        &["athena", "digest", "--limit", "25"],
        Duration::from_secs(90),
        &format!("ATHENA digest refresh for {source}"),
    )?;
    let _ = run_arda_cli(
        &arda_root,
        &["export", "athena-digest-pipeline"],
        Duration::from_secs(90),
        "ATHENA digest pipeline projection",
    );

    let state_json = fs::read_to_string(Path::new(&arda_root).join(receipt_path))
        .ok()
        .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
        .unwrap_or_else(|| serde_json::json!({ "generated_at_utc": unix_timestamp_fallback() }));
    let entries_total = count_valid_jsonl_records(
        &Path::new(&arda_root).join(result_path),
        "ATHENA digest result",
    )?;

    Ok(local_action_result_from_state(
        &state_json,
        format!("ATHENA digest refreshed ({entries_total} entries)"),
        receipt_path.to_string(),
        result_path.to_string(),
    ))
}

#[tauri::command]
fn run_athena_policy_readiness_preview(
    action_id: String,
    source: String,
) -> Result<LocalOperatorActionResult, String> {
    if action_id != "athena.promote_policy_ready" {
        return Err(format!(
            "unsupported ATHENA policy readiness preview action: {action_id}"
        ));
    }

    let arda_root = resolve_ardas_root();
    let receipt_path = "data/athena/policy_readiness.jsonl";
    let output = run_arda_cli(
        &arda_root,
        &["athena", "policy-readiness", "--limit", "25"],
        Duration::from_secs(90),
        &format!("ATHENA policy readiness preview for {source}"),
    )?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let preview_json = serde_json::from_str::<serde_json::Value>(&stdout).map_err(|error| {
        format!("failed to parse ATHENA policy readiness preview JSON: {error}")
    })?;
    let reviewed_total = count_json_array_records(&preview_json);
    let _receipt_records = count_valid_jsonl_records(
        &Path::new(&arda_root).join(receipt_path),
        "ATHENA policy readiness receipt",
    )?;
    let state_json = serde_json::json!({
        "generated_at_utc": unix_timestamp_fallback(),
        "status": "preview_complete",
        "reviewed_total": reviewed_total,
    });

    Ok(local_action_result_from_state(
        &state_json,
        format!("ATHENA policy readiness preview refreshed ({reviewed_total} sources reviewed)"),
        receipt_path.to_string(),
        receipt_path.to_string(),
    ))
}

#[tauri::command]
fn reveal_source_path(app: AppHandle, source_path: String) -> Result<SourceRevealResult, String> {
    let arda_root = resolve_ardas_root();
    let resolved = resolve_scoped_source_path(&arda_root, &source_path)?;
    app.opener()
        .reveal_item_in_dir(&resolved)
        .map_err(|error| format!("failed to reveal source path: {error}"))?;

    let resolved_path = resolved.to_string_lossy().to_string();
    Ok(SourceRevealResult {
        ok: true,
        source_path,
        resolved_path: resolved_path.clone(),
        message: format!("Revealed source path {resolved_path}"),
    })
}

#[tauri::command]
fn open_source_path(app: AppHandle, source_path: String) -> Result<SourceRevealResult, String> {
    let arda_root = resolve_ardas_root();
    let resolved = resolve_scoped_source_path(&arda_root, &source_path)?;
    let resolved_path = resolved.to_string_lossy().to_string();
    app.opener()
        .open_path(resolved_path.clone(), None::<&str>)
        .map_err(|error| format!("failed to open source path: {error}"))?;

    Ok(SourceRevealResult {
        ok: true,
        source_path,
        resolved_path: resolved_path.clone(),
        message: format!("Opened source path {resolved_path}"),
    })
}

#[tauri::command]
fn read_source_image_preview(source_path: String) -> Result<SourceImagePreviewResult, String> {
    let arda_root = resolve_ardas_root();
    let resolved = resolve_scoped_source_path(&arda_root, &source_path)?;
    let mime_type = image_mime_for_path(&resolved)
        .ok_or_else(|| "Source path is not a supported preview image".to_string())?;
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("failed to read source image metadata: {error}"))?;
    if metadata.len() > IMAGE_PREVIEW_MAX_BYTES {
        return Err(format!(
            "Source image is too large for inline preview: {} bytes > {} bytes",
            metadata.len(),
            IMAGE_PREVIEW_MAX_BYTES
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("failed to read source image preview: {error}"))?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let resolved_path = resolved.to_string_lossy().to_string();

    Ok(SourceImagePreviewResult {
        ok: true,
        source_path,
        resolved_path,
        mime_type: mime_type.to_string(),
        size_bytes: metadata.len(),
        data_url: format!("data:{mime_type};base64,{encoded}"),
    })
}

#[tauri::command]
fn read_source_video_preview(source_path: String) -> Result<SourceVideoPreviewResult, String> {
    let arda_root = resolve_ardas_root();
    let resolved = resolve_scoped_source_path(&arda_root, &source_path)?;
    let mime_type = video_mime_for_path(&resolved)
        .ok_or_else(|| "Source path is not a supported preview video".to_string())?;
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("failed to read source video metadata: {error}"))?;
    if metadata.len() > VIDEO_PREVIEW_MAX_BYTES {
        return Err(format!(
            "Source video is too large for inline preview: {} bytes > {} bytes",
            metadata.len(),
            VIDEO_PREVIEW_MAX_BYTES
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("failed to read source video preview: {error}"))?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let resolved_path = resolved.to_string_lossy().to_string();

    Ok(SourceVideoPreviewResult {
        ok: true,
        source_path,
        resolved_path,
        mime_type: mime_type.to_string(),
        size_bytes: metadata.len(),
        data_url: format!("data:{mime_type};base64,{encoded}"),
    })
}

#[tauri::command]
fn read_source_pdf_preview(source_path: String) -> Result<SourcePdfPreviewResult, String> {
    let arda_root = resolve_ardas_root();
    let resolved = resolve_scoped_source_path(&arda_root, &source_path)?;
    let mime_type = pdf_mime_for_path(&resolved)
        .ok_or_else(|| "Source path is not a supported preview PDF".to_string())?;
    let metadata = fs::metadata(&resolved)
        .map_err(|error| format!("failed to read source PDF metadata: {error}"))?;
    if metadata.len() > PDF_PREVIEW_MAX_BYTES {
        return Err(format!(
            "Source PDF is too large for inline preview: {} bytes > {} bytes",
            metadata.len(),
            PDF_PREVIEW_MAX_BYTES
        ));
    }
    let bytes = fs::read(&resolved)
        .map_err(|error| format!("failed to read source PDF preview: {error}"))?;
    let encoded = general_purpose::STANDARD.encode(bytes);
    let resolved_path = resolved.to_string_lossy().to_string();

    Ok(SourcePdfPreviewResult {
        ok: true,
        source_path,
        resolved_path,
        mime_type: mime_type.to_string(),
        size_bytes: metadata.len(),
        data_url: format!("data:{mime_type};base64,{encoded}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn hermes_terminal_uses_the_bundled_terminal_entry() {
        assert_eq!(hermes_terminal_window_path(), "terminal.html");
    }

    fn temp_path(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_else(|_| 0);
        std::env::temp_dir().join(format!("arda-hud-{name}-{unique}"))
    }

    #[test]
    fn athena_jsonl_parser_counts_only_valid_nonempty_records() {
        let path = temp_path("athena-jsonl-valid");
        fs::write(
            &path,
            concat!(
                "{\"path\":\"human/one.md\",\"classification\":\"project_signal\"}\n",
                "\n",
                "{\"path\":\"human/two.md\",\"classification\":\"reference\"}\n",
            ),
        )
        .expect("write jsonl fixture");

        let count =
            count_valid_jsonl_records(&path, "ATHENA fixture").expect("valid JSONL should parse");

        assert_eq!(count, 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn athena_jsonl_parser_rejects_malformed_records_with_line_number() {
        let path = temp_path("athena-jsonl-invalid");
        fs::write(
            &path,
            concat!(
                "{\"path\":\"human/one.md\"}\n",
                "not-json\n",
                "{\"path\":\"human/two.md\"}\n",
            ),
        )
        .expect("write malformed jsonl fixture");

        let error = count_valid_jsonl_records(&path, "ATHENA fixture")
            .expect_err("malformed JSONL should fail");

        assert!(error.contains("record 2"));
        assert!(error.contains("ATHENA fixture"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn provider_intelligence_parser_counts_providers_from_state() {
        let state_json = serde_json::json!({
            "generated_at_utc": "2026-05-28T06:20:00Z",
            "providers": {
                "openrouter": { "enabled": true },
                "opencode": { "enabled": true },
                "mistral": { "enabled": false },
            }
        });

        assert_eq!(count_provider_intelligence_records(&state_json), 3);
    }

    #[test]
    fn provider_intelligence_parser_handles_missing_providers() {
        let state_json = serde_json::json!({
            "generated_at_utc": "2026-05-28T06:20:00Z",
            "status": "empty"
        });

        assert_eq!(count_provider_intelligence_records(&state_json), 0);
    }

    #[test]
    fn athena_policy_preview_parser_counts_json_array_rows() {
        let preview = serde_json::json!([
            { "source_id": "src_one", "policy_readiness": "reference_only" },
            { "source_id": "src_two", "policy_readiness": "policy_ready" }
        ]);

        assert_eq!(count_json_array_records(&preview), 2);
    }

    #[test]
    fn local_action_result_prefers_generated_at_utc_for_athena_receipts() {
        let state_json = serde_json::json!({
            "generated_at_utc": "2026-05-28T06:06:00Z",
            "generated_at": "older",
            "status": "ready"
        });

        let result = local_action_result_from_state(
            &state_json,
            "ATHENA digest refreshed (2 entries)".to_string(),
            "core/state/athena_runtime.json".to_string(),
            "data/athena/digest.jsonl".to_string(),
        );

        assert!(result.ok);
        assert_eq!(result.generated_at, "2026-05-28T06:06:00Z");
        assert_eq!(result.receipt_path, "core/state/athena_runtime.json");
        assert_eq!(result.result_path, "data/athena/digest.jsonl");
        assert_eq!(result.message, "ATHENA digest refreshed (2 entries)");
    }

    #[test]
    fn resolves_relative_source_paths_inside_workspace() {
        let root = std::env::current_dir().expect("current dir should resolve");
        let root_string = root.to_string_lossy().to_string();
        let resolved = resolve_scoped_source_path(&root_string, "Cargo.toml")
            .expect("workspace Cargo.toml should resolve");
        assert!(resolved.starts_with(root));
    }

    #[test]
    fn rejects_source_paths_outside_workspace() {
        let root = std::env::current_dir().expect("current dir should resolve");
        let root_string = root.to_string_lossy().to_string();
        let error = resolve_scoped_source_path(&root_string, "/tmp")
            .expect_err("absolute path outside root should be rejected");
        assert!(error.contains("outside the Annunimas workspace"));
    }

    #[test]
    fn image_preview_mime_supports_common_safe_image_types() {
        assert_eq!(
            image_mime_for_path(Path::new("screen.png")),
            Some("image/png")
        );
        assert_eq!(
            image_mime_for_path(Path::new("photo.JPG")),
            Some("image/jpeg")
        );
        assert_eq!(
            image_mime_for_path(Path::new("clip.webp")),
            Some("image/webp")
        );
        assert_eq!(image_mime_for_path(Path::new("document.pdf")), None);
    }

    #[test]
    fn video_preview_mime_supports_browser_playable_video_types() {
        assert_eq!(
            video_mime_for_path(Path::new("capture.mp4")),
            Some("video/mp4")
        );
        assert_eq!(
            video_mime_for_path(Path::new("loop.WEBM")),
            Some("video/webm")
        );
        assert_eq!(
            video_mime_for_path(Path::new("reference.mov")),
            Some("video/quicktime")
        );
        assert_eq!(video_mime_for_path(Path::new("archive.mkv")), None);
    }

    #[test]
    fn pdf_preview_mime_supports_pdf_only() {
        assert_eq!(
            pdf_mime_for_path(Path::new("packet.PDF")),
            Some("application/pdf")
        );
        assert_eq!(pdf_mime_for_path(Path::new("packet.docx")), None);
    }
}

fn _get_memory_gb() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(kb) = line.split_whitespace().nth(1) {
                    if let Ok(kb_val) = kb.parse::<u64>() {
                        return kb_val as f64 / 1024.0 / 1024.0;
                    }
                }
            }
        }
    }
    0.0
}

#[tauri::command]
fn fetch_inventory_tree(
    numenor_path: String,
    relative_path: String,
    max_depth: Option<u8>,
) -> FileReadResult {
    let depth = max_depth.unwrap_or(4).clamp(1, 8);
    let target = match scoped_join(&numenor_path, &relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: relative_path,
            }
        }
    };

    if !target.exists() {
        return FileReadResult {
            success: false,
            content: None,
            error: Some("Path does not exist".to_string()),
            path: target.to_string_lossy().to_string(),
        };
    }

    let root = Path::new(&numenor_path);
    let tree = build_inventory_tree(root, &target, 0, depth);
    match serde_json::to_string(&tree) {
        Ok(payload) => FileReadResult {
            success: true,
            content: Some(payload),
            error: None,
            path: target.to_string_lossy().to_string(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: target.to_string_lossy().to_string(),
        },
    }
}

#[tauri::command]
fn create_scoped_folder(numenor_path: String, relative_path: String) -> FileReadResult {
    let target = match scoped_join(&numenor_path, &relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: relative_path,
            }
        }
    };

    match fs::create_dir_all(&target) {
        Ok(_) => FileReadResult {
            success: true,
            content: Some("Folder created".to_string()),
            error: None,
            path: target.to_string_lossy().to_string(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: target.to_string_lossy().to_string(),
        },
    }
}

#[tauri::command]
fn rename_scoped_path(
    numenor_path: String,
    old_relative_path: String,
    new_relative_path: String,
) -> FileReadResult {
    let old_path = match scoped_join(&numenor_path, &old_relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: old_relative_path,
            }
        }
    };
    let new_path = match scoped_join(&numenor_path, &new_relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: new_relative_path,
            }
        }
    };

    match fs::rename(&old_path, &new_path) {
        Ok(_) => FileReadResult {
            success: true,
            content: Some("Renamed".to_string()),
            error: None,
            path: new_path.to_string_lossy().to_string(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: old_path.to_string_lossy().to_string(),
        },
    }
}

#[tauri::command]
fn delete_scoped_path(
    numenor_path: String,
    relative_path: String,
    recursive: bool,
) -> FileReadResult {
    if relative_path == "." || relative_path.trim().is_empty() {
        return FileReadResult {
            success: false,
            content: None,
            error: Some("Refusing to delete root path".to_string()),
            path: relative_path,
        };
    }

    let target = match scoped_join(&numenor_path, &relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: relative_path,
            }
        }
    };

    let result = if target.is_dir() {
        if recursive {
            fs::remove_dir_all(&target)
        } else {
            fs::remove_dir(&target)
        }
    } else {
        fs::remove_file(&target)
    };

    match result {
        Ok(_) => FileReadResult {
            success: true,
            content: Some("Deleted".to_string()),
            error: None,
            path: target.to_string_lossy().to_string(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: target.to_string_lossy().to_string(),
        },
    }
}

#[tauri::command]
fn write_scoped_file(
    numenor_path: String,
    relative_path: String,
    content: String,
) -> FileReadResult {
    let target = match scoped_join(&numenor_path, &relative_path) {
        Ok(path) => path,
        Err(e) => {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e),
                path: relative_path,
            }
        }
    };

    if target.is_dir() {
        return FileReadResult {
            success: false,
            content: None,
            error: Some("Refusing to overwrite a directory".to_string()),
            path: target.to_string_lossy().to_string(),
        };
    }

    if let Some(parent) = target.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return FileReadResult {
                success: false,
                content: None,
                error: Some(e.to_string()),
                path: parent.to_string_lossy().to_string(),
            };
        }
    }

    match fs::write(&target, content) {
        Ok(_) => FileReadResult {
            success: true,
            content: Some("File written".to_string()),
            error: None,
            path: target.to_string_lossy().to_string(),
        },
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(e.to_string()),
            path: target.to_string_lossy().to_string(),
        },
    }
}

#[tauri::command]
fn approve_human_augmentation_action(
    numenor_path: String,
    decision_class: String,
    command_signature: Option<String>,
    approvers: Vec<String>,
    evidence: Vec<String>,
    expires_at_utc: Option<String>,
    note: Option<String>,
    status: Option<String>,
) -> FileReadResult {
    let build_root = Path::new(&numenor_path).join(".cache").join("arda-build");
    let target_dir = build_root.join("target");
    let tmp_dir = build_root.join("tmp");
    let _ = fs::create_dir_all(&target_dir);
    let _ = fs::create_dir_all(&tmp_dir);

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-cli")
        .arg("--")
        .arg("control")
        .arg("approve-human-augmentation")
        .arg(&decision_class)
        .current_dir(&numenor_path)
        .env("ARDA_BUILD_CACHE_ROOT", build_root.as_os_str())
        .env("CARGO_TARGET_DIR", target_dir.as_os_str())
        .env("TMPDIR", tmp_dir.as_os_str());

    if let Some(signature) = command_signature {
        if !signature.trim().is_empty() {
            command.arg("--command-signature").arg(signature);
        }
    }
    for approver in approvers {
        command.arg("--approver").arg(approver);
    }
    for item in evidence {
        command.arg("--evidence").arg(item);
    }
    if let Some(expires_at_utc) = expires_at_utc {
        if !expires_at_utc.trim().is_empty() {
            command.arg("--expires-at-utc").arg(expires_at_utc);
        }
    }
    if let Some(note) = note {
        if !note.trim().is_empty() {
            command.arg("--note").arg(note);
        }
    }
    if let Some(status) = status {
        if !status.trim().is_empty() {
            command.arg("--status").arg(status);
        }
    }

    match command.output() {
        Ok(result) => {
            if result.status.success() {
                FileReadResult {
                    success: true,
                    content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
                    error: None,
                    path: format!(
                        "{}/core/state/human_augmentation_runtime.json",
                        numenor_path
                    ),
                }
            } else {
                FileReadResult {
                    success: false,
                    content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
                    error: Some(String::from_utf8_lossy(&result.stderr).to_string()),
                    path: numenor_path,
                }
            }
        }
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(format!("failed to launch approval command: {}", e)),
            path: numenor_path,
        },
    }
}

#[tauri::command]
fn review_arandur_recommendation_action(
    numenor_path: String,
    recommendation_id: String,
    decision: String,
    reviewed_by: String,
    note: Option<String>,
) -> FileReadResult {
    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-aule")
        .arg("--features")
        .arg("full-cli")
        .arg("--bin")
        .arg("arda-cli")
        .arg("--")
        .arg("prometheus")
        .arg("autopilot")
        .arg("review-recommendation")
        .arg(&recommendation_id)
        .arg("--decision")
        .arg(&decision)
        .arg("--reviewed-by")
        .arg(&reviewed_by)
        .arg("--root")
        .arg(&numenor_path)
        .current_dir(&numenor_path);
    if let Some(note) = note.filter(|note| !note.trim().is_empty()) {
        command.arg("--note").arg(note);
    }
    match command.output() {
        Ok(result) if result.status.success() => FileReadResult {
            success: true,
            content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
            error: None,
            path: format!("{numenor_path}/data/arandur/recommendations.jsonl"),
        },
        Ok(result) => FileReadResult {
            success: false,
            content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
            error: Some(String::from_utf8_lossy(&result.stderr).to_string()),
            path: numenor_path,
        },
        Err(error) => FileReadResult {
            success: false,
            content: None,
            error: Some(format!(
                "failed to launch Arandur recommendation review: {error}"
            )),
            path: numenor_path,
        },
    }
}

fn run_approved_queue_cli(arda_root: String, args: &[&str]) -> Result<String, String> {
    if arda_root.trim().is_empty() {
        return Err("ARDA root is required".to_string());
    }
    let mut command = Command::new("arda-cli");
    command
        .arg("prometheus")
        .arg("autopilot")
        .args(args)
        .arg("--root")
        .arg(&arda_root)
        .current_dir(&arda_root);
    let output = run_bounded_command(command, Duration::from_secs(45), "approved queue action")?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
fn cancel_approved_queue_task_action(
    arda_root: String,
    task_id: String,
    reason: String,
) -> Result<String, String> {
    run_approved_queue_cli(
        arda_root,
        &["cancel-approved-task", &task_id, "--reason", &reason],
    )
}

#[tauri::command]
fn retry_approved_queue_task_action(arda_root: String, task_id: String) -> Result<String, String> {
    run_approved_queue_cli(arda_root, &["retry-approved-task", &task_id])
}

#[tauri::command]
fn record_ceo_council_session_action(
    numenor_path: String,
    objective: String,
    ceo_identity: Option<String>,
    cto_identity: Option<String>,
    cto_mode: Option<String>,
    ingress: Option<String>,
    channel_ref: Option<String>,
    loop_class: Option<String>,
    decision_class: Option<String>,
    triad_required: bool,
    participants: Vec<String>,
    proposals: Vec<String>,
    objections: Vec<String>,
    synthesis: Option<String>,
    outcome_status: Option<String>,
    human_escalated: bool,
    validators_invoked: Vec<String>,
    memory_lanes: Vec<String>,
    memory_writes: Vec<String>,
    promoted_private_memory: bool,
) -> FileReadResult {
    let build_root = Path::new(&numenor_path).join(".cache").join("arda-build");
    let target_dir = build_root.join("target");
    let tmp_dir = build_root.join("tmp");
    let _ = fs::create_dir_all(&target_dir);
    let _ = fs::create_dir_all(&tmp_dir);

    let mut command = Command::new("cargo");
    command
        .arg("run")
        .arg("-p")
        .arg("arda-cli")
        .arg("--")
        .arg("control")
        .arg("record-ceo-council-session")
        .arg(&objective)
        .current_dir(&numenor_path)
        .env("ARDA_BUILD_CACHE_ROOT", build_root.as_os_str())
        .env("CARGO_TARGET_DIR", target_dir.as_os_str())
        .env("TMPDIR", tmp_dir.as_os_str());

    if let Some(value) = ceo_identity {
        if !value.trim().is_empty() {
            command.arg("--ceo-identity").arg(value);
        }
    }
    if let Some(value) = cto_identity {
        if !value.trim().is_empty() {
            command.arg("--cto-identity").arg(value);
        }
    }
    if let Some(value) = cto_mode {
        if !value.trim().is_empty() {
            command.arg("--cto-mode").arg(value);
        }
    }
    if let Some(value) = ingress {
        if !value.trim().is_empty() {
            command.arg("--ingress").arg(value);
        }
    }
    if let Some(value) = channel_ref {
        if !value.trim().is_empty() {
            command.arg("--channel-ref").arg(value);
        }
    }
    if let Some(value) = loop_class {
        if !value.trim().is_empty() {
            command.arg("--loop-class").arg(value);
        }
    }
    if let Some(value) = decision_class {
        if !value.trim().is_empty() {
            command.arg("--decision-class").arg(value);
        }
    }
    if triad_required {
        command.arg("--triad-required");
    }
    for value in participants {
        command.arg("--participant").arg(value);
    }
    for value in proposals {
        command.arg("--proposal").arg(value);
    }
    for value in objections {
        command.arg("--objection").arg(value);
    }
    if let Some(value) = synthesis {
        if !value.trim().is_empty() {
            command.arg("--synthesis").arg(value);
        }
    }
    if let Some(value) = outcome_status {
        if !value.trim().is_empty() {
            command.arg("--outcome-status").arg(value);
        }
    }
    if human_escalated {
        command.arg("--human-escalated");
    }
    for value in validators_invoked {
        command.arg("--validator").arg(value);
    }
    for value in memory_lanes {
        command.arg("--memory-lane").arg(value);
    }
    for value in memory_writes {
        command.arg("--memory-write").arg(value);
    }
    if promoted_private_memory {
        command.arg("--promoted-private-memory");
    }

    match command.output() {
        Ok(result) => {
            if result.status.success() {
                FileReadResult {
                    success: true,
                    content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
                    error: None,
                    path: format!("{}/core/state/ceo_council_runtime.json", numenor_path),
                }
            } else {
                FileReadResult {
                    success: false,
                    content: Some(String::from_utf8_lossy(&result.stdout).to_string()),
                    error: Some(String::from_utf8_lossy(&result.stderr).to_string()),
                    path: numenor_path,
                }
            }
        }
        Err(e) => FileReadResult {
            success: false,
            content: None,
            error: Some(format!("failed to launch CEO council command: {}", e)),
            path: numenor_path,
        },
    }
}

#[tauri::command]
fn start_hud_pulse_stream(
    app: AppHandle,
    state: State<'_, HudPulseStreamState>,
    interval_ms: Option<u64>,
) -> Result<String, String> {
    if state.running.load(Ordering::SeqCst) {
        return Ok("hud pulse stream already running".to_string());
    }

    let interval = interval_ms.unwrap_or(15000).max(1000);
    state.running.store(true, Ordering::SeqCst);

    let running_flag = Arc::clone(&state.running);
    let app_handle = app.clone();

    let handle = thread::spawn(move || {
        let mut sequence: u64 = 0;
        while running_flag.load(Ordering::SeqCst) {
            let ts_unix_ms = std::time::SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let payload = HudPulseEvent {
                ts_unix_ms,
                status: "healthy".to_string(),
                source: "tauri".to_string(),
                sequence,
            };

            let _ = app_handle.emit("arda://hud-pulse", payload);
            sequence = sequence.saturating_add(1);
            thread::sleep(Duration::from_millis(interval));
        }
    });

    let mut guard = state
        .handle
        .lock()
        .map_err(|_| "hud pulse stream mutex poisoned".to_string())?;
    if let Some(prev) = guard.take() {
        let _ = prev.join();
    }
    *guard = Some(handle);

    Ok(format!("hud pulse stream started @ {}ms", interval))
}

#[tauri::command]
fn stop_hud_pulse_stream(state: State<'_, HudPulseStreamState>) -> Result<String, String> {
    if !state.running.load(Ordering::SeqCst) {
        return Ok("hud pulse stream already stopped".to_string());
    }

    state.running.store(false, Ordering::SeqCst);

    let mut guard = state
        .handle
        .lock()
        .map_err(|_| "hud pulse stream mutex poisoned".to_string())?;
    if let Some(handle) = guard.take() {
        let _ = handle.join();
    }

    Ok("hud pulse stream stopped".to_string())
}

fn hermes_runtime_config() -> HermesRuntimeConfig {
    HermesRuntimeConfig::from_env()
}

fn hermes_runtime_url() -> String {
    hermes_runtime_config().url()
}

fn hermes_runtime_port_open(config: &HermesRuntimeConfig) -> bool {
    let Ok(addr) = config.socket_addr() else {
        return false;
    };
    TcpStream::connect_timeout(&addr, Duration::from_millis(350)).is_ok()
}

fn hermes_runtime_probe(config: &HermesRuntimeConfig) -> Result<String, String> {
    let addr = config.socket_addr()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(700))
        .map_err(|error| format!("Hermes runtime port probe failed: {}", error))?;
    stream
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|error| format!("failed to set Hermes runtime read timeout: {}", error))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(700)))
        .map_err(|error| format!("failed to set Hermes runtime write timeout: {}", error))?;

    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: arda-hud-hermes-probe\r\nAccept: text/html,*/*\r\nConnection: close\r\n\r\n",
        config.host, config.port
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("Hermes runtime HTTP probe write failed: {}", error))?;

    let mut buffer = Vec::with_capacity(8192);
    let mut chunk = [0_u8; 2048];
    while buffer.len() < 8192 {
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(error) => {
                if buffer.is_empty() {
                    return Err(format!("Hermes runtime HTTP probe read failed: {}", error));
                }
                break;
            }
        }
    }

    Ok(String::from_utf8_lossy(&buffer).to_string())
}

fn hermes_runtime_identity_verified(config: &HermesRuntimeConfig) -> bool {
    let Ok(response) = hermes_runtime_probe(config) else {
        return false;
    };
    let lower = response.to_ascii_lowercase();
    (response.starts_with("HTTP/1.1 200") || response.starts_with("HTTP/1.0 200"))
        && (lower.contains("hermes agent dashboard")
            || (lower.contains("hermes") && lower.contains("dashboard")))
}

fn hermes_runtime_owned_process_running(state: &HermesRuntimeState) -> bool {
    let Ok(mut guard) = state.child.lock() else {
        return false;
    };
    let Some(child) = guard.as_mut() else {
        return false;
    };
    match child.try_wait() {
        Ok(Some(_status)) => {
            *guard = None;
            false
        }
        Ok(None) => true,
        Err(_error) => {
            *guard = None;
            false
        }
    }
}

fn read_hermes_runtime_status_inner(state: &HermesRuntimeState) -> HermesRuntimeStatus {
    let config = hermes_runtime_config();
    let port_open = hermes_runtime_port_open(&config);
    let identity_verified = port_open && hermes_runtime_identity_verified(&config);
    let owned_process_running = hermes_runtime_owned_process_running(state);
    let (state_label, message) = if identity_verified {
        let mode = if owned_process_running {
            "ARDA-owned Hermes runtime process is running"
        } else {
            "Verified existing Hermes runtime listener is available"
        };
        ("ready", format!("{mode}: {}", config.url()))
    } else if port_open {
        (
            "blocked",
            format!(
                "Port {} is listening, but it did not identify as Hermes runtime",
                config.port
            ),
        )
    } else if owned_process_running {
        (
            "starting",
            format!(
                "ARDA-owned Hermes runtime process is running, waiting for {}",
                config.url()
            ),
        )
    } else {
        (
            "offline",
            format!("Hermes runtime is not listening at {}", config.url()),
        )
    };
    HermesRuntimeStatus {
        url: config.url(),
        host: config.host,
        port: config.port,
        port_open,
        identity_verified,
        owned_process_running,
        state: state_label.to_string(),
        message,
    }
}

fn wait_for_hermes_runtime_ready(config: &HermesRuntimeConfig, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if hermes_runtime_identity_verified(config) {
            return true;
        }
        thread::sleep(Duration::from_millis(150));
    }
    false
}

fn ensure_hermes_runtime_process(state: &HermesRuntimeState) -> Result<(bool, bool), String> {
    let config = hermes_runtime_config();
    if hermes_runtime_port_open(&config) {
        if hermes_runtime_identity_verified(&config) {
            return Ok((false, true));
        }
        return Err(format!(
            "Port {} is already listening, but it did not identify as Hermes runtime; refusing to attach",
            config.port
        ));
    }

    let mut guard = state
        .child
        .lock()
        .map_err(|_| "hermes runtime process mutex poisoned".to_string())?;

    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_status)) => {
                *guard = None;
            }
            Ok(None) => {
                drop(guard);
                if wait_for_hermes_runtime_ready(&config, Duration::from_secs(6)) {
                    return Ok((false, false));
                }
                return Err(format!(
                    "Hermes runtime process is running but port {} did not become ready",
                    config.port
                ));
            }
            Err(error) => {
                *guard = None;
                return Err(format!(
                    "Failed to inspect Hermes runtime process: {}",
                    error
                ));
            }
        }
    }

    let port = config.port.to_string();
    let child = Command::new("hermes")
        .args([
            "dashboard",
            "--host",
            config.host.as_str(),
            "--port",
            &port,
            "--no-open",
            "--skip-build",
            "--tui",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Failed to launch `hermes runtime`: {}", error))?;

    *guard = Some(child);
    drop(guard);

    if wait_for_hermes_runtime_ready(&config, Duration::from_secs(12)) {
        Ok((true, false))
    } else {
        state.cleanup_owned_child();
        Err(format!(
            "Launched `hermes runtime`, but {} did not become ready or did not identify as Hermes Agent Runtime",
            config.url()
        ))
    }
}

#[tauri::command]
fn read_hermes_runtime_status(
    state: State<'_, HermesRuntimeState>,
) -> Result<HermesRuntimeStatus, String> {
    Ok(read_hermes_runtime_status_inner(&state))
}

#[tauri::command]
async fn ensure_hermes_runtime_surface(
    state: State<'_, HermesRuntimeState>,
) -> Result<HermesRuntimeWindowResult, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let (launched_process, already_listening) = ensure_hermes_runtime_process(&state)?;
        let config = hermes_runtime_config();
        Ok(HermesRuntimeWindowResult {
            window_label: HERMES_RUNTIME_WINDOW_LABEL.to_string(),
            url: config.url(),
            port: config.port,
            launched_process,
            already_listening,
        })
    })
    .await
    .map_err(|error| format!("Hermes runtime readiness task failed: {error}"))?
}

#[tauri::command]
fn open_hermes_runtime_window(
    app: AppHandle,
    state: State<'_, HermesRuntimeState>,
) -> Result<HermesRuntimeWindowResult, String> {
    if let Some(window) = app.get_webview_window(HERMES_RUNTIME_WINDOW_LABEL) {
        let (_launched_process, already_listening) = ensure_hermes_runtime_process(&state)?;
        let config = hermes_runtime_config();
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(HermesRuntimeWindowResult {
            window_label: HERMES_RUNTIME_WINDOW_LABEL.to_string(),
            url: config.url(),
            port: config.port,
            launched_process: false,
            already_listening,
        });
    }

    let (launched_process, already_listening) = ensure_hermes_runtime_process(&state)?;
    let config = hermes_runtime_config();
    let url = config.url();
    let parsed_url = tauri::Url::parse(&url).map_err(|error| error.to_string())?;

    let mut builder = WebviewWindowBuilder::new(
        &app,
        HERMES_RUNTIME_WINDOW_LABEL,
        WebviewUrl::External(parsed_url),
    )
    .title("Hermes Dashboard — ARDA")
    .inner_size(1240.0, 860.0)
    .resizable(true)
    .focused(true)
    .decorations(true);

    if let Some(main) = app.get_webview_window("main") {
        if let Ok(position) = main.outer_position() {
            builder = builder.position(position.x as f64 + 96.0, position.y as f64 + 64.0);
        } else {
            builder = builder.center();
        }
    } else {
        builder = builder.center();
    }

    let window = builder.build().map_err(|e| e.to_string())?;
    let _ = window.emit(
        "hermes-runtime-sync",
        serde_json::json!({
            "url": url,
            "port": config.port,
            "launchedProcess": launched_process,
            "alreadyListening": already_listening,
        }),
    );

    Ok(HermesRuntimeWindowResult {
        window_label: HERMES_RUNTIME_WINDOW_LABEL.to_string(),
        url,
        port: config.port,
        launched_process,
        already_listening,
    })
}

fn hermes_terminal_window_path() -> &'static str {
    HERMES_TERMINAL_WINDOW_PATH
}

#[tauri::command]
fn open_hermes_terminal_window(app: AppHandle) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(HERMES_TERMINAL_WINDOW_LABEL) {
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus().map_err(|error| error.to_string())?;
        return Ok(HERMES_TERMINAL_WINDOW_LABEL.to_string());
    }

    let mut builder = WebviewWindowBuilder::new(
        &app,
        HERMES_TERMINAL_WINDOW_LABEL,
        WebviewUrl::App(hermes_terminal_window_path().into()),
    )
    .title("Hermes CLI — ARDA")
    .inner_size(1040.0, 680.0)
    .min_inner_size(720.0, 420.0)
    .resizable(true)
    .focused(true)
    .decorations(true);

    if let Some(main) = app.get_webview_window("main") {
        if let Ok(position) = main.outer_position() {
            builder = builder.position(position.x as f64 + 120.0, position.y as f64 + 88.0);
        } else {
            builder = builder.center();
        }
    } else {
        builder = builder.center();
    }

    builder.build().map_err(|error| error.to_string())?;
    Ok(HERMES_TERMINAL_WINDOW_LABEL.to_string())
}

#[tauri::command]
fn open_workstation_window(
    app: AppHandle,
    state: State<'_, HermesRuntimeState>,
    request: WorkstationWindowRequest,
) -> Result<String, String> {
    if let Some(window) = app.get_webview_window(&request.window_label) {
        if request.workstation_id == "hermes_runtime_workstation"
            || request.source_zone_id.as_deref() == Some("hermes_runtime")
        {
            let _ = ensure_hermes_runtime_process(&state)?;
        }
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(request.window_label);
    }

    let section = request.source_zone_id.unwrap_or_default();
    let anchor = request.origin_anchor_id.unwrap_or_default();
    let is_hermes_runtime_workstation =
        request.workstation_id == "hermes_runtime_workstation" || section == "hermes_runtime";
    let webview_url = if is_hermes_runtime_workstation {
        let _ = ensure_hermes_runtime_process(&state)?;
        let url = hermes_runtime_url();
        let parsed_url = tauri::Url::parse(&url).map_err(|error| error.to_string())?;
        WebviewUrl::External(parsed_url)
    } else {
        let path = format!(
            "index.html?__view=panel&__windowId={}&__windowRole=workstation&__workstation={}&__section={}&__anchor={}&__presentation={}",
            request.window_label,
            request.workstation_id,
            section,
            anchor,
            request.presentation_mode,
        );
        WebviewUrl::App(path.into())
    };

    let mut builder = WebviewWindowBuilder::new(&app, request.window_label.clone(), webview_url)
        .title(request.title)
        .inner_size(request.width, request.height)
        .resizable(true)
        .focused(true)
        .decorations(true);

    if let Some(main) = app.get_webview_window("main") {
        builder = builder.center();
        if let Ok(is_fullscreen) = main.is_fullscreen() {
            if is_fullscreen {
                builder = builder.fullscreen(false);
            }
        }
    } else {
        builder = builder.center();
    }

    let window = builder.build().map_err(|e| e.to_string())?;
    let _ = window.emit(
        "workstation-sync",
        serde_json::json!({
            "workstationId": request.workstation_id,
            "sourceZoneId": section,
            "originAnchorId": anchor,
            "presentationMode": request.presentation_mode,
            "subtitle": request.subtitle,
        }),
    );
    Ok(request.window_label)
}

const MIRROMERE_WINDOW_LABEL: &str = "arda-mirromere";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MirromereDisplayGeometry {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MirromereDisplayObservation {
    display_id: String,
    geometry: MirromereDisplayGeometry,
    primary: bool,
    connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MirromereWindowRequest {
    selected_display_id: String,
    allow_primary_fallback: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MirromereDisplayResolution {
    available: bool,
    message: String,
    display_id: Option<String>,
    geometry: Option<MirromereDisplayGeometry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MirromereWindowResult {
    available: bool,
    status: String,
    message: String,
    window_label: Option<String>,
    display_id: Option<String>,
    geometry: Option<MirromereDisplayGeometry>,
}

fn resolve_mirromere_display(
    request: &MirromereWindowRequest,
    displays: &[MirromereDisplayObservation],
) -> MirromereDisplayResolution {
    let matches = displays
        .iter()
        .filter(|display| display.display_id == request.selected_display_id)
        .collect::<Vec<_>>();
    let Some(display) = matches.first().copied() else {
        return MirromereDisplayResolution {
            available: false,
            message: "Selected Mirromere display is not connected.".to_string(),
            display_id: Some(request.selected_display_id.clone()),
            geometry: None,
        };
    };
    if matches.len() > 1 {
        return MirromereDisplayResolution {
            available: false,
            message: "Selected Mirromere display identity is ambiguous.".to_string(),
            display_id: Some(request.selected_display_id.clone()),
            geometry: None,
        };
    }
    if !display.connected {
        return MirromereDisplayResolution {
            available: false,
            message: "Selected Mirromere display disconnected.".to_string(),
            display_id: Some(display.display_id.clone()),
            geometry: None,
        };
    }
    if display.primary && !request.allow_primary_fallback {
        return MirromereDisplayResolution {
            available: false,
            message: "Primary-display fallback is disabled for Mirromere.".to_string(),
            display_id: Some(display.display_id.clone()),
            geometry: None,
        };
    }
    MirromereDisplayResolution {
        available: true,
        message: "Selected Mirromere display is available.".to_string(),
        display_id: Some(display.display_id.clone()),
        geometry: Some(display.geometry.clone()),
    }
}

fn stable_monitor_id(monitor: &tauri::Monitor) -> Option<String> {
    monitor
        .name()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| format!("display:{name}"))
}

fn monitors_match(left: &tauri::Monitor, right: &tauri::Monitor) -> bool {
    left.name() == right.name()
        && left.position() == right.position()
        && left.size() == right.size()
}

fn observe_mirromere_displays(app: &AppHandle) -> Result<Vec<MirromereDisplayObservation>, String> {
    let monitors = app
        .available_monitors()
        .map_err(|error| error.to_string())?;
    let primary = app.primary_monitor().map_err(|error| error.to_string())?;
    Ok(monitors
        .iter()
        .filter_map(|monitor| {
            let display_id = stable_monitor_id(monitor)?;
            Some(MirromereDisplayObservation {
                display_id,
                geometry: MirromereDisplayGeometry {
                    x: monitor.position().x,
                    y: monitor.position().y,
                    width: monitor.size().width,
                    height: monitor.size().height,
                },
                primary: primary
                    .as_ref()
                    .is_some_and(|primary_monitor| monitors_match(monitor, primary_monitor)),
                connected: true,
            })
        })
        .collect())
}

#[tauri::command]
fn list_mirromere_displays(app: AppHandle) -> Result<Vec<MirromereDisplayObservation>, String> {
    observe_mirromere_displays(&app)
}

#[tauri::command]
fn open_mirromere_window(
    app: AppHandle,
    request: MirromereWindowRequest,
) -> Result<MirromereWindowResult, String> {
    let displays = observe_mirromere_displays(&app)?;
    let resolution = resolve_mirromere_display(&request, &displays);
    if !resolution.available {
        if let Some(window) = app.get_webview_window(MIRROMERE_WINDOW_LABEL) {
            let _ = window.close();
        }
        return Ok(MirromereWindowResult {
            available: false,
            status: "unavailable".to_string(),
            message: resolution.message,
            window_label: None,
            display_id: resolution.display_id,
            geometry: None,
        });
    }

    let geometry = resolution
        .geometry
        .clone()
        .ok_or_else(|| "Resolved Mirromere display has no geometry.".to_string())?;
    let position = Position::Physical(PhysicalPosition::new(geometry.x, geometry.y));
    let size = Size::Physical(PhysicalSize::new(geometry.width, geometry.height));
    let status = if let Some(window) = app.get_webview_window(MIRROMERE_WINDOW_LABEL) {
        window
            .set_position(position)
            .map_err(|error| error.to_string())?;
        window.set_size(size).map_err(|error| error.to_string())?;
        let _ = window.unminimize();
        window.show().map_err(|error| error.to_string())?;
        "updated"
    } else {
        let path = "index.html?__view=mirromere&__windowId=arda-mirromere&__windowRole=mirromere";
        let window =
            WebviewWindowBuilder::new(&app, MIRROMERE_WINDOW_LABEL, WebviewUrl::App(path.into()))
                .title("Mirromere")
                .decorations(false)
                .resizable(false)
                .focused(false)
                .visible(false)
                .build()
                .map_err(|error| error.to_string())?;
        window
            .set_position(position)
            .map_err(|error| error.to_string())?;
        window.set_size(size).map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        "opened"
    };

    Ok(MirromereWindowResult {
        available: true,
        status: status.to_string(),
        message: resolution.message,
        window_label: Some(MIRROMERE_WINDOW_LABEL.to_string()),
        display_id: resolution.display_id,
        geometry: Some(geometry),
    })
}

#[cfg(test)]
mod mirromere_window_tests {
    use super::*;

    fn display(
        display_id: &str,
        primary: bool,
        connected: bool,
        geometry: MirromereDisplayGeometry,
    ) -> MirromereDisplayObservation {
        MirromereDisplayObservation {
            display_id: display_id.to_string(),
            geometry,
            primary,
            connected,
        }
    }

    fn request(display_id: &str) -> MirromereWindowRequest {
        MirromereWindowRequest {
            selected_display_id: display_id.to_string(),
            allow_primary_fallback: false,
        }
    }

    fn geometry(x: i32, width: u32) -> MirromereDisplayGeometry {
        MirromereDisplayGeometry {
            x,
            y: 0,
            width,
            height: 1080,
        }
    }

    #[test]
    fn resolves_requested_connected_secondary_display() {
        let resolved = resolve_mirromere_display(
            &request("display:secondary"),
            &[display(
                "display:secondary",
                false,
                true,
                geometry(1920, 1920),
            )],
        );
        assert!(resolved.available);
        assert_eq!(resolved.geometry, Some(geometry(1920, 1920)));
    }

    #[test]
    fn rejects_absent_requested_display() {
        let resolved = resolve_mirromere_display(&request("display:missing"), &[]);
        assert!(!resolved.available);
        assert!(resolved.geometry.is_none());
    }

    #[test]
    fn rejects_disconnected_selected_display() {
        let resolved = resolve_mirromere_display(
            &request("display:secondary"),
            &[display(
                "display:secondary",
                false,
                false,
                geometry(1920, 1920),
            )],
        );
        assert!(!resolved.available);
        assert!(resolved.message.contains("disconnected"));
    }

    #[test]
    fn rejects_primary_when_fallback_is_disabled() {
        let resolved = resolve_mirromere_display(
            &request("display:primary"),
            &[display("display:primary", true, true, geometry(0, 1920))],
        );
        assert!(!resolved.available);
        assert!(resolved.message.contains("fallback is disabled"));
    }

    #[test]
    fn uses_current_geometry_for_same_stable_display_id() {
        let resolved = resolve_mirromere_display(
            &request("display:secondary"),
            &[display(
                "display:secondary",
                false,
                true,
                geometry(-2560, 2560),
            )],
        );
        assert_eq!(resolved.geometry, Some(geometry(-2560, 2560)));
    }
}

fn resolve_window(
    app: &AppHandle,
    window_label: Option<String>,
) -> Result<tauri::WebviewWindow, String> {
    let label = window_label.unwrap_or_else(|| "main".to_string());
    app.get_webview_window(&label)
        .ok_or_else(|| format!("Window '{}' not found", label))
}

#[tauri::command]
fn close_window(app: AppHandle, window_label: Option<String>) -> Result<(), String> {
    let window = resolve_window(&app, window_label)?;
    window.close().map_err(|e| e.to_string())
}

#[tauri::command]
fn minimize_window(app: AppHandle, window_label: Option<String>) -> Result<(), String> {
    let window = resolve_window(&app, window_label)?;
    window.minimize().map_err(|e| e.to_string())
}

#[tauri::command]
async fn start_dragging(app: AppHandle, window_label: Option<String>) -> Result<(), String> {
    let window = resolve_window(&app, window_label)?;
    window.start_dragging().map_err(|e| e.to_string())
}

#[tauri::command]
fn toggle_fullscreen(app: AppHandle, window_label: Option<String>) -> Result<(), String> {
    let window = resolve_window(&app, window_label)?;
    window
        .set_fullscreen(!window.is_fullscreen().map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

struct AppState {
    pty_pair: Arc<AsyncMutex<portable_pty::PtyPair>>,
    writer: Arc<AsyncMutex<Box<dyn Write + Send>>>,
    reader: Arc<AsyncMutex<Box<dyn BufRead + Send>>>,
}

#[tauri::command]
async fn async_create_shell(state: State<'_, AppState>) -> Result<(), String> {
    state
        .pty_pair
        .lock()
        .await
        .slave
        .spawn_command(CommandBuilder::new("bash"))
        .map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
async fn async_write_to_pty(data: &str, state: State<'_, AppState>) -> Result<(), String> {
    write!(state.writer.lock().await, "{data}").map_err(|error| error.to_string())
}

#[tauri::command]
async fn async_read_from_pty(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let mut reader = state.reader.lock().await;
    let data = {
        let data = reader.fill_buf().map_err(|error| error.to_string())?;
        if data.is_empty() {
            return Ok(None);
        }
        data.to_vec()
    };
    let text = String::from_utf8(data.clone()).map_err(|error| error.to_string())?;
    reader.consume(data.len());
    Ok(Some(text))
}

#[tauri::command]
async fn async_resize_pty(rows: u16, cols: u16, state: State<'_, AppState>) -> Result<(), String> {
    state
        .pty_pair
        .lock()
        .await
        .master
        .resize(portable_pty::PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| error.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SurfaceBridgeResult {
    ok: bool,
    message: String,
    window_label: Option<String>,
    source_zone_id: String,
    slot_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitorSurfaceRequest {
    slot_id: String,
    source_zone_id: String,
    focus_mode: String,
    title: Option<String>,
    width: Option<f64>,
    height: Option<f64>,
}

pub(crate) fn surface_bridge_window_label(slot_id: &str) -> String {
    format!("arda-monitor-surface--{slot_id}")
}

fn is_allowed_surface_source(source_zone_id: &str) -> bool {
    if source_zone_id == "hermes_runtime" {
        return true;
    }
    if source_zone_id.starts_with("service_") {
        return true;
    }
    source_zone_id.starts_with("monitor_left_") || source_zone_id.starts_with("view_desk_")
}

pub(crate) fn is_allowed_focus_mode(mode: &str) -> bool {
    matches!(
        mode,
        "in_scene_workstation" | "native_window" | "external_browser" | "remote_preview"
    )
}

#[tauri::command]
fn create_monitor_surface(
    app: AppHandle,
    state: State<'_, HermesRuntimeState>,
    request: MonitorSurfaceRequest,
) -> Result<SurfaceBridgeResult, String> {
    if !is_allowed_focus_mode(&request.focus_mode) {
        return Ok(SurfaceBridgeResult {
            ok: false,
            message: format!(
                "Surface focus mode '{}' is not permitted for monitor surfaces",
                request.focus_mode
            ),
            window_label: None,
            source_zone_id: request.source_zone_id,
            slot_id: request.slot_id,
        });
    }

    if !is_allowed_surface_source(&request.source_zone_id) {
        return Ok(SurfaceBridgeResult {
            ok: false,
            message: format!(
                "Source zone '{}' is not permitted for monitor surface creation",
                request.source_zone_id
            ),
            window_label: None,
            source_zone_id: request.source_zone_id,
            slot_id: request.slot_id,
        });
    }

    let window_label = surface_bridge_window_label(&request.slot_id);

    if let Some(window) = app.get_webview_window(&window_label) {
        let _ = window.unminimize();
        let _ = window.show();
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(SurfaceBridgeResult {
            ok: true,
            message: "Monitor surface window already exists; focused.".to_string(),
            window_label: Some(window_label),
            source_zone_id: request.source_zone_id.clone(),
            slot_id: request.slot_id.clone(),
        });
    }

    // For hermes_runtime source, ensure the runtime process is available.
    if request.source_zone_id == "hermes_runtime" {
        let _ = ensure_hermes_runtime_process(&state)?;
    }

    let title = request
        .title
        .unwrap_or_else(|| format!("ARDA Monitor — {}", request.slot_id));
    let width = request.width.unwrap_or(820.0);
    let height = request.height.unwrap_or(560.0);

    let webview_url = if request.source_zone_id == "hermes_runtime" {
        let config = hermes_runtime_config();
        let parsed_url = tauri::Url::parse(&config.url()).map_err(|error| error.to_string())?;
        WebviewUrl::External(parsed_url)
    } else {
        let path = format!(
            "index.html?__view=panel&__windowId={}&__windowRole=monitor&__slot={}&__source={}&#38;",
            window_label, request.slot_id, request.source_zone_id,
        );
        WebviewUrl::App(path.into())
    };

    let mut builder = WebviewWindowBuilder::new(&app, window_label.clone(), webview_url)
        .title(title)
        .inner_size(width, height)
        .resizable(true)
        .focused(true)
        .decorations(true);

    if let Some(main) = app.get_webview_window("main") {
        builder = builder.center();
        if let Ok(is_fullscreen) = main.is_fullscreen() {
            if is_fullscreen {
                builder = builder.fullscreen(false);
            }
        }
    } else {
        builder = builder.center();
    }

    let window = builder
        .build()
        .map_err(|error| format!("failed to create monitor surface window: {error}"))?;

    let _ = window.emit(
        "monitor-surface-sync",
        serde_json::json!({
            "slotId": request.slot_id,
            "sourceZoneId": request.source_zone_id,
            "focusMode": request.focus_mode,
            "windowLabel": &window_label,
        }),
    );

    Ok(SurfaceBridgeResult {
        ok: true,
        message: format!(
            "Created monitor surface for slot '{}' (source: {})",
            request.slot_id, request.source_zone_id
        ),
        window_label: Some(window_label),
        source_zone_id: request.source_zone_id.clone(),
        slot_id: request.slot_id.clone(),
    })
}

#[tauri::command]
fn dismiss_monitor_surface(app: AppHandle, window_label: Option<String>) -> Result<String, String> {
    let label = window_label
        .as_deref()
        .ok_or_else(|| "window_label is required".to_string())?;
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Monitor surface window '{}' not found", label))?;
    window.close().map_err(|error| error.to_string())?;
    Ok(format!("Dismissed monitor surface window '{}'", label))
}

#[cfg(test)]
mod surface_bridge_tests {
    use super::*;

    #[test]
    fn test_is_allowed_focus_mode() {
        assert!(is_allowed_focus_mode("in_scene_workstation"));
        assert!(is_allowed_focus_mode("native_window"));
        assert!(is_allowed_focus_mode("external_browser"));
        assert!(is_allowed_focus_mode("remote_preview"));
        assert!(!is_allowed_focus_mode("bogus_mode"));
        assert!(!is_allowed_focus_mode(""));
    }

    #[test]
    fn test_is_allowed_surface_source() {
        assert!(is_allowed_surface_source("hermes_runtime"));
        assert!(is_allowed_surface_source("service_beelink_grafana"));
        assert!(is_allowed_surface_source("monitor_left_1"));
        assert!(is_allowed_surface_source("view_desk_aux"));
        assert!(!is_allowed_surface_source("arbitrary_string"));
        assert!(!is_allowed_surface_source("settings"));
    }

    #[test]
    fn test_surface_bridge_window_label() {
        assert_eq!(
            surface_bridge_window_label("monitor_left_1"),
            "arda-monitor-surface--monitor_left_1"
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let pty_system = portable_pty::native_pty_system();
    let pty_pair = pty_system
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("failed to open pty");
    let reader = pty_pair
        .master
        .try_clone_reader()
        .expect("failed to clone pty reader");
    let writer = pty_pair
        .master
        .take_writer()
        .expect("failed to take pty writer");

    tauri::Builder::default()
        .manage(AppState {
            pty_pair: Arc::new(AsyncMutex::new(pty_pair)),
            writer: Arc::new(AsyncMutex::new(writer)),
            reader: Arc::new(AsyncMutex::new(Box::new(BufReader::new(reader)))),
        })
        .manage(HudPulseStreamState::default())
        .manage(HermesRuntimeState::default())
        .manage(WorkbenchEventStreamState::default())
        .manage(MonitorSurfaceState::default())
        .manage(TypedMonitorSurfaceState::new())
        .manage(BrowserCaptureState::default())
        .manage(PtyCaptureState::default())
        .manage(mirromere::MirromereInteractionReceiptState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            validate_project_contract,
            attach_project_contract,
            plan_workbench_run,
            approve_workbench_run,
            complete_workbench_run_node,
            execute_workbench_provider_node,
            cancel_workbench_run,
            get_workbench_run,
            get_workbench_run_events,
            start_workbench_run_event_stream,
            read_file,
            get_arda_root,
            get_hud_render_context,
            get_numenor_path,
            run_chronos_provider_checks,
            run_charon_provider_intelligence_refresh,
            run_queue_cleanup_preview,
            run_hades_recurring_maintenance,
            run_repeated_audit_preview,
            run_setup_readiness_check,
            run_setup_repair_preflight,
            run_setup_repair_execution_gate,
            read_charon_json,
            run_athena_knowledge_ingestion,
            run_athena_digest_refresh,
            run_athena_policy_readiness_preview,
            reveal_source_path,
            open_source_path,
            read_source_image_preview,
            read_source_video_preview,
            read_source_pdf_preview,
            fetch_inventory_tree,
            create_scoped_folder,
            rename_scoped_path,
            delete_scoped_path,
            write_scoped_file,
            approve_human_augmentation_action,
            review_arandur_recommendation_action,
            cancel_approved_queue_task_action,
            retry_approved_queue_task_action,
            record_ceo_council_session_action,
            start_hud_pulse_stream,
            stop_hud_pulse_stream,
            ensure_hermes_runtime_surface,
            read_hermes_runtime_status,
            open_hermes_runtime_window,
            open_hermes_terminal_window,
            open_workstation_window,
            list_mirromere_displays,
            open_mirromere_window,
            mirromere::get_mirromere_surface,
            mirromere::request_mirromere_interaction,
            close_window,
            minimize_window,
            start_dragging,
            toggle_fullscreen,
            async_create_shell,
            async_write_to_pty,
            async_read_from_pty,
            async_resize_pty,
            create_monitor_surface,
            dismiss_monitor_surface,
            claim_monitor_slot,
            release_monitor_slot,
            push_surface_payload,
            refresh_monitor_slot_lease,
            claim_monitor_surface,
            release_monitor_surface,
            refresh_monitor_surface_lease,
            patch_monitor_surface_playback,
            get_monitor_surface_registry,
            restore_monitor_surface_registry,
            start_browser_capture,
            navigate_browser_capture,
            click_browser_capture,
            scroll_browser_capture,
            type_browser_capture,
            key_browser_capture,
            get_browser_capture_frame,
            get_browser_capture_status,
            stop_browser_capture,
            start_pty_capture,
            get_pty_capture_status,
            write_pty_capture,
            stop_pty_capture,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("error while building tauri application: {error}");
            std::process::exit(1);
        })
        .run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
                app_handle.state::<BrowserCaptureState>().cleanup_all();
                app_handle.state::<PtyCaptureState>().cleanup_all();
                app_handle
                    .state::<HermesRuntimeState>()
                    .cleanup_owned_child();
            }
        });
}
