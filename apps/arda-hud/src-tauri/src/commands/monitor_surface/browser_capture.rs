use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use tauri::State;
use tungstenite::protocol::Role;
use tungstenite::{Message, WebSocket};

const BROWSER_START_TIMEOUT: Duration = Duration::from_secs(20);
const FRAME_WAIT_SLICE: Duration = Duration::from_millis(500);
const MJPEG_BOUNDARY: &str = "arda-frame";
const CAPTURE_WIDTH: f64 = 1280.0;
const CAPTURE_HEIGHT: f64 = 720.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserRuntime {
    Direct(String),
    FlatpakBraveFiles(String),
    FlatpakBrave,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserLaunchPlan {
    pub program: String,
    pub args: Vec<String>,
    pub profile_path: String,
}

impl BrowserLaunchPlan {
    pub fn new(
        runtime: BrowserRuntime,
        session_id: &str,
        url: &str,
        cdp_port: u16,
    ) -> Result<Self, String> {
        validate_session_id(session_id)?;
        validate_http_url(url)?;
        let profile_path = format!("/tmp/arda-hud-browser-{session_id}");
        let (program, mut args, needs_no_sandbox) = match runtime {
            BrowserRuntime::Direct(program) => (program, Vec::new(), false),
            BrowserRuntime::FlatpakBraveFiles(program) => (program, Vec::new(), true),
            BrowserRuntime::FlatpakBrave => (
                "flatpak".to_string(),
                vec![
                    "run".to_string(),
                    "--command=brave".to_string(),
                    "com.brave.Browser".to_string(),
                ],
                false,
            ),
        };
        if needs_no_sandbox {
            args.push("--no-sandbox".to_string());
        }
        args.extend([
            "--headless=new".to_string(),
            "--remote-debugging-address=127.0.0.1".to_string(),
            format!("--remote-debugging-port={cdp_port}"),
            "--remote-allow-origins=*".to_string(),
            format!("--user-data-dir={profile_path}"),
            "--no-first-run".to_string(),
            "--no-default-browser-check".to_string(),
            "--disable-background-networking".to_string(),
            "--disable-component-update".to_string(),
            "--disable-sync".to_string(),
            "--metrics-recording-only".to_string(),
            "--mute-audio".to_string(),
            "--autoplay-policy=no-user-gesture-required".to_string(),
            "--window-size=1280,720".to_string(),
            url.to_string(),
        ]);
        Ok(Self {
            program,
            args,
            profile_path,
        })
    }

    fn spawn(&self) -> Result<Child, String> {
        Command::new(&self.program)
            .args(&self.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("failed to launch owned browser process: {error}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedFrame {
    pub revision: u64,
    pub jpeg: Vec<u8>,
}

#[derive(Default)]
struct FrameState {
    revision: u64,
    jpeg: Vec<u8>,
    failure: Option<String>,
}

#[derive(Clone, Default)]
pub struct FrameHub {
    inner: Arc<(Mutex<FrameState>, Condvar)>,
}

impl FrameHub {
    pub fn publish(&self, jpeg: Vec<u8>) -> u64 {
        let (lock, ready) = &*self.inner;
        let mut state = lock.lock().expect("frame hub lock poisoned");
        state.revision = state.revision.saturating_add(1);
        state.jpeg = jpeg;
        state.failure = None;
        ready.notify_all();
        state.revision
    }

    fn fail(&self, reason: String) {
        let (lock, ready) = &*self.inner;
        if let Ok(mut state) = lock.lock() {
            state.failure = Some(reason);
            ready.notify_all();
        }
    }

    fn failure(&self) -> Option<String> {
        self.inner
            .0
            .lock()
            .ok()
            .and_then(|state| state.failure.clone())
    }

    pub fn revision(&self) -> u64 {
        self.inner
            .0
            .lock()
            .map(|state| state.revision)
            .unwrap_or_default()
    }

    pub fn latest_after(&self, revision: u64) -> Option<PublishedFrame> {
        let state = self.inner.0.lock().ok()?;
        (state.revision > revision && !state.jpeg.is_empty()).then(|| PublishedFrame {
            revision: state.revision,
            jpeg: state.jpeg.clone(),
        })
    }

    pub fn wait_for_revision_after(
        &self,
        revision: u64,
        timeout: Duration,
    ) -> Option<PublishedFrame> {
        let (lock, ready) = &*self.inner;
        let state = lock.lock().ok()?;
        let (state, _) = ready
            .wait_timeout_while(state, timeout, |current| {
                current.revision <= revision && current.failure.is_none()
            })
            .ok()?;
        (state.revision > revision).then(|| PublishedFrame {
            revision: state.revision,
            jpeg: state.jpeg.clone(),
        })
    }

    fn wake(&self) {
        self.inner.1.notify_all();
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartBrowserCaptureRequest {
    pub session_id: String,
    pub owner: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopBrowserCaptureRequest {
    pub session_id: String,
    pub owner: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateBrowserCaptureRequest {
    pub session_id: String,
    pub owner: String,
    pub expected_revision: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClickBrowserCaptureRequest {
    pub session_id: String,
    pub owner: String,
    pub expected_revision: u64,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCaptureDescriptor {
    pub session_id: String,
    pub owner: String,
    pub revision: u64,
    pub url: String,
    pub stream_url: String,
    pub transport: &'static str,
    pub muted: bool,
    pub process_id: u32,
    pub frame_revision: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserCaptureFrame {
    pub revision: u64,
    pub jpeg_base64: String,
}

struct OwnedBrowserCapture {
    descriptor: BrowserCaptureDescriptor,
    cdp_port: u16,
    child: Child,
    shutdown: Arc<AtomicBool>,
    frames: FrameHub,
    capture_thread: Option<JoinHandle<()>>,
    stream_thread: Option<JoinHandle<()>>,
    profile_path: String,
}

impl OwnedBrowserCapture {
    fn snapshot(&mut self) -> BrowserCaptureDescriptor {
        self.descriptor.frame_revision = self.frames.revision();
        self.descriptor.clone()
    }

    fn stop(mut self) {
        self.shutdown.store(true, Ordering::Release);
        self.frames.wake();
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(thread) = self.capture_thread.take() {
            let _ = thread.join();
        }
        if let Some(thread) = self.stream_thread.take() {
            let _ = thread.join();
        }
        let _ = std::fs::remove_dir_all(&self.profile_path);
    }
}

#[derive(Clone, Default)]
pub struct BrowserCaptureState {
    sessions: Arc<Mutex<HashMap<String, OwnedBrowserCapture>>>,
}

impl BrowserCaptureState {
    pub(crate) fn start(
        &self,
        request: StartBrowserCaptureRequest,
    ) -> Result<BrowserCaptureDescriptor, String> {
        validate_session_id(&request.session_id)?;
        if request.owner.trim().is_empty() {
            return Err("browser capture owner is required".to_string());
        }
        validate_http_url(&request.url)?;
        {
            let sessions = self
                .sessions
                .lock()
                .map_err(|_| "browser capture registry lock poisoned".to_string())?;
            if sessions.contains_key(&request.session_id) {
                return Err(format!(
                    "browser capture session '{}' already exists",
                    request.session_id
                ));
            }
        }

        let runtime = discover_browser_runtime()?;
        let cdp_port = reserve_loopback_port()?;
        let stream_listener = TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| format!("failed to bind browser MJPEG stream: {error}"))?;
        let stream_port = stream_listener
            .local_addr()
            .map_err(|error| format!("failed to inspect browser MJPEG listener: {error}"))?
            .port();
        let plan = BrowserLaunchPlan::new(runtime, &request.session_id, &request.url, cdp_port)?;
        let mut child = plan.spawn()?;
        let process_id = child.id();
        let shutdown = Arc::new(AtomicBool::new(false));
        let frames = FrameHub::default();
        let capture_thread = spawn_cdp_capture_thread(cdp_port, frames.clone(), shutdown.clone());
        let stream_thread =
            spawn_mjpeg_server_thread(stream_listener, frames.clone(), shutdown.clone())?;

        let deadline = Instant::now() + BROWSER_START_TIMEOUT;
        let first = wait_for_live_frame(&frames, 0, deadline);
        let second = first
            .as_ref()
            .and_then(|frame| wait_for_live_frame(&frames, frame.revision, deadline));
        let Some(second) = second else {
            shutdown.store(true, Ordering::Release);
            frames.wake();
            let _ = child.kill();
            let _ = child.wait();
            let _ = capture_thread.join();
            let _ = stream_thread.join();
            let reason = frames.failure().unwrap_or_else(|| {
                format!(
                    "browser capture did not publish two changing frames before startup timeout (observed revision {})",
                    frames.revision()
                )
            });
            return Err(reason);
        };

        let descriptor = BrowserCaptureDescriptor {
            session_id: request.session_id.clone(),
            owner: request.owner,
            revision: 1,
            url: request.url,
            stream_url: loopback_mjpeg_url(stream_port, &request.session_id),
            transport: "mjpeg",
            muted: true,
            process_id,
            frame_revision: second.revision,
        };
        let capture = OwnedBrowserCapture {
            descriptor: descriptor.clone(),
            cdp_port,
            child,
            shutdown,
            frames,
            capture_thread: Some(capture_thread),
            stream_thread: Some(stream_thread),
            profile_path: plan.profile_path,
        };
        self.sessions
            .lock()
            .map_err(|_| "browser capture registry lock poisoned".to_string())?
            .insert(request.session_id, capture);
        Ok(descriptor)
    }

    pub(crate) fn status(&self, session_id: &str) -> Result<BrowserCaptureDescriptor, String> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "browser capture registry lock poisoned".to_string())?;
        let capture = sessions
            .get_mut(session_id)
            .ok_or_else(|| format!("browser capture session '{session_id}' is unavailable"))?;
        if let Some(reason) = capture.frames.failure() {
            return Err(format!("browser capture stream failed: {reason}"));
        }
        match capture.child.try_wait() {
            Ok(None) => Ok(capture.snapshot()),
            Ok(Some(status)) => Err(format!("browser capture process exited: {status}")),
            Err(error) => Err(format!("browser capture process status failed: {error}")),
        }
    }

    pub(crate) fn frame(
        &self,
        session_id: &str,
        after_revision: u64,
    ) -> Result<Option<BrowserCaptureFrame>, String> {
        // Native WebKit can reject an otherwise healthy multipart MJPEG image;
        // expose the same captured revisions for Tauri IPC CanvasTexture delivery.
        let sessions = self
            .sessions
            .lock()
            .map_err(|_| "browser capture registry lock poisoned".to_string())?;
        let capture = sessions
            .get(session_id)
            .ok_or_else(|| format!("browser capture session '{session_id}' is unavailable"))?;
        if let Some(reason) = capture.frames.failure() {
            return Err(format!("browser capture stream failed: {reason}"));
        }
        Ok(capture
            .frames
            .latest_after(after_revision)
            .map(|frame| BrowserCaptureFrame {
                revision: frame.revision,
                jpeg_base64: base64::engine::general_purpose::STANDARD.encode(frame.jpeg),
            }))
    }

    pub(crate) fn navigate(
        &self,
        request: NavigateBrowserCaptureRequest,
    ) -> Result<BrowserCaptureDescriptor, String> {
        validate_http_url(&request.url)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "browser capture registry lock poisoned".to_string())?;
        let capture = sessions.get_mut(&request.session_id).ok_or_else(|| {
            format!(
                "browser capture session '{}' is unavailable",
                request.session_id
            )
        })?;
        ensure_browser_running(capture)?;
        authorize_browser_control(
            &capture.descriptor,
            &request.owner,
            request.expected_revision,
        )?;
        execute_cdp_commands(
            capture.cdp_port,
            vec![json!({"method": "Page.navigate", "params": {"url": &request.url}})],
        )?;
        capture.descriptor.url = request.url;
        capture.descriptor.revision = capture.descriptor.revision.saturating_add(1);
        Ok(capture.snapshot())
    }

    pub(crate) fn click(
        &self,
        request: ClickBrowserCaptureRequest,
    ) -> Result<BrowserCaptureDescriptor, String> {
        let commands = browser_click_commands(request.x, request.y)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "browser capture registry lock poisoned".to_string())?;
        let capture = sessions.get_mut(&request.session_id).ok_or_else(|| {
            format!(
                "browser capture session '{}' is unavailable",
                request.session_id
            )
        })?;
        ensure_browser_running(capture)?;
        authorize_browser_control(
            &capture.descriptor,
            &request.owner,
            request.expected_revision,
        )?;
        execute_cdp_commands(capture.cdp_port, commands)?;
        capture.descriptor.revision = capture.descriptor.revision.saturating_add(1);
        Ok(capture.snapshot())
    }

    pub(crate) fn stop(&self, request: StopBrowserCaptureRequest) -> Result<(), String> {
        let capture = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "browser capture registry lock poisoned".to_string())?;
            let current = sessions.get(&request.session_id).ok_or_else(|| {
                format!(
                    "browser capture session '{}' is unavailable",
                    request.session_id
                )
            })?;
            if current.descriptor.owner != request.owner {
                return Err("browser capture owner mismatch".to_string());
            }
            sessions
                .remove(&request.session_id)
                .expect("capture existed while registry lock was held")
        };
        capture.stop();
        Ok(())
    }

    pub fn cleanup_all(&self) {
        let captures = self
            .sessions
            .lock()
            .map(|mut sessions| {
                sessions
                    .drain()
                    .map(|(_, capture)| capture)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for capture in captures {
            capture.stop();
        }
    }
}

#[tauri::command]
pub async fn start_browser_capture(
    state: State<'_, BrowserCaptureState>,
    request: StartBrowserCaptureRequest,
) -> Result<BrowserCaptureDescriptor, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.start(request))
        .await
        .map_err(|error| format!("browser capture startup task failed: {error}"))?
}

#[tauri::command]
pub fn get_browser_capture_status(
    state: State<'_, BrowserCaptureState>,
    session_id: String,
) -> Result<BrowserCaptureDescriptor, String> {
    state.status(&session_id)
}

#[tauri::command]
pub fn get_browser_capture_frame(
    state: State<'_, BrowserCaptureState>,
    session_id: String,
    after_revision: u64,
) -> Result<Option<BrowserCaptureFrame>, String> {
    state.frame(&session_id, after_revision)
}

#[tauri::command]
pub async fn navigate_browser_capture(
    state: State<'_, BrowserCaptureState>,
    request: NavigateBrowserCaptureRequest,
) -> Result<BrowserCaptureDescriptor, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.navigate(request))
        .await
        .map_err(|error| format!("browser capture navigation task failed: {error}"))?
}

#[tauri::command]
pub async fn click_browser_capture(
    state: State<'_, BrowserCaptureState>,
    request: ClickBrowserCaptureRequest,
) -> Result<BrowserCaptureDescriptor, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.click(request))
        .await
        .map_err(|error| format!("browser capture input task failed: {error}"))?
}

#[tauri::command]
pub async fn stop_browser_capture(
    state: State<'_, BrowserCaptureState>,
    request: StopBrowserCaptureRequest,
) -> Result<(), String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.stop(request))
        .await
        .map_err(|error| format!("browser capture stop task failed: {error}"))?
}

pub fn loopback_mjpeg_url(port: u16, session_id: &str) -> String {
    format!("http://127.0.0.1:{port}/session/{session_id}.mjpeg")
}

fn wait_for_live_frame(
    frames: &FrameHub,
    revision: u64,
    deadline: Instant,
) -> Option<PublishedFrame> {
    while Instant::now() < deadline {
        if frames.failure().is_some() {
            return None;
        }
        if let Some(frame) = frames.wait_for_revision_after(revision, FRAME_WAIT_SLICE) {
            return Some(frame);
        }
    }
    None
}

fn validate_session_id(session_id: &str) -> Result<(), String> {
    if session_id.is_empty()
        || session_id.len() > 96
        || !session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(
            "browser capture session ID must use 1-96 ASCII letters, digits, '-' or '_'"
                .to_string(),
        );
    }
    Ok(())
}

fn validate_http_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    if url.chars().any(char::is_whitespace)
        || !(lower.starts_with("http://") || lower.starts_with("https://"))
    {
        return Err("browser capture navigation requires an HTTP(S) URL".to_string());
    }
    Ok(())
}

pub(crate) fn authorize_browser_control(
    descriptor: &BrowserCaptureDescriptor,
    owner: &str,
    expected_revision: u64,
) -> Result<(), String> {
    if descriptor.owner != owner {
        return Err("browser capture owner mismatch".to_string());
    }
    if descriptor.revision != expected_revision {
        return Err(format!(
            "browser capture revision conflict: expected {expected_revision}, current {}",
            descriptor.revision
        ));
    }
    Ok(())
}

fn ensure_browser_running(capture: &mut OwnedBrowserCapture) -> Result<(), String> {
    if let Some(reason) = capture.frames.failure() {
        return Err(format!("browser capture stream failed: {reason}"));
    }
    match capture.child.try_wait() {
        Ok(None) => Ok(()),
        Ok(Some(status)) => Err(format!("browser capture process exited: {status}")),
        Err(error) => Err(format!("browser capture process status failed: {error}")),
    }
}

pub(crate) fn browser_click_commands(x: f64, y: f64) -> Result<Vec<Value>, String> {
    if !x.is_finite()
        || !y.is_finite()
        || !(0.0..=CAPTURE_WIDTH).contains(&x)
        || !(0.0..=CAPTURE_HEIGHT).contains(&y)
    {
        return Err(format!(
            "browser pointer coordinates must be finite and inside {CAPTURE_WIDTH}x{CAPTURE_HEIGHT}"
        ));
    }
    Ok(vec![
        json!({
            "method": "Input.dispatchMouseEvent",
            "params": {"type": "mousePressed", "x": x, "y": y, "button": "left", "clickCount": 1}
        }),
        json!({
            "method": "Input.dispatchMouseEvent",
            "params": {"type": "mouseReleased", "x": x, "y": y, "button": "left", "clickCount": 1}
        }),
    ])
}

fn discover_browser_runtime() -> Result<BrowserRuntime, String> {
    if let Ok(program) = std::env::var("ARDA_HUD_BROWSER_BIN") {
        let program = program.trim();
        if !program.is_empty() {
            return Ok(BrowserRuntime::Direct(program.to_string()));
        }
    }
    for candidate in [
        "/usr/bin/chromium",
        "/usr/bin/chromium-browser",
        "/usr/bin/google-chrome",
        "/usr/bin/google-chrome-stable",
    ] {
        if std::path::Path::new(candidate).is_file() {
            return Ok(BrowserRuntime::Direct(candidate.to_string()));
        }
    }
    for candidate in [
        "/var/lib/flatpak/app/com.brave.Browser/x86_64/stable/active/files/brave/brave",
        "/var/home/mythos/.local/share/flatpak/app/com.brave.Browser/x86_64/stable/active/files/brave/brave",
    ] {
        if std::path::Path::new(candidate).is_file() {
            return Ok(BrowserRuntime::FlatpakBraveFiles(candidate.to_string()));
        }
    }
    let flatpak_available = Command::new("flatpak")
        .args(["info", "com.brave.Browser"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if flatpak_available {
        return Ok(BrowserRuntime::FlatpakBrave);
    }
    Err("no supported Chromium browser runtime is installed".to_string())
}

fn reserve_loopback_port() -> Result<u16, String> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|error| format!("failed to reserve browser CDP port: {error}"))?;
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| format!("failed to inspect browser CDP port: {error}"))
}

fn spawn_cdp_capture_thread(
    cdp_port: u16,
    frames: FrameHub,
    shutdown: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        if let Err(error) = run_cdp_capture(cdp_port, &frames, &shutdown) {
            if !shutdown.load(Ordering::Acquire) {
                frames.fail(error);
            }
        }
    })
}

fn run_cdp_capture(cdp_port: u16, frames: &FrameHub, shutdown: &AtomicBool) -> Result<(), String> {
    let deadline = Instant::now() + BROWSER_START_TIMEOUT;
    let websocket_url = loop {
        if shutdown.load(Ordering::Acquire) {
            return Ok(());
        }
        match read_cdp_page_target(cdp_port) {
            Ok(url) => break url,
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => return Err(error),
        }
    };
    if std::env::var_os("ARDA_HUD_CAPTURE_DEBUG").is_some() {
        eprintln!("browser-capture connecting {websocket_url}");
    }
    let mut socket = connect_loopback_websocket(cdp_port, &websocket_url)?;
    if std::env::var_os("ARDA_HUD_CAPTURE_DEBUG").is_some() {
        eprintln!("browser-capture connected");
    }

    socket
        .send(Message::Text(
            json!({"id": 1, "method": "Page.enable"}).to_string().into(),
        ))
        .map_err(|error| format!("failed to enable browser CDP page: {error}"))?;
    socket
        .send(Message::Text(
            json!({
                "id": 2,
                "method": "Page.startScreencast",
                "params": {
                    "format": "jpeg",
                    "quality": 80,
                    "maxWidth": 1280,
                    "maxHeight": 720,
                    "everyNthFrame": 1
                }
            })
            .to_string()
            .into(),
        ))
        .map_err(|error| format!("failed to start browser CDP screencast: {error}"))?;
    if std::env::var_os("ARDA_HUD_CAPTURE_DEBUG").is_some() {
        eprintln!("browser-capture commands sent");
    }

    let mut command_id = 3_u64;
    while !shutdown.load(Ordering::Acquire) {
        let message = match socket.read() {
            Ok(message) => message,
            Err(tungstenite::Error::Io(error))
                if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) =>
            {
                continue;
            }
            Err(error) => return Err(format!("browser CDP stream ended: {error}")),
        };
        let Message::Text(text) = message else {
            continue;
        };
        let payload: Value = serde_json::from_str(text.as_str())
            .map_err(|error| format!("browser CDP emitted invalid JSON: {error}"))?;
        if std::env::var_os("ARDA_HUD_CAPTURE_DEBUG").is_some() {
            eprintln!(
                "browser-capture cdp id={:?} method={:?} error={:?}",
                payload.get("id"),
                payload.get("method"),
                payload.get("error")
            );
        }
        if payload.get("id").and_then(Value::as_u64) == Some(2) {
            if let Some(error) = payload.get("error") {
                return Err(format!("browser rejected CDP screencast startup: {error}"));
            }
        }
        if payload.get("method").and_then(Value::as_str) != Some("Page.screencastFrame") {
            continue;
        }
        let params = payload
            .get("params")
            .ok_or_else(|| "browser CDP frame omitted params".to_string())?;
        let data = params
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| "browser CDP frame omitted JPEG data".to_string())?;
        let jpeg = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|error| format!("browser CDP frame used invalid base64: {error}"))?;
        if jpeg.len() < 4 || !jpeg.starts_with(&[0xff, 0xd8]) || !jpeg.ends_with(&[0xff, 0xd9]) {
            return Err("browser CDP frame was not a complete JPEG".to_string());
        }
        frames.publish(jpeg);
        if let Some(frame_session_id) = params.get("sessionId").and_then(Value::as_u64) {
            socket
                .send(Message::Text(
                    json!({
                        "id": command_id,
                        "method": "Page.screencastFrameAck",
                        "params": { "sessionId": frame_session_id }
                    })
                    .to_string()
                    .into(),
                ))
                .map_err(|error| format!("failed to acknowledge browser CDP frame: {error}"))?;
            command_id = command_id.saturating_add(1);
        }
    }
    let _ = socket.close(None);
    Ok(())
}

fn execute_cdp_commands(port: u16, commands: Vec<Value>) -> Result<(), String> {
    let websocket_url = read_cdp_page_target(port)?;
    let mut socket = connect_loopback_websocket(port, &websocket_url)?;
    for (index, mut command) in commands.into_iter().enumerate() {
        let command_id = index as u64 + 1;
        let command_object = command
            .as_object_mut()
            .ok_or_else(|| "browser CDP command must be a JSON object".to_string())?;
        command_object.insert("id".to_string(), Value::from(command_id));
        socket
            .send(Message::Text(command.to_string().into()))
            .map_err(|error| format!("failed to send browser CDP control command: {error}"))?;

        loop {
            let message = socket
                .read()
                .map_err(|error| format!("browser CDP control response failed: {error}"))?;
            let Message::Text(text) = message else {
                continue;
            };
            let response: Value = serde_json::from_str(text.as_str())
                .map_err(|error| format!("browser CDP control returned invalid JSON: {error}"))?;
            if response.get("id").and_then(Value::as_u64) != Some(command_id) {
                continue;
            }
            if let Some(error) = response.get("error") {
                return Err(format!("browser rejected CDP control command: {error}"));
            }
            break;
        }
    }
    let _ = socket.close(None);
    Ok(())
}

fn connect_loopback_websocket(
    port: u16,
    websocket_url: &str,
) -> Result<WebSocket<TcpStream>, String> {
    let prefix = format!("ws://127.0.0.1:{port}");
    let path = websocket_url
        .strip_prefix(&prefix)
        .filter(|path| path.starts_with('/'))
        .ok_or_else(|| "browser CDP exposed a non-loopback websocket target".to_string())?;
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(2))
        .map_err(|error| format!("failed to connect to browser CDP socket: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set browser CDP handshake timeout: {error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to set browser CDP write timeout: {error}"))?;
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|error| format!("failed to write browser CDP websocket handshake: {error}"))?;
    let mut response = Vec::with_capacity(1024);
    let mut chunk = [0_u8; 512];
    while !response.windows(4).any(|window| window == b"\r\n\r\n") {
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("failed to read browser CDP websocket handshake: {error}"))?;
        if read == 0 {
            return Err("browser CDP closed during websocket handshake".to_string());
        }
        response.extend_from_slice(&chunk[..read]);
        if response.len() > 8192 {
            return Err("browser CDP websocket handshake exceeded 8 KiB".to_string());
        }
    }
    let response_text = String::from_utf8_lossy(&response);
    if !response_text.starts_with("HTTP/1.1 101 ") {
        return Err(format!(
            "browser CDP websocket handshake was rejected: {}",
            response_text.lines().next().unwrap_or("invalid response")
        ));
    }
    stream
        .set_read_timeout(Some(FRAME_WAIT_SLICE))
        .map_err(|error| format!("failed to set browser CDP frame timeout: {error}"))?;
    Ok(WebSocket::from_raw_socket(stream, Role::Client, None))
}

fn read_cdp_page_target(port: u16) -> Result<String, String> {
    let body = read_loopback_http(port, "/json/list")?;
    let targets: Value = serde_json::from_str(&body)
        .map_err(|error| format!("browser CDP target list was invalid JSON: {error}"))?;
    targets
        .as_array()
        .into_iter()
        .flatten()
        .find(|target| target.get("type").and_then(Value::as_str) == Some("page"))
        .and_then(|target| target.get("webSocketDebuggerUrl"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "browser CDP did not expose a page target".to_string())
}

fn read_loopback_http(port: u16, path: &str) -> Result<String, String> {
    let address = SocketAddr::from(([127, 0, 0, 1], port));
    let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(250))
        .map_err(|error| format!("failed to connect to browser CDP at {address}: {error}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(1)))
        .map_err(|error| format!("failed to set browser CDP HTTP timeout: {error}"))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| format!("failed to request browser CDP target list: {error}"))?;
    let mut response = Vec::with_capacity(2048);
    let (header_end, content_length) = loop {
        let mut chunk = [0_u8; 1024];
        let read = stream
            .read(&mut chunk)
            .map_err(|error| format!("failed to read browser CDP target list: {error}"))?;
        if read == 0 {
            return Err("browser CDP closed before completing its target list".to_string());
        }
        response.extend_from_slice(&chunk[..read]);
        if response.len() > 64 * 1024 {
            return Err("browser CDP target list exceeded 64 KiB".to_string());
        }
        let Some(header_end) = response
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .map(|index| index + 4)
        else {
            continue;
        };
        let headers = String::from_utf8_lossy(&response[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then(|| value.trim().parse::<usize>().ok())
                    .flatten()
            })
            .ok_or_else(|| "browser CDP omitted Content-Length".to_string())?;
        if response.len() >= header_end + content_length {
            break (header_end, content_length);
        }
    };
    let headers = String::from_utf8_lossy(&response[..header_end]);
    if !headers.starts_with("HTTP/1.1 2") && !headers.starts_with("HTTP/1.0 2") {
        return Err(format!(
            "browser CDP returned {}",
            headers.lines().next().unwrap_or("unknown status")
        ));
    }
    String::from_utf8(response[header_end..header_end + content_length].to_vec())
        .map_err(|error| format!("browser CDP target list was not UTF-8: {error}"))
}

fn spawn_mjpeg_server_thread(
    listener: TcpListener,
    frames: FrameHub,
    shutdown: Arc<AtomicBool>,
) -> Result<JoinHandle<()>, String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("failed to configure browser MJPEG listener: {error}"))?;
    Ok(thread::spawn(move || {
        let mut clients = Vec::new();
        while !shutdown.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let client_frames = frames.clone();
                    let client_shutdown = shutdown.clone();
                    clients.push(thread::spawn(move || {
                        let _ = serve_mjpeg_client(stream, &client_frames, &client_shutdown);
                    }));
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(error) => {
                    frames.fail(format!("browser MJPEG listener failed: {error}"));
                    break;
                }
            }
        }
        for client in clients {
            let _ = client.join();
        }
    }))
}

fn serve_mjpeg_client(
    mut stream: TcpStream,
    frames: &FrameHub,
    shutdown: &AtomicBool,
) -> Result<(), String> {
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("failed to configure browser MJPEG client: {error}"))?;
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary={MJPEG_BOUNDARY}\r\nCache-Control: no-store, no-cache, must-revalidate\r\nPragma: no-cache\r\nAccess-Control-Allow-Origin: *\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n"
    )
    .map_err(|error| format!("failed to write browser MJPEG headers: {error}"))?;
    let mut revision = 0;
    while !shutdown.load(Ordering::Acquire) {
        let Some(frame) = frames.wait_for_revision_after(revision, FRAME_WAIT_SLICE) else {
            if frames.failure().is_some() {
                break;
            }
            continue;
        };
        write!(
            stream,
            "--{MJPEG_BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\nX-Frame-Revision: {}\r\n\r\n",
            frame.jpeg.len(),
            frame.revision
        )
        .and_then(|_| stream.write_all(&frame.jpeg))
        .and_then(|_| stream.write_all(b"\r\n"))
        .and_then(|_| stream.flush())
        .map_err(|error| format!("browser MJPEG client disconnected: {error}"))?;
        revision = frame.revision;
    }
    Ok(())
}
