use super::*;

#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_release_cancels_open_connection() {
    active_rebind(false, false, false, 1);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_release_wire_ack_loss_is_retryable() {
    active_rebind(false, false, false, 2);
}
#[test]
#[ignore = "requires real user/mount namespaces and bubblewrap"]
fn active_execution_release_retries_releasing_owner() {
    active_rebind(false, false, false, 3);
}

pub(super) fn lose_ack(
    socket: &Path,
    snapshot: &RetainedSnapshot,
    run: &str,
) -> anyhow::Result<()> {
    use arda_engine::objectives::keeper_client::KeeperRequest;
    use std::io::{BufRead, BufReader, Write};
    let proxy = socket.with_file_name("lost-release.sock");
    let listener = UnixListener::bind(&proxy)?;
    let target = socket.to_owned();
    let forward = std::thread::spawn(move || {
        let (mut downstream, _) = listener.accept().unwrap();
        downstream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut payload = String::new();
        BufReader::new(&mut downstream)
            .read_line(&mut payload)
            .unwrap();
        let mut upstream = UnixStream::connect(target).unwrap();
        upstream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        upstream
            .set_write_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        upstream.write_all(payload.as_bytes()).unwrap();
        let mut response = String::new();
        BufReader::new(upstream).read_line(&mut response).unwrap();
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();
        assert_eq!(response["ok"], true);
        // Do not deliver the successful keeper ACK to the admission client.
    });
    let lost = exchange::<_, serde_json::Value>(
        &proxy,
        &KeeperRequest::Release {
            snapshot: snapshot.clone(),
            run: run.into(),
        },
        Duration::from_secs(6),
    );
    forward.join().unwrap();
    assert!(lost.is_err(), "missing reply must fail transport");
    anyhow::bail!("fixture loses Release ACK on wire")
}

pub(super) fn finish(
    mode: u8,
    store: &ObjectiveStore,
    client: &Arc<RebindClient>,
    snapshot: &RetainedSnapshot,
    run: &str,
    db: &Path,
    durable: &Path,
) {
    let denied: serde_json::Value = exchange(
        Path::new(&snapshot.endpoint),
        &Request::Release {
            capability: "invalid".into(),
        },
        Duration::from_secs(2),
    )
    .unwrap();
    assert_eq!(denied["ok"], false);
    assert!(
        !client.witness.lock().unwrap().as_ref().unwrap().exited(),
        "invalid Release cancelled work"
    );
    let owner = rusqlite::Connection::open(durable.join("owner.sqlite3")).unwrap();
    if mode == 3 {
        // Model a durable Release intent whose prior delivery failed while this
        // same owner still holds the live child. This is not cleanup evidence.
        owner
            .execute("UPDATE snapshots SET state='releasing' WHERE run=?1", [run])
            .unwrap();
    }
    let began = Instant::now();
    store
        .apply_control(
            "rebind",
            ControlAction::Cancel,
            "cancel",
            "operator",
            chrono::Utc::now().timestamp_millis(),
        )
        .unwrap();
    let result = store.reconcile_snapshot_commits();
    let engine = rusqlite::Connection::open(db).unwrap();
    if mode == 2 {
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Release ACK on wire"));
        let count: i64 = engine
            .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0, "lost ACK invented Engine cleanup receipt");
    } else {
        result.unwrap();
    }
    assert!(
        began.elapsed() < Duration::from_secs(5),
        "Release waited for command timeout"
    );
    let witness = client.witness.lock().unwrap();
    let witness = witness.as_ref().unwrap();
    assert!(witness.exited());
    assert!(!Path::new(&format!("/proc/{}", witness.pid)).exists());
    for ancestor in client.supervisors.lock().unwrap().iter() {
        assert!(ancestor.exited(), "Release ACK preceded supervisor exit");
        assert!(!Path::new(&format!("/proc/{}", ancestor.pid)).exists());
    }
    let state: String = owner
        .query_row("SELECT state FROM snapshots WHERE run=?1", [run], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(state, "released");
    assert!(!Path::new(&snapshot.endpoint).exists());
    // Reopen Engine with its original authority and retry; no new Prepare.
    let reopened = ObjectiveStore::open_existing(db)
        .unwrap()
        .with_snapshot_admission(client.clone());
    reopened.reconcile_snapshot_commits().unwrap();
    reopened.reconcile_snapshot_commits().unwrap();
    let count: i64 = engine
        .query_row("SELECT count(*) FROM retained_snapshot_releases", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
    assert_eq!(client.prepares.load(Ordering::SeqCst), 1);
}
