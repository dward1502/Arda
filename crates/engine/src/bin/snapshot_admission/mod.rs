//! Bind capture to the store's pre-existing admission, across namespace cloning.
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

type Identity = (u32, PathBuf, PathBuf, Option<(u64, u64)>, String);
mod physical;

pub struct Admission {
    root: PathBuf,
    device: u64,
    inode: u64,
    topology: Vec<Vec<u8>>,
    physical: physical::Physical,
}

// Mount IDs and propagation peer IDs change when cloning/privatizing a
// namespace; enumeration order can change too. Preserve parent chains (including
// stacked mounts), not raw IDs or listing order, alongside every source field.
fn stable(topology: &[u8]) -> Result<Vec<Vec<u8>>> {
    let entries = topology
        .split(|b| *b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields: Vec<_> = line.split(|b| *b == b' ').collect();
            let separator = fields
                .iter()
                .position(|field| *field == b"-")
                .context("invalid admission topology")?;
            if separator < 6 || fields.len() != separator + 4 {
                bail!("invalid admission topology");
            }
            let mut stable = Vec::new();
            for field in fields[2..6].iter().chain(fields[separator + 1..].iter()) {
                stable.extend_from_slice(field);
                stable.push(0);
            }
            Ok((fields[0].to_vec(), (fields[1].to_vec(), stable)))
        })
        .collect::<Result<std::collections::BTreeMap<_, _>>>()?;
    let mut result = Vec::new();
    for id in entries.keys() {
        let mut cursor = id;
        let mut chain = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        while let Some((parent, fields)) = entries.get(cursor) {
            if !seen.insert(cursor) {
                bail!("cyclic admission topology");
            }
            chain.extend_from_slice(fields);
            chain.push(0);
            if parent == cursor {
                break;
            }
            cursor = parent;
        }
        result.push(chain);
    }
    result.sort();
    Ok(result)
}
impl Admission {
    pub fn before_clone(root: &Path, encoded: &str) -> Result<Self> {
        let (version, approved, anchor, id, digest): Identity = serde_json::from_str(encoded)?;
        let topology = fs::read("/proc/thread-self/mountinfo")?;
        if version != 2
            || approved != root
            || anchor != approved
            || digest != format!("{:x}", Sha256::digest(&topology))
        {
            bail!("snapshot admission topology or root does not match");
        }
        let (device, inode) = id.context("snapshot admission requires physical identity")?;
        let admission = Self {
            root: approved,
            device,
            inode,
            topology: stable(&topology)?,
            physical: physical::Physical::pin(root, &topology)?,
        };
        admission.check_root(&fs::metadata(root)?)?;
        if topology != fs::read("/proc/thread-self/mountinfo")? {
            bail!("admission changed while binding physical mounts");
        }
        Ok(admission)
    }
    pub fn after_clone(&self, root: &Path) -> Result<()> {
        if root != self.root || stable(&fs::read("/proc/thread-self/mountinfo")?)? != self.topology {
            bail!("snapshot capture topology changed");
        }
        self.check_root(&fs::metadata(root)?)
    }
    pub fn check_root(&self, metadata: &fs::Metadata) -> Result<()> {
        if !metadata.is_dir() || metadata.dev() != self.device || metadata.ino() != self.inode {
            bail!("snapshot capture root changed");
        }
        Ok(())
    }
    pub fn check_capture(&self, staged: &Path) -> Result<()> {
        self.check_root(&fs::metadata(staged)?)?;
        self.physical.verify(staged)
    }
}

#[cfg(test)]
mod tests {
    use super::stable;
    #[test]
    fn clone_identity_ignores_ids_and_listing_order_but_preserves_stacks() {
        let original = b"1 0 8:1 / / rw shared:1 - ext4 /dev/root rw\n2 1 8:2 /one /work rw - ext4 /dev/data rw\n3 2 8:3 /two /work rw - ext4 /dev/other rw\n";
        let cloned = b"33 22 8:3 /two /work rw - ext4 /dev/other rw\n11 0 8:1 / / rw - ext4 /dev/root rw\n22 11 8:2 /one /work rw - ext4 /dev/data rw\n";
        assert_eq!(stable(original).unwrap(), stable(cloned).unwrap());
        let reversed = b"11 0 8:1 / / rw - ext4 /dev/root rw\n22 33 8:2 /one /work rw - ext4 /dev/data rw\n33 11 8:3 /two /work rw - ext4 /dev/other rw\n";
        assert_ne!(stable(original).unwrap(), stable(reversed).unwrap());
    }
}
