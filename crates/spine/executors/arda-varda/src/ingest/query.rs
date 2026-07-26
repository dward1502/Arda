// sigil: REPAIR
//
// Local corpus query path. Phase 3: instead of re-scanning the books
// directory on every call, consult the in-memory `DigestIndex` which
// caches shallow + deep entry snippets and is invalidated by books-dir
// mtime or TTL. Scoring now searches the extracted-knowledge fields
// (concepts, novel_ideas, patterns, applicability, integration_hooks)
// produced by the Phase 2 LLM extractor.

use arda_core::error::Result;

use super::index::{score_entry_in_corpus, tokenize_query, IndexEntry, ScoreHit};
use super::{athena_error, AthenaStore, QueryCitation, QueryMatch, QueryResponse};

impl AthenaStore {
    pub fn query(&self, query: &str, limit: usize) -> Result<QueryResponse> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(athena_error("query term cannot be empty"));
        }
        let tokens = tokenize_query(trimmed);

        let entries = self.with_digest_index(|idx| idx.entries.clone())?;
        let mut matches = entries
            .iter()
            .filter_map(|entry| {
                let hit = score_entry_in_corpus(&tokens, entry, &entries);
                (hit.score > 0.0).then(|| build_query_match(entry, hit))
            })
            .collect::<Vec<_>>();

        matches.sort_by(|a, b| b.score.total_cmp(&a.score));
        if matches.len() > limit {
            matches.truncate(limit);
        }

        let total_matches = matches.len();
        Ok(QueryResponse {
            query: query.to_string(),
            total_matches,
            matches,
            suggestion: if total_matches == 0 {
                Some(
                    "No local corpus match. Recommend ingest of relevant source before proceeding."
                        .to_string(),
                )
            } else {
                None
            },
        })
    }

    pub(crate) fn stream_query_matches<F>(
        &self,
        query: &str,
        limit: usize,
        mut emit: F,
    ) -> Result<usize>
    where
        F: FnMut(QueryMatch) -> bool,
    {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(athena_error("query term cannot be empty"));
        }
        if limit == 0 {
            return Ok(0);
        }
        let tokens = tokenize_query(trimmed);
        let entries = self.with_digest_index(|idx| idx.entries.clone())?;
        let mut emitted = 0;
        for entry in &entries {
            let hit = score_entry_in_corpus(&tokens, entry, &entries);
            if hit.score <= 0.0 {
                continue;
            }
            if !emit(build_query_match(entry, hit)) {
                break;
            }
            emitted += 1;
            if emitted >= limit {
                break;
            }
        }
        Ok(emitted)
    }
}

fn build_query_match(entry: &IndexEntry, hit: ScoreHit) -> QueryMatch {
    QueryMatch {
        source_id: entry.source_id.clone(),
        book_ref: entry.book_ref.clone(),
        score: hit.score,
        digest_status: entry.digest_status.clone(),
        shallow_only: !entry.has_extracted_knowledge,
        title: entry.title.clone(),
        summary: entry.summary.clone(),
        relevance_tags: entry.relevance_tags.clone(),
        concepts_hit: hit.concepts_hit,
        extraction_status: entry.extraction_status.clone(),
        confidence_self_report: entry.confidence_self_report,
        citations: hit
            .citation_spans
            .into_iter()
            .map(|span| QueryCitation {
                source_id: entry.source_id.clone(),
                doc_id: entry.book_ref.clone(),
                span,
            })
            .collect(),
    }
}
