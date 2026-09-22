use super::*;

#[test]
#[ignore = "subprocess fixture invoked by cross_process_lock_and_exit_preserve_uncertain_execution"]
fn research_lock_child_process() {
    let Some(root) = std::env::var_os("ARDA_TEST_RESEARCH_OPERATION_ROOT") else {
        return;
    };
    let mode = std::env::var("ARDA_TEST_RESEARCH_OPERATION_MODE").unwrap();
    let result = claim(
        Path::new(&root),
        "process-event",
        "digest",
        "session",
        "owner",
    );
    match mode.as_str() {
        "contended" => {
            assert!(format!("{:?}", result.err().unwrap()).contains("Research is pending"))
        }
        "exit-after-intent" => {
            let Claim::Started(operation) = result.unwrap() else {
                panic!("fresh execution")
            };
            executing(&operation).unwrap();
            // Deliberately skip Rust destructors: kernel process teardown must
            // release the operation lock while persisted intent survives.
            std::process::exit(0);
        }
        _ => panic!("unsupported child fixture"),
    }
}

#[test]
fn cross_process_lock_and_exit_preserve_uncertain_execution() {
    let root = tempfile::tempdir().unwrap();
    let child = |mode: &str| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "harness::operator_messages::research_replay::tests::research_lock_child_process",
                "--nocapture",
                "--ignored",
            ])
            .env("ARDA_TEST_RESEARCH_OPERATION_ROOT", root.path())
            .env("ARDA_TEST_RESEARCH_OPERATION_MODE", mode)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("running 1 test"));
    };
    let Claim::Started(operation) =
        claim(root.path(), "process-event", "digest", "session", "owner").unwrap()
    else {
        panic!("fresh claim")
    };
    child("contended");
    drop(operation);
    child("exit-after-intent");
    let Claim::Complete(response) =
        claim(root.path(), "process-event", "digest", "session", "owner").unwrap()
    else {
        panic!("must not repeat after process exit")
    };
    assert!(response.summary.contains("interrupted"));
    assert!(!root.path().join("data/runs").exists());
}

#[test]
fn saved_replay_refreshes_expiry_and_isolates_unrelated_corruption() {
    let root = tempfile::tempdir().unwrap();
    let operation = match claim(root.path(), "event", "digest", "session", "owner").unwrap() {
        Claim::Started(operation) => operation,
        Claim::Complete(_) => panic!("new operation"),
    };
    let mut body = serde_json::json!({
        "schema_version":"arda.research.question-brief.v1", "execution_authorized":false,
        "subject":{"owner":"owner", "event_id":"event"}, "question_id":"question",
        "summary":"Advisory fixture", "citations":[], "uncertainty":[],
        "expires_at_utc":(Utc::now()-Duration::seconds(1)).to_rfc3339()
    });
    let hash = format!("{:x}", Sha256::digest(serde_json::to_vec(&body).unwrap()));
    let id = format!("question-brief-{hash}");
    body["brief_id"] = id.clone().into();
    body["content_sha256"] = format!("sha256:{hash}").into();
    let dir = root.path().join("data/workbench/research/briefs");
    fs::create_dir_all(&dir).unwrap();
    let target = dir.join(format!("{id}.json"));
    fs::write(&target, serde_json::to_vec(&body).unwrap()).unwrap();
    let cached = GatewayOperatorResponse {
        schema_version: "arda.gateway-operator-response.v1".into(),
        summary: "Earlier unexpired result".into(),
        evidence_refs: vec![format!("arda://research/briefs/{id}")],
        session_id: "session".into(),
        run_id: None,
    };
    finish(&operation, &cached).unwrap();
    drop(operation);
    let Claim::Complete(response) =
        claim(root.path(), "event", "digest", "session", "owner").unwrap()
    else {
        panic!("no refetch")
    };
    assert!(response.summary.contains("EXPIRED"));
    assert!(response
        .summary
        .contains("Assimilation is pending or failed"));
    fs::write(dir.join("unrelated.json"), b"broken").unwrap();
    assert!(matches!(
        claim(root.path(), "event", "digest", "session", "owner").unwrap(),
        Claim::Complete(_)
    ));
    let other = match claim(
        root.path(),
        "other-event",
        "other-digest",
        "session",
        "owner",
    )
    .unwrap()
    {
        Claim::Started(operation) => operation,
        Claim::Complete(_) => panic!("new operation"),
    };
    finish(&other, &cached).unwrap();
    drop(other);
    assert!(
        claim(
            root.path(),
            "other-event",
            "other-digest",
            "session",
            "owner"
        )
        .is_err(),
        "same-owner wrong-event journal must fail closed"
    );
    fs::write(target, b"broken target").unwrap();
    assert!(claim(root.path(), "event", "digest", "session", "owner").is_err());
}

#[test]
fn liveness_is_lock_owned_and_preexecution_is_retryable() {
    let root = tempfile::tempdir().unwrap();
    let start = || match claim(root.path(), "event", "digest", "session", "owner").unwrap() {
        Claim::Started(operation) => operation,
        Claim::Complete(_) => panic!("must remain pre-execution"),
    };
    let operation = start();
    let mut record = read_record(&operation.path).unwrap();
    record.started_at = Utc::now() - Duration::seconds(120);
    write_record(&operation.path, &record).unwrap();
    assert!(claim(root.path(), "event", "digest", "session", "owner").is_err());
    let inherited_descriptor = operation._lock.try_clone().unwrap();
    drop(operation);
    let operation = start();
    drop(inherited_descriptor);
    executing(&operation).unwrap();
    let path = operation.path.clone();
    drop(operation);
    match claim(root.path(), "event", "digest", "session", "owner").unwrap() {
        Claim::Complete(response) => assert!(response.summary.contains("interrupted")),
        Claim::Started(_) => panic!("uncertain execution must never restart"),
    }
    assert!(read_record(&path).unwrap().phase == Phase::Terminal);
    assert!(claim(root.path(), "event", "different", "session", "owner").is_err());
    fs::write(&path, b"malformed").unwrap();
    assert!(claim(root.path(), "event", "digest", "session", "owner").is_err());
}
