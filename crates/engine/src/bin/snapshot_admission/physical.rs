//! Pin physical descendant mount roots before cloning, then check the actual
//! staged grant. Path-equivalent mountinfo is not physical authorization.
use anyhow::{bail, Context, Result};
use std::{
    collections::BTreeMap,
    ffi::CString,
    fs::{self, File},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStringExt, fs::MetadataExt},
    },
    path::{Path, PathBuf},
};

struct Entry {
    id: u64,
    parent: u64,
    path: PathBuf,
    options: Vec<u8>,
}
fn unescape(raw: &[u8]) -> Result<PathBuf> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'\\' {
            let digits = raw.get(i + 1..i + 4).context("invalid mount escape")?;
            if !digits.iter().all(|b| (b'0'..=b'7').contains(b)) {
                bail!("invalid mount escape");
            }
            let value = u16::from(digits[0] - b'0') * 64
                + u16::from(digits[1] - b'0') * 8
                + u16::from(digits[2] - b'0');
            out.push(u8::try_from(value)?);
            i += 4;
        } else {
            out.push(raw[i]);
            i += 1;
        }
    }
    Ok(std::ffi::OsString::from_vec(out).into())
}
fn entries(raw: &[u8]) -> Result<Vec<Entry>> {
    raw.split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split(|b| *b == b' ').collect();
            if fields.len() < 10 {
                bail!("invalid mount entry");
            }
            Ok(Entry {
                id: std::str::from_utf8(fields[0])?.parse()?,
                parent: std::str::from_utf8(fields[1])?.parse()?,
                path: unescape(fields[4])?,
                options: fields[5].to_vec(),
            })
        })
        .collect()
}
fn mount_id(file: &File) -> Result<u64> {
    let mut stat: libc::statx = unsafe { std::mem::zeroed() };
    let result = unsafe {
        libc::statx(
            file.as_raw_fd(),
            c"".as_ptr(),
            libc::AT_EMPTY_PATH,
            libc::STATX_MNT_ID,
            &mut stat,
        )
    };
    if result != 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    if stat.stx_mask & libc::STATX_MNT_ID == 0 {
        bail!("kernel did not provide physical mount identity");
    }
    Ok(stat.stx_mnt_id)
}
fn open(root: &File, relative: &Path) -> Result<File> {
    use std::os::unix::ffi::OsStrExt;
    let path = if relative.as_os_str().is_empty() {
        Path::new(".")
    } else {
        relative
    };
    let path = CString::new(path.as_os_str().as_bytes())?;
    #[repr(C)]
    struct How {
        flags: u64,
        mode: u64,
        resolve: u64,
    }
    let how = How {
        flags: (libc::O_PATH | libc::O_CLOEXEC) as u64,
        mode: 0,
        resolve: 0x08 | 0x04,
    }; // BENEATH | NO_SYMLINKS
    let fd = unsafe {
        libc::syscall(
            libc::SYS_openat2,
            root.as_raw_fd(),
            path.as_ptr(),
            &how,
            std::mem::size_of::<How>(),
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    Ok(unsafe { File::from_raw_fd(fd as i32) })
}
struct Bound {
    handle: File,
    options: Vec<u8>,
}
pub struct Physical {
    mounts: BTreeMap<PathBuf, Bound>,
}
impl Physical {
    pub fn pin(root: &Path, topology: &[u8]) -> Result<Self> {
        Self::bind(root, topology, false)
    }
    fn bind(root: &Path, topology: &[u8], staged: bool) -> Result<Self> {
        use std::os::unix::fs::OpenOptionsExt;
        let anchor = fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(root)?;
        let rows = entries(topology)?;
        let root_id = mount_id(&anchor)?;
        let root_entry = rows
            .iter()
            .find(|entry| entry.id == root_id)
            .context("root mount not in admission topology")?;
        // The staged root is a newly bound mount, often reached through a
        // /proc/self/fd alias. Enumerate its mount-ID subtree, not that alias.
        let mountpoint = if staged {
            root_entry.path.clone()
        } else {
            root.to_path_buf()
        };
        let parents: BTreeMap<_, _> = rows.iter().map(|entry| (entry.id, entry.parent)).collect();
        let options = rows
            .iter()
            .find(|entry| entry.id == root_id)
            .context("root mount not in admission topology")?
            .options
            .clone();
        let mut mounts = BTreeMap::new();
        mounts.insert(
            PathBuf::new(),
            Bound {
                handle: anchor.try_clone()?,
                options,
            },
        );
        for entry in rows {
            if staged {
                let mut cursor = entry.id;
                let mut seen = std::collections::BTreeSet::new();
                while cursor != root_id {
                    if !seen.insert(cursor) {
                        break;
                    }
                    let Some(parent) = parents.get(&cursor) else {
                        break;
                    };
                    cursor = *parent;
                }
                if cursor != root_id {
                    continue;
                }
            }
            let relative = match entry.path.strip_prefix(&mountpoint) {
                Ok(relative) => relative,
                Err(_) if staged => bail!("staged descendant escaped its mount root"),
                Err(_) => continue,
            };
            let handle = open(&anchor, relative)?;
            // Covered/stacked entries cannot be bound through pathname lookup.
            // Reject explicitly instead of treating the visible root as all layers.
            if mount_id(&handle)? != entry.id {
                bail!("covered mount requires explicit reconciliation before admission");
            }
            if relative.as_os_str().is_empty() {
                continue;
            }
            if mounts
                .insert(
                    relative.to_path_buf(),
                    Bound {
                        handle,
                        options: entry.options,
                    },
                )
                .is_some()
            {
                bail!("ambiguous descendant mount");
            }
        }
        Ok(Self { mounts })
    }
    pub fn verify(&self, staged: &Path) -> Result<()> {
        let before = fs::read("/proc/thread-self/mountinfo")?;
        let actual = Self::bind(staged, &before, true)?;
        if self.mounts.len() != actual.mounts.len() {
            bail!("captured descendant mount set changed");
        }
        for (relative, expected) in &self.mounts {
            let received = actual
                .mounts
                .get(relative)
                .context("captured descendant mount location changed")?;
            let a = expected.handle.metadata()?;
            let b = received.handle.metadata()?;
            if a.dev() != b.dev()
                || a.ino() != b.ino()
                || a.mode() & libc::S_IFMT != b.mode() & libc::S_IFMT
                || expected.options != received.options
            {
                bail!("captured descendant physical authority changed");
            }
        }
        if before != fs::read("/proc/thread-self/mountinfo")? {
            bail!("captured mount tree moved during verification");
        }
        Ok(())
    }
}
