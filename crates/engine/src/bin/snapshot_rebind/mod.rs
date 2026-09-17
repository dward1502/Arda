//! Accept authenticated newer Commit or terminal Release during execution.
//! Save the request without an ACK; the serial owner applies it after teardown.
use super::*;

pub struct Pending {
    pub stream: UnixStream,
    pub request: Request,
}

pub struct Control<'a> {
    pub listener: &'a UnixListener,
    pub manifest: &'a Manifest,
    pub digest: &'a str,
    pub pending: &'a mut Option<Pending>,
}
impl Control<'_> {
    pub fn poll(
        &mut self,
        binding: &snapshot_lease::Binding,
        execution_deadline: Instant,
    ) -> Result<bool> {
        let mut ready = libc::pollfd {
            fd: self.listener.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        if unsafe { libc::poll(&mut ready, 1, 0) } < 0 {
            return Err(std::io::Error::last_os_error()).context("poll rebind control");
        }
        if ready.revents & libc::POLLIN == 0 {
            return Ok(false);
        }
        let (mut stream, _) = self.listener.accept()?;
        let read_deadline = execution_deadline.min(Instant::now() + Duration::from_millis(20));
        let admitted = (|| -> Result<Request> {
            same_user(&stream)?;
            let lease_remaining = binding.remaining(&binding.lease, snapshot_lease::now_ms()?)?;
            let request: Request = serde_json::from_slice(&read_request_until(
                &mut stream,
                read_deadline.min(Instant::now() + lease_remaining),
            )?)?;
            match &request {
                Request::Release { capability } => {
                    if capability != &self.manifest.capability {
                        bail!("snapshot capability mismatch");
                    }
                    // Release has no lease-renewal semantics. Queue it without
                    // ACK; the serial owner checks teardown before retiring pins.
                    Ok(request)
                }
                Request::Commit {
                    capability,
                    manifest_digest,
                    lease,
                } => {
                    if capability != &self.manifest.capability || manifest_digest != self.digest {
                        bail!("snapshot admission binding mismatch");
                    }
                    if lease.generation <= binding.lease.generation {
                        bail!("active snapshot requires newer rebind generation");
                    }
                    // Validate using the same rules as serial Commit, without
                    // mutating the current lease or extending its deadline.
                    let mut candidate = Some(binding.clone());
                    snapshot_lease::Binding::commit(
                        &mut candidate,
                        lease.clone(),
                        snapshot_lease::now_ms()?,
                    )?;
                    Ok(request)
                }
                Request::Execute { lease, .. } | Request::Runtime { lease, .. } => {
                    binding.remaining(lease, snapshot_lease::now_ms()?)?;
                    bail!("snapshot execution busy");
                }
                _ => bail!("snapshot execution busy"),
            }
        })();
        match admitted {
            Ok(request) => {
                *self.pending = Some(Pending { stream, request });
                Ok(true)
            }
            Err(error) => {
                // A peer that does not drain replies must not stall supervision.
                stream.set_nonblocking(true)?;
                let _ = writeln!(
                    stream,
                    "{}",
                    serde_json::json!({"ok": false, "error": error.to_string()})
                );
                Ok(false)
            }
        }
    }
}
