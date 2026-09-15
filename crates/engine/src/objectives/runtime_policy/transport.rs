//! Bounded immutable policy transfer. No JSON or credential values cross argv.
use super::{ValidatedRuntimePolicy, MAX_POLICY_BYTES};
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::io::Write;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::{FileExt, MetadataExt};
const SEALS: i32 = libc::F_SEAL_WRITE | libc::F_SEAL_GROW | libc::F_SEAL_SHRINK | libc::F_SEAL_SEAL;

pub fn seal(policy: &ValidatedRuntimePolicy) -> Result<OwnedFd> {
    let bytes = policy.bytes()?;
    seal_bytes(&bytes, c"arda-runtime-policy", MAX_POLICY_BYTES)
}

pub(crate) fn seal_bytes(bytes: &[u8], label: &std::ffi::CStr, limit: usize) -> Result<OwnedFd> {
    if bytes.is_empty() || bytes.len() > limit {
        bail!("runtime policy exceeds transport limit");
    }
    let raw =
        unsafe { libc::memfd_create(label.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
    if raw < 0 {
        return Err(std::io::Error::last_os_error()).context("create policy memfd");
    }
    let mut file = unsafe { File::from_raw_fd(raw) };
    file.write_all(bytes)?;
    if unsafe { libc::fcntl(file.as_raw_fd(), libc::F_ADD_SEALS, SEALS) } < 0 {
        return Err(std::io::Error::last_os_error()).context("seal policy memfd");
    }
    Ok(file.into())
}

pub fn read(fd: OwnedFd) -> Result<ValidatedRuntimePolicy> {
    ValidatedRuntimePolicy::parse(&read_bytes(fd, "arda-runtime-policy", MAX_POLICY_BYTES)?)
}

pub(crate) fn read_bytes(fd: OwnedFd, label: &str, limit: usize) -> Result<Vec<u8>> {
    let file = File::from(fd);
    let seals = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GET_SEALS) };
    if seals < 0 || seals & SEALS != SEALS {
        bail!("runtime policy descriptor is not immutable");
    }
    let metadata = file.metadata()?;
    // Check the kernel's memfd identity in addition to seals: ordinary files or
    // sealable tmpfs files must not become an accidental transport alternative.
    let link = std::fs::read_link(format!("/proc/self/fd/{}", file.as_raw_fd()))?;
    if !metadata.is_file()
        || !link
            .as_os_str()
            .as_encoded_bytes()
            .eq(format!("/memfd:{label} (deleted)").as_bytes())
        || seals < 0
        || seals & SEALS != SEALS
        || metadata.len() == 0
        || metadata.len() > limit as u64
    {
        bail!("invalid sealed runtime policy descriptor");
    }
    let mut bytes = vec![0; metadata.size() as usize];
    file.read_exact_at(&mut bytes, 0)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Seek, SeekFrom};
    fn memfd(bytes: &[u8], sealed: bool) -> OwnedFd {
        let raw = unsafe {
            libc::memfd_create(
                c"arda-runtime-policy".as_ptr(),
                libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING,
            )
        };
        assert!(raw >= 0);
        let mut file = unsafe { File::from_raw_fd(raw) };
        file.write_all(bytes).unwrap();
        file.seek(SeekFrom::End(0)).unwrap();
        if sealed {
            assert_eq!(unsafe { libc::fcntl(raw, libc::F_ADD_SEALS, SEALS) }, 0);
        }
        file.into()
    }
    #[test]
    fn rejects_unsealed_oversized_malformed_and_ordinary_descriptors() {
        for fd in [
            memfd(b"{}", false),
            memfd(&vec![b' '; MAX_POLICY_BYTES + 1], true),
            memfd(b"{", true),
            tempfile::tempfile().unwrap().into(),
        ] {
            let raw = fd.as_raw_fd();
            assert!(read(fd).is_err());
            assert_eq!(unsafe { libc::fcntl(raw, libc::F_GETFD) }, -1);
        }
    }
    #[test]
    fn sealed_transport_roundtrips_independent_of_cursor() {
        let policy = ValidatedRuntimePolicy::validate(super::super::tests::fixture()).unwrap();
        let fd = seal(&policy).unwrap();
        let raw = fd.as_raw_fd();
        assert_eq!(
            unsafe { libc::fcntl(raw, libc::F_GETFD) } & libc::FD_CLOEXEC,
            libc::FD_CLOEXEC
        );
        let reopened = read(fd).unwrap();
        assert_eq!(reopened.digest(), policy.digest());
    }
}
