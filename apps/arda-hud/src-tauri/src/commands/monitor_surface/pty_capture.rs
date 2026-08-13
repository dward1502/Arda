use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tauri::State;

const PTY_ROWS: u16 = 28;
const PTY_COLS: u16 = 96;
const MAX_OUTPUT_BYTES: usize = 32 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartPtyCaptureRequest {
    pub session_id: String,
    pub owner: String,
    pub command: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyCaptureControlRequest {
    pub session_id: String,
    pub owner: String,
    pub expected_revision: u64,
    pub data: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopPtyCaptureRequest {
    pub session_id: String,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PtyCaptureDescriptor {
    pub session_id: String,
    pub owner: String,
    pub revision: u64,
    pub output_revision: u64,
    pub process_id: Option<u32>,
    pub rows: u16,
    pub cols: u16,
    pub output: String,
}

#[derive(Default)]
struct PtyOutputState {
    revision: u64,
    bytes: Vec<u8>,
    failure: Option<String>,
}

struct OwnedPtyCapture {
    descriptor: PtyCaptureDescriptor,
    child: Box<dyn Child + Send + Sync>,
    writer: Box<dyn Write + Send>,
    master: Box<dyn MasterPty + Send>,
    output: Arc<Mutex<PtyOutputState>>,
    reader_thread: Option<JoinHandle<()>>,
}

impl OwnedPtyCapture {
    fn snapshot(&mut self) -> Result<PtyCaptureDescriptor, String> {
        if let Some(status) = self
            .child
            .try_wait()
            .map_err(|error| format!("PTY process status failed: {error}"))?
        {
            return Err(format!("PTY process exited: {status}"));
        }
        let output = self
            .output
            .lock()
            .map_err(|_| "PTY output lock poisoned".to_string())?;
        if let Some(reason) = &output.failure {
            return Err(format!("PTY stream failed: {reason}"));
        }
        self.descriptor.output_revision = output.revision;
        self.descriptor.output = String::from_utf8_lossy(&output.bytes).into_owned();
        Ok(self.descriptor.clone())
    }

    fn stop(mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        drop(self.writer);
        drop(self.master);
        if let Some(thread) = self.reader_thread.take() {
            let _ = thread.join();
        }
    }
}

#[derive(Clone, Default)]
pub struct PtyCaptureState {
    sessions: Arc<Mutex<HashMap<String, OwnedPtyCapture>>>,
}

impl PtyCaptureState {
    pub(crate) fn start(
        &self,
        request: StartPtyCaptureRequest,
    ) -> Result<PtyCaptureDescriptor, String> {
        validate_identity(&request.session_id, &request.owner)?;
        if request.command.trim().is_empty() {
            return Err("PTY command is required".to_string());
        }
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "PTY registry lock poisoned".to_string())?;
        if sessions.contains_key(&request.session_id) {
            return Err(format!(
                "PTY session '{}' already exists",
                request.session_id
            ));
        }

        let pair = native_pty_system()
            .openpty(PtySize {
                rows: PTY_ROWS,
                cols: PTY_COLS,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| format!("failed to open PTY: {error}"))?;
        let mut command = CommandBuilder::new("bash");
        command.args(["--noprofile", "--norc", "-lc", &request.command]);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| format!("failed to spawn PTY command: {error}"))?;
        let process_id = child.process_id();
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| format!("failed to clone PTY reader: {error}"))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| format!("failed to take PTY writer: {error}"))?;
        let output = Arc::new(Mutex::new(PtyOutputState::default()));
        let thread_output = output.clone();
        let reader_thread = thread::spawn(move || {
            let mut buffer = [0_u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        if let Ok(mut state) = thread_output.lock() {
                            state.bytes.extend_from_slice(&buffer[..count]);
                            if state.bytes.len() > MAX_OUTPUT_BYTES {
                                let drain = state.bytes.len() - MAX_OUTPUT_BYTES;
                                state.bytes.drain(..drain);
                            }
                            state.revision = state.revision.saturating_add(1);
                        } else {
                            break;
                        }
                    }
                    Err(error) => {
                        if let Ok(mut state) = thread_output.lock() {
                            state.failure = Some(error.to_string());
                        }
                        break;
                    }
                }
            }
        });
        let descriptor = PtyCaptureDescriptor {
            session_id: request.session_id.clone(),
            owner: request.owner,
            revision: 1,
            output_revision: 0,
            process_id,
            rows: PTY_ROWS,
            cols: PTY_COLS,
            output: String::new(),
        };
        sessions.insert(
            request.session_id,
            OwnedPtyCapture {
                descriptor: descriptor.clone(),
                child,
                writer,
                master: pair.master,
                output,
                reader_thread: Some(reader_thread),
            },
        );
        Ok(descriptor)
    }

    pub(crate) fn status(&self, session_id: &str) -> Result<PtyCaptureDescriptor, String> {
        self.sessions
            .lock()
            .map_err(|_| "PTY registry lock poisoned".to_string())?
            .get_mut(session_id)
            .ok_or_else(|| format!("PTY session '{session_id}' is unavailable"))?
            .snapshot()
    }

    pub(crate) fn write(
        &self,
        request: PtyCaptureControlRequest,
    ) -> Result<PtyCaptureDescriptor, String> {
        validate_identity(&request.session_id, &request.owner)?;
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| "PTY registry lock poisoned".to_string())?;
        let capture = sessions
            .get_mut(&request.session_id)
            .ok_or_else(|| format!("PTY session '{}' is unavailable", request.session_id))?;
        authorize(capture, &request.owner, request.expected_revision)?;
        capture
            .writer
            .write_all(request.data.as_bytes())
            .and_then(|_| capture.writer.flush())
            .map_err(|error| format!("PTY input failed: {error}"))?;
        capture.descriptor.revision = capture.descriptor.revision.saturating_add(1);
        capture.snapshot()
    }

    pub(crate) fn stop(&self, request: StopPtyCaptureRequest) -> Result<(), String> {
        validate_identity(&request.session_id, &request.owner)?;
        let capture = {
            let mut sessions = self
                .sessions
                .lock()
                .map_err(|_| "PTY registry lock poisoned".to_string())?;
            let capture = sessions
                .get(&request.session_id)
                .ok_or_else(|| format!("PTY session '{}' is unavailable", request.session_id))?;
            if capture.descriptor.owner != request.owner {
                return Err(format!(
                    "PTY session '{}' is owned by '{}'",
                    request.session_id, capture.descriptor.owner
                ));
            }
            sessions
                .remove(&request.session_id)
                .expect("PTY session disappeared while locked")
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

fn validate_identity(session_id: &str, owner: &str) -> Result<(), String> {
    if session_id.trim().is_empty()
        || !session_id
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '-' | '_'))
    {
        return Err(
            "PTY session_id must contain only ASCII letters, digits, '-' or '_'".to_string(),
        );
    }
    if owner.trim().is_empty() {
        return Err("PTY owner is required".to_string());
    }
    Ok(())
}

fn authorize(capture: &OwnedPtyCapture, owner: &str, expected_revision: u64) -> Result<(), String> {
    if capture.descriptor.owner != owner {
        return Err(format!(
            "PTY session '{}' is owned by '{}'",
            capture.descriptor.session_id, capture.descriptor.owner
        ));
    }
    if capture.descriptor.revision != expected_revision {
        return Err(format!(
            "PTY revision conflict for session '{}'",
            capture.descriptor.session_id
        ));
    }
    Ok(())
}

#[tauri::command]
pub fn start_pty_capture(
    state: State<'_, PtyCaptureState>,
    request: StartPtyCaptureRequest,
) -> Result<PtyCaptureDescriptor, String> {
    state.start(request)
}
#[tauri::command]
pub fn get_pty_capture_status(
    state: State<'_, PtyCaptureState>,
    session_id: String,
) -> Result<PtyCaptureDescriptor, String> {
    state.status(&session_id)
}
#[tauri::command]
pub fn write_pty_capture(
    state: State<'_, PtyCaptureState>,
    request: PtyCaptureControlRequest,
) -> Result<PtyCaptureDescriptor, String> {
    state.write(request)
}
#[tauri::command]
pub fn stop_pty_capture(
    state: State<'_, PtyCaptureState>,
    request: StopPtyCaptureRequest,
) -> Result<(), String> {
    state.stop(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn request(session_id: &str, owner: &str, command: &str) -> StartPtyCaptureRequest {
        StartPtyCaptureRequest {
            session_id: session_id.to_string(),
            owner: owner.to_string(),
            command: command.to_string(),
        }
    }

    fn wait_for_output(
        state: &PtyCaptureState,
        session_id: &str,
        text: &str,
    ) -> PtyCaptureDescriptor {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let descriptor = state.status(session_id).expect("PTY remains live");
            if descriptor.output.contains(text) {
                return descriptor;
            }
            assert!(
                Instant::now() < deadline,
                "PTY output did not contain {text:?}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn live_session_streams_authorizes_handoff_and_cleans_up() {
        let state = PtyCaptureState::default();
        let started = state
            .start(request(
                "p9-live-pty",
                "arda.agent.hermes",
                "printf 'ARDA_PTY_READY\\n'; while IFS= read -r line; do printf 'ARDA_PTY:%s\\n' \"$line\"; done",
            ))
            .expect("live PTY starts");
        assert!(started.process_id.is_some());
        let ready = wait_for_output(&state, "p9-live-pty", "ARDA_PTY_READY");
        assert!(ready.output_revision > 0);

        let rejected = state.write(PtyCaptureControlRequest {
            session_id: "p9-live-pty".to_string(),
            owner: "arda.agent.other".to_string(),
            expected_revision: ready.revision,
            data: "wrong-owner\n".to_string(),
        });
        assert!(rejected.unwrap_err().contains("owned by"));

        let handed_off = state
            .write(PtyCaptureControlRequest {
                session_id: "p9-live-pty".to_string(),
                owner: "arda.agent.hermes".to_string(),
                expected_revision: ready.revision,
                data: "workstation-intervention\n".to_string(),
            })
            .expect("same owner and revision can intervene");
        assert_eq!(handed_off.revision, ready.revision + 1);
        let streamed = wait_for_output(&state, "p9-live-pty", "ARDA_PTY:workstation-intervention");
        assert!(streamed.output_revision > ready.output_revision);

        state
            .stop(StopPtyCaptureRequest {
                session_id: "p9-live-pty".to_string(),
                owner: "arda.agent.hermes".to_string(),
            })
            .expect("owned cleanup succeeds");
        assert!(state
            .status("p9-live-pty")
            .unwrap_err()
            .contains("unavailable"));
    }

    #[test]
    fn exposes_process_failure_and_cleanup_all() {
        let state = PtyCaptureState::default();
        state
            .start(request("p9-failing-pty", "arda.agent.hermes", "exit 17"))
            .expect("failing process starts");
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            match state.status("p9-failing-pty") {
                Err(error) if error.contains("exited") => break,
                _ if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
                result => panic!("PTY failure was not exposed: {result:?}"),
            }
        }
        state.cleanup_all();
        assert!(state
            .status("p9-failing-pty")
            .unwrap_err()
            .contains("unavailable"));
    }
}
