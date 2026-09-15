use super::*;
use std::process::Command as SyncCommand;

#[test]
#[ignore = "requires qualified unprivileged user/mount namespaces"]
fn captured_workspace_and_profile_keep_original_writes() {
    const TEST: &str = "adapters::hermes::workspace::capture_tests::captured_workspace_and_profile_keep_original_writes";
    if std::env::var_os("ARDA_CAPTURE_POSITIVE_CHILD").is_none() {
        assert!(SyncCommand::new("unshare")
            .args([
                "--user",
                "--map-root-user",
                "--mount",
                "--propagation",
                "shared"
            ])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env("ARDA_CAPTURE_POSITIVE_CHILD", "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    for (profile, authorized, writable) in [
        (false, false, true),
        (false, true, true),
        (true, false, true),
        (false, false, false),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let home = temp.path().join("profile");
        let outside = temp.path().join("outside");
        let original = temp.path().join("original");
        let grant = if profile { &home } else { &root };
        let nested = grant.join("nested");
        for path in [&root, &home, &outside, &original, &nested] {
            fs::create_dir_all(path).unwrap();
        }
        struct Mounts(Vec<PathBuf>);
        impl Drop for Mounts {
            fn drop(&mut self) {
                for path in self.0.iter().rev() {
                    assert!(SyncCommand::new("umount")
                        .arg(path)
                        .status()
                        .unwrap()
                        .success());
                }
            }
        }
        let mut mounts = Mounts(vec![]);
        if authorized {
            assert!(SyncCommand::new("mount")
                .arg("--bind")
                .arg(&original)
                .arg(&nested)
                .status()
                .unwrap()
                .success());
            mounts.0.push(nested.clone());
        }
        let pinned = PinnedWorkspace::open(&root).unwrap();
        let environment = if profile {
            BTreeMap::from([("HERMES_HOME".into(), home.to_string_lossy().into_owned())])
        } else {
            BTreeMap::new()
        };
        let mut command = pinned
            .command(Path::new("/bin/sh"), &root, &environment, writable)
            .unwrap();
        command.args([
            "-c",
            if profile {
                "printf original > \"$HERMES_HOME/nested/escape\""
            } else {
                "printf original > nested/escape"
            },
        ]);
        drop(pinned);
        assert!(SyncCommand::new("mount")
            .arg("--bind")
            .arg(&outside)
            .arg(&nested)
            .status()
            .unwrap()
            .success());
        mounts.0.push(nested.clone());
        let runtime = tokio::runtime::Runtime::new().unwrap();
        // A Command can be spawned repeatedly; each child gets its own FD copies.
        for _ in 0..2 {
            let output = runtime.block_on(async { command.output().await }).unwrap();
            assert_eq!(output.status.success(), writable, "{output:?}");
            assert!(!outside.join("escape").exists());
        }
        drop(command);
        drop(mounts);
        let target = if authorized {
            original.join("escape")
        } else {
            nested.join("escape")
        };
        if writable {
            assert_eq!(fs::read_to_string(target).unwrap(), "original");
        } else {
            assert!(!target.exists());
        }
    }
}
