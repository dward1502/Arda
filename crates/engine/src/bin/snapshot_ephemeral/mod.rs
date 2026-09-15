//! Ephemeral namespace capabilities for the ordinary subprocess adapter.
//! No keeper lease or durable state is created by this mode.
use super::{mount, snapshot_admission::Admission};
use anyhow::{bail, Context, Result};
use arda_engine::objectives::captured_fds;
use std::ffi::OsString;
use std::fs::{self, File};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::net::UnixStream;

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

fn fd(value: &OsString) -> Result<i32> {
    let fd = value.to_str().context("non-UTF8 descriptor")?.parse()?;
    if fd < 3 {
        bail!("capture descriptor must not be standard IO");
    }
    Ok(fd)
}

pub fn capture(args: &[OsString]) -> Result<()> {
    if args.len() != 4 && args.len() != 6 {
        bail!("invalid ephemeral capture arguments");
    }
    let parent = unsafe { File::from_raw_fd(fd(&args[0])?) };
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    let channel = unsafe { UnixStream::from_raw_fd(fd(&args[1])?) };
    channel.set_write_timeout(Some(std::time::Duration::from_secs(5)))?;
    let mut admissions = Vec::new();
    for pair in args[2..].as_chunks::<2>().0 {
        let root = Path::new(&pair[0]);
        if root == Path::new("/") {
            bail!("workspace/profile root cannot be filesystem root");
        }
        admissions.push((
            root,
            Admission::before_clone(root, pair[1].to_str().context("invalid admission")?)?,
        ));
    }
    let uid = unsafe { libc::getuid() };
    let gid = unsafe { libc::getgid() };
    if unsafe { libc::unshare(libc::CLONE_NEWUSER | libc::CLONE_NEWNS) } == -1 {
        return Err(std::io::Error::last_os_error()).context("isolate ephemeral capture");
    }
    fs::write("/proc/self/setgroups", "deny")?;
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    fs::write("/proc/self/uid_map", format!("0 {uid} 1\n"))?;
    fs::write("/proc/self/gid_map", format!("0 {gid} 1\n"))?;
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    mount(None, Path::new("/"), None, libc::MS_REC | libc::MS_PRIVATE)?;
    // Holding the whole private namespace retains the captured mounts without
    // staging directories. These FDs belong to that namespace, not the caller's.
    let mut files = vec![
        File::open("/proc/self/ns/user")?,
        File::open("/proc/self/ns/mnt")?,
    ];
    for (root, admission) in &admissions {
        admission.after_clone(root)?;
    }
    if admissions.len() == 2 {
        arda_engine::objectives::validate_snapshot_owner_paths(
            admissions[0].0,
            &[admissions[1].0],
        )?;
    }
    for (root, admission) in admissions {
        // Give the grant its own mount-ID subtree for physical verification.
        // This self-bind exists only in the already-private captured namespace.
        mount(Some(root), root, None, libc::MS_BIND | libc::MS_REC)?;
        let file = File::open(root)?;
        admission.check_capture(root)?;
        admission.check_descriptor(&file)?;
        files.push(file);
    }
    captured_fds::send(
        &channel,
        &files.iter().map(AsRawFd::as_raw_fd).collect::<Vec<_>>(),
    )?;
    Ok(())
}

pub fn launch(args: &[OsString]) -> Result<()> {
    if args.len() < 4 {
        bail!("invalid captured launch arguments");
    }
    let parent = unsafe { File::from_raw_fd(fd(&args[0])?) };
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    if unsafe { libc::fcntl(parent.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC) } == -1 {
        return Err(std::io::Error::last_os_error()).context("close parent capability on exec");
    }
    let user = unsafe { File::from_raw_fd(fd(&args[1])?) };
    let mounts = unsafe { File::from_raw_fd(fd(&args[2])?) };
    if unsafe { libc::setns(user.as_raw_fd(), libc::CLONE_NEWUSER) } == -1 {
        return Err(std::io::Error::last_os_error()).context("enter captured user namespace");
    }
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    if unsafe { libc::setns(mounts.as_raw_fd(), libc::CLONE_NEWNS) } == -1 {
        return Err(std::io::Error::last_os_error()).context("enter captured namespace");
    }
    std::env::set_current_dir("/")?;
    drop(user);
    drop(mounts);
    captured_fds::arm_parent_death(parent.as_raw_fd())?;
    // Exec preserves the ordinary adapter's process-group and pipe ownership.
    // Bubblewrap establishes its own PID namespace and parent-death boundary.
    Err(Command::new("/usr/bin/bwrap")
        .env_clear()
        .args(&args[3..])
        .exec())
    .context("launch captured bubblewrap")
}
