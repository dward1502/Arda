// sigil: REPAIR
//
// In-memory digest index. Phase 3 of ATHENA. Builds a flat searchable
// snapshot of every book (shallow + latest deep) so `query()` doesn't
// re-scan the books directory on every call.
//
// Cache invalidation is books-dir mtime + a soft TTL. Any ingest or
// deep_analyze write updates the directory mtime; the next query
// notices and rebuilds. Background refresh is not (yet) implemented —
// rebuild happens lazily on the first stale read.

use annunimas_core::error::Result;
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use super::{athena_error, ExtractedKnowledge};

const INDEX_TTL_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub(super) struct IndexEntry {
    pub source_id: String,
    pub book_ref: String,
    pub title: String,
    pub summary: String,
    pub relevance_tags: Vec<String>,
    pub concepts: Vec<String>,
    pub novel_ideas: Vec<String>,
    pub patterns: Vec<String>,
    pub applicability: String,
    pub integration_hooks: Vec<String>,
    pub comparable_systems: Vec<String>,
    pub deep_summary: String,
    pub extraction_status: String,
    pub digest_status: String,
    pub confidence_self_report: f64,
}

#[derive(Debug)]
pub(super) struct DigestIndex {
    pub entries: Vec<IndexEntry>,
    pub built_at: Instant,
    pub source_dir_mtime: Option<SystemTime>,
}

impl DigestIndex {
    pub(super) fn is_fresh(&self, current_mtime: Option<SystemTime>) -> bool {
        if self.built_at.elapsed() > Duration::from_secs(INDEX_TTL_SECS) {
            return false;
        }
        self.source_dir_mtime == current_mtime
    }
}

pub(super) fn books_dir_mtime(books_dir: &Path) -> Option<SystemTime> {
    fs::metadata(books_dir).ok()?.modified().ok()
}

pub(super) fn rebuild_index(
    books_dir: &Path,
    book_ref_fn: impl Fn(&str) -> String,
) -> Result<DigestIndex> {
    let mut entries = Vec::new();
    let mtime = books_dir_mtime(books_dir);
    let dir_iter = match fs::read_dir(books_dir) {
        Ok(it) => it,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(DigestIndex {
                entries,
                built_at: Instant::now(),
                source_dir_mtime: mtime,
            });
        }
        Err(err) => return Err(athena_error(format!("read books_dir: {err}"))),
    };
    for entry in dir_iter {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
            continue;
        }
        let Some(source_id) = path
            .file_stem()
            .and_then(|v| v.to_str())
            .map(|s| s.to_string())
        else {
            continue;
        };
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let mut shallow_data: Option<ShallowSlice> = None;
        let mut latest_deep: Option<DeepSlice> = None;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: Value = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(_) => continue,
            };
            match value.get("stage").and_then(Value::as_str) {
                Some("shallow") => {
                    shallow_data = parse_shallow(&value);
                }
                Some("deep") => {
                    latest_deep = parse_deep(&value);
                }
                _ => {}
            }
        }
        let Some(shallow) = shallow_data else {
            continue;
        };
        let (
            concepts,
            novel_ideas,
            patterns,
            applicability,
            integration_hooks,
            comparable,
            confidence,
        ) = latest_deep
            .as_ref()
            .and_then(|d| d.extracted.as_ref())
            .map(|k| {
                (
                    k.concepts.clone(),
                    k.novel_ideas.clone(),
                    k.patterns.clone(),
                    k.applicability_to_annunimas.clone(),
                    k.integration_hooks.clone(),
                    k.comparable_systems.clone(),
                    k.confidence_self_report,
                )
            })
            .unwrap_or_default();
        entries.push(IndexEntry {
            source_id: source_id.clone(),
            book_ref: book_ref_fn(&source_id),
            title: shallow.title,
            summary: shallow.summary,
            relevance_tags: shallow.relevance_tags,
            concepts,
            novel_ideas,
            patterns,
            applicability,
            integration_hooks,
            comparable_systems: comparable,
            deep_summary: latest_deep
                .as_ref()
                .map(|d| d.full_summary.clone())
                .unwrap_or_default(),
            extraction_status: latest_deep
                .as_ref()
                .map(|d| d.extraction_status.clone())
                .unwrap_or_default(),
            digest_status: if latest_deep.is_some() {
                "deep".to_string()
            } else {
                "shallow".to_string()
            },
            confidence_self_report: confidence,
        });
    }
    Ok(DigestIndex {
        entries,
        built_at: Instant::now(),
        source_dir_mtime: mtime,
    })
}

struct ShallowSlice {
    title: String,
    summary: String,
    relevance_tags: Vec<String>,
}

struct DeepSlice {
    full_summary: String,
    extraction_status: String,
    extracted: Option<ExtractedKnowledge>,
}

fn parse_shallow(value: &Value) -> Option<ShallowSlice> {
    let data = value.get("data")?;
    Some(ShallowSlice {
        title: data
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        summary: data
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        relevance_tags: data
            .get("relevance_tags")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

fn parse_deep(value: &Value) -> Option<DeepSlice> {
    let data = value.get("data")?;
    let extracted = data
        .get("extracted_knowledge")
        .and_then(|v| serde_json::from_value::<ExtractedKnowledge>(v.clone()).ok());
    Some(DeepSlice {
        full_summary: data
            .get("full_summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        extraction_status: data
            .get("extraction_status")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        extracted,
    })
}

pub(super) fn score_entry(query_terms: &[String], entry: &IndexEntry) -> ScoreHit {
    let mut score = 0.0_f64;
    let mut concepts_hit: Vec<String> = Vec::new();
    let title_lower = entry.title.to_ascii_lowercase();
    let summary_lower = entry.summary.to_ascii_lowercase();
    let deep_summary_lower = entry.deep_summary.to_ascii_lowercase();
    let applicability_lower = entry.applicability.to_ascii_lowercase();

    for term in query_terms {
        if term.is_empty() {
            continue;
        }
        if title_lower.contains(term) {
            score += 2.5;
        }
        if summary_lower.contains(term) {
            score += 1.5;
        }
        if applicability_lower.contains(term) {
            score += 1.5;
        }
        if deep_summary_lower.contains(term) {
            score += 1.2;
        }
        for tag in &entry.relevance_tags {
            if tag.to_ascii_lowercase().contains(term) {
                score += 1.0;
            }
        }
        for concept in &entry.concepts {
            if concept.to_ascii_lowercase().contains(term) {
                score += 2.0;
                if !concepts_hit.iter().any(|c| c == concept) {
                    concepts_hit.push(concept.clone());
                }
            }
        }
        for idea in &entry.novel_ideas {
            if idea.to_ascii_lowercase().contains(term) {
                score += 1.8;
            }
        }
        for pattern in &entry.patterns {
            if pattern.to_ascii_lowercase().contains(term) {
                score += 1.4;
            }
        }
        for hook in &entry.integration_hooks {
            if hook.to_ascii_lowercase().contains(term) {
                score += 1.2;
            }
        }
        for comp in &entry.comparable_systems {
            if comp.to_ascii_lowercase().contains(term) {
                score += 0.8;
            }
        }
    }

    // small deep-confidence bonus so a deep+high-confidence match beats a
    // shallow-only match at equal text-score. Only applied when the text
    // score is already positive, otherwise no-hit sources would surface.
    if score > 0.0 && entry.digest_status == "deep" && entry.confidence_self_report > 0.5 {
        score += 0.5 * entry.confidence_self_report;
    }

    ScoreHit {
        score,
        concepts_hit,
    }
}

pub(super) struct ScoreHit {
    pub score: f64,
    pub concepts_hit: Vec<String>,
}

pub(super) fn tokenize_query(query: &str) -> Vec<String> {
    let lowered = query.to_ascii_lowercase();
    let cleaned: String = lowered
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '/' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let mut tokens: Vec<String> = cleaned
        .split_whitespace()
        .filter(|s| s.len() > 1 && !STOPWORDS.contains(s))
        .map(|s| s.to_string())
        .collect();
    // Always include the full original query as a token so multi-word
    // exact phrases still match (e.g. "agent context protocol").
    let full = query.trim().to_ascii_lowercase();
    if !full.is_empty() && !tokens.iter().any(|t| t == &full) {
        tokens.push(full);
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

const STOPWORDS: &[&str] = &[
    "a", "an", "the", "and", "or", "of", "for", "to", "in", "on", "with", "by", "from", "is", "are",
];

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(
        source_id: &str,
        title: &str,
        summary: &str,
        tags: &[&str],
        concepts: &[&str],
        deep_summary: &str,
    ) -> IndexEntry {
        IndexEntry {
            source_id: source_id.into(),
            book_ref: format!("books/{source_id}.jsonl"),
            title: title.into(),
            summary: summary.into(),
            relevance_tags: tags.iter().map(|s| s.to_string()).collect(),
            concepts: concepts.iter().map(|s| s.to_string()).collect(),
            novel_ideas: vec![],
            patterns: vec![],
            applicability: String::new(),
            integration_hooks: vec![],
            comparable_systems: vec![],
            deep_summary: deep_summary.into(),
            extraction_status: "llm_extraction_complete".into(),
            digest_status: "deep".into(),
            confidence_self_report: 0.8,
        }
    }

    #[test]
    fn tokenize_keeps_phrase_and_strips_stopwords() {
        let toks = tokenize_query("the agent context protocol");
        assert!(toks.contains(&"agent".to_string()));
        assert!(toks.contains(&"context".to_string()));
        assert!(toks.contains(&"protocol".to_string()));
        assert!(toks.contains(&"the agent context protocol".to_string()));
        assert!(!toks.contains(&"the".to_string()));
    }

    #[test]
    fn score_matches_concept_via_deep_field() {
        let e = entry(
            "src_a",
            "udapy/rust-agentic-skills",
            "agentic skills focused on rust",
            &["rust", "agentic-skills"],
            &["Agent Context Protocol (ACP)"],
            "ACP routing and RPI methodology",
        );
        let hit = score_entry(&tokenize_query("agent context protocol"), &e);
        assert!(hit.score > 5.0, "score was {}", hit.score);
        assert!(hit.concepts_hit.iter().any(|c| c.contains("ACP")));
    }

    #[test]
    fn empty_query_scores_zero() {
        let e = entry("s", "t", "u", &[], &[], "");
        let hit = score_entry(&tokenize_query(""), &e);
        assert_eq!(hit.score, 0.0);
    }

    #[test]
    fn deep_high_confidence_outranks_shallow_at_same_text_score() {
        let mut deep = entry("d", "x", "", &["rust"], &[], "");
        deep.digest_status = "deep".into();
        deep.confidence_self_report = 0.9;
        let mut shallow = entry("s", "x", "", &["rust"], &[], "");
        shallow.digest_status = "shallow".into();
        shallow.confidence_self_report = 0.0;
        let d_score = score_entry(&tokenize_query("rust"), &deep).score;
        let s_score = score_entry(&tokenize_query("rust"), &shallow).score;
        assert!(d_score > s_score, "deep={} shallow={}", d_score, s_score);
    }
}
