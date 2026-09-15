//! systemd readiness is emitted only after storage, policy and socket binding.
use anyhow::{bail, Result};
use std::{
    ffi::OsStr,
    os::{
        linux::net::SocketAddrExt,
        unix::{
            ffi::OsStrExt,
            net::{SocketAddr, UnixDatagram},
        },
    },
};

pub(super) fn notify(address: Option<&OsStr>) -> Result<()> {
    let Some(address) = address else {
        return Ok(());
    };
    let bytes = address.as_bytes();
    let target = if bytes.first() == Some(&b'@') {
        SocketAddr::from_abstract_name(&bytes[1..])?
    } else if bytes.first() == Some(&b'/') {
        SocketAddr::from_pathname(address)?
    } else {
        bail!("invalid readiness endpoint");
    };
    let socket = UnixDatagram::unbound()?;
    socket.set_write_timeout(Some(std::time::Duration::from_secs(1)))?;
    socket.send_to_addr(b"READY=1", &target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_notification_and_invalid_endpoint() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("notify.sock");
        let listener = UnixDatagram::bind(&path).unwrap();
        listener
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        notify(Some(path.as_os_str())).unwrap();
        let mut bytes = [0; 64];
        let count = listener.recv(&mut bytes).unwrap();
        assert_eq!(&bytes[..count], b"READY=1");
        assert!(notify(Some(OsStr::new("relative.sock"))).is_err());
        notify(None).unwrap();
    }
}
