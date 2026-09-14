//! Own a provider leader until cleanup, including early-return error paths.
use std::process::{Child, ExitStatus};
use std::time::{Duration, Instant};

pub struct ProviderChild {
    child: Child,
    reaped: bool,
    complete: bool,
    deadline: Option<Instant>,
}

impl ProviderChild {
    pub fn new(child: Child) -> Self {
        Self {
            child,
            reaped: false,
            complete: false,
            deadline: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.child.id()
    }

    pub fn take_output(
        &mut self,
    ) -> (
        Option<std::process::ChildStdout>,
        Option<std::process::ChildStderr>,
    ) {
        (self.child.stdout.take(), self.child.stderr.take())
    }

    pub fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let status = self.child.try_wait()?;
        self.reaped |= status.is_some();
        Ok(status)
    }

    pub fn begin_cleanup(&mut self) -> Instant {
        *self
            .deadline
            .get_or_insert_with(|| Instant::now() + Duration::from_secs(2))
    }

    pub fn finish(&mut self) {
        self.complete = true;
    }
}

impl Drop for ProviderChild {
    fn drop(&mut self) {
        if self.complete || self.reaped {
            return;
        }
        let deadline = self.begin_cleanup();
        // The unreaped leader still reserves its identity. Never signal a group
        // after try_wait has reaped it; numeric PID/PGID reuse is then possible.
        unsafe {
            libc::kill(-(self.child.id() as i32), libc::SIGKILL);
        }
        let _ = self.child.kill();
        while Instant::now() < deadline {
            match self.try_wait() {
                Ok(Some(_)) => break,
                Err(error) if error.kind() != std::io::ErrorKind::Interrupted => break,
                _ => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        // Best effort is not proof of tree disappearance. The caller's poisoned
        // state remains set on every error, even if this guard reaps the leader.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    use std::process::Command;

    #[test]
    fn early_return_terminates_and_reaps_owned_leader() {
        let mut command = Command::new("/bin/sleep");
        command.arg("30").process_group(0);
        let child = ProviderChild::new(command.spawn().unwrap());
        let raw = unsafe { libc::syscall(libc::SYS_pidfd_open, child.id(), 0) };
        assert!(raw >= 0);
        let fd = unsafe { OwnedFd::from_raw_fd(raw as i32) };
        let pid = child.id();
        let started = Instant::now();
        drop(child); // models any ? return after spawn
        let mut poll = libc::pollfd {
            fd: fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let dead = unsafe { libc::poll(&mut poll, 1, 0) } == 1;
        // Fixture cleanup must not manufacture the pre-cleanup success verdict.
        if !dead {
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    fd.as_raw_fd(),
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
        }
        assert!(dead, "early return left provider alive");
        assert!(started.elapsed() < Duration::from_secs(3));
        let mut status = 0;
        assert_eq!(
            unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}
