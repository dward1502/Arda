#![cfg(target_os = "linux")]
#[path = "../src/bin/snapshot_admission/mod.rs"]
#[allow(dead_code)] // Only physical tree witness code is exercised here.
mod admission;
use admission::physical::Physical;
use std::{fs, path::Path, process::Command};
fn pin(path: &Path) -> Physical {
    Physical::pin(path, &fs::read("/proc/thread-self/mountinfo").unwrap()).unwrap()
}
#[test]
#[ignore = "requires qualified user/mount namespaces"]
fn pinned_backing_comparison_catches_aliases_without_reopening_paths() {
    const CHILD: &str = "ARDA_TREE_WITNESS_CHILD";
    if std::env::var_os(CHILD).is_none() {
        assert!(Command::new("unshare")
            .args([
                "--user",
                "--map-root-user",
                "--mount",
                "--propagation",
                "private"
            ])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "pinned_backing_comparison_catches_aliases_without_reopening_paths",
                "--ignored",
                "--nocapture"
            ])
            .env(CHILD, "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let durable = temp.path().join("durable");
    let source = temp.path().join("source");
    let sibling = temp.path().join("durable-sibling");
    for path in [&durable, &source, &sibling] {
        fs::create_dir(path).unwrap();
    }
    fs::create_dir(durable.join("child")).unwrap();
    fs::create_dir(source.join("nested")).unwrap();
    fs::write(durable.join("secret-file"), "synthetic").unwrap();
    let protected = pin(&durable);
    let clean = pin(&source);
    let separate = pin(&sibling);
    assert!(!protected
        .witness()
        .unwrap()
        .overlaps(&separate.witness().unwrap())
        .unwrap());
    assert!(!protected
        .witness()
        .unwrap()
        .overlaps(&clean.witness().unwrap())
        .unwrap());
    assert!(protected
        .witness()
        .unwrap()
        .overlaps(&pin(&durable.join("secret-file")).witness().unwrap())
        .unwrap());
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(durable.join("child"))
        .arg(source.join("nested"))
        .status()
        .unwrap()
        .success());
    let imported = pin(&source);
    assert!(imported
        .witness()
        .unwrap()
        .overlaps(&protected.witness().unwrap())
        .unwrap());
    assert!(Command::new("umount")
        .arg("--lazy")
        .arg(source.join("nested"))
        .status()
        .unwrap()
        .success());
    fs::rename(&source, temp.path().join("renamed-source")).unwrap();
    fs::rename(&durable, temp.path().join("renamed-durable")).unwrap();
    let late_child = pin(&temp.path().join("renamed-durable/child"));
    // Cached backing coordinates span different eras here. They are not an
    // admission decision: the live-pin comparator must refuse the stale root.
    assert!(protected.overlaps_current(&late_child).is_err());
    let refreshed = pin(&temp.path().join("renamed-durable"));
    assert!(refreshed.overlaps_current(&late_child).unwrap());
    // No source pathname exists now. The comparator uses only pinned witnesses.
    assert!(imported
        .witness()
        .unwrap()
        .overlaps(&protected.witness().unwrap())
        .unwrap());
    assert!(!clean
        .witness()
        .unwrap()
        .overlaps(&protected.witness().unwrap())
        .unwrap());
    let original = imported.witness().unwrap();
    let ro = original.readonly().unwrap();
    assert!(ro
        .mounts
        .iter()
        .all(|m| m.options.contains(&b"ro".to_vec()) && !m.options.contains(&b"rw".to_vec())));
    assert!(original.digest().unwrap() != ro.digest().unwrap());
}
