use super::super::{run_bounded, workspace::PinnedWorkspace};
use super::*;
use crate::adapters::AdapterCancellation;
use std::{
    collections::BTreeMap,
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};

#[tokio::test(flavor = "current_thread")]
async fn construction_thread_can_retire_before_real_captured_launch() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().to_owned();
    let mut prepared = std::thread::spawn(move || {
        PinnedWorkspace::open(&path)
            .unwrap()
            .command(Path::new("/bin/sh"), &path, &BTreeMap::new(), true)
            .unwrap()
    })
    .join()
    .unwrap();
    prepared
        .command
        .args(["-c", "printf owner-ok"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let result = run_bounded(
        prepared,
        Instant::now() + Duration::from_secs(3),
        &AdapterCancellation::new(),
        100,
        1024,
    )
    .await
    .unwrap();

    assert_eq!(result.stdout, b"owner-ok");
}

#[tokio::test(flavor = "current_thread")]
async fn aborting_caller_cancels_and_reaps_owned_child() {
    let root = tempfile::tempdir().unwrap();
    let marker = root.path().join("child.pid");
    let mut command = Command::new("/usr/bin/python3");
    command
        .args([
            "-I",
            "-c",
            "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)",
        ])
        .arg(&marker)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    super::super::configure_process_group(&mut command);
    let cancel = AdapterCancellation::new();
    let token = cancel.clone();
    let owner = tokio::spawn(async move {
        run_bounded(
            PreparedCommand::plain(command),
            Instant::now() + Duration::from_secs(60),
            &token,
            100,
            1024,
        )
        .await
    });
    let pid: i32 = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let Ok(text) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = text.parse() {
                    break pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    owner.abort();
    assert!(matches!(owner.await, Err(error) if error.is_cancelled()));
    assert!(*cancel.subscribe().borrow());
    tokio::time::timeout(Duration::from_secs(3), async {
        while unsafe { libc::kill(pid, 0) } == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("launch owner must reap after caller abort");
    let error = std::io::Error::last_os_error();
    assert_eq!(error.raw_os_error(), Some(libc::ESRCH));
}
