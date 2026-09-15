//! Typed runtime-grant witnesses and namespace-local captures.
use super::{physical::Physical, stable};
use anyhow::{bail, Result};
use arda_engine::objectives::runtime_policy::{GrantAccess, GrantKind, RuntimeGrant};
use arda_engine::objectives::snapshot_protocol::CapturedRuntimeGrant;

use std::{
    fs::{self, File},
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::{Path, PathBuf},
};

pub struct GrantAdmission {
    grant_binding: Vec<u8>,
    source: PathBuf,
    kind: GrantKind,
    witness: File,
    physical: Physical,
    topology: Vec<Vec<u8>>,
}
pub struct CapturedGrant {
    pub descriptor: File,
    pub identity: CapturedRuntimeGrant,
}
impl GrantAdmission {
    pub fn before_clone(grant: &RuntimeGrant) -> Result<Self> {
        let bytes = fs::read("/proc/thread-self/mountinfo")?;
        let witness = Physical::open_source(&grant.source)?;
        let metadata = witness.metadata()?;
        if !match grant.kind {
            GrantKind::File => metadata.is_file(),
            GrantKind::Directory => metadata.is_dir(),
        } {
            bail!("runtime source has wrong declared type");
        }
        let physical = Physical::pin_handle(&grant.source, witness.try_clone()?, &bytes)?;
        if bytes != fs::read("/proc/thread-self/mountinfo")? {
            bail!("runtime admission topology changed");
        }
        Ok(Self {
            grant_binding: serde_json::to_vec(grant)?,
            source: grant.source.clone(),
            kind: grant.kind.clone(),
            witness,
            physical,
            topology: stable(&bytes)?,
        })
    }
    pub fn exposed_devices(&self) -> Result<std::collections::BTreeSet<u64>> {
        self.physical.exposed_devices()
    }
    fn check_identity(&self, file: &File) -> Result<()> {
        let a = self.witness.metadata()?;
        let b = file.metadata()?;
        if (a.dev(), a.ino(), a.mode() & libc::S_IFMT)
            != (b.dev(), b.ino(), b.mode() & libc::S_IFMT)
        {
            bail!("runtime grant identity changed");
        }
        Ok(())
    }
    pub fn after_clone(&self) -> Result<()> {
        if self.topology != stable(&fs::read("/proc/thread-self/mountinfo")?)? {
            bail!("runtime namespace topology changed");
        }
        self.check_identity(&Physical::open_source(&self.source)?)
    }
    /// All bundle after_clone checks must finish before calling this method.
    pub fn capture(&self, grant: &RuntimeGrant, slot: &Path) -> Result<CapturedGrant> {
        if serde_json::to_vec(grant)? != self.grant_binding {
            bail!("runtime capture policy mismatch");
        }
        let source = Physical::open_source(&self.source)?;
        self.check_identity(&source)?;
        match self.kind {
            GrantKind::Directory => fs::create_dir(slot)?,
            GrantKind::File => {
                File::options().write(true).create_new(true).open(slot)?;
            }
        }
        let flags = libc::MS_BIND
            | if self.kind == GrantKind::Directory {
                libc::MS_REC
            } else {
                0
            };
        use std::os::unix::ffi::OsStrExt;
        let source_path = std::ffi::CString::new(format!("/proc/self/fd/{}", source.as_raw_fd()))?;
        let target_path = std::ffi::CString::new(slot.as_os_str().as_bytes())?;
        if unsafe {
            libc::mount(
                source_path.as_ptr(),
                target_path.as_ptr(),
                std::ptr::null(),
                flags,
                std::ptr::null(),
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().into());
        }
        use std::os::unix::fs::OpenOptionsExt;
        let descriptor = File::options()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(slot)?;
        self.check_identity(&descriptor)?;
        self.physical.verify_descriptor(&descriptor, false)?;
        if grant.access == GrantAccess::ReadWrite {
            let mut stats = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            if unsafe { libc::fstatvfs(descriptor.as_raw_fd(), stats.as_mut_ptr()) } != 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            if unsafe { stats.assume_init() }.f_flag & libc::ST_RDONLY != 0 {
                bail!("session-state capture inherited a read-only root");
            }
        }
        if grant.access == GrantAccess::ReadOnly {
            #[repr(C)]
            struct Attr {
                set: u64,
                clear: u64,
                propagation: u64,
                userns: u64,
            }
            let attr = Attr {
                set: 1,
                clear: 0,
                propagation: 0,
                userns: 0,
            }; // MOUNT_ATTR_RDONLY
            let flags = libc::AT_EMPTY_PATH as u32
                | if self.kind == GrantKind::Directory {
                    0x8000
                } else {
                    0
                }; // AT_RECURSIVE
            if unsafe {
                libc::syscall(
                    libc::SYS_mount_setattr,
                    descriptor.as_raw_fd(),
                    c"".as_ptr(),
                    flags,
                    &attr,
                    std::mem::size_of::<Attr>(),
                )
            } != 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
            self.physical.verify_descriptor(&descriptor, true)?;
        }
        let metadata = descriptor.metadata()?;
        let topology_digest = Physical::capture_digest(&descriptor)?;
        Ok(CapturedGrant {
            descriptor,
            identity: CapturedRuntimeGrant {
                id: grant.id.clone(),
                destination: grant.destination.clone(),
                kind: grant.kind.clone(),
                access: grant.access.clone(),
                device: metadata.dev(),
                inode: metadata.ino(),
                topology_digest,
            },
        })
    }
}
