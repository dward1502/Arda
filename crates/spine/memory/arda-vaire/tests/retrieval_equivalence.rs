use arda_vaire::retrieval_eval::{
    evaluate_adapter, LexicalBaselineAdapter, RetrievalAdapter, RetrievalDataset,
    RETRIEVAL_EVAL_SCHEMA_VERSION,
};

fn dataset() -> RetrievalDataset {
    serde_json::from_str(include_str!("fixtures/retrieval_equivalence_v1.json"))
        .expect("retrieval equivalence fixture")
}

#[test]
fn equivalent_dataset_has_stable_schema_and_unique_ids() {
    let dataset = dataset();
    assert_eq!(dataset.schema_version, RETRIEVAL_EVAL_SCHEMA_VERSION);
    assert!(dataset.validate().is_ok());
    assert_eq!(dataset.documents.len(), 8);
    assert_eq!(dataset.queries.len(), 8);
}

#[test]
fn lexical_baseline_meets_equivalent_dataset_gate() {
    let report = evaluate_adapter(&LexicalBaselineAdapter, &dataset(), 3)
        .expect("evaluate lexical baseline");

    assert_eq!(report.adapter, "mnemosyne-lexical-v1");
    assert_eq!(report.query_count, 8);
    assert_eq!(report.hit_at_1, 1.0);
    assert_eq!(report.recall_at_k, 1.0);
    assert_eq!(report.mean_reciprocal_rank, 1.0);
}

struct ReverseAdapter;

impl RetrievalAdapter for ReverseAdapter {
    fn name(&self) -> &str {
        "external-reverse-test"
    }

    fn retrieve(
        &self,
        _query: &str,
        documents: &[arda_vaire::retrieval_eval::RetrievalDocument],
        limit: usize,
    ) -> Vec<String> {
        documents
            .iter()
            .rev()
            .take(limit)
            .map(|document| document.id.clone())
            .collect()
    }
}

#[test]
fn public_adapter_contract_evaluates_external_rankers_on_same_dataset() {
    let report =
        evaluate_adapter(&ReverseAdapter, &dataset(), 3).expect("evaluate external adapter");

    assert_eq!(report.adapter, "external-reverse-test");
    assert!(report.hit_at_1 < 1.0);
    assert!(report.recall_at_k < 1.0);
}
