//! Independently supervised owner; never launched or drained by the Arda daemon.
use anyhow::{bail, Context, Result};
use arda_engine::objectives::{
    keeper_client::{exchange, read_frame, same_user, KeeperRequest, KeeperResponse},
    snapshot_protocol::{Manifest, Request},
    RetainedSnapshot,
};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    os::unix::{fs::MetadataExt, net::UnixListener, process::CommandExt},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

mod keeper_owner;
mod keeper_storage;
use keeper_owner::Owner;

fn private_directory(path: &Path) -> Result<PathBuf> {
    let canonical = fs::canonicalize(path)?;
    let metadata = fs::metadata(&canonical)?;
    if canonical != path
        || !metadata.is_dir()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        bail!("keeper directories must be canonical, owned, private directories");
    }
    Ok(canonical)
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() == 2 && args[0] == "--initialize" {
        return keeper_storage::initialize(&private_directory(Path::new(&args[1]))?);
    }
    if args.len() != 3 {
        bail!("usage: arda-snapshot-keeper DURABLE_DIRECTORY RUNTIME_DIRECTORY WORKER_BINARY");
    }
    unsafe {
        libc::umask(0o077);
    }
    let durable = private_directory(Path::new(&args[0]))?;
    let runtime = private_directory(Path::new(&args[1]))?;
    let worker = fs::canonicalize(&args[2])?;
    let (db, _durable_lock, _endpoint_lock) = keeper_storage::open(&durable, &runtime)?;
    let socket = runtime.join("keeper.sock");
    // Both independent locks and the endpoint-owner binding are required before
    // removing a stale socket. Absence is never interpreted as release proof.
    if socket.exists() {
        use std::os::unix::fs::FileTypeExt;
        if !fs::symlink_metadata(&socket)?.file_type().is_socket() {
            bail!("keeper socket path is not a socket");
        }
        fs::remove_file(&socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    let mut owner = Owner {
        db,
        durable,
        runtime,
        worker,
        children: BTreeMap::new(),
    };
    for stream in listener.incoming() {
        let mut stream = stream?;
        if same_user(&stream).is_err() {
            continue;
        }
        let result = read_frame(&mut stream, Instant::now() + Duration::from_secs(2))
            .and_then(|bytes| Ok(serde_json::from_slice::<KeeperRequest>(&bytes)?))
            .and_then(|request| owner.handle(request));
        // Never serialize request/capability/error payloads to operator output.
        let response = result.unwrap_or(KeeperResponse {
            ok: false,
            snapshot: None,
        });
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        let _ = writeln!(stream, "{}", serde_json::to_string(&response)?);
    }
    Ok(())
}
