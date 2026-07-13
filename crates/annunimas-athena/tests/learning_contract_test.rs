use annunimas_athena::learning::{
    emit_delta_to_root, KnowledgeDelta, KNOWLEDGE_DELTA_RELATIVE_PATH,
    KNOWLEDGE_DELTA_SCHEMA_VERSION,
};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_knowledge_delta_schema_and_emit() {
    let temp_dir = tempdir().unwrap();

    let delta = KnowledgeDelta::new(
        "human/library/athena/sources/test.md",
        0.85,
        0.1,
        "Test knowledge about Annunimas Learning Loop",
        3600,
    );

    assert_eq!(delta.schema_version, KNOWLEDGE_DELTA_SCHEMA_VERSION);
    assert_eq!(delta.source_path, "human/library/athena/sources/test.md");
    assert!((delta.confidence - 0.85).abs() < f32::EPSILON);
    assert!((delta.uncertainty - 0.1).abs() < f32::EPSILON);
    assert_eq!(
        delta.delta_content,
        "Test knowledge about Annunimas Learning Loop"
    );
    assert!(delta.created_at_unix > 0);
    assert_eq!(delta.expires_at_unix, delta.created_at_unix + 3600);
    assert!(!delta.is_expired());
    assert!(delta.is_valid_contract_shape());

    let serialized = serde_json::to_string(&delta).unwrap();
    let deserialized: KnowledgeDelta = serde_json::from_str(&serialized).unwrap();
    assert_eq!(delta, deserialized);

    emit_delta_to_root(&delta, temp_dir.path()).unwrap();

    let emitted = fs::read_to_string(temp_dir.path().join(KNOWLEDGE_DELTA_RELATIVE_PATH)).unwrap();
    let lines: Vec<&str> = emitted.lines().collect();
    assert_eq!(lines.len(), 1);

    let emitted_delta: KnowledgeDelta = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(emitted_delta, delta);
}

#[test]
fn test_knowledge_delta_rejects_invalid_contract_shape() {
    let temp_dir = tempdir().unwrap();
    let mut delta = KnowledgeDelta::new("source.md", 0.5, 0.2, "content", 60);
    delta.confidence = 1.5;

    let error = emit_delta_to_root(&delta, temp_dir.path()).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert!(!temp_dir.path().join(KNOWLEDGE_DELTA_RELATIVE_PATH).exists());
}
