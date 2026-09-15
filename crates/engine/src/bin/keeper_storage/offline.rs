//! Offline opening never initializes endpoint state or sweeps snapshot rows.
use super::*;
use std::io::Read;

pub struct Offline {
    pub db: Connection,
    _inspection_copy: Option<tempfile::TempDir>,
    // Connection and locks must drop before pins.
    _owner_lock: File,
    _endpoint_lock: Option<File>,
    _durable: File,
    _runtime: Option<File>,
}

pub fn open(durable: &Path, runtime: &Path, owner: &str, writable: bool) -> Result<Offline> {
    uuid::Uuid::parse_str(owner)?;
    validate_durable(durable)?;
    let pin = |path: &Path| -> Result<File> {
        if fs::canonicalize(path)? != path {
            bail!("offline owner paths must be canonical");
        }
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path)?;
        let meta = file.metadata()?;
        if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o077 != 0 {
            bail!("offline directory is not private and owned");
        }
        Ok(file)
    };
    let durable_pin = pin(durable)?;
    let coordinate = std::path::PathBuf::from(format!("/proc/self/fd/{}", durable_pin.as_raw_fd()));
    let owner_lock = existing(&coordinate.join("owner.lock"))?;
    owner_lock
        .try_lock_exclusive()
        .context("keeper must be stopped for offline reconciliation")?;
    let mut saved = String::new();
    existing(&coordinate.join("owner.identity"))?
        .take(128)
        .read_to_string(&mut saved)?;
    if saved != owner {
        bail!("durable owner identity mismatch");
    }
    let (runtime_pin, endpoint_lock) = match fs::symlink_metadata(runtime) {
        Ok(_) => {
            let file = pin(runtime)?;
            let path = std::path::PathBuf::from(format!("/proc/self/fd/{}", file.as_raw_fd()));
            let lock = existing(&path.join("endpoint.lock"))?;
            lock.try_lock_exclusive()
                .context("keeper endpoint remains owned")?;
            let mut saved = String::new();
            existing(&path.join("endpoint.owner"))?
                .take(128)
                .read_to_string(&mut saved)?;
            if saved != owner {
                bail!("runtime endpoint owner mismatch");
            }
            (Some(file), Some(lock))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (None, None),
        Err(e) => return Err(e.into()),
    };
    let mut path = coordinate.join("owner.sqlite3");
    let _database = existing(&path)?;
    // SQLite READ_ONLY may still create or update WAL/SHM beside its input.
    // Owner exclusion makes the database and committed WAL a consistent pair.
    // Recover only a private copy, never immutable=1 (which may omit WAL data).
    let inspection_copy = if writable {
        None
    } else {
        let copy = tempfile::Builder::new()
            .prefix("arda-keeper-inspect-")
            .tempdir()?;
        for name in [
            "owner.sqlite3",
            "owner.sqlite3-wal",
            "owner.sqlite3-journal",
        ] {
            let source = coordinate.join(name);
            match fs::symlink_metadata(&source) {
                Ok(_) => {
                    let mut input = existing(&source)?;
                    let mut output = create(&copy.path().join(name))?;
                    std::io::copy(&mut input, &mut output)?;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::NotFound && name != "owner.sqlite3" => {}
                Err(error) => return Err(error.into()),
            }
        }
        path = copy.path().join("owner.sqlite3");
        Some(copy)
    };
    let flags = if writable {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let db = if writable {
        Connection::open_with_flags_and_vfs(&path, flags, anchored_vfs::register()?)?
    } else {
        // Recovery writes, if needed, are confined to the private scratch copy.
        Connection::open_with_flags(
            &path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX
                | OpenFlags::SQLITE_OPEN_NOFOLLOW,
        )?
    };
    db.busy_timeout(Duration::from_secs(1))?;
    if writable {
        db.execute_batch("PRAGMA synchronous=FULL;")?;
    } else {
        db.execute_batch("PRAGMA query_only=ON;")?;
    }
    let identities: Vec<String> = db
        .prepare("SELECT id FROM owner_identity")?
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    if identities != [owner] {
        bail!("owner database identity mismatch");
    }
    Ok(Offline {
        db,
        _inspection_copy: inspection_copy,
        _owner_lock: owner_lock,
        _endpoint_lock: endpoint_lock,
        _durable: durable_pin,
        _runtime: runtime_pin,
    })
}
