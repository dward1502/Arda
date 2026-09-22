use super::*;

#[test]
fn unrelated_ledger_growth_does_not_disable_matching_records() {
    use std::io::Write;
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("ledger.jsonl");
    let mut file = std::io::BufWriter::new(std::fs::File::create(&path).unwrap());
    let padding = vec![b' '; 1023];
    for _ in 0..17 * 1024 {
        file.write_all(&padding).unwrap();
        file.write_all(b"\n").unwrap();
    }
    file.write_all(b"{\"match\":true}\n").unwrap();
    drop(file);
    assert!(any_record(&path, |value| value["match"] == true).unwrap());
}

#[test]
fn explicit_deep_pipeline_survives_interleaved_ingest() {
    let root = tempfile::tempdir().unwrap();
    let store = arda_varda::ingest::AthenaStore::new_isolated(root.path()).unwrap();
    let first = store
        .ingest(
            "https://example.invalid/interleaved",
            "workbench_research",
            "first context",
        )
        .unwrap();
    let second = store
        .ingest(
            "https://example.invalid/interleaved",
            "workbench_research",
            "second context",
        )
        .unwrap();
    let first_deep = store
        .deep_analyze_for_pipeline(&first.id, &first.pipeline_id)
        .unwrap();
    let second_deep = store
        .deep_analyze_for_pipeline(&second.id, &second.pipeline_id)
        .unwrap();
    assert_ne!(first.pipeline_id, second.pipeline_id);
    assert_eq!(first_deep.pipeline_id, first.pipeline_id);
    assert_eq!(second_deep.pipeline_id, second.pipeline_id);
    let events = std::fs::read_to_string(root.path().join("side-effects/warden.jsonl")).unwrap();
    let pipelines: Vec<String> = events
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .filter(|event| event["event"] == "athena_deep_complete")
        .map(|event| event["pipeline_id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        pipelines,
        vec![first.pipeline_id.clone(), second.pipeline_id.clone()]
    );
    assert!(store
        .deep_analyze_for_pipeline(&first.id, "unknown-pipeline")
        .is_err());
}

#[test]
fn local_capture_lineage_checks_independent_ledgers_without_fetching() {
    for url in [
        "https://example.invalid/lineage",
        "https://example.invalid/",
    ] {
        check_local_lineage(url);
    }
}

fn check_local_lineage(url: &str) {
    let temp = tempfile::tempdir().unwrap();
    let query = "fixture research question";
    let context = "fixture research context";
    let capture = crate::harness::ResearchStorePolicy::Isolated
        .ingest_capture(
            temp.path(),
            context,
            arda_varda::ingest::CrawlMarkdownResult {
                pipeline_id: String::new(),
                url: url.into(),
                filter: "canonical_http_text".into(),
                query: Some(query.into()),
                markdown: "Fixture source material, not a real HTTP response.".into(),
                success: true,
                provider: "fixture".into(),
            },
        )
        .unwrap();
    let first_pipeline = capture.record.pipeline_id;
    let capture = crate::harness::ResearchStorePolicy::Isolated
        .ingest_capture(
            temp.path(),
            context,
            arda_varda::ingest::CrawlMarkdownResult {
                pipeline_id: String::new(),
                url: url.into(),
                query: Some(query.into()),
                markdown: "A second bounded capture for the same source.".into(),
                provider: "fixture".into(),
                filter: "canonical_http_text".into(),
                success: true,
            },
        )
        .unwrap();
    assert_ne!(first_pipeline, capture.record.pipeline_id);
    assert_eq!(capture.deep.pipeline_id, capture.record.pipeline_id);
    let evaluation = digest_value(&serde_json::json!({
        "policy_readiness": capture.deep.data.policy_readiness,
        "confidence": capture.deep.data.confidence,
        "triad_passed": capture.deep.data.triad_analysis.passed,
        "extraction_status": capture.deep.data.extraction_status,
    }));
    let value = serde_json::json!({
        "citation_id":"fixture", "title":"fixture", "discovered_url":url,
        "canonical_url":url, "content_sha256":"not-a-raw-http-attestation",
        "excerpt":"fixture", "stance":"neutral", "varda_source_id":capture.record.id,
        "varda_pipeline_id":capture.record.pipeline_id, "crawl_receipt_path":capture.receipt.artifact_path,
        "policy_readiness":capture.deep.data.policy_readiness, "confidence":capture.deep.data.confidence,
        "normalized_source_id":"fixture", "fetched_at_utc":"fixture", "expires_at_utc":"fixture",
        "freshness_status":"fixture", "evaluation_digest":evaluation, "expiry_digest":"fixture",
        "receipt_references":[], "evidence_boundary":"fixture", "prompt_injection_detected":false,
        "prompt_injection_signals":[]
    });
    let citation: BriefCitation = serde_json::from_value(value.clone()).unwrap();
    verify(temp.path(), &citation, query, context).unwrap();
    for path in [
        temp.path().join("digest.jsonl"),
        temp.path().join("crawl_receipts.jsonl"),
        temp.path()
            .join("books")
            .join(format!("{}.jsonl", citation.varda_source_id)),
    ] {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
        writeln!(file, "malformed unrelated record").unwrap();
    }
    verify(temp.path(), &citation, query, context).unwrap();
    // Raw HTTP authenticity and fitness are intentionally outside this verifier.
    for (field, replacement) in [
        ("canonical_url", "https://different.invalid/lineage"),
        ("varda_pipeline_id", "different-pipeline"),
        ("crawl_receipt_path", "/untrusted/arbitrary/path"),
        ("policy_readiness", "forged-readiness"),
        ("evaluation_digest", "sha256:forged"),
        ("varda_source_id", "../../escape"),
    ] {
        let mut changed = value.clone();
        changed[field] = replacement.into();
        let changed = serde_json::from_value(changed).unwrap();
        assert!(
            verify(temp.path(), &changed, query, context).is_err(),
            "{field}"
        );
    }
    assert!(verify(temp.path(), &citation, "different query", context).is_err());
    assert!(verify(temp.path(), &citation, query, "different context").is_err());
    std::fs::remove_file(temp.path().join("crawl_receipts.jsonl")).unwrap();
    assert!(verify(temp.path(), &citation, query, context).is_err());
    assert!(!temp.path().join("crawl_receipts.jsonl").exists());
    let absent = temp.path().join("absent-store");
    assert!(verify(&absent, &citation, query, context).is_err());
    assert!(!absent.exists());
}
