//! Explicit owner initialization and separately locked endpoint ownership.
use anyhow::{bail, Context, Result};
use fs2::FileExt;
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
fn connection(path: &Path) -> Result<Connection> {
    let _file = existing(path)?;
    let db = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
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
    let db = connection(&durable.join("owner.sqlite3"))?;
    db.execute_batch("CREATE TABLE owner_identity(id TEXT PRIMARY KEY); CREATE TABLE snapshots(run TEXT PRIMARY KEY, workspace TEXT NOT NULL, identity TEXT NOT NULL, state TEXT NOT NULL, authority TEXT);")?;
    db.execute("INSERT INTO owner_identity(id) VALUES(?1)", [&identity])?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    File::open(durable)?.sync_all()?;
    Ok(())
}
pub fn open(durable: &Path, runtime: &Path) -> Result<(Connection, File, File)> {
    validate_durable(durable)?;
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
    let db = connection(&durable.join("owner.sqlite3"))?;
    let saved: String = db.query_row("SELECT id FROM owner_identity", [], |row| row.get(0))?;
    if saved != identity {
        bail!("owner journal identity mismatch");
    }
    db.execute(
        "UPDATE snapshots SET state='lost' WHERE state != 'released'",
        [],
    )?;
    Ok((db, lock, endpoint_lock))
}
