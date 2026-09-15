//! Independently supervised owner; never launched or drained by the Arda daemon.
use anyhow::{bail, Context, Result};
use arda_engine::objectives::{
    keeper_client::{exchange, read_frame, same_user, KeeperRequest, KeeperResponse},
    snapshot_protocol::{Manifest, Request},
    RetainedSnapshot,
};

use rusqlite::{params, OptionalExtension};
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

mod keeper_config;
mod keeper_managed;
mod keeper_owner;
mod keeper_readiness;
mod keeper_reconcile;
mod keeper_storage;
#[path = "snapshot_admission/physical.rs"]
mod snapshot_physical;
use arda_engine::objectives::runtime_policy::ValidatedRuntimePolicy;
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
    if args.first().is_some_and(|arg| arg == "reconcile") {
        return keeper_reconcile::main(args.into_iter().skip(1).collect());
    }
    if args.len() == 2 && args[0] == "--validate-policy" {
        let policy = keeper_config::load(Path::new(&args[1]))?;
        println!(
            "{}",
            serde_json::json!({"status":"schema_valid", "policy_digest":policy.digest(), "grant_count":policy.policy().grants.len()})
        );
        return Ok(());
    }
    if args.len() == 2 && args[0] == "--initialize" {
        return keeper_storage::initialize(&private_directory(Path::new(&args[1]))?);
    }
    if args.len() != 3 && !(args.len() == 5 && args[3] == "--runtime-policy") {
        bail!("usage: arda-snapshot-keeper DURABLE_DIRECTORY RUNTIME_DIRECTORY WORKER_BINARY [--runtime-policy POLICY]");
    }
    let runtime_policy = if args.len() == 5 {
        Some(keeper_config::load(Path::new(&args[4]))?)
    } else {
        None
    };
    unsafe {
        libc::umask(0o077);
    }
    let durable = private_directory(Path::new(&args[0]))?;
    let runtime = private_directory(Path::new(&args[1]))?;
    let worker = fs::canonicalize(&args[2])?;
    let reservations = runtime_policy
        .as_ref()
        .map(|policy| {
            let state = policy
                .policy()
                .grants
                .iter()
                .find(|g| {
                    g.role == arda_engine::objectives::runtime_policy::GrantRole::SessionState
                })
                .context("session allocation source missing")?;
            keeper_owner::pending::Reservations::pin(&durable, &runtime, &state.source)
        })
        .transpose()?;
    let (db, _durable_lock, _endpoint_lock) = if let Some(reservations) = &reservations {
        let (durable, runtime) = reservations.storage_pins()?;
        keeper_storage::open_pinned(durable, runtime)?
    } else {
        keeper_storage::open(&durable, &runtime)?
    };
    if let Some(reservations) = &reservations {
        reservations.bind_storage(&_durable_lock, &_endpoint_lock)?;
    }
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
    let managed = keeper_managed::capture(&db, &runtime)?;
    let mut owner = Owner {
        managed,
        reservations,
        db,
        durable,
        runtime,
        worker,
        runtime_policy,
        children: BTreeMap::new(),
        failed_qualifications: Default::default(),
    };
    keeper_readiness::notify(std::env::var_os("NOTIFY_SOCKET").as_deref())?;
    for stream in listener.incoming() {
        let mut stream = stream?;
        if same_user(&stream).is_err() {
            continue;
        }
        let result = read_frame(&mut stream, Instant::now() + Duration::from_secs(2))
            .and_then(|bytes| Ok(serde_json::from_slice::<KeeperRequest>(&bytes)?))
            .and_then(|request| owner.handle(request));
        // Never serialize request/capability/error payloads to operator output.
        if let Err(error) = &result {
            let phase = [
                "managed_binding",
                "runtime_allocation",
                "pending_pins",
                "worker_spawn",
                "worker_qualification",
            ]
            .into_iter()
            .find(|phase| error.chain().any(|cause| cause.to_string() == *phase))
            .unwrap_or("request_validation");
            eprintln!("snapshot keeper request rejected: phase={phase}");
        }
        let response = result.unwrap_or(KeeperResponse {
            ok: false,
            snapshot: None,
        });
        stream.set_write_timeout(Some(Duration::from_secs(1)))?;
        let _ = writeln!(stream, "{}", serde_json::to_string(&response)?);
    }
    Ok(())
}
