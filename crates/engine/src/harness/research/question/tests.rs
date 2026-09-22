use super::*;

#[test]
fn oversized_self_consistent_brief_is_rejected_before_projection() {
    let root = tempfile::tempdir().unwrap();
    let body = serde_json::json!({
        "schema_version":"arda.research.question-brief.v1", "execution_authorized":false,
        "subject":{"kind":"question","owner":"owner","event_id":"event"},
        "question_id":"fixture-question", "summary":"x".repeat(256 * 1024),
        "citations":[], "uncertainty":[],
        "expires_at_utc":(Utc::now()+ChronoDuration::minutes(5)).to_rfc3339()
    });
    let (id, _) = publish(root.path(), body.clone()).unwrap();
    let error = read_answer(root.path(), "owner", &id)
        .expect_err("oversized brief must not reach projection or worker context");
    assert!(format!("{error:?}").contains("byte limit"));
    assert!(recover_answer(root.path(), "owner", "event")
        .expect("oversized candidates must be skipped before parsing or binding")
        .is_none());
    let mut valid = body;
    valid["summary"] = "bounded fixture".into();
    let (valid_id, _) = publish(root.path(), valid).unwrap();
    let (_, refs) = recover_answer(root.path(), "owner", "event")
        .unwrap()
        .unwrap();
    assert!(refs.contains(&format!("arda://research/briefs/{valid_id}")));
    assert!(!refs.contains(&format!("arda://research/briefs/{id}")));
}

#[test]
fn brief_read_limit_is_inclusive_and_rejects_one_extra_byte() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("boundary.json");
    let bytes = vec![b' '; 256 * 1024];
    fs::write(&path, &bytes).unwrap();
    assert_eq!(read_brief_bytes(&path).unwrap(), bytes);
    let mut oversized = bytes;
    oversized.push(b' ');
    fs::write(&path, oversized).unwrap();
    assert!(read_brief_bytes(&path).is_err());
}

#[test]
fn publication_remains_recoverable_without_assimilation_receipt() {
    let root = tempfile::tempdir().unwrap();
    let body = serde_json::json!({
        "schema_version":"arda.research.question-brief.v1", "execution_authorized":false,
        "subject":{"kind":"question","owner":"owner","event_id":"event"},
        "question_id":"fixture-question", "summary":"No source evidence in fixture.",
        "citations":[], "uncertainty":[],
        "expires_at_utc":(Utc::now()+ChronoDuration::minutes(5)).to_rfc3339()
    });
    let (id, _) = publish(root.path(), body).unwrap();
    fs::write(
        root.path()
            .join("data/workbench/research/briefs/unrelated.json"),
        b"broken",
    )
    .unwrap();
    let (summary, refs) = recover_answer(root.path(), "owner", "event")
        .unwrap()
        .unwrap();
    assert!(summary.contains("Assimilation is pending or failed"));
    assert!(refs.contains(&format!("arda://research/briefs/{id}")));
    assert!(read_answer(root.path(), "other", &id).is_err());
    assert!(recover_answer(root.path(), "owner", "different")
        .unwrap()
        .is_none());
    assert!(!root.path().join("data/runs").exists());
    let path = root
        .path()
        .join("data/workbench/research/briefs")
        .join(format!("{id}.json"));
    let parsed: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let raw = format!("\n{}\n", serde_json::to_string_pretty(&parsed).unwrap()).into_bytes();
    fs::write(&path, &raw).unwrap();
    let (loaded, semantic) = load_verified_brief(root.path(), "owner", &id, Some("event")).unwrap();
    assert_eq!(
        loaded, raw,
        "retain original bytes rather than reconstructed JSON"
    );
    assert_eq!(semantic["question_id"], "fixture-question");
    assert!(load_verified_brief(root.path(), "owner", &id, Some("other-event")).is_err());
}
