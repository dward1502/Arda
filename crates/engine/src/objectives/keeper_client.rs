//! Bounded private transport to the independently supervised snapshot owner.
use super::{RetainedSnapshot, SnapshotAdmission};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const CONTROL_LIMIT: usize = 65_536;
pub const CONTROL_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum KeeperRequest {
    Prepare {
        run: String,
        workspace: PathBuf,
        identity: String,
    },
    Commit {
        snapshot: RetainedSnapshot,
        lease: super::snapshot_protocol::Lease,
    },
    Release {
        snapshot: RetainedSnapshot,
        run: String,
    },
    QueryTerminalRevocation {
        run: String,
        workspace: String,
        identity: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeeperFailure {
    MissingRevokedAuthority,
    OperatorAbandoned,
}
impl std::fmt::Display for KeeperFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::MissingRevokedAuthority => "revoked keeper admission has no saved authority",
            Self::OperatorAbandoned => "operator abandoned this execution attempt",
        })
    }
}
impl std::error::Error for KeeperFailure {}

#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KeeperResponse {
    pub ok: bool,
    pub snapshot: Option<RetainedSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<KeeperFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_revocation: Option<super::terminal_revocation::TerminalRevocationProof>,
}

/// Never derive Debug on transport payloads or capability-bearing clients.
pub struct KeeperClient {
    endpoint: PathBuf,
}
impl KeeperClient {
    pub fn new(endpoint: PathBuf) -> Self {
        Self { endpoint }
    }
    fn request(&self, request: KeeperRequest) -> Result<KeeperResponse> {
        let response: KeeperResponse = exchange(&self.endpoint, &request, CONTROL_TIMEOUT)?;
        if let Some(failure) = response.failure {
            if !response.ok
                && response.snapshot.is_none()
                && response.terminal_revocation.is_none()
                && (failure == KeeperFailure::OperatorAbandoned
                    || matches!(request, KeeperRequest::Release { .. }))
            {
                return Err(failure.into());
            }
            bail!("inconsistent keeper failure response");
        }
        if !response.ok {
            bail!("snapshot keeper refused operation; explicit reconciliation may be required");
        }
        match &request {
            KeeperRequest::QueryTerminalRevocation { .. } => {
                if response.snapshot.is_some() || response.terminal_revocation.is_none() {
                    bail!("keeper omitted non-capability revocation proof");
                }
            }
            _ if response.terminal_revocation.is_some() => {
                bail!("unexpected keeper revocation proof")
            }
            _ => {}
        }
        Ok(response)
    }
}
impl SnapshotAdmission for KeeperClient {
    fn terminal_revocation(
        &self,
        run: &str,
        workspace: &str,
        identity: &str,
    ) -> Result<super::terminal_revocation::TerminalRevocationProof> {
        let proof = self
            .request(KeeperRequest::QueryTerminalRevocation {
                run: run.into(),
                workspace: workspace.into(),
                identity: identity.into(),
            })?
            .terminal_revocation
            .context("keeper omitted terminal revocation proof")?;
        if proof.run != run || proof.workspace != workspace || proof.identity != identity {
            bail!("keeper terminal revocation response binding mismatch");
        }
        Ok(proof)
    }
    fn prepare(&self, run: &str, workspace: &Path, identity: &str) -> Result<RetainedSnapshot> {
        self.request(KeeperRequest::Prepare {
            run: run.into(),
            workspace: workspace.into(),
            identity: identity.into(),
        })?
        .snapshot
        .context("snapshot keeper omitted prepared authority")
    }
    fn commit(
        &self,
        snapshot: &RetainedSnapshot,
        run_id: &str,
        generation: i64,
        owner: &str,
        expires_ms: i64,
    ) -> Result<()> {
        self.request(KeeperRequest::Commit {
            snapshot: snapshot.clone(),
            lease: super::snapshot_protocol::Lease {
                run_id: run_id.into(),
                generation,
                owner: owner.into(),
                expires_ms,
            },
        })?;
        Ok(())
    }
    fn release(&self, snapshot: &RetainedSnapshot, run: &str) -> Result<()> {
        self.request(KeeperRequest::Release {
            snapshot: snapshot.clone(),
            run: run.into(),
        })?;
        Ok(())
    }
}

pub fn same_user(stream: &UnixStream) -> Result<()> {
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of_val(&cred) as libc::socklen_t;
    if unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    } != 0
        || cred.uid != unsafe { libc::geteuid() }
    {
        bail!("snapshot peer identity rejected");
    }
    Ok(())
}

pub fn read_frame(stream: &mut UnixStream, deadline: Instant) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        let left = deadline
            .checked_duration_since(Instant::now())
            .context("snapshot transport deadline exceeded")?;
        stream.set_read_timeout(Some(left))?;
        let mut byte = [0];
        if stream.read(&mut byte)? == 0 {
            bail!("incomplete snapshot control response");
        }
        bytes.push(byte[0]);
        if bytes.len() > CONTROL_LIMIT {
            bail!("snapshot control frame exceeds limit");
        }
        if byte[0] == b'\n' {
            return Ok(bytes);
        }
    }
}

pub fn exchange<T: Serialize, R: serde::de::DeserializeOwned>(
    endpoint: &Path,
    request: &T,
    budget: Duration,
) -> Result<R> {
    // Nonblocking connect avoids an unbounded wait on a full Unix backlog.
    let deadline = Instant::now() + budget;
    let address = std::os::unix::net::SocketAddr::from_pathname(endpoint)?;
    let fd = unsafe {
        libc::socket(
            libc::AF_UNIX,
            libc::SOCK_STREAM | libc::SOCK_NONBLOCK | libc::SOCK_CLOEXEC,
            0,
        )
    };
    if fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }
    use std::os::fd::FromRawFd;
    let mut stream = unsafe { UnixStream::from_raw_fd(fd) };
    // sockaddr_un has no portable std accessor. Build only a pathname address;
    // from_pathname above validates the platform's pathname length and NULs.
    use std::os::unix::ffi::OsStrExt;
    let mut raw: libc::sockaddr_un = unsafe { std::mem::zeroed() };
    raw.sun_family = libc::AF_UNIX as _;
    for (out, byte) in raw.sun_path.iter_mut().zip(
        address
            .as_pathname()
            .context("pathname required")?
            .as_os_str()
            .as_bytes(),
    ) {
        *out = *byte as _;
    }
    if unsafe {
        libc::connect(
            fd,
            (&raw as *const libc::sockaddr_un).cast(),
            std::mem::size_of_val(&raw) as _,
        )
    } != 0
    {
        // EAGAIN/full backlog and EINPROGRESS are not a successful connection.
        // Fail boundedly; durable callers retry the same operation/authority.
        bail!("snapshot control connection unavailable");
    }
    same_user(&stream)?;
    let mut bytes = serde_json::to_vec(request)?;
    bytes.push(b'\n');
    if bytes.len() > CONTROL_LIMIT {
        bail!("snapshot control request exceeds limit");
    }
    stream.set_nonblocking(false)?;
    let mut offset = 0;
    while offset < bytes.len() {
        let left = deadline
            .checked_duration_since(Instant::now())
            .context("snapshot transport deadline exceeded")?;
        stream.set_write_timeout(Some(left))?;
        let n = stream.write(&bytes[offset..])?;
        if n == 0 {
            bail!("snapshot control write closed");
        }
        offset += n;
    }
    serde_json::from_slice(&read_frame(&mut stream, deadline)?)
        .context("invalid snapshot control response")
}
