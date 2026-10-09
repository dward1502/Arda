//! Parse once from admitted object identity, not mutable host pathnames.
use super::CapturedGrant;
use anyhow::{ensure, Context, Result};
use arda_core::local_runtime_endpoint::LocalRuntimeEndpoint;
use arda_engine::objectives::runtime_policy::{
    GrantAccess, GrantKind, GrantRole, ValidatedRuntimePolicy,
};
use std::{
    fs::File,
    io::Read,
    os::{fd::AsRawFd, unix::fs::MetadataExt},
    path::Path,
};

#[cfg(test)]
#[path = "endpoint_tests.rs"]
mod authority_tests;

const DESTINATION: &str = "/run/arda-runtime/services.toml";

pub(super) fn capture(
    policy: &ValidatedRuntimePolicy,
    grants: &[CapturedGrant],
) -> Result<Option<String>> {
    let Some(declared) = policy
        .policy()
        .grants
        .iter()
        .find(|g| g.destination == Path::new(DESTINATION))
    else {
        return Ok(None);
    };
    ensure!(
        declared.role == GrantRole::NetworkInput
            && declared.kind == GrantKind::File
            && declared.access == GrantAccess::ReadOnly,
        "Manwe manifest requires readonly NetworkInput file authority"
    );
    let captured = grants
        .iter()
        .find(|g| g.identity.id == declared.id)
        .context("missing captured Manwe manifest")?;
    ensure!(
        captured.identity.destination == declared.destination
            && captured.identity.kind == declared.kind
            && captured.identity.access == declared.access,
        "Manwe manifest grant identity mismatch"
    );
    let descriptor = &captured.descriptor;
    let metadata = descriptor.metadata()?;
    ensure!(
        metadata.dev() == captured.identity.device && metadata.ino() == captured.identity.inode,
        "Manwe manifest descriptor identity mismatch"
    );
    Ok(Some(read_endpoint(descriptor)?))
}

fn read_endpoint(descriptor: &File) -> Result<String> {
    ensure!(
        descriptor.metadata()?.is_file(),
        "Manwe manifest is not regular"
    );
    // Grant capture supplies O_PATH. Reopen that exact pinned object for reading.
    let mut raw = Vec::new();
    File::open(format!("/proc/self/fd/{}", descriptor.as_raw_fd()))?
        .take(65_537)
        .read_to_end(&mut raw)?;
    ensure!(raw.len() <= 65_536, "Manwe manifest exceeds bound");
    Ok(LocalRuntimeEndpoint::from_services_toml(std::str::from_utf8(&raw)?)?.base_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::OpenOptionsExt;
    fn manifest(port: u16) -> String {
        format!("[[service]]\nname='manwe'\nstart={{args=['--bind=127.0.0.1','--port={port}']}}\nhealth={{url='http://127.0.0.1:{port}/healthz'}}\n")
    }
    #[test]
    fn descriptor_survives_path_substitution_and_value_survives_in_place_write() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("services.toml");
        std::fs::write(&path, manifest(17171)).unwrap();
        let fd = File::options()
            .read(true)
            .custom_flags(libc::O_PATH)
            .open(&path)
            .unwrap();
        std::fs::rename(&path, temp.path().join("old")).unwrap();
        std::fs::write(&path, manifest(27171)).unwrap();
        let selected = read_endpoint(&fd).unwrap();
        assert_eq!(selected, "http://127.0.0.1:17171");
        std::fs::write(temp.path().join("old"), manifest(37171)).unwrap();
        assert_eq!(selected, "http://127.0.0.1:17171");
        assert_eq!(read_endpoint(&fd).unwrap(), "http://127.0.0.1:37171");
    }
    #[test]
    fn malformed_and_oversized_manifests_refuse() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("manifest");
        for raw in [
            b"bad".to_vec(),
            vec![b' '; 65_537],
            manifest(0).into_bytes(),
        ] {
            std::fs::write(&path, raw).unwrap();
            assert!(read_endpoint(&File::open(&path).unwrap()).is_err());
        }
        assert!(read_endpoint(&File::open(temp.path()).unwrap()).is_err());
    }
}
