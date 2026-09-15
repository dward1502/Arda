#![cfg(target_os = "linux")]
#[path = "../src/bin/snapshot_admission/mod.rs"]
#[allow(dead_code)] // Shared production module has additional binary callers.
mod admission;
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::MetadataExt, path::Path, process::Command};
fn mount(source: &Path, target: &Path) {
    assert!(Command::new("mount")
        .arg("--rbind")
        .arg(source)
        .arg(target)
        .status()
        .unwrap()
        .success());
}
#[test]
#[ignore = "requires real user/mount namespaces"]
fn capture_physical_races() {
    let Ok(case) = std::env::var("ARDA_CAPTURE_CASE") else {
        for case in ["replace-source", "import-outside", "unchanged"] {
            for alias in ["0", "1"] {
                assert!(
                    Command::new("unshare")
                        .args(["-Urnm", "--propagation", "private"])
                        .arg(std::env::current_exe().unwrap())
                        .args([
                            "--exact",
                            "capture_physical_races",
                            "--ignored",
                            "--nocapture"
                        ])
                        .env("ARDA_CAPTURE_CASE", case)
                        .env("ARDA_CAPTURE_ALIAS", alias)
                        .status()
                        .unwrap()
                        .success(),
                    "case {case}"
                );
            }
        }
        return;
    };
    let temp = tempfile::tempdir_in("/var/tmp").unwrap();
    let root = temp.path().join("root");
    let source = temp.path().join("source");
    let outside = temp.path().join("outside");
    let stage = temp.path().join("staging/root");
    fs::create_dir(temp.path().join("staging")).unwrap();

    for path in [&root, &source, &outside, &stage] {
        fs::create_dir(path).unwrap();
    }
    fs::create_dir(root.join("inner")).unwrap();
    fs::create_dir(outside.join("nested")).unwrap();
    fs::write(source.join("marker"), "ORIGINAL").unwrap();
    if case == "import-outside" {
        mount(&source, &outside.join("nested"));
    } else {
        mount(&source, &root.join("inner"));
    }
    let meta = fs::metadata(&root).unwrap();
    let topology = fs::read("/proc/self/mountinfo").unwrap();
    let identity = serde_json::to_string(&(
        2u32,
        &root,
        &root,
        Some((meta.dev(), meta.ino())),
        format!("{:x}", Sha256::digest(&topology)),
    ))
    .unwrap();
    let bound = admission::Admission::before_clone(&root, &identity).unwrap();
    if case == "replace-source" {
        assert!(Command::new("umount")
            .arg("-l")
            .arg(root.join("inner"))
            .status()
            .unwrap()
            .success());
        fs::rename(&source, temp.path().join("old-source")).unwrap();
        fs::create_dir(&source).unwrap();
        fs::write(source.join("marker"), "REPLACEMENT").unwrap();
        mount(&source, &root.join("inner"));
    }
    assert_eq!(unsafe { libc::unshare(libc::CLONE_NEWNS) }, 0);
    assert!(Command::new("mount")
        .args(["--make-rprivate", "/"])
        .status()
        .unwrap()
        .success());
    // The old topology/root-only guard permits both attacks.
    bound.after_clone(&root).unwrap();
    // Production opens its state descriptor inside the cloned namespace.
    let stage_owner = fs::File::open(temp.path()).unwrap();
    if case == "import-outside" {
        fs::rename(root.join("inner"), temp.path().join("old-inner")).unwrap();
        fs::rename(&outside, root.join("inner")).unwrap();
    }
    mount(&root, &stage);
    bound.check_root(&fs::metadata(&stage).unwrap()).unwrap();
    use std::os::fd::AsRawFd;
    let verify_path = if std::env::var("ARDA_CAPTURE_ALIAS").unwrap() == "1" {
        std::path::PathBuf::from(format!(
            "/proc/self/fd/{}/staging/root",
            stage_owner.as_raw_fd()
        ))
    } else {
        stage.clone()
    };
    if case == "unchanged" {
        bound.check_capture(&verify_path).unwrap();
        assert_eq!(
            fs::read_to_string(stage.join("inner/marker")).unwrap(),
            "ORIGINAL"
        );
    } else {
        assert!(
            bound.check_capture(&verify_path).is_err(),
            "unauthorized capture accepted"
        );
        let marker = if case == "import-outside" {
            "inner/nested/marker"
        } else {
            "inner/marker"
        };
        assert!(stage.join(marker).exists(), "attack was not reproduced");
    }
    // The process owns its mount namespace; don't recursively remove through
    // live bind mounts during TempDir cleanup. Unmount before removing fixtures.
    assert!(Command::new("umount")
        .args(["-R"])
        .arg(&stage)
        .status()
        .unwrap()
        .success());
    assert!(Command::new("umount")
        .arg(if case == "import-outside" {
            root.join("inner/nested")
        } else {
            root.join("inner")
        })
        .status()
        .unwrap()
        .success());
}
