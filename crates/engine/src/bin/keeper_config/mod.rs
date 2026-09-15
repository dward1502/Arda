//! Read-only runtime-policy validation shared by startup and installation checks.
use anyhow::{bail, Result};
use arda_engine::objectives::runtime_policy::{ValidatedRuntimePolicy, MAX_POLICY_BYTES};
use std::{
    fs,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

pub(super) fn load(path: &Path) -> Result<ValidatedRuntimePolicy> {
    let file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o077 != 0
    {
        bail!("runtime policy must be a private owned regular file");
    }
    let mut bytes = Vec::new();
    file.take((MAX_POLICY_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    ValidatedRuntimePolicy::parse(&bytes)
}
