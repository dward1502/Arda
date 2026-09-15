//! A captured command owns its reserved parent slot until the launch owner reaps.
use std::fs::File;
use std::io;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;
use tokio::process::{Child, Command};
#[cfg(all(test, target_os = "linux"))]
mod tests;

pub(super) struct PreparedCommand {
    pub(super) command: Command,
    parent_slot: Option<File>,
}

impl PreparedCommand {
    pub(super) fn captured(command: Command, parent_slot: File) -> Self {
        Self {
            command,
            parent_slot: Some(parent_slot),
        }
    }

    #[cfg(test)]
    pub(super) fn plain(command: Command) -> Self {
        Self {
            command,
            parent_slot: None,
        }
    }

    // Call only inside an owner that remains on this OS thread through reap.
    pub(super) fn spawn_checked(
        &mut self,
        before_spawn: impl FnOnce() -> io::Result<()>,
    ) -> io::Result<Child> {
        #[cfg(target_os = "linux")]
        if let Some(slot) = &self.parent_slot {
            let identity = crate::objectives::captured_fds::spawning_thread_pidfd()?;
            if unsafe { libc::dup3(identity.as_raw_fd(), slot.as_raw_fd(), libc::O_CLOEXEC) } == -1
            {
                return Err(io::Error::last_os_error());
            }
        }
        #[cfg(not(target_os = "linux"))]
        if self.parent_slot.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "captured execution requires Linux",
            ));
        }
        before_spawn()?;
        self.command.spawn()
    }

    #[cfg(test)]
    pub(super) fn spawn_owned(&mut self) -> io::Result<Child> {
        self.spawn_checked(|| Ok(()))
    }

    #[cfg(test)]
    pub(super) fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<std::ffi::OsStr>,
    {
        self.command.args(args);
        self
    }
    #[cfg(test)]
    pub(super) fn arg(&mut self, arg: impl AsRef<std::ffi::OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }
    #[cfg(test)]
    pub(super) fn stdin(&mut self, value: std::process::Stdio) -> &mut Self {
        self.command.stdin(value);
        self
    }
    #[cfg(test)]
    pub(super) fn stdout(&mut self, value: std::process::Stdio) -> &mut Self {
        self.command.stdout(value);
        self
    }
    #[cfg(test)]
    pub(super) fn stderr(&mut self, value: std::process::Stdio) -> &mut Self {
        self.command.stderr(value);
        self
    }
    #[cfg(test)]
    pub(super) fn as_std(&self) -> &std::process::Command {
        self.command.as_std()
    }
    #[cfg(test)]
    pub(super) fn kill_on_drop(&mut self, value: bool) -> &mut Self {
        self.command.kill_on_drop(value);
        self
    }
    #[cfg(test)]
    pub(super) async fn output(&mut self) -> io::Result<std::process::Output> {
        self.command
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        self.spawn_owned()?.wait_with_output().await
    }
    #[cfg(test)]
    pub(super) fn spawn(&mut self) -> io::Result<Child> {
        self.spawn_owned()
    }
}

pub(super) struct CallerDrop(pub(super) Option<crate::adapters::AdapterCancellation>);
impl Drop for CallerDrop {
    fn drop(&mut self) {
        if let Some(cancellation) = &self.0 {
            cancellation.cancel();
        }
    }
}
