//! Linux filesystem containment for an already selected provider workspace.
//! The descriptor pins the directory; polling only requests bounded cleanup.
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use tokio::process::Command;

#[derive(Debug, thiserror::Error)]
#[error("provider workspace or mount topology changed")]
pub(super) struct WorkspaceChanged;

#[derive(Debug)]
pub(super) struct PinnedWorkspace {
    directory: File,
    supplied: PathBuf,
    canonical: PathBuf,
    topology: Vec<u8>,
}

impl PinnedWorkspace {
    pub(super) fn open(path: &Path) -> io::Result<Self> {
        let supplied = std::path::absolute(path)?;
        let canonical = supplied.canonicalize()?;
        let directory = File::open(&canonical)?;
        if !directory.metadata()?.is_dir() {
            return Err(io::Error::other("provider workspace is not a directory"));
        }
        let pinned = Self {
            directory,
            supplied,
            canonical,
            topology: fs::read("/proc/self/mountinfo")?,
        };
        pinned.validate()?;
        Ok(pinned)
    }

    pub(super) fn validate(&self) -> io::Result<()> {
        let current = self.supplied.canonicalize()?;
        let metadata = fs::metadata(&current)?;
        let pinned = self.directory.metadata()?;
        if current != self.canonical
            || (metadata.dev(), metadata.ino()) != (pinned.dev(), pinned.ino())
            || fs::read("/proc/self/mountinfo")? != self.topology
        {
            return Err(io::Error::other(
                "provider workspace or mount topology changed",
            ));
        }
        Ok(())
    }

    pub(super) fn admission_identity(&self) -> io::Result<String> {
        let metadata = self.directory.metadata()?;
        crate::objectives::encode_workspace_identity(
            &self.canonical,
            &self.canonical,
            Some((metadata.dev(), metadata.ino())),
            &self.topology,
        )
        .map_err(io::Error::other)
    }

    pub(super) async fn changed(&self) {
        loop {
            if self.validate().is_err() {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    }

    pub(super) fn command(
        &self,
        executable: &Path,
        cwd: &Path,
        environment: &BTreeMap<String, String>,
        writable: bool,
    ) -> io::Result<Command> {
        self.validate()
            .map_err(|_| io::Error::other(WorkspaceChanged))?;
        fs::metadata(executable)?;
        let root_fd = self.directory.try_clone()?;
        let mut inherited = vec![root_fd];
        let mut command = Command::new("/usr/bin/bwrap");
        command.args([
            "--unshare-user",
            "--unshare-pid",
            "--unshare-ipc",
            "--unshare-uts",
            "--die-with-parent",
            "--disable-userns",
            "--cap-drop",
            "ALL",
            "--ro-bind",
            "/",
            "/",
            "--proc",
            "/proc",
            "--dev",
            "/dev",
        ]);
        command.args(["--tmpfs", "/var/tmp", "--setenv", "TMPDIR", "/var/tmp"]);
        command
            .arg(if writable {
                "--bind-fd"
            } else {
                "--ro-bind-fd"
            })
            .arg(inherited[0].as_raw_fd().to_string())
            .arg(&self.canonical);
        // Only an explicitly configured worker profile is writable. Never grant
        // the operator's entire HOME as a runtime-state exception.
        if let Some(home) = environment.get("HERMES_HOME") {
            let home = Path::new(home).canonicalize()?;
            if home.starts_with(&self.canonical) || self.canonical.starts_with(&home) {
                return Err(io::Error::other(
                    "worker profile overlaps the project workspace",
                ));
            }
            let profile = File::open(&home)?;
            if !profile.metadata()?.is_dir() {
                return Err(io::Error::other("worker profile is not a directory"));
            }
            command
                .arg("--bind-fd")
                .arg(profile.as_raw_fd().to_string())
                .arg(home);
            inherited.push(profile);
        }
        command.arg("--chdir").arg(cwd).arg("--").arg(executable);
        // SAFETY: only raw close_range and async-signal-safe fcntl calls run after fork. The captured
        // files keep the descriptors alive until spawn; bwrap consumes bind FDs.
        unsafe {
            command.pre_exec(move || {
                // Preserve Rust's spawn-error pipe until exec while preventing
                // arbitrary inherited host directory/file capabilities leaking.
                // Fail closed on kernels without close_range(CLOEXEC).
                if libc::syscall(libc::SYS_close_range, 3u32, u32::MAX, 4u32) == -1 {
                    return Err(io::Error::last_os_error());
                }
                for file in &inherited {
                    let fd = file.as_raw_fd();
                    if libc::fcntl(fd, libc::F_SETFD, 0) == -1 {
                        return Err(io::Error::last_os_error());
                    }
                }
                Ok(())
            });
        }
        Ok(command)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::process::Stdio;

    #[test]
    #[ignore = "requires unshare user/mount namespaces and mount"]
    fn mount_change_after_command_construction_cannot_redirect_writes() {
        use std::process::Command as SyncCommand;
        const TEST: &str = "adapters::hermes::workspace::tests::mount_change_after_command_construction_cannot_redirect_writes";
        if std::env::var_os("ARDA_PREEXEC_MOUNT_CHILD").is_none() {
            let status = SyncCommand::new("unshare")
                .args([
                    "--user",
                    "--map-root-user",
                    "--mount",
                    "--propagation",
                    "private",
                ])
                .arg(std::env::current_exe().unwrap())
                .args(["--exact", TEST, "--ignored", "--nocapture"])
                .env("ARDA_PREEXEC_MOUNT_CHILD", "1")
                .status()
                .unwrap();
            assert!(status.success(), "pre-exec mount race regression failed");
            return;
        }
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let nested = root.join("nested");
        let outside = temp.path().join("outside");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(&outside).unwrap();
        let pinned = PinnedWorkspace::open(&root).unwrap();
        let mut command = pinned
            .command(Path::new("/bin/sh"), &root, &BTreeMap::new(), true)
            .unwrap();
        command.args(["-c", "printf escaped > nested/escape"]);
        // Deterministically replace a descendant after the last parent-side
        // validation, but before bubblewrap consumes its bind source.
        assert!(SyncCommand::new("mount")
            .arg("--bind")
            .arg(&outside)
            .arg(&nested)
            .status()
            .unwrap()
            .success());
        struct Unmount(PathBuf);
        impl Drop for Unmount {
            fn drop(&mut self) {
                let status = SyncCommand::new("umount").arg(&self.0).status();
                if !status.is_ok_and(|status| status.success()) {
                    eprintln!("failed to unmount pre-exec fixture");
                }
            }
        }
        let guard = Unmount(nested);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let output = runtime.block_on(async { command.output().await }).unwrap();
        let escaped = outside.join("escape").exists();
        drop(guard);
        assert!(
            !escaped,
            "provider wrote into a mount introduced after validation: {output:?}"
        );
    }

    #[test]
    fn admission_identity_binds_captured_topology_and_rejects_legacy_tuple() {
        let temp = tempfile::tempdir().unwrap();
        let mut pinned = PinnedWorkspace::open(temp.path()).unwrap();
        let original = pinned.admission_identity().unwrap();
        let metadata = pinned.directory.metadata().unwrap();
        let legacy = serde_json::to_string(&(
            &pinned.canonical,
            &pinned.canonical,
            Some((metadata.dev(), metadata.ino())),
        ))
        .unwrap();
        assert_ne!(original, legacy);
        // A different captured baseline must not be replaced with current mounts
        // when the execution-side identity is compared to durable admission.
        pinned.topology.push(b'\n');
        assert_ne!(original, pinned.admission_identity().unwrap());
        assert!(pinned.validate().is_err());
    }

    #[tokio::test]
    async fn unrelated_inheritable_directory_fd_cannot_escape_read_only_mounts() {
        use std::os::fd::FromRawFd;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let outside = File::open(temp.path()).unwrap();
        // F_DUPFD deliberately creates a non-CLOEXEC host directory capability.
        let raw = unsafe { libc::fcntl(outside.as_raw_fd(), libc::F_DUPFD, 200) };
        assert!(raw >= 200);
        let inherited = unsafe { File::from_raw_fd(raw) };
        let script = "import os,sys; fd=os.open('escape',os.O_WRONLY|os.O_CREAT,0o600,dir_fd=int(sys.argv[1])); os.close(fd)";
        let control = Command::new("/usr/bin/python3")
            .args(["-c", script, &raw.to_string()])
            .output()
            .await
            .unwrap();
        assert!(control.status.success());
        assert!(temp.path().join("escape").exists());
        fs::remove_file(temp.path().join("escape")).unwrap();
        let pinned = PinnedWorkspace::open(&root).unwrap();
        let output = pinned
            .command(Path::new("/usr/bin/python3"), &root, &BTreeMap::new(), true)
            .unwrap()
            .args(["-c", script, &raw.to_string()])
            .output()
            .await
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Bad file descriptor"),
            "{:?}",
            output
        );
        assert!(!temp.path().join("escape").exists());
        drop(inherited);
    }

    #[tokio::test]
    async fn changed_workspace_stops_and_joins_running_provider() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        let template = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../config/adapters/hermes-workbench.toml");
        let config = temp.path().join("adapter.toml");
        fs::write(
            &config,
            fs::read_to_string(template)
                .unwrap()
                .replace("executable = \"hermes\"", "executable = \"/bin/sh\""),
        )
        .unwrap();
        let adapter = super::super::HermesAdapter::load(
            config,
            &root,
            &root,
            &BTreeMap::from([("PATH".into(), "/usr/bin:/bin".into())]),
        )
        .unwrap();
        let saved = adapter
            .workspace
            .as_ref()
            .unwrap()
            .admission_identity()
            .unwrap();
        adapter.require_admission_workspace(&saved).unwrap();
        let mut command = adapter
            .workspace
            .as_ref()
            .unwrap()
            .command(Path::new("/bin/sh"), &root, &BTreeMap::new(), true)
            .unwrap();
        command
            .args([
                "-c",
                "while :; do printf tick >> progress; sleep 0.01; done",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        super::super::configure_process_group(&mut command);
        let cancellation = crate::adapters::AdapterCancellation::new();
        let execution = adapter.run_contained(
            command,
            std::time::Duration::from_secs(5),
            &cancellation,
            500,
            1024,
        );
        let moved = temp.path().join("moved");
        let change = async {
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                while !root.join("progress").exists() {
                    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                }
            })
            .await
            .unwrap();
            fs::rename(&root, &moved).unwrap();
            fs::create_dir(&root).unwrap();
        };
        let (result, ()) = tokio::join!(execution, change);
        assert!(
            matches!(
                result,
                Err(super::super::HermesAdapterError::WorkspaceChanged)
            ),
            "{:?}",
            result.as_ref().err()
        );
        let stopped = fs::read(moved.join("progress")).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(stopped, fs::read(moved.join("progress")).unwrap());
        assert!(!root.join("progress").exists());
        // The same classification must hold between chat and export, before
        // another subprocess is spawned (not only inside the polling branch).
        assert!(matches!(
            adapter.contained_command(),
            Err(super::super::HermesAdapterError::WorkspaceChanged)
        ));
        let replacement = PinnedWorkspace::open(&root).unwrap();
        assert_ne!(saved, replacement.admission_identity().unwrap());
    }

    #[tokio::test]
    async fn pinned_running_worker_cannot_write_retargeted_directory() {
        let temp = tempfile::tempdir().unwrap();
        let original = temp.path().join("original");
        let replacement = temp.path().join("replacement");
        let alias = temp.path().join("alias");
        fs::create_dir(&original).unwrap();
        fs::create_dir(&replacement).unwrap();
        symlink(&original, &alias).unwrap();
        let pinned = PinnedWorkspace::open(&alias).unwrap();
        let mut command = pinned
            .command(Path::new("/bin/sh"), &original, &BTreeMap::new(), true)
            .unwrap();
        command.arg("-c").arg("printf ready; read gate; printf original > ./allowed; if printf wrong > \"$1/forbidden\"; then exit 99; fi")
            .arg("fixture").arg(&alias)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child = command.spawn().unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut ready = [0; 5];
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            child.stdout.as_mut().unwrap().read_exact(&mut ready),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(&ready, b"ready");
        fs::remove_file(&alias).unwrap();
        symlink(&replacement, &alias).unwrap();
        assert!(pinned.validate().is_err());
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"continue\n")
            .await
            .unwrap();
        let output =
            tokio::time::timeout(std::time::Duration::from_secs(5), child.wait_with_output())
                .await
                .unwrap()
                .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(fs::read(original.join("allowed")).unwrap(), b"original");
        assert!(!replacement.join("forbidden").exists());
        assert!(pinned
            .command(Path::new("/bin/true"), &original, &BTreeMap::new(), true)
            .is_err());
    }
}
