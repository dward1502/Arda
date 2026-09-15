use arda_engine::objectives::ObjectiveStore;
use std::fs;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[test]
fn daemon_restart_refuses_lost_or_replaced_authority_before_starting_children() {
    for damage in ["directory", "database", "replacement", "marker"] {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("data/arda");
        let database = directory.join("objectives.sqlite3");
        drop(ObjectiveStore::initialize(&database).unwrap());
        fs::write(root.path().join("services.toml"),
            "[[service]]\nname='must-not-start'\nrequired=true\nstart.command='/usr/bin/touch'\nstart.args=['child-started']\nstart.cwd='.'\n").unwrap();
        match damage {
            "directory" => fs::rename(&directory, root.path().join("saved")).unwrap(),
            "marker" => fs::rename(
                directory.join("objectives.sqlite3.authority.json"),
                root.path().join("saved-marker"),
            )
            .unwrap(),
            _ => {
                fs::rename(&database, root.path().join("saved.sqlite3")).unwrap();
                if damage == "replacement" {
                    fs::copy(root.path().join("saved.sqlite3"), &database).unwrap();
                }
            }
        }
        let database_before = fs::read(&database).ok();
        let mut daemon = Command::new(env!("CARGO_BIN_EXE_arda"))
            .current_dir(root.path())
            .env("ARDA_REPO_ROOT", root.path())
            .env_remove("ARDA_SNAPSHOT_KEEPER_SOCKET")
            .args([
                "--operator-id",
                "fixture-owner",
                "--harness-addr",
                "127.0.0.1:0",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = daemon.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                daemon.kill().unwrap();
                daemon.wait().unwrap();
                panic!("daemon accepted {damage} authority");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert!(!status.success(), "{damage}");
        let mut diagnostic = String::new();
        std::io::Read::read_to_string(&mut daemon.stderr.take().unwrap(), &mut diagnostic).unwrap();
        assert!(
            diagnostic.contains("ObjectiveStore"),
            "{damage}: {diagnostic}"
        );
        assert!(!root.path().join("child-started").exists());
        assert_eq!(database_before, fs::read(&database).ok());
        if damage == "directory" {
            assert!(!directory.exists());
        }
    }
}
