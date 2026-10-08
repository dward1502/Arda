use arda_varda::{benchmark::run_retrieval_benchmark, ingest::AthenaStore};

// Integration tests share the production process-wide ingest admission gate.
// Serialize fixtures rather than relaxing production limits or racing env vars.
static INGEST_FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("../fixtures/retrieval_benchmark_v1.json")).unwrap()
}
fn run(value: &serde_json::Value) -> arda_core::error::Result<serde_json::Value> {
    let _guard = INGEST_FIXTURE.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.json");
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    let store = AthenaStore::new_isolated(root.path().join("store")).unwrap();
    Ok(serde_json::to_value(run_retrieval_benchmark(path, &store)?).unwrap())
}
#[test]
fn repository_question_corpus_has_measured_coverage_and_source_spans() {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/reliability_questions_v1.json")).unwrap();
    let report = run(&value).unwrap();
    assert_eq!(report["queries_total"], 24);
    assert!(report["bm25"]["recall_at_k"].as_f64().unwrap() >= 0.95);
    assert!(report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .all(|case| case["citation_spans_valid"] == true));
}

#[test]
fn benchmark_compares_same_corpus_and_records_ranked_source_spans() {
    let report = run(&fixture()).unwrap();
    assert_eq!(report["lexical"]["query_count"], 3);
    assert_eq!(report["bm25"]["query_count"], 3);
    assert_eq!(report["bm25"]["limit"], 3);
    assert_eq!(report["bm25"]["hit_at_1"], 1.0);
    assert_eq!(report["fixture_sha256"].as_str().unwrap().len(), 64);
    assert!(!report["cases"][0]["citations"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(report["cases"][0]["citation_spans_valid"]
        .as_bool()
        .unwrap());
}
#[test]
fn multi_relevant_recall_is_not_top_one_hit_rate() {
    let mut value = fixture();
    value["queries"] = serde_json::json!([{
        "query":"OAuth authentication", "expected_document_ids":["auth", "retrieval"]
    }]);
    let report = run(&value).unwrap();
    assert_eq!(report["recall_at_1"], 0.5);
}
#[test]
fn canonical_source_collision_is_rejected() {
    let mut value = fixture();
    value["documents"][0]["content"] = serde_json::json!("R07 collision probe 56887");
    value["documents"][1]["content"] = serde_json::json!("R07 collision probe 149132");
    assert!(run(&value).is_err());
}

#[test]
fn populated_store_is_rejected_before_fixture_ingest() {
    let _guard = INGEST_FIXTURE.lock().unwrap();
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("fixture.json");
    std::fs::write(&path, serde_json::to_vec(&fixture()).unwrap()).unwrap();
    let store = AthenaStore::new_isolated(root.path().join("store")).unwrap();
    store
        .ingest("unrelated zebra corpus", "test", "foreign document")
        .unwrap();
    let before = std::fs::read(store.digest_path()).unwrap();
    let error = run_retrieval_benchmark(&path, &store).unwrap_err();
    assert!(error.to_string().contains("empty Books directory"));
    assert_eq!(std::fs::read(store.digest_path()).unwrap(), before);
}

#[test]
fn duplicate_document_identity_is_rejected() {
    let mut value = fixture();
    value["documents"][1]["id"] = serde_json::json!("auth");
    value["queries"][1]["expected_document_ids"] = serde_json::json!(["auth"]);
    assert!(run(&value).is_err());
}
#[test]
fn duplicate_content_cannot_alias_two_judged_documents() {
    let mut value = fixture();
    value["documents"][1]["content"] = value["documents"][0]["content"].clone();
    assert!(run(&value).is_err());
}
