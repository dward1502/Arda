use super::*;
use rusqlite::Connection;
use std::os::unix::fs::PermissionsExt;
use uuid::Uuid;
#[path = "../../../tests/fixtures/installed_policy.rs"]
#[allow(dead_code)]
mod installed_policy;

pub(super) fn crash(point: &str) {
    if std::env::var("ARDA_TEST_PREPARE_CRASH").as_deref() == Ok(point) {
        std::process::exit(86);
    }
}
fn binary(name: &str) -> PathBuf {
    std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(name)
}
fn open(root: &Path) -> (Owner, fs::File, fs::File) {
    let durable = root.join("durable");
    let runtime = PathBuf::from(fs::read_to_string(root.join("runtime-path")).unwrap());
    let policy = crate::keeper_config::load(&root.join("policy/runtime-policy.json")).unwrap();
    let reservations =
        pending::Reservations::pin(&durable, &runtime, &root.join("policy/sessions")).unwrap();
    let (d, r) = reservations.storage_pins().unwrap();
    let (db, dl, rl) = crate::keeper_storage::open_pinned(d, r).unwrap();
    reservations.bind_storage(&dl, &rl).unwrap();
    let managed = crate::keeper_managed::capture(&db, &runtime).unwrap();
    (
        Owner {
            managed,
            reservations: Some(reservations),
            runtime_policy: Some(policy),
            db,
            durable,
            runtime,
            worker: binary("arda-snapshot-worker"),
            children: BTreeMap::new(),
            failed_qualifications: Default::default(),
        },
        dl,
        rl,
    )
}
fn request(root: &Path) -> KeeperRequest {
    use sha2::{Digest, Sha256};
    let workspace = root.join("workspace");
    let metadata = workspace.metadata().unwrap();
    let identity = serde_json::to_string(&(
        2,
        &workspace,
        &workspace,
        Some((metadata.dev(), metadata.ino())),
        format!(
            "{:x}",
            Sha256::digest(fs::read("/proc/thread-self/mountinfo").unwrap())
        ),
    ))
    .unwrap();
    KeeperRequest::Prepare {
        run: "crash-run".into(),
        workspace,
        identity,
    }
}
struct Managed(String);
impl Drop for Managed {
    fn drop(&mut self) {
        for action in ["stop", "unmask", "reset-failed"] {
            let _ = Command::new("systemctl")
                .args(["--user", action, "--runtime", &self.0])
                .output();
        }
    }
}
fn systemctl(action: &str, unit: &str) {
    let out = Command::new("systemctl")
        .args(["--user", action, "--runtime", unit])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
fn cli(
    action: &str,
    root: &Path,
    runtime: &Path,
    owner: &str,
    extra: &[&str],
) -> serde_json::Value {
    let out = Command::new(binary("arda-snapshot-keeper"))
        .args(["reconcile", action])
        .arg("--durable")
        .arg(root.join("durable"))
        .arg("--runtime")
        .arg(runtime)
        .args(["--owner", owner, "--run", "crash-run", "--json"])
        .args(extra)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}: {}",
        action,
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn row(db: &Connection) -> (String, Option<String>) {
    db.query_row(
        "SELECT state,authority FROM snapshots WHERE run='crash-run'",
        [],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}

#[test]
#[ignore = "configured runtime grants, user systemd and user/mount namespaces; no provider calls"]
fn configured_preparation_crash_matrix() {
    if let Some(root) = std::env::var_os("ARDA_TEST_PREPARE_ROOT") {
        unsafe {
            libc::umask(0o077);
        }
        let root = PathBuf::from(root);
        let (mut owner, _d, _r) = open(&root);
        owner.handle(request(&root)).unwrap();
        panic!("preparation checkpoint was not reached");
    }
    for point in [
        "preparing",
        "allocation_intent",
        "allocation_directory",
        "allocation_saved",
        "worker_spawned",
        "qualified",
        "prepared",
    ] {
        let root = tempfile::tempdir_in("/var/tmp").unwrap();
        for name in ["durable", "workspace", "policy"] {
            let path = root.path().join(name);
            fs::create_dir_all(&path).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
        }
        let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
        fs::set_permissions(runtime.path(), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(
            root.path().join("runtime-path"),
            runtime.path().as_os_str().as_encoded_bytes(),
        )
        .unwrap();
        installed_policy::write(&root.path().join("policy"));
        crate::keeper_storage::initialize(&root.path().join("durable")).unwrap();
        let unit = Managed(format!(
            "arda-prepare-test-{}.service",
            Uuid::new_v4().simple()
        ));
        let result = Command::new("systemd-run")
            .args([
                "--user",
                "--wait",
                "--pipe",
                "--service-type=exec",
                "--unit",
                &unit.0,
                "-p",
                "ProtectControlGroups=yes",
                "-p",
                "Delegate=no",
                "-p",
                "KillMode=control-group",
                "-p",
                "RuntimeDirectoryPreserve=yes",
                "-p",
                "TimeoutStopSec=2",
                "-p",
                "RuntimeMaxSec=30",
            ])
            .arg(format!("--setenv=ARDA_KEEPER_SYSTEMD_UNIT={}", unit.0))
            .arg(format!(
                "--setenv=ARDA_TEST_PREPARE_ROOT={}",
                root.path().display()
            ))
            .arg(format!("--setenv=ARDA_TEST_PREPARE_CRASH={point}"))
            .arg(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "keeper_owner::preparation_tests::configured_preparation_crash_matrix",
                "--nocapture",
            ])
            .output()
            .unwrap();
        assert_eq!(
            result.status.code(),
            Some(86),
            "{point}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let db = Connection::open(root.path().join("durable/owner.sqlite3")).unwrap();
        let before = row(&db);
        assert_eq!(
            before.0,
            if point == "prepared" {
                "prepared"
            } else {
                "preparing"
            }
        );
        assert_eq!(before.1.is_some(), point == "prepared");
        let owner_id: String = db
            .query_row("SELECT id FROM owner_identity", [], |r| r.get(0))
            .unwrap();
        let binding = crate::keeper_managed::load(&db, "crash-run").unwrap();
        assert_eq!(binding.owner, owner_id);
        assert_eq!(binding.unit, unit.0);
        drop(db);
        let (mut owner, d, r) = open(root.path());
        assert_eq!(row(&owner.db), ("lost".into(), before.1.clone()));
        // Retry uses the original saved identity, not the parent's mount namespace.
        let identity: String = owner
            .db
            .query_row("SELECT identity FROM snapshots", [], |r| r.get(0))
            .unwrap();
        let error = owner
            .handle(KeeperRequest::Prepare {
                run: "crash-run".into(),
                workspace: root.path().join("workspace"),
                identity,
            })
            .err()
            .expect("restart must refuse replacement admission");
        assert!(
            error.to_string().contains("reconciliation"),
            "{point}: {error:#}"
        );
        assert_eq!(row(&owner.db), ("lost".into(), before.1.clone()));
        let count: i64 = owner
            .db
            .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
        if point != "preparing" {
            let state: (String, Option<u64>, Option<u64>, Option<String>) = owner.db.query_row("SELECT state,device,inode,policy FROM runtime_allocations WHERE run='crash-run'", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).unwrap();
            let allocated = !["allocation_intent", "allocation_directory"].contains(&point);
            assert_eq!(state.0, if allocated { "allocated" } else { "allocating" });
            assert_eq!(
                state.1.is_some() && state.2.is_some() && state.3.is_some(),
                allocated
            );
            let entries = fs::read_dir(root.path().join("policy/sessions"))
                .unwrap()
                .count();
            assert_eq!(entries, usize::from(point != "allocation_intent"));
        }
        drop((owner, d, r));
        let inspect = cli("inspect", root.path(), runtime.path(), &owner_id, &[]);
        assert!(inspect["blockers"]
            .as_str()
            .unwrap()
            .contains("runtime-masked"));
        systemctl("reset-failed", &unit.0);
        systemctl("mask", &unit.0);
        let inspect = cli("inspect", root.path(), runtime.path(), &owner_id, &[]);
        assert!(inspect["blockers"].is_null(), "{point}: {inspect}");
        let evidence = root.path().join("stop.json");
        cli(
            "stop-proof",
            root.path(),
            runtime.path(),
            &owner_id,
            &[
                "--output",
                evidence.to_str().unwrap(),
                "--confirm-managed-stop",
            ],
        );
        let request = Uuid::new_v4().to_string();
        let args = [
            "--expect-record-digest",
            inspect["record_digest"].as_str().unwrap(),
            "--managed-stop-evidence",
            evidence.to_str().unwrap(),
            "--request-id",
            &request,
            "--operator",
            "preparation-test",
            "--reason",
            "configured preparation crash",
            "--confirm-terminal-revocation",
        ];
        let receipt = cli("revoke", root.path(), runtime.path(), &owner_id, &args);
        assert_eq!(receipt["worker_cleanup_ack"], false);
        assert_eq!(receipt["cleanup_verified"], false);
        assert_eq!(receipt["artifacts_retained"], true);
        assert_eq!(
            cli("revoke", root.path(), runtime.path(), &owner_id, &args),
            receipt
        );
        let db = Connection::open(root.path().join("durable/owner.sqlite3")).unwrap();
        assert_eq!(row(&db), ("reconciled_revoked".into(), before.1));
        crate::keeper_reconcile::release_ack(&db, "crash-run", runtime.path()).unwrap();
        assert_eq!(
            crate::keeper_managed::load(&db, "crash-run").unwrap(),
            binding
        );
        eprintln!("qualified preparation crash + explicit terminal reconciliation: {point}");
    }
}
