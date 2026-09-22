use arda_engine::harness::ResearchStorePolicy;
use arda_varda::ingest::CrawlMarkdownResult;

#[test]
fn isolated_harness_store_keeps_research_lifecycle_under_fixture_root() {
    let fixture = tempfile::tempdir().unwrap();
    let root = fixture.path().join("athena");
    let store = ResearchStorePolicy::Isolated.open(&root).unwrap();
    let url = "https://example.com/research-isolation";
    assert!(store
        .interceptor_names()
        .iter()
        .all(|name| !name.to_lowercase().contains("mnemosyne")));
    let result = ResearchStorePolicy::Isolated
        .ingest_capture(
            &root,
            "fixture",
            CrawlMarkdownResult {
                pipeline_id: String::new(),
                url: url.to_string(),
                filter: "canonical_http_text".to_string(),
                query: Some("fixture".to_string()),
                markdown: "Bounded research evidence for a fixture.".to_string(),
                success: true,
                provider: "fixture_no_network".to_string(),
            },
        )
        .unwrap();
    let record = result.record;
    let capture = result.receipt;
    let deep = result.deep;
    assert_eq!(deep.pipeline_id, record.pipeline_id);
    assert_eq!(
        std::fs::read_to_string(&capture.artifact_path).unwrap(),
        "Bounded research evidence for a fixture."
    );
    assert!(root
        .join("books")
        .join(format!("{}.jsonl", record.id))
        .exists());
    assert!(root.join("crawl_receipts.jsonl").exists());
    assert!(root.join("operator-library/sources").exists());
    let observations = std::fs::read_to_string(root.join("side-effects/warden.jsonl")).unwrap();
    assert!(observations.contains("athena_deep_"));
    assert!(!root.join("side-effects/hades.jsonl").exists());
}
