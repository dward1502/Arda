#[cfg(test)]
mod canary {
    #[test]
    fn learning_owns_struct_and_const() {
        let _ = arda_learning::KnowledgeDelta {
            schema_version: String::new(),
            source_path: String::new(),
            confidence: 0.0,
            uncertainty: 0.0,
            created_at_unix: 0,
            expires_at_unix: 0,
            delta_content: String::new(),
        };
        assert_eq!(
            arda_learning::KNOWLEDGE_DELTA_SCHEMA_VERSION,
            "arda.athena.knowledge_delta.v1"
        );
        assert_eq!(
            arda_learning::KNOWLEDGE_DELTA_RELATIVE_PATH,
            "data/athena/knowledge_deltas.jsonl"
        );
    }
}
