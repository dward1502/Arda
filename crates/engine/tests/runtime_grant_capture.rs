#![cfg(target_os = "linux")]
#[path = "../src/bin/snapshot_admission/mod.rs"]
#[allow(dead_code)] // Only runtime grant admission is exercised here.
mod admission;
use admission::grant::GrantAdmission;
use arda_engine::objectives::runtime_policy::{GrantAccess, GrantKind, GrantRole, RuntimeGrant};
use std::{fs, path::Path, process::Command};
fn grant(source: &Path, kind: GrantKind, access: GrantAccess) -> RuntimeGrant {
    RuntimeGrant {
        id: "test".into(),
        source: source.into(),
        destination: "/captured".into(),
        kind,
        access,
        role: GrantRole::Runtime,
    }
}
#[test]
fn rejects_wrong_types_and_symlinks_without_opening_devices() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("file");
    fs::write(&file, "test").unwrap();
    let link = temp.path().join("link");
    std::os::unix::fs::symlink(&file, &link).unwrap();
    let parent = temp.path().join("alias");
    std::os::unix::fs::symlink(temp.path(), &parent).unwrap();
    for (path, kind) in [
        (temp.path().to_path_buf(), GrantKind::File),
        (file, GrantKind::Directory),
        (link, GrantKind::File),
        (parent.join("file"), GrantKind::File),
        ("/dev/null".into(), GrantKind::File),
    ] {
        assert!(GrantAdmission::before_clone(&grant(&path, kind, GrantAccess::ReadOnly)).is_err());
    }
}
#[test]
#[ignore = "requires qualified user/mount namespaces"]
fn retained_file_and_recursive_readonly_directory() {
    if std::env::var_os("ARDA_GRANT_CAPTURE_CHILD").is_none() {
        assert!(Command::new("unshare")
            .args(["-Urnm", "--propagation", "private"])
            .arg(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "retained_file_and_recursive_readonly_directory",
                "--ignored",
                "--nocapture"
            ])
            .env("ARDA_GRANT_CAPTURE_CHILD", "1")
            .status()
            .unwrap()
            .success());
        return;
    }
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("file");
    fs::write(&source, "original").unwrap();
    let tree = temp.path().join("tree");
    fs::create_dir(&tree).unwrap();
    fs::create_dir(tree.join("nested")).unwrap();
    let nested = temp.path().join("nested-source");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("sentinel"), "nested").unwrap();
    assert!(Command::new("mount")
        .arg("--bind")
        .arg(&nested)
        .arg(tree.join("nested"))
        .status()
        .unwrap()
        .success());
    let file_grant = grant(&source, GrantKind::File, GrantAccess::ReadOnly);
    let tree_grant = grant(&tree, GrantKind::Directory, GrantAccess::ReadOnly);
    let file_admission = GrantAdmission::before_clone(&file_grant).unwrap();
    let tree_admission = GrantAdmission::before_clone(&tree_grant).unwrap();
    file_admission.after_clone().unwrap();
    tree_admission.after_clone().unwrap();
    let file_slot = temp.path().join("file-slot");
    let tree_slot = temp.path().join("tree-slot");
    let file_capture = file_admission.capture(&file_grant, &file_slot).unwrap();
    let tree_capture = tree_admission.capture(&tree_grant, &tree_slot).unwrap();
    fs::rename(&source, temp.path().join("original-file")).unwrap();
    fs::write(&source, "replacement").unwrap();
    assert_eq!(fs::read_to_string(&file_slot).unwrap(), "original");
    assert!(fs::write(&file_slot, "bad").is_err());
    assert!(fs::write(tree_slot.join("nested/sentinel"), "bad").is_err());
    assert!(fs::write(tree_slot.join("new"), "bad").is_err());
    fs::write(nested.join("host-writable"), "allowed host change").unwrap();
    assert!(file_admission
        .exposed_devices()
        .unwrap()
        .contains(&file_capture.identity.device));
    assert!(tree_admission
        .exposed_devices()
        .unwrap()
        .contains(&tree_capture.identity.device));
    drop((file_capture, tree_capture));
    for slot in [&tree_slot, &file_slot, &tree.join("nested")] {
        assert!(Command::new("umount")
            .arg("-l")
            .arg(slot)
            .status()
            .unwrap()
            .success());
    }
}
