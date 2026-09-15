#![cfg(target_os = "linux")]
#[path = "../src/bin/snapshot_admission/mod.rs"]
#[allow(dead_code)] // Only physical identity admission is exercised here.
mod admission;
use arda_engine::objectives::physical_tree::Physical;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};
fn identity(root: &Path, version: u32) -> String {
    let topology = fs::read("/proc/thread-self/mountinfo").unwrap();
    let tree = Physical::pin(root, &topology).unwrap().witness().unwrap();
    let object = &tree.mounts[0].object;
    let digest = if version == 2 {
        format!("{:x}", Sha256::digest(&topology))
    } else {
        tree.digest().unwrap()
    };
    serde_json::to_string(&(
        version,
        root,
        root,
        Some((object.device, object.inode)),
        digest,
    ))
    .unwrap()
}
fn mount(args: &[&str]) {
    assert!(Command::new("mount").args(args).status().unwrap().success());
}
fn unmount(path: &Path) {
    assert!(Command::new("umount").arg(path).status().unwrap().success());
}
#[test]
fn versions_and_replaced_root_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    let v2 = identity(&root, 2);
    let v3 = identity(&root, 3);
    admission::Admission::before_clone(&root, &v2).unwrap();
    admission::Admission::before_clone(&root, &v3).unwrap();
    assert!(admission::Admission::before_clone(&root, &identity(&root, 4)).is_err());
    fs::rename(&root, temp.path().join("original")).unwrap();
    fs::create_dir(&root).unwrap();
    assert!(admission::Admission::before_clone(&root, &v2).is_err());
    assert!(admission::Admission::before_clone(&root, &v3).is_err());
}
#[test]
#[ignore = "requires real user/mount namespaces"]
fn retained_tree_rejects_descendant_and_option_changes_but_accepts_unrelated_mounts() {
    if std::env::var_os("ARDA_TREE_IDENTITY_CHILD").is_none() {
        assert!(Command::new("unshare")
            .args(["-Urnm", "--propagation", "private"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "retained_tree_rejects_descendant_and_option_changes_but_accepts_unrelated_mounts",
                "--ignored",
                "--nocapture"
            ])
            .env("ARDA_TREE_IDENTITY_CHILD", "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("root");
    let nested = root.join("nested");
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    let unrelated = temp.path().join("unrelated");
    for p in [&root, &nested, &a, &b, &unrelated] {
        fs::create_dir(p).unwrap();
    }
    let v2 = identity(&root, 2);
    let v3 = identity(&root, 3);
    mount(&["--bind", a.to_str().unwrap(), unrelated.to_str().unwrap()]);
    assert!(admission::Admission::before_clone(&root, &v2).is_err());
    admission::Admission::before_clone(&root, &v3).unwrap();
    mount(&["--bind", a.to_str().unwrap(), nested.to_str().unwrap()]);
    assert!(admission::Admission::before_clone(&root, &v3).is_err());
    let admitted_nested = identity(&root, 3);
    let pinned = admission::Admission::before_clone(&root, &admitted_nested).unwrap();
    mount(&["-o", "remount,bind,ro", nested.to_str().unwrap()]);
    assert!(admission::Admission::before_clone(&root, &admitted_nested).is_err());
    assert!(pinned.after_clone(&root).is_err());
    drop(pinned);
    unmount(&nested);
    assert!(admission::Admission::before_clone(&root, &admitted_nested).is_err());
    mount(&["--bind", b.to_str().unwrap(), nested.to_str().unwrap()]);
    assert!(admission::Admission::before_clone(&root, &admitted_nested).is_err());
    mount(&["--bind", a.to_str().unwrap(), nested.to_str().unwrap()]);
    assert!(admission::Admission::before_clone(&root, &admitted_nested).is_err());
    unmount(&nested);
    unmount(&nested);
    unmount(&unrelated);
}
