//! Same-user Unix execution adapter. No TCP listener or remote approval bypass.
use super::{
    presentation::{present, PresentationRequest, PresentationSource},
    typed::{emit_registry_changed, TypedMonitorSurfaceState},
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::Path,
    time::Duration,
};
use tauri::{AppHandle, Manager};

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Status,
    Present { request: PresentationRequest },
}

fn validate_asset(root: &Path, source: &PresentationSource) -> Result<(), String> {
    let PresentationSource::Asset { path, mime } = source else {
        return Ok(());
    };
    let imports = root
        .join("data/media/imports")
        .canonicalize()
        .map_err(|_| "media imports unavailable")?;
    let file = root
        .join(path)
        .canonicalize()
        .map_err(|_| "media asset unavailable")?;
    if !file.starts_with(&imports) || !file.is_file() {
        return Err("asset escapes media imports".into());
    }
    let metadata = fs::metadata(&file).map_err(|e| e.to_string())?;
    if metadata.len() > 128 * 1024 * 1024 {
        return Err("asset exceeds 128 MiB presentation limit".into());
    }
    // Inspect the bytes, not the attachment's extension or declared type.
    let output = std::process::Command::new("file")
        .args(["--brief", "--mime-type", "--"])
        .arg(&file)
        .output()
        .map_err(|e| e.to_string())?;
    let actual = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success()
        || (actual != *mime && !(actual == "text/plain" && mime == "text/markdown"))
    {
        return Err(format!(
            "media MIME mismatch: declared {mime}, detected {actual}"
        ));
    }
    Ok(())
}

fn handle(app: &AppHandle, bytes: &[u8]) -> Result<Value, String> {
    let command: Command = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let state = app.state::<TypedMonitorSurfaceState>();
    match command {
        Command::Status => Ok(
            json!({"ok":state.presentation_ready(),"ready":state.presentation_ready(),"schemaVersion":"arda.presentation-adapter.v1","registry":state.snapshot(),"outcome":"publication_only"}),
        ),
        Command::Present { request } => {
            if !state.presentation_ready() {
                return Err("deferred: HUD registry has not restored yet".into());
            }
            if !request.ambient_allowed {
                return Err("explicit ambient permission required".into());
            }
            let root = crate::get_arda_root();
            validate_asset(Path::new(&root), &request.source)?;
            let result = present(&state, request)?;
            let registry = state.snapshot();
            emit_registry_changed(
                app,
                "claim",
                &result.session.slot_id,
                &registry,
                Some(result.session.clone()),
            );
            Ok(json!({"ok":true,"result":result}))
        }
    }
}

pub fn start(app: AppHandle) -> Result<(), String> {
    let runtime = std::env::var("XDG_RUNTIME_DIR").map_err(|_| "XDG_RUNTIME_DIR missing")?;
    let parent = Path::new(&runtime).join("arda-hud");
    let uid = unsafe { libc::geteuid() };
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&parent)
        .or_else(|e| {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                Ok(())
            } else {
                Err(e)
            }
        })
        .map_err(|e| e.to_string())?;
    let metadata = fs::symlink_metadata(&parent).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        return Err("presentation socket directory must be owned by this user and private".into());
    }
    let socket = parent.join("presentation.sock");
    if let Ok(metadata) = fs::symlink_metadata(&socket) {
        if !metadata.file_type().is_socket() || metadata.uid() != uid {
            return Err("unsafe existing presentation socket".into());
        }
        if UnixStream::connect(&socket).is_ok() {
            return Err("presentation adapter already running".into());
        }
        fs::remove_file(&socket).map_err(|e| e.to_string())?;
    }
    let listener = UnixListener::bind(&socket).map_err(|e| e.to_string())?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    std::thread::Builder::new()
        .name("hud-presentation".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
                let mut bytes = Vec::new();
                let read = BufReader::new((&mut stream).take(65_537)).read_until(b'\n', &mut bytes);
                let response =
                    if read.is_err() || bytes.len() > 65_536 || bytes.last() != Some(&b'\n') {
                        json!({"ok":false,"error":"invalid or oversized presentation request"})
                    } else {
                        handle(&app, &bytes)
                            .unwrap_or_else(|error| json!({"ok":false,"error":error}))
                    };
                if let Ok(mut data) = serde_json::to_vec(&response) {
                    data.push(b'\n');
                    let _ = stream.write_all(&data);
                }
            }
        })
        .map_err(|e| e.to_string())?;
    Ok(())
}
