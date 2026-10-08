use arda_varda::ingest::AthenaStore;

#[test]
fn note_body_is_searchable_with_exact_utf8_citations_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let content = format!(
        "{}\nCafé continuity requires zephyrquartz receipts.",
        "Ordinary heading ".repeat(8)
    );
    let id = {
        let store = AthenaStore::new_isolated(root.path()).unwrap();
        let record = store
            .ingest(&content, "fixture", "R07 source coverage")
            .unwrap();
        assert!(record.shallow.summary.contains("zephyrquartz"));
        record.id
    };
    let reopened = AthenaStore::new_isolated(root.path()).unwrap();
    let response = reopened.query("zephyrquartz", 3).unwrap();
    assert_eq!(response.matches.len(), 1);
    let hit = &response.matches[0];
    assert_eq!(hit.source_id, id);
    assert!(hit.shallow_only);
    let citation = hit
        .citations
        .iter()
        .find(|c| c.span.field == "summary")
        .unwrap();
    assert_eq!(
        hit.summary.get(citation.span.start..citation.span.end),
        Some("zephyrquartz")
    );
    assert!(content.contains(&hit.summary));
    assert_eq!(citation.doc_id, hit.book_ref);
}

#[test]
fn source_excerpt_is_bounded_but_raw_input_is_not_replaced() {
    let root = tempfile::tempdir().unwrap();
    let store = AthenaStore::new_isolated(root.path()).unwrap();
    let content = format!("Research note\n{}endingmarker", "é".repeat(20_000));
    let record = store.ingest(&content, "fixture", "R07 bound").unwrap();
    assert_eq!(record.raw_input, content);
    assert_eq!(record.shallow.summary.chars().count(), 16_384);
    assert!(content.starts_with(&record.shallow.summary));
}
