//! Descriptor-anchored scratch state on a separate trusted runtime filesystem.
use anyhow::{bail, Context, Result};
use std::ffi::{CString, OsString};
use std::fs::{self, File};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::ffi::OsStringExt;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

pub struct StateDirectory {
    parent: File,
    directory: File,
    name: CString,
    staging_mounted: bool,
    removed: bool,
}

#[repr(C)]
struct OpenHow {
    flags: u64,
    mode: u64,
    resolve: u64,
}

fn directory(path: &Path) -> Result<File> {
    let path = super::cpath(path)?;
    let how = OpenHow {
        flags: (libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC) as u64,
        mode: 0,
        resolve: 0x04 | 0x02, // RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS
    };
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            libc::AT_FDCWD,
            path.as_ptr(),
            &how,
            std::mem::size_of::<OpenHow>(),
        )
    };
    if fd == -1 {
        return Err(std::io::Error::last_os_error())
            .context("open trusted state parent without symlinks");
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}

fn decode_mount_path(encoded: &[u8]) -> Result<PathBuf> {
    let mut decoded = Vec::new();
    let mut i = 0;
    while i < encoded.len() {
        if encoded[i] == b'\\' {
            if i + 3 >= encoded.len()
                || !encoded[i + 1..i + 4]
                    .iter()
                    .all(|b| (b'0'..=b'7').contains(b))
            {
                bail!("invalid mount path escape");
            }
            let value = (u16::from(encoded[i + 1] - b'0') * 64)
                + (u16::from(encoded[i + 2] - b'0') * 8)
                + u16::from(encoded[i + 3] - b'0');
            decoded.push(u8::try_from(value)?);
            i += 4;
        } else {
            decoded.push(encoded[i]);
            i += 1;
        }
    }
    Ok(PathBuf::from(OsString::from_vec(decoded)))
}

impl StateDirectory {
    pub fn create_excluding(
        state: &Path,
        root: &Path,
        exposed: Option<&std::collections::BTreeSet<u64>>,
    ) -> Result<Self> {
        if !state.is_absolute()
            || state
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
        {
            bail!("state path must be absolute without dot components");
        }
        let parent = directory(state.parent().context("state parent required")?)?;
        let metadata = parent.metadata()?;
        if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
            bail!("state parent must be owned by worker user and private (0700)");
        }
        // Deliberately conservative support policy: runtime state must live on a
        if exposed.is_some_and(|devices| devices.contains(&metadata.dev())) {
            bail!("state filesystem overlaps a configured runtime grant");
        }
        // separate filesystem, not on any filesystem mounted in the workspace.
        // This also excludes bind aliases whose apparent ancestors are disjoint.
        if metadata.dev() == fs::metadata(root)?.dev()
            || metadata.dev() == fs::metadata("/usr")?.dev()
        {
            bail!("state filesystem overlaps a provider grant");
        }
        let device = format!(
            "{}:{}",
            libc::major(metadata.dev()),
            libc::minor(metadata.dev())
        );
        for line in fs::read("/proc/self/mountinfo")?
            .split(|b| *b == b'\n')
            .filter(|l| !l.is_empty())
        {
            let fields: Vec<_> = line.split(|b| *b == b' ').collect();
            if fields.len() < 6 {
                bail!("invalid mountinfo record");
            }
            let target = decode_mount_path(fields[4])?;
            if (target.starts_with(root) && fields[2] == device.as_bytes())
                || (exposed.is_none() && target.starts_with("/usr") && target != Path::new("/usr"))
            {
                bail!("workspace/state alias or nested runtime mount is unsupported");
            }
        }
        let name = super::cpath(Path::new(state.file_name().context("state name required")?))?;
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } == -1 {
            return Err(std::io::Error::last_os_error()).context("create fresh state directory");
        }
        let fd = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd == -1 {
            let error = std::io::Error::last_os_error();
            unsafe {
                libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), libc::AT_REMOVEDIR);
            }
            return Err(error).context("pin new state directory");
        }
        Ok(Self {
            parent,
            directory: unsafe { File::from_raw_fd(fd) },
            name,
            staging_mounted: false,
            removed: false,
        })
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))
    }

    pub fn mark_staging_mounted(&mut self) {
        self.staging_mounted = true;
    }

    pub fn cleanup(&mut self) -> Result<()> {
        if self.removed {
            return Ok(());
        }
        // Namespace-local detach, then empty-directory removal only. Never walk
        // or recursively delete through a retained workspace mount.
        if self.staging_mounted {
            let staging = super::cpath(&self.path().join("staging"))?;
            if unsafe { libc::umount2(staging.as_ptr(), libc::MNT_DETACH) } == -1 {
                return Err(std::io::Error::last_os_error())
                    .context("detach owned snapshot staging");
            }
            self.staging_mounted = false;
        }
        for (name, flags) in [(c"control.sock", 0), (c"staging", libc::AT_REMOVEDIR)] {
            if unsafe { libc::unlinkat(self.directory.as_raw_fd(), name.as_ptr(), flags) } == -1 {
                let error = std::io::Error::last_os_error();
                if error.raw_os_error() != Some(libc::ENOENT) {
                    return Err(error).context("remove owned snapshot artifact");
                }
            }
        }
        if unsafe {
            libc::unlinkat(
                self.parent.as_raw_fd(),
                self.name.as_ptr(),
                libc::AT_REMOVEDIR,
            )
        } == -1
        {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ENOENT) {
                return Err(error).context("remove owned snapshot directory");
            }
        }
        self.removed = true;
        Ok(())
    }
}

impl Drop for StateDirectory {
    fn drop(&mut self) {
        // Error-path best effort only; explicit release reports cleanup errors.
        let _ = self.cleanup();
    }
}
