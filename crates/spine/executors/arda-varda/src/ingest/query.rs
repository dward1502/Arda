// sigil: REPAIR
//
// Local corpus query path. Phase 3: instead of re-scanning the books
// directory on every call, consult the in-memory `DigestIndex` which
// caches shallow + deep entry snippets and is invalidated by books-dir
// mtime or TTL. Scoring now searches the extracted-knowledge fields
// (concepts, novel_ideas, patterns, applicability, integration_hooks)
// produced by the Phase 2 LLM extractor.

use arda_core::error::Result;

use super::index::{score_entry, tokenize_query};
use super::{athena_error, AthenaStore, QueryMatch, QueryResponse};

impl AthenaStore {
    pub fn query(&self, query: &str, limit: usize) -> Result<QueryResponse> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(athena_error("query term cannot be empty"));
        }
        let tokens = tokenize_query(trimmed);

        let mut matches = self.with_digest_index(|idx| {
            let mut out: Vec<QueryMatch> = Vec::new();
            for entry in &idx.entries {
                let hit = score_entry(&tokens, entry);
                if hit.score <= 0.0 {
                    continue;
                }
                out.push(QueryMatch {
                    source_id: entry.source_id.clone(),
                    book_ref: entry.book_ref.clone(),
                    score: hit.score,
                    digest_status: entry.digest_status.clone(),
                    title: entry.title.clone(),
                    summary: entry.summary.clone(),
                    relevance_tags: entry.relevance_tags.clone(),
                    concepts_hit: hit.concepts_hit,
                    extraction_status: entry.extraction_status.clone(),
                    confidence_self_report: entry.confidence_self_report,
                });
            }
            out
        })?;

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
}
