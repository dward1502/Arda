//! Evaluation adapters only: retrieval ownership stays with the production store.
use super::RetrievalBenchmarkFixture;
use crate::ingest::QueryMatch;
use arda_vaire::retrieval_eval::{
    RetrievalAdapter, RetrievalDataset, RetrievalDocument, RetrievalQuery,
    RETRIEVAL_EVAL_SCHEMA_VERSION,
};
use std::collections::HashMap;

pub(super) fn dataset(fixture: &RetrievalBenchmarkFixture) -> RetrievalDataset {
    RetrievalDataset {
        schema_version: RETRIEVAL_EVAL_SCHEMA_VERSION.into(),
        documents: fixture
            .documents
            .iter()
            .map(|doc| RetrievalDocument {
                id: doc.id.clone(),
                content: doc.content.clone(),
                tags: vec![],
                scope: String::new(),
            })
            .collect(),
        queries: fixture
            .queries
            .iter()
            .enumerate()
            .map(|(i, query)| RetrievalQuery {
                id: format!("q{i}"),
                query: query.query.clone(),
                relevant_document_ids: query.expected_document_ids.clone(),
            })
            .collect(),
    }
}

pub(super) struct ProductionRankings(pub HashMap<String, Vec<String>>);
impl RetrievalAdapter for ProductionRankings {
    fn name(&self) -> &str {
        "varda-production-field-weighted-bm25"
    }
    fn retrieve(&self, query: &str, _: &[RetrievalDocument], limit: usize) -> Vec<String> {
        self.0
            .get(query)
            .into_iter()
            .flatten()
            .take(limit)
            .cloned()
            .collect()
    }
}

// This benchmark ingests shallow text only. Validate byte spans against the
// actual returned indexed field, not merely a nonempty citation label. This
// proves extractive provenance, not semantic correctness of generated answers.
pub(super) fn citation_spans_valid(item: &QueryMatch) -> bool {
    !item.citations.is_empty()
        && item.citations.iter().all(|citation| {
            let field = match citation.span.field.as_str() {
                "title" => Some(item.title.as_str()),
                "summary" => Some(item.summary.as_str()),
                field => field
                    .strip_prefix("relevance_tags[")
                    .and_then(|s| s.strip_suffix(']'))
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|i| item.relevance_tags.get(i))
                    .map(String::as_str),
            };
            citation.source_id == item.source_id
                && citation.doc_id == item.book_ref
                && !citation.doc_id.is_empty()
                && !citation.span.text.is_empty()
                && field.and_then(|s| s.get(citation.span.start..citation.span.end))
                    == Some(citation.span.text.as_str())
        })
}
