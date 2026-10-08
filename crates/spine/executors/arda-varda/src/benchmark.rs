mod comparison;
use crate::ingest::{AthenaStore, QueryCitation};
use arda_core::error::{ArdaError, Result};
use arda_vaire::retrieval_eval::{
    evaluate_adapter, LexicalBaselineAdapter, RetrievalAdapter, RetrievalEvaluation,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::Path, time::Instant};

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalBenchmarkFixture {
    pub schema_version: String,
    pub provenance: RetrievalBenchmarkProvenance,
    pub documents: Vec<RetrievalBenchmarkDocument>,
    pub queries: Vec<RetrievalBenchmarkQuery>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalBenchmarkProvenance {
    pub owner: String,
    pub purpose: String,
    pub created_at_utc: String,
    pub license: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalBenchmarkDocument {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RetrievalBenchmarkQuery {
    pub query: String,
    pub expected_document_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalBenchmarkReport {
    pub schema_version: &'static str,
    pub fixture_schema_version: String,
    pub fixture_path: String,
    pub queries_total: usize,
    pub fixture_sha256: String,
    pub lexical: RetrievalEvaluation,
    pub bm25: RetrievalEvaluation,
    pub recall_at_1: f64,
    pub citation_correctness_rate: f64,
    pub shallow_only_rate: f64,
    pub classification_cache_profile: crate::ingest::ClassificationCacheProfile,
    pub latency_micros_total: u128,
    pub latency_micros_mean: u128,
    pub cases: Vec<RetrievalBenchmarkCase>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RetrievalBenchmarkCase {
    pub query: String,
    pub expected_document_ids: Vec<String>,
    pub matched_document_id: Option<String>,
    pub matched_source_id: Option<String>,
    pub score: Option<f64>,
    pub recall_at_1: bool,
    pub citation_correct: bool,
    pub shallow_only: bool,
    pub latency_micros: u128,
    pub ranked_document_ids: Vec<String>,
    pub lexical_document_ids: Vec<String>,
    pub citations: Vec<QueryCitation>,
    pub citation_spans_valid: bool,
}

/// The caller must exclusively own a fresh isolated store for this run.
/// Reusing a populated or warmed store is rejected before any corpus writes.
pub fn run_retrieval_benchmark(
    fixture_path: impl AsRef<Path>,
    store: &AthenaStore,
) -> Result<RetrievalBenchmarkReport> {
    let fixture_path = fixture_path.as_ref();
    let bytes = std::fs::read(fixture_path).map_err(benchmark_error)?;
    let fixture: RetrievalBenchmarkFixture =
        serde_json::from_slice(&bytes).map_err(benchmark_error)?;
    validate_fixture(&fixture)?;
    store.require_pristine_benchmark_store()?;

    let mut source_to_document = HashMap::with_capacity(fixture.documents.len());
    for document in &fixture.documents {
        let record = store.ingest(&document.content, "retrieval-benchmark", "benchmark corpus")?;
        source_to_document.insert(record.id, document.id.clone());
    }

    let dataset = comparison::dataset(&fixture);
    let mut rankings = comparison::ProductionRankings(HashMap::new());
    let mut cases = Vec::with_capacity(fixture.queries.len());
    for judgment in &fixture.queries {
        let started = Instant::now();
        let response = store.query(&judgment.query, 3)?;
        let elapsed = started.elapsed().as_micros();
        let matched = response.matches.first();
        let matched_document_id = matched.and_then(|item| source_to_document.get(&item.source_id));
        let recall_at_1 =
            matched_document_id.is_some_and(|id| judgment.expected_document_ids.contains(id));
        let citation_correct = matched.is_some_and(comparison::citation_spans_valid);
        let ranked_document_ids = response
            .matches
            .iter()
            .map(|item| {
                source_to_document
                    .get(&item.source_id)
                    .cloned()
                    .ok_or_else(|| benchmark_error("store contains a source outside the fixture"))
            })
            .collect::<Result<Vec<_>>>()?;
        rankings
            .0
            .insert(judgment.query.clone(), ranked_document_ids.clone());
        cases.push(RetrievalBenchmarkCase {
            query: judgment.query.clone(),
            expected_document_ids: judgment.expected_document_ids.clone(),
            matched_document_id: matched_document_id.cloned(),
            matched_source_id: matched.map(|item| item.source_id.clone()),
            score: matched.map(|item| item.score),
            recall_at_1,
            citation_correct,
            shallow_only: matched.is_some_and(|item| item.shallow_only),
            latency_micros: elapsed,
            ranked_document_ids,
            lexical_document_ids: LexicalBaselineAdapter.retrieve(
                &judgment.query,
                &dataset.documents,
                3,
            ),
            citations: matched
                .map(|item| item.citations.clone())
                .unwrap_or_default(),
            citation_spans_valid: matched.is_some_and(comparison::citation_spans_valid),
        });
    }

    let queries_total = cases.len();
    let denominator = queries_total.max(1) as f64;
    let latency_micros_total = cases.iter().map(|case| case.latency_micros).sum();
    Ok(RetrievalBenchmarkReport {
        schema_version: "arda.varda.retrieval_benchmark_report.v2",
        fixture_schema_version: fixture.schema_version,
        fixture_path: fixture_path.display().to_string(),
        queries_total,
        fixture_sha256: format!("{:x}", Sha256::digest(&bytes)),
        lexical: evaluate_adapter(&LexicalBaselineAdapter, &dataset, 3).map_err(benchmark_error)?,
        bm25: evaluate_adapter(&rankings, &dataset, 3).map_err(benchmark_error)?,
        recall_at_1: cases
            .iter()
            .map(|case| {
                let relevant = case
                    .expected_document_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>();
                f64::from(case.recall_at_1) / relevant.len() as f64
            })
            .sum::<f64>()
            / denominator,
        citation_correctness_rate: cases.iter().filter(|case| case.citation_correct).count() as f64
            / denominator,
        shallow_only_rate: cases.iter().filter(|case| case.shallow_only).count() as f64
            / denominator,
        classification_cache_profile: AthenaStore::profile_classification_cache(10_000),
        latency_micros_total,
        latency_micros_mean: latency_micros_total / queries_total.max(1) as u128,
        cases,
    })
}

fn validate_fixture(fixture: &RetrievalBenchmarkFixture) -> Result<()> {
    if fixture.schema_version != "arda.varda.retrieval_benchmark_fixture.v1" {
        return Err(benchmark_error("unsupported benchmark fixture schema"));
    }
    if fixture.documents.is_empty() || fixture.queries.is_empty() {
        return Err(benchmark_error(
            "benchmark fixture requires documents and queries",
        ));
    }
    let document_ids = fixture
        .documents
        .iter()
        .map(|document| document.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let unique_content = fixture
        .documents
        .iter()
        .map(|document| AthenaStore::canonical_source_identity(&document.content).1)
        .collect::<std::collections::HashSet<_>>();
    if document_ids.len() != fixture.documents.len()
        || unique_content.len() != fixture.documents.len()
    {
        return Err(benchmark_error("duplicate document identity or content"));
    }
    if fixture
        .documents
        .iter()
        .any(|document| document.id.trim().is_empty() || document.content.trim().is_empty())
        || fixture.queries.iter().any(|query| {
            query.query.trim().is_empty()
                || query.expected_document_ids.is_empty()
                || query
                    .expected_document_ids
                    .iter()
                    .any(|id| !document_ids.contains(id.as_str()))
        })
    {
        return Err(benchmark_error(
            "benchmark fixture contains invalid judgments",
        ));
    }
    Ok(())
}

fn benchmark_error(error: impl std::fmt::Display) -> ArdaError {
    ArdaError::Agent {
        agent: "athena".to_string(),
        message: format!("retrieval benchmark: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn checked_in_fixture_produces_complete_recall_and_correct_citations() {
        let dir = tempdir().expect("tempdir");
        let store = AthenaStore::new_isolated(dir.path()).expect("store");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/retrieval_benchmark_v1.json");

        let report = run_retrieval_benchmark(&fixture, &store).expect("benchmark");

        assert_eq!(report.queries_total, 3);
        assert_eq!(report.recall_at_1, 1.0);
        assert_eq!(report.citation_correctness_rate, 1.0);
        assert_eq!(report.shallow_only_rate, 1.0);
        assert_eq!(report.cases.len(), 3);
        assert!(report.cases.iter().all(|case| case.recall_at_1));
        assert!(report.cases.iter().all(|case| case.citation_correct));
    }

    #[test]
    fn malformed_fixture_is_rejected() {
        let dir = tempdir().expect("tempdir");
        let fixture = dir.path().join("fixture.json");
        std::fs::write(&fixture, "not json").expect("write fixture");
        let store = AthenaStore::new_isolated(dir.path().join("store")).expect("store");

        assert!(run_retrieval_benchmark(&fixture, &store).is_err());
    }
}
