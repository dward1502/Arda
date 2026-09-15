use super::*;
use std::process::Command as SyncCommand;

#[test]
#[ignore = "requires qualified unprivileged user/mount namespaces"]
fn physical_profile_aliases_cannot_make_readonly_workspace_writable() {
    const CHILD: &str = "ARDA_PHYSICAL_ALIAS_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = SyncCommand::new("unshare")
            .args(["--user", "--map-root-user", "--mount", "--propagation", "private"])
            .arg(std::env::current_exe().unwrap())
            .args(["--exact", "adapters::hermes::workspace::physical_alias_tests::physical_profile_aliases_cannot_make_readonly_workspace_writable", "--ignored", "--nocapture"])
            .env(CHILD, "1").output().unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    for source_kind in ["root", "child", "ancestor", "crossed-descendant"] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let nested = root.join("nested");
        let profile = temp.path().join("profile");
        fs::create_dir_all(&nested).unwrap();
        fs::create_dir(&profile).unwrap();
        let (source, target) = match source_kind {
            "root" => (root.as_path(), profile.as_path()),
            "child" => (nested.as_path(), profile.as_path()),
            "ancestor" => (temp.path(), profile.as_path()),
            _ => (profile.as_path(), nested.as_path()),
        };
        assert!(SyncCommand::new("mount")
            .arg("--bind")
            .arg(source)
            .arg(target)
            .status()
            .unwrap()
            .success());
        let pinned = PinnedWorkspace::open(&root).unwrap();
        let environment = BTreeMap::from([("HERMES_HOME".into(), profile.display().to_string())]);
        assert!(
            pinned
                .command(Path::new("/bin/true"), &root, &environment, false)
                .is_err(),
            "accepted {source_kind}"
        );
        assert!(SyncCommand::new("umount")
            .arg(target)
            .status()
            .unwrap()
            .success());
    }
}
