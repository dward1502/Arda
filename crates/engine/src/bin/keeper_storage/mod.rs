//! Explicit owner initialization and separately locked endpoint ownership.
use anyhow::{bail, Context, Result};
use fs2::FileExt;
mod abandonment;
pub(super) fn migrate_abandonments(db: &Connection) -> Result<()> {
    abandonment::migrate(db)
}
mod anchored_vfs;
pub mod offline;

pub struct PinnedConnection {
    // Drop SQLite before the directory anchors it uses for main/WAL/SHM.
    connection: Connection,
    _durable: File,
    _runtime: File,
}
impl std::ops::Deref for PinnedConnection {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.connection
    }
}
impl PinnedConnection {
    pub fn transaction(&mut self) -> rusqlite::Result<rusqlite::Transaction<'_>> {
        self.connection.transaction()
    }
}
use rusqlite::{Connection, OpenFlags};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    os::{
        fd::AsRawFd,
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
    path::Path,
    time::Duration,
};

pub fn validate_durable(path: &Path) -> Result<()> {
    arda_engine::objectives::validate_snapshot_owner_paths(Path::new("/usr"), &[path])?;
    let directory = File::open(path)?;
    let mut info: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(directory.as_raw_fd(), &mut info) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    // Qualify a local persistence substrate; this cannot certify storage hardware.
    // In particular, never place reboot tombstones on tmpfs/ramfs/overlay.
    if !matches!(info.f_type as u64, 0xef53 | 0x58465342 | 0x9123683e) {
        bail!("keeper journal requires a qualified ext4, XFS or Btrfs filesystem");
    }
    Ok(())
}
fn create(path: &Path) -> Result<File> {
    Ok(OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?)
}
fn existing(path: &Path) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    let meta = file.metadata()?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.mode() & 0o077 != 0
        || meta.nlink() != 1
    {
        bail!("invalid private owner file");
    }
    Ok(file)
}
fn connection(path: &Path, anchored: bool) -> Result<Connection> {
    let _file = existing(path)?;
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_NO_MUTEX
        | OpenFlags::SQLITE_OPEN_NOFOLLOW;
    let db = if anchored {
        Connection::open_with_flags_and_vfs(path, flags, anchored_vfs::register()?)?
    } else {
        Connection::open_with_flags(path, flags)?
    };
    db.busy_timeout(Duration::from_secs(1))?;
    db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")?;
    Ok(db)
}
pub fn initialize(durable: &Path) -> Result<()> {
    validate_durable(durable)?;
    if fs::read_dir(durable)?.next().is_some() {
        bail!(
            "initialization requires an empty owner directory; existing state needs reconciliation"
        );
    }
    let lock = create(&durable.join("owner.lock"))?;
    lock.try_lock_exclusive()?;
    // Marker precedes database creation. Interrupted initialization cannot be
    // silently retried, nor can normal startup reconstruct a missing database.
    let identity = uuid::Uuid::new_v4().to_string();
    let mut marker = create(&durable.join("owner.identity"))?;
    marker.write_all(identity.as_bytes())?;
    marker.sync_all()?;
    File::open(durable)?.sync_all()?;
    create(&durable.join("owner.sqlite3"))?.sync_all()?;
    let db = connection(&durable.join("owner.sqlite3"), false)?;
    db.execute_batch("CREATE TABLE owner_identity(id TEXT PRIMARY KEY); CREATE TABLE snapshots(run TEXT PRIMARY KEY, workspace TEXT NOT NULL, identity TEXT NOT NULL, state TEXT NOT NULL, authority TEXT);")?;
    db.execute("INSERT INTO owner_identity(id) VALUES(?1)", [&identity])?;
    abandonment::migrate(&db)?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    File::open(durable)?.sync_all()?;
    Ok(())
}
pub fn open(durable: &Path, runtime: &Path) -> Result<(PinnedConnection, File, File)> {
    validate_durable(durable)?;
    let pin = |path: &Path| -> Result<File> {
        Ok(OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW)
            .open(path)?)
    };
    open_pinned(pin(durable)?, pin(runtime)?)
}
pub fn open_pinned(durable_pin: File, runtime_pin: File) -> Result<(PinnedConnection, File, File)> {
    for file in [&durable_pin, &runtime_pin] {
        let metadata = file.metadata()?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            bail!("owner directory pin is not private");
        }
    }
    let mut info: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(durable_pin.as_raw_fd(), &mut info) } != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if !matches!(info.f_type as u64, 0xef53 | 0x58465342 | 0x9123683e) {
        bail!("unqualified journal filesystem");
    }
    let durable_path =
        std::path::PathBuf::from(format!("/proc/self/fd/{}", durable_pin.as_raw_fd()));
    let runtime_path =
        std::path::PathBuf::from(format!("/proc/self/fd/{}", runtime_pin.as_raw_fd()));
    let durable = durable_path.as_path();
    let runtime = runtime_path.as_path();
    let lock = existing(&durable.join("owner.lock"))
        .context("owner not initialized; explicit initialization/reconciliation required")?;
    lock.try_lock_exclusive()
        .context("snapshot keeper already owned")?;
    let marker = existing(&durable.join("owner.identity"))?;
    use std::io::Read;
    let mut identity = String::new();
    marker.take(128).read_to_string(&mut identity)?;
    uuid::Uuid::parse_str(&identity).context("invalid owner identity")?;
    // Lock the endpoint independently BEFORE touching the journal or socket.
    let endpoint_lock = match create(&runtime.join("endpoint.lock")) {
        Ok(file) => file,
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) =>
        {
            existing(&runtime.join("endpoint.lock"))?
        }
        Err(error) => return Err(error),
    };
    endpoint_lock
        .try_lock_exclusive()
        .context("snapshot endpoint already owned")?;
    match create(&runtime.join("endpoint.owner")) {
        Ok(mut file) => {
            file.write_all(identity.as_bytes())?;
            file.sync_all()?;
            File::open(runtime)?.sync_all()?;
        }
        Err(error)
            if error
                .downcast_ref::<std::io::Error>()
                .is_some_and(|e| e.kind() == std::io::ErrorKind::AlreadyExists) =>
        {
            let mut saved = String::new();
            existing(&runtime.join("endpoint.owner"))?
                .take(128)
                .read_to_string(&mut saved)?;
            if saved != identity {
                bail!(
                    "endpoint belongs to another durable owner; explicit reconciliation required"
                );
            }
        }
        Err(error) => return Err(error),
    }
    let db = connection(&durable.join("owner.sqlite3"), true)?;
    let saved: String = db.query_row("SELECT id FROM owner_identity", [], |row| row.get(0))?;
    if saved != identity {
        bail!("owner journal identity mismatch");
    }
    abandonment::migrate(&db)?;
    db.execute(
        "UPDATE snapshots SET state='lost' WHERE state NOT IN ('released','reconciled_revoked')
         AND NOT EXISTS(SELECT 1 FROM snapshot_operator_abandonments a WHERE a.run=snapshots.run)",
        [],
    )?;
    Ok((
        PinnedConnection {
            connection: db,
            _durable: durable_pin,
            _runtime: runtime_pin,
        },
        lock,
        endpoint_lock,
    ))
}
