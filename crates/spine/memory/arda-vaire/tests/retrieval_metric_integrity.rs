use arda_vaire::retrieval_eval::{evaluate_adapter, RetrievalAdapter, RetrievalDataset};

struct Ranked(Vec<String>);
impl RetrievalAdapter for Ranked {
    fn name(&self) -> &str {
        "adversarial-ranking"
    }
    fn retrieve(
        &self,
        _: &str,
        _: &[arda_vaire::retrieval_eval::RetrievalDocument],
        _: usize,
    ) -> Vec<String> {
        self.0.clone()
    }
}
fn dataset() -> RetrievalDataset {
    serde_json::from_value(serde_json::json!({
        "schema_version": "arda.mnemosyne.retrieval-eval.v1",
        "documents": [
            {"id":"a", "content":"alpha", "scope":"fixture"},
            {"id":"b", "content":"beta", "scope":"fixture"}
        ],
        "queries": [{"id":"q", "query":"alpha", "relevant_document_ids":["a"]}]
    }))
    .unwrap()
}
#[test]
fn duplicate_results_cannot_inflate_recall() {
    let report = evaluate_adapter(&Ranked(vec!["a".into(), "a".into()]), &dataset(), 2).unwrap();
    assert_eq!(report.recall_at_k, 1.0);
}
#[test]
fn evaluator_enforces_requested_cutoff() {
    let report = evaluate_adapter(&Ranked(vec!["b".into(), "a".into()]), &dataset(), 1).unwrap();
    assert_eq!(report.recall_at_k, 0.0);
    assert_eq!(report.mean_reciprocal_rank, 0.0);
}
