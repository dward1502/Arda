//! Pin physical descendant mount roots before cloning, then check the actual
//! staged grant. Path-equivalent mountinfo is not physical authorization.
use super::tree_witness::{BackingView, MountWitness, ObjectId, TreeWitness};
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
    major: u32,
    minor: u32,
    backing: PathBuf,
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
            let separator = fields
                .iter()
                .position(|f| *f == b"-")
                .context("missing mount separator")?;
            if separator < 6 || fields.len() != separator + 4 {
                bail!("invalid mount suffix");
            }
            let (major, minor) = std::str::from_utf8(fields[2])?
                .split_once(':')
                .context("invalid mount device")?;
            Ok(Entry {
                major: major.parse()?,
                minor: minor.parse()?,
                backing: unescape(fields[3])?,
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
    if relative.as_os_str().is_empty() {
        return Ok(root.try_clone()?);
    }
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
    backing: BackingView,
}
pub struct Physical {
    mounts: BTreeMap<PathBuf, Bound>,
    source_coordinate: Option<PathBuf>,
}
impl Physical {
    pub fn pin(root: &Path, topology: &[u8]) -> Result<Self> {
        Self::bind(root, topology, false)
    }
    pub fn pin_handle(root: &Path, anchor: File, topology: &[u8]) -> Result<Self> {
        Self::bind_handle(root, anchor, topology, false)
    }
    fn bind(root: &Path, topology: &[u8], staged: bool) -> Result<Self> {
        let anchor = Self::open_source(root)?;
        Self::bind_handle(root, anchor, topology, staged)
    }
    pub fn open_source(root: &Path) -> Result<File> {
        open(&File::open("/")?, root.strip_prefix("/")?)
    }
    fn bind_handle(root: &Path, anchor: File, topology: &[u8], staged: bool) -> Result<Self> {
        let rows = entries(topology)?;
        if rows
            .iter()
            .map(|row| row.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != rows.len()
        {
            bail!("duplicate mount identities");
        }
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
        use std::os::unix::ffi::OsStrExt;
        let backing = if staged {
            root_entry.backing.clone()
        } else {
            let suffix = root
                .strip_prefix(&root_entry.path)
                .context("source coordinate outside containing mount")?;
            let coordinate_root = Self::open_source(&root_entry.path)?;
            if mount_id(&coordinate_root)? != root_id {
                bail!("containing mount is covered");
            }
            let coordinate = open(&coordinate_root, suffix)?;
            let expected = anchor.metadata()?;
            let actual = coordinate.metadata()?;
            if mount_id(&coordinate)? != root_id
                || (
                    expected.dev(),
                    expected.ino(),
                    expected.mode() & libc::S_IFMT,
                ) != (actual.dev(), actual.ino(), actual.mode() & libc::S_IFMT)
            {
                bail!("source backing coordinate changed");
            }
            if suffix.as_os_str().is_empty() {
                root_entry.backing.clone()
            } else {
                root_entry.backing.join(suffix)
            }
        };
        let mut mounts = BTreeMap::new();
        mounts.insert(
            PathBuf::new(),
            Bound {
                handle: anchor.try_clone()?,
                options,
                backing: BackingView {
                    major: root_entry.major,
                    minor: root_entry.minor,
                    root: backing.as_os_str().as_bytes().to_vec(),
                },
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
                        backing: BackingView {
                            major: entry.major,
                            minor: entry.minor,
                            root: entry.backing.as_os_str().as_bytes().to_vec(),
                        },
                    },
                )
                .is_some()
            {
                bail!("ambiguous descendant mount");
            }
        }
        Ok(Self {
            mounts,
            source_coordinate: if staged { None } else { Some(root.to_owned()) },
        })
    }
    pub fn verify(&self, staged: &Path) -> Result<()> {
        // Internal staging may be addressed beneath its owned /proc/self/fd
        // anchor. This is not a policy source: retain that established API,
        // then verify the exact opened descriptor's physical authority.
        use std::os::unix::fs::OpenOptionsExt;
        let descriptor = File::options()
            .read(true)
            .custom_flags(libc::O_PATH | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(staged)?;
        self.verify_descriptor(&descriptor, false)
    }
    pub fn exposed_devices(&self) -> Result<std::collections::BTreeSet<u64>> {
        self.mounts
            .values()
            .map(|m| Ok(m.handle.metadata()?.dev()))
            .collect()
    }

    pub fn open_relative(&self, path: &Path) -> Result<File> {
        if path.is_absolute()
            || path
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            bail!("relative authority path is not canonical");
        }
        open(&self.mounts[Path::new("")].handle, path)
    }

    pub fn matches_file(&self, relative: &Path, expected: &File) -> Result<()> {
        let actual = self.open_relative(relative)?.metadata()?;
        let expected = expected.metadata()?;
        if (actual.dev(), actual.ino(), actual.mode() & libc::S_IFMT)
            != (
                expected.dev(),
                expected.ino(),
                expected.mode() & libc::S_IFMT,
            )
        {
            bail!("storage file does not belong to pinned reservation");
        }
        Ok(())
    }

    pub fn mkdir_private_child(&self, basename: &str) -> Result<File> {
        // The caller must exclude every provider from this private parent.
        // Linux has no atomic mkdir-and-return-fd operation; hostile same-UID
        // control-plane mutation inside creation/open is outside this boundary.
        let parent = self
            .mounts
            .get(Path::new(""))
            .context("missing root pin")?
            .handle
            .metadata()?;
        if !parent.is_dir()
            || parent.uid() != unsafe { libc::geteuid() }
            || parent.mode() & 0o077 != 0
        {
            bail!("allocation parent is not keeper-private");
        }
        if basename.is_empty() || basename == "." || basename == ".." || basename.contains('/') {
            bail!("invalid allocation basename");
        }
        let name = CString::new(basename)?;
        let root = &self.mounts[Path::new("")].handle;
        let metadata = root.metadata()?;
        if !metadata.is_dir()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.mode() & 0o077 != 0
        {
            bail!("allocation root must remain private and owned");
        }
        if unsafe { libc::mkdirat(root.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        let child = self.open_relative(Path::new(basename))?;
        if fs::read_dir(format!("/proc/self/fd/{}", child.as_raw_fd()))?
            .next()
            .is_some()
        {
            bail!("new allocation is not empty");
        }
        let sync_fd = unsafe {
            libc::openat(
                root.as_raw_fd(),
                c".".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
            )
        };
        if sync_fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        unsafe { File::from_raw_fd(sync_fd) }.sync_all()?;
        Ok(child)
    }

    /// Refuse stale source coordinates; never adopt a replacement for a pin.
    /// This is an admission consistency check, not a lock against host mutation.
    pub fn revalidate_coordinate(&self) -> Result<()> {
        let coordinate = self
            .source_coordinate
            .as_ref()
            .context("capture has no live admission coordinate")?;
        let before = fs::read("/proc/thread-self/mountinfo")?;
        let root = self
            .mounts
            .get(Path::new(""))
            .context("missing pinned root")?;
        let current = Self::pin_handle(coordinate, root.handle.try_clone()?, &before)?;
        if self.witness()? != current.witness()?
            || before != fs::read("/proc/thread-self/mountinfo")?
        {
            bail!("pinned source coordinate changed; fresh admission refused");
        }
        Ok(())
    }

    /// Compare only live, reauthenticated pins. Cached serialized witnesses
    /// alone cannot establish disjointness after an ordinary directory rename.
    pub fn overlaps_current(&self, other: &Self) -> Result<bool> {
        self.revalidate_coordinate()?;
        other.revalidate_coordinate()?;
        let result = self.witness()?.overlaps(&other.witness()?)?;
        self.revalidate_coordinate()?;
        other.revalidate_coordinate()?;
        Ok(result)
    }
    pub fn capture_digest(descriptor: &File) -> Result<String> {
        Self::capture_witness(descriptor)?.digest()
    }
    pub fn witness(&self) -> Result<TreeWitness> {
        use std::os::unix::ffi::OsStrExt;
        let mut rows = Vec::new();
        for (path, bound) in &self.mounts {
            let metadata = bound.handle.metadata()?;
            let mut options: Vec<Vec<u8>> =
                bound.options.split(|b| *b == b',').map(Vec::from).collect();
            options.sort();
            rows.push(MountWitness {
                relative: path.as_os_str().as_bytes().to_vec(),
                object: ObjectId {
                    device: metadata.dev(),
                    inode: metadata.ino(),
                    kind: metadata.mode() & libc::S_IFMT,
                },
                backing: bound.backing.clone(),
                options,
            });
        }
        rows.sort_by(|a, b| a.relative.cmp(&b.relative));
        let result = TreeWitness {
            version: 1,
            mounts: rows,
        };
        result.validate()?;
        Ok(result)
    }
    pub fn capture_witness(descriptor: &File) -> Result<TreeWitness> {
        let before = fs::read("/proc/thread-self/mountinfo")?;
        let capture = Self::bind_handle(Path::new("/"), descriptor.try_clone()?, &before, true)?;
        let result = capture.witness()?;
        if before != fs::read("/proc/thread-self/mountinfo")? {
            bail!("capture topology changed while digesting");
        }
        Ok(result)
    }

    pub fn verify_descriptor(&self, staged: &File, readonly: bool) -> Result<()> {
        let before = fs::read("/proc/thread-self/mountinfo")?;
        let actual = Self::bind_handle(Path::new("/"), staged.try_clone()?, &before, true)?;
        let expected_witness = if readonly {
            self.witness()?.readonly()?
        } else {
            self.witness()?
        };
        if expected_witness != actual.witness()? {
            bail!("captured backing authority changed");
        }

        if before != fs::read("/proc/thread-self/mountinfo")? {
            bail!("captured mount tree moved during verification");
        }
        Ok(())
    }
}
