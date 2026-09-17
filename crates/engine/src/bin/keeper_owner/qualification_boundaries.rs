use super::*;
use std::{cell::Cell, process::Command, time::Duration};

// External reap makes Child::try_wait deterministically return ECHILD.
// This models unprovable ownership, not an uninterruptible kernel task.
fn unprovable_child() -> Child {
    let mut child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    child.kill().unwrap();
    let mut status = 0;
    assert_eq!(
        unsafe { libc::waitpid(child.id() as i32, &mut status, 0) },
        child.id() as i32
    );
    assert_eq!(
        child.try_wait().unwrap_err().raw_os_error(),
        Some(libc::ECHILD)
    );
    child
}

#[test]
fn final_queue_destruction_preserves_unproven_pins() {
    struct Pins(Rc<Cell<usize>>);
    impl Drop for Pins {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let released = Rc::new(Cell::new(0));
    let queue = Rc::new(RefCell::new(vec![Unreaped {
        child: unprovable_child(),
        pins: Some(Pins(released.clone())),
    }]));
    let last_owner = queue.clone();
    drop(queue);
    assert_eq!(released.get(), 0);
    drop(last_owner);
    assert_eq!(
        released.get(),
        0,
        "unproven final destruction released pins"
    );
}

#[test]
fn pending_cleanup_refuses_prepare_but_allows_commit_and_release() {
    use super::super::Owner;
    use crate::{keeper_storage, KeeperRequest, RetainedSnapshot};
    use arda_engine::objectives::snapshot_protocol::Lease;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    let durable = tempfile::tempdir_in("/var/tmp").unwrap();
    let runtime = tempfile::tempdir_in("/dev/shm").unwrap();
    use std::os::unix::fs::PermissionsExt;
    for path in [durable.path(), runtime.path()] {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }
    keeper_storage::initialize(durable.path()).unwrap();
    let (db, _owner_lock, _endpoint_lock) =
        keeper_storage::open(durable.path(), runtime.path()).unwrap();
    let endpoint = runtime.path().join("fixture.sock");
    let listener = UnixListener::bind(&endpoint).unwrap();
    let child = Command::new("/bin/sleep").arg("30").spawn().unwrap();
    let pid = child.id();
    let snapshot = RetainedSnapshot {
        endpoint: endpoint.to_str().unwrap().into(),
        capability: "fixture-capability".into(),
        manifest_digest: "fixture-digest".into(),
    };
    db.execute(
        "INSERT INTO snapshots VALUES ('existing','/fixture','identity','prepared',?1)",
        [serde_json::to_string(&snapshot).unwrap()],
    )
    .unwrap();
    db.execute(
        "INSERT INTO snapshots VALUES ('failed','/fixture','identity','preparing',NULL)",
        [],
    )
    .unwrap();
    let cleanup = Cleanup::default();
    cleanup.0.borrow_mut().push(Unreaped {
        child: unprovable_child(),
        pins: None,
    });
    let mut owner = Owner {
        managed: None,
        reservations: None,
        runtime_policy: None,
        db,
        durable: durable.path().into(),
        runtime: runtime.path().into(),
        worker: "/unused".into(),
        children: [("existing".into(), child)].into(),
        failed_qualifications: cleanup.clone(),
    };
    let error = owner
        .handle(KeeperRequest::Prepare {
            run: "new".into(),
            workspace: "/fixture".into(),
            identity: "identity".into(),
        })
        .err()
        .unwrap();
    assert!(error
        .to_string()
        .contains("preparation cleanup remains unproven"));
    let server = std::thread::spawn(move || {
        for expected in ["commit", "release"] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
            assert!(line.to_lowercase().contains(expected), "{line}");
            if expected == "release" {
                assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGKILL) }, 0);
            }
            stream.write_all(b"{\"ok\":true}\n").unwrap();
        }
    });
    owner
        .handle(KeeperRequest::Commit {
            snapshot: snapshot.clone(),
            lease: Lease {
                run_id: "existing".into(),
                generation: 1,
                owner: "fixture".into(),
                expires_ms: i64::MAX,
            },
        })
        .unwrap();
    assert!(cleanup.pending());
    owner
        .handle(KeeperRequest::Release {
            snapshot,
            run: "existing".into(),
        })
        .unwrap();
    server.join().unwrap();
    assert!(owner.children.is_empty());
    let rows = owner
        .db
        .prepare("SELECT run,state FROM snapshots ORDER BY run")
        .unwrap()
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![
            ("existing".into(), "released".into()),
            ("failed".into(), "preparing".into())
        ]
    );
    drop(cleanup);
    drop(owner);
    let persisted = rusqlite::Connection::open(durable.path().join("owner.sqlite3")).unwrap();
    assert_eq!(
        persisted
            .query_row("SELECT state FROM snapshots WHERE run='failed'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        "preparing"
    );
}
