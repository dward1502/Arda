//! Equivalent-dataset retrieval evaluation for Mnemosyne and external rankers.
//!
//! Adapters receive the same immutable corpus and query text. This keeps
//! lexical, BM25, vector, and hybrid candidates comparable before a production
//! index is selected.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const RETRIEVAL_EVAL_SCHEMA_VERSION: &str = "arda.mnemosyne.retrieval-eval.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalDocument {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalQuery {
    pub id: String,
    pub query: String,
    pub relevant_document_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalDataset {
    pub schema_version: String,
    pub documents: Vec<RetrievalDocument>,
    pub queries: Vec<RetrievalQuery>,
}

impl RetrievalDataset {
    pub fn validate(&self) -> Result<(), RetrievalEvaluationError> {
        if self.schema_version != RETRIEVAL_EVAL_SCHEMA_VERSION {
            return Err(RetrievalEvaluationError::UnsupportedSchema(
                self.schema_version.clone(),
            ));
        }
        if self.documents.is_empty() || self.queries.is_empty() {
            return Err(RetrievalEvaluationError::EmptyDataset);
        }

        let mut document_ids = HashSet::new();
        for document in &self.documents {
            if document.id.is_empty() || !document_ids.insert(document.id.as_str()) {
                return Err(RetrievalEvaluationError::DuplicateOrEmptyId(
                    document.id.clone(),
                ));
            }
        }

        let mut query_ids = HashSet::new();
        for query in &self.queries {
            if query.id.is_empty() || !query_ids.insert(query.id.as_str()) {
                return Err(RetrievalEvaluationError::DuplicateOrEmptyId(
                    query.id.clone(),
                ));
            }
            if query.relevant_document_ids.is_empty()
                || query
                    .relevant_document_ids
                    .iter()
                    .any(|id| !document_ids.contains(id.as_str()))
            {
                return Err(RetrievalEvaluationError::InvalidRelevanceSet(
                    query.id.clone(),
                ));
            }
        }
        Ok(())
    }
}

pub trait RetrievalAdapter {
    fn name(&self) -> &str;

    fn retrieve(&self, query: &str, documents: &[RetrievalDocument], limit: usize) -> Vec<String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LexicalBaselineAdapter;

impl RetrievalAdapter for LexicalBaselineAdapter {
    fn name(&self) -> &str {
        "mnemosyne-lexical-v1"
    }

    fn retrieve(&self, query: &str, documents: &[RetrievalDocument], limit: usize) -> Vec<String> {
        let terms = terms(query);
        let mut ranked = documents
            .iter()
            .map(|document| {
                let haystack = format!(
                    "{} {} {}",
                    document.content.to_ascii_lowercase(),
                    document.tags.join(" ").to_ascii_lowercase(),
                    document.scope.to_ascii_lowercase()
                );
                let matched = terms
                    .iter()
                    .filter(|term| haystack.contains(term.as_str()))
                    .count();
                (document.id.clone(), matched)
            })
            .collect::<Vec<_>>();
        ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        ranked
            .into_iter()
            .take(limit.max(1))
            .map(|(id, _)| id)
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalEvaluation {
    pub schema_version: String,
    pub dataset_schema_version: String,
    pub adapter: String,
    pub query_count: usize,
    pub limit: usize,
    pub hit_at_1: f64,
    pub recall_at_k: f64,
    pub mean_reciprocal_rank: f64,
}

pub fn evaluate_adapter(
    adapter: &dyn RetrievalAdapter,
    dataset: &RetrievalDataset,
    limit: usize,
) -> Result<RetrievalEvaluation, RetrievalEvaluationError> {
    dataset.validate()?;
    let limit = limit.max(1);
    let mut hit_at_1 = 0usize;
    let mut recall_sum = 0.0;
    let mut reciprocal_rank_sum = 0.0;

    for query in &dataset.queries {
        let relevant = query
            .relevant_document_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let ranked = adapter.retrieve(&query.query, &dataset.documents, limit);
        if ranked
            .first()
            .is_some_and(|document_id| relevant.contains(document_id.as_str()))
        {
            hit_at_1 += 1;
        }
        let relevant_retrieved = ranked
            .iter()
            .filter(|document_id| relevant.contains(document_id.as_str()))
            .count();
        recall_sum += relevant_retrieved as f64 / relevant.len() as f64;
        if let Some(rank) = ranked
            .iter()
            .position(|document_id| relevant.contains(document_id.as_str()))
        {
            reciprocal_rank_sum += 1.0 / (rank + 1) as f64;
        }
    }

    let query_count = dataset.queries.len();
    Ok(RetrievalEvaluation {
        schema_version: "arda.mnemosyne.retrieval-evaluation.v1".to_owned(),
        dataset_schema_version: dataset.schema_version.clone(),
        adapter: adapter.name().to_owned(),
        query_count,
        limit,
        hit_at_1: hit_at_1 as f64 / query_count as f64,
        recall_at_k: recall_sum / query_count as f64,
        mean_reciprocal_rank: reciprocal_rank_sum / query_count as f64,
    })
}

#[derive(Debug, thiserror::Error)]
pub enum RetrievalEvaluationError {
    #[error("unsupported retrieval dataset schema: {0}")]
    UnsupportedSchema(String),
    #[error("retrieval dataset must contain documents and queries")]
    EmptyDataset,
    #[error("retrieval dataset contains a duplicate or empty id: {0}")]
    DuplicateOrEmptyId(String),
    #[error("query has an empty or unknown relevance set: {0}")]
    InvalidRelevanceSet(String),
}

fn terms(value: &str) -> Vec<String> {
    value
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.len() >= 3)
        .map(str::to_ascii_lowercase)
        .collect()
}
