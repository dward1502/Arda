//! Synchronous capture barrier; the returned command owns all capabilities.
use std::fs::File;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
#[cfg(test)]
mod tests;

struct CaptureChild(std::process::Child);
impl std::ops::Deref for CaptureChild {
    type Target = std::process::Child;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for CaptureChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for CaptureChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(super) fn worker() -> io::Result<PathBuf> {
    let executable = std::env::current_exe()?;
    let mut directory = executable
        .parent()
        .ok_or_else(|| io::Error::other("executable directory missing"))?;
    // Cargo integration/library test executables are under target/*/deps.
    if directory.file_name().is_some_and(|name| name == "deps") {
        directory = directory
            .parent()
            .ok_or_else(|| io::Error::other("build directory missing"))?;
    }
    let worker = directory.join("arda-snapshot-worker");
    if !worker.is_file() {
        return Err(io::Error::other(
            "trusted sibling arda-snapshot-worker is not installed",
        ));
    }
    Ok(worker)
}

pub(super) fn capture(
    worker: &Path,
    grants: &[(PathBuf, String)],
    deadline: Instant,
    cancellation: &super::CaptureStop,
) -> io::Result<Vec<File>> {
    if cancellation.is_cancelled() || Instant::now() >= deadline {
        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "capture cancelled or expired",
        ));
    }
    let (parent, child) = UnixStream::pair()?;
    parent.set_read_timeout(Some(Duration::from_millis(20)))?;
    let fd = child.as_raw_fd();
    let parent_identity = crate::objectives::captured_fds::spawning_thread_pidfd()?;
    let parent_fd = parent_identity.as_raw_fd();
    let mut command = Command::new(worker);
    command
        .env_clear()
        .arg("--capture-ephemeral")
        .arg(parent_fd.to_string())
        .arg(fd.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    for (root, identity) in grants {
        command.arg(root).arg(identity);
    }
    unsafe {
        command.pre_exec(move || {
            crate::objectives::captured_fds::arm_parent_death(parent_fd)?;
            if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 4u32) == -1
                || libc::fcntl(fd, libc::F_SETFD, 0) == -1
                || libc::fcntl(parent_fd, libc::F_SETFD, 0) == -1
            {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut process = CaptureChild(command.spawn()?);
    drop(child);
    let result = loop {
        if cancellation.is_cancelled() || Instant::now() >= deadline {
            break Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "capture cancelled or expired",
            ));
        }
        match crate::objectives::captured_fds::receive(&parent, grants.len() + 2) {
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                continue
            }
            result => break result,
        }
    };
    let status = loop {
        match process.try_wait() {
            Ok(Some(status)) => break Ok(status),
            Ok(None)
                if result.is_ok() && !cancellation.is_cancelled() && Instant::now() < deadline =>
            {
                std::thread::sleep(Duration::from_millis(5))
            }
            _ => {
                let _ = process.kill();
                let _ = process.wait();
                break Err(io::Error::other(
                    "capture helper failed or exceeded startup deadline",
                ));
            }
        }
    };
    let files = result?;
    if !status?.success() {
        return Err(io::Error::other("capture helper exited unsuccessfully"));
    }
    Ok(files.into_iter().map(File::from).collect())
}
