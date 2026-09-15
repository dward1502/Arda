//! Durable provisioning boundary. Routine access never creates authority.
use anyhow::{bail, Context, Result};
use fs2::FileExt;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Authority {
    version: u32,
    device: u64,
    inode: u64,
    nonce: String,
}

pub(super) fn marker_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".authority.json");
    PathBuf::from(name)
}

pub(super) fn normalize(path: &Path) -> Result<PathBuf> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let name = path
        .file_name()
        .context("ObjectiveStore needs a file name")?;
    Ok(std::fs::canonicalize(parent)
        .context("resolve ObjectiveStore authority directory")?
        .join(name))
}

impl Authority {
    pub(super) fn load(path: &Path) -> Result<Self> {
        let mut marker = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(marker_path(path))
            .context("open ObjectiveStore authority; explicit provisioning required")?;
        FileExt::lock_shared(&marker)?;
        if !marker.metadata()?.is_file() || marker.metadata()?.len() > 4096 {
            bail!("invalid ObjectiveStore authority file");
        }
        let mut bytes = Vec::new();
        marker.read_to_end(&mut bytes)?;
        let authority: Self = serde_json::from_slice(&bytes)
            .context("invalid or interrupted ObjectiveStore provisioning; restore authority, do not recreate")?;
        if authority.version != 1 || uuid::Uuid::parse_str(&authority.nonce).is_err() {
            bail!("unsupported ObjectiveStore authority");
        }
        authority.check_path(path)?;
        Ok(authority)
    }

    pub(super) fn check_path(&self, path: &Path) -> Result<()> {
        let metadata = std::fs::symlink_metadata(path)
            .context("ObjectiveStore authority database is missing")?;
        if !metadata.is_file() || metadata.dev() != self.device || metadata.ino() != self.inode {
            bail!("ObjectiveStore authority database was replaced; restore original authority");
        }
        Ok(())
    }

    pub(super) fn check_connection(&self, connection: &Connection) -> Result<()> {
        let nonce: String = connection
            .query_row(
                "SELECT nonce FROM objective_store_authority WHERE singleton = 1",
                [],
                |row| row.get(0),
            )
            .context("ObjectiveStore authority identity is missing")?;
        if nonce != self.nonce {
            bail!("ObjectiveStore authority identity mismatch");
        }
        Ok(())
    }
}

/// Called only by the explicit provisioning API. The marker is reserved first:
/// a crash leaves a refusal, never permission to silently reset replay history.
pub(super) fn provision(path: &Path, new_only: bool) -> Result<Authority> {
    if new_only && (path.try_exists()? || marker_path(path).try_exists()?) {
        bail!("ObjectiveStore is already present or provisioned; initialization refused");
    }
    if marker_path(path).try_exists()? {
        return Authority::load(path);
    }
    let mut marker = match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(marker_path(path))
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            return Authority::load(path)
        }
        Err(error) => return Err(error.into()),
    };
    FileExt::lock_exclusive(&marker)?;
    File::open(path.parent().context("ObjectiveStore parent")?)?.sync_all()?;
    // Hold the file so its inode cannot be recycled during provisioning.
    let database = match OpenOptions::new()
        .read(true)
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
    {
        Ok(file) => file,
        Err(error) if !new_only && error.kind() == std::io::ErrorKind::AlreadyExists => {
            OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)?
        }
        Err(error) => return Err(error.into()),
    };
    let metadata = database.metadata()?;
    if !metadata.is_file() {
        bail!("ObjectiveStore database is not a regular file");
    }
    let authority = Authority {
        version: 1,
        device: metadata.dev(),
        inode: metadata.ino(),
        nonce: uuid::Uuid::new_v4().to_string(),
    };
    let connection =
        Connection::open_with_flags(path, OpenFlags::default() & !OpenFlags::SQLITE_OPEN_CREATE)?;
    connection.busy_timeout(std::time::Duration::from_secs(5))?;
    super::migrations::apply(&connection)?;
    connection.execute_batch("CREATE TABLE objective_store_authority (singleton INTEGER PRIMARY KEY CHECK(singleton = 1), nonce TEXT NOT NULL)")?;
    connection.execute(
        "INSERT INTO objective_store_authority VALUES (1, ?1)",
        [&authority.nonce],
    )?;
    drop(connection);
    database.sync_all()?;
    authority.check_path(path)?;
    serde_json::to_writer(&mut marker, &authority)?;
    marker.write_all(b"\n")?;
    marker.sync_all()?;
    File::open(path.parent().context("ObjectiveStore parent")?)?.sync_all()?;
    Ok(authority)
}
