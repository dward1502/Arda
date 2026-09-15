//! Clone-independent, byte-preserving physical tree witnesses. No FD authority
//! is serialized: the keeper must retain the corresponding pins through Inspect.
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::{Component, Path},
};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectId {
    pub device: u64,
    pub inode: u64,
    pub kind: u32,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackingView {
    pub major: u32,
    pub minor: u32,
    pub root: Vec<u8>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MountWitness {
    pub relative: Vec<u8>,
    pub object: ObjectId,
    pub backing: BackingView,
    pub options: Vec<Vec<u8>>,
}
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeWitness {
    pub version: u32,
    pub mounts: Vec<MountWitness>,
}
fn clean(raw: &[u8], absolute: bool) -> bool {
    let path = Path::new(OsStr::from_bytes(raw));
    raw.len() <= 4096
        && !raw.contains(&0)
        && path.is_absolute() == absolute
        && path
            .components()
            .all(|c| matches!(c, Component::RootDir | Component::Normal(_)))
        && path
            .components()
            .collect::<std::path::PathBuf>()
            .as_os_str()
            .as_bytes()
            == raw
        && !raw.ends_with(b" (deleted)")
}
impl TreeWitness {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.mounts.is_empty()
            || self.mounts.len() > 128
            || !self.mounts[0].relative.is_empty()
        {
            bail!("invalid tree witness shape");
        }
        let mut previous: Option<&[u8]> = None;
        for mount in &self.mounts {
            if !clean(&mount.relative, false)
                || !clean(&mount.backing.root, true)
                || previous.is_some_and(|p| p >= mount.relative.as_slice())
                || mount.object.inode == 0
                || ![libc::S_IFDIR, libc::S_IFREG].contains(&mount.object.kind)
                || libc::major(mount.object.device) != mount.backing.major
                || libc::minor(mount.object.device) != mount.backing.minor
            {
                bail!("invalid tree object or backing coordinate");
            }
            previous = Some(&mount.relative);
            if mount.options.is_empty()
                || mount.options.len() > 64
                || mount
                    .options
                    .iter()
                    .any(|o| o.is_empty() || o.len() > 256 || o.contains(&0) || o.contains(&b','))
                || mount.options.windows(2).any(|p| p[0] >= p[1])
                || mount
                    .options
                    .iter()
                    .filter(|o| o.as_slice() == b"ro" || o.as_slice() == b"rw")
                    .count()
                    != 1
            {
                bail!("invalid mount option witness");
            }
        }
        Ok(())
    }
    pub fn readonly(&self) -> Result<Self> {
        self.validate()?;
        let mut copy = self.clone();
        for mount in &mut copy.mounts {
            mount
                .options
                .retain(|o| o.as_slice() != b"rw" && o.as_slice() != b"ro");
            mount.options.push(b"ro".to_vec());
            mount.options.sort();
        }
        copy.validate()?;
        Ok(copy)
    }
    pub fn digest(&self) -> Result<String> {
        self.validate()?;
        Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(self)?)))
    }
    /// Compare recorded mount-tree coordinates, NOT arbitrary current objects.
    /// A false result is not an exclusion decision without live pin/coordinate
    /// revalidation. This does not inventory hardlinked descendants or freeze
    /// contents/ancestry; those are outside retained mount-tree guarantees.
    pub fn overlaps(&self, other: &Self) -> Result<bool> {
        self.validate()?;
        other.validate()?;
        Ok(self.mounts.iter().any(|a| {
            other.mounts.iter().any(|b| {
                if a.object == b.object {
                    return true;
                }
                let left = Path::new(OsStr::from_bytes(&a.backing.root));
                let right = Path::new(OsStr::from_bytes(&b.backing.root));
                a.backing.major == b.backing.major
                    && a.backing.minor == b.backing.minor
                    && (left.starts_with(right) || right.starts_with(left))
            })
        }))
    }
}
