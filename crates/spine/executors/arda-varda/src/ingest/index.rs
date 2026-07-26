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

use arda_core::error::Result;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};

use super::{athena_error, CitationSpan, ExtractedKnowledge};

const INDEX_TTL_SECS: u64 = 300;
const INDEX_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub triad_passed: bool,
    pub policy_readiness: String,
    pub has_extracted_knowledge: bool,
}

#[derive(Debug)]
pub(super) struct DigestIndex {
    pub entries: Vec<IndexEntry>,
    pub built_at: Instant,
    pub source_dir_mtime: Option<SystemTime>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedDigestIndex {
    schema_version: u32,
    source_revision: Option<u64>,
    entries: Vec<IndexEntry>,
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
    let mut newest = fs::metadata(books_dir).ok()?.modified().ok();
    for entry in fs::read_dir(books_dir).ok()?.flatten() {
        let modified = entry
            .metadata()
            .ok()
            .and_then(|metadata| metadata.modified().ok());
        if modified.is_some_and(|candidate| newest.is_none_or(|current| candidate > current)) {
            newest = modified;
        }
    }
    newest
}

fn source_revision(mtime: Option<SystemTime>) -> Option<u64> {
    mtime?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_nanos()).ok())
}

pub(super) fn load_index(index_path: &Path, books_dir: &Path) -> Result<Option<DigestIndex>> {
    let Some(persisted) = load_persisted(index_path)? else {
        return Ok(None);
    };
    let mtime = books_dir_mtime(books_dir);
    if persisted.source_revision != source_revision(mtime) {
        return Ok(None);
    }
    Ok(Some(DigestIndex {
        entries: persisted.entries,
        built_at: Instant::now(),
        source_dir_mtime: mtime,
    }))
}

fn load_persisted(index_path: &Path) -> Result<Option<PersistedDigestIndex>> {
    let content = match fs::read_to_string(index_path) {
        Ok(content) => content,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(athena_error(format!("read digest index: {err}"))),
    };
    let persisted: PersistedDigestIndex = match serde_json::from_str(&content) {
        Ok(persisted) => persisted,
        Err(_) => return Ok(None),
    };
    if persisted.schema_version != INDEX_SCHEMA_VERSION {
        return Ok(None);
    }
    Ok(Some(persisted))
}

pub(super) fn persist_index(index_path: &Path, index: &DigestIndex) -> Result<()> {
    persist_payload(
        index_path,
        &PersistedDigestIndex {
            schema_version: INDEX_SCHEMA_VERSION,
            source_revision: source_revision(index.source_dir_mtime),
            entries: index.entries.clone(),
        },
    )
}

fn persist_payload(index_path: &Path, persisted: &PersistedDigestIndex) -> Result<()> {
    let parent = index_path
        .parent()
        .ok_or_else(|| athena_error("digest index path has no parent"))?;
    fs::create_dir_all(parent)?;
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(index_path.with_extension("lock"))?;
    lock.lock_exclusive()?;
    let temp_path = index_path.with_extension(format!("tmp-{}", std::process::id()));
    let result = (|| -> Result<()> {
        let bytes = serde_json::to_vec(persisted)
            .map_err(|err| athena_error(format!("serialize digest index: {err}")))?;
        let mut temp = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temp_path)?;
        temp.write_all(&bytes)?;
        temp.sync_all()?;
        fs::rename(&temp_path, index_path)?;
        Ok(())
    })();
    let _ = lock.unlock();
    if result.is_err() {
        let _ = fs::remove_file(temp_path);
    }
    result
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
        if let Some(index_entry) = build_index_entry(&path, &source_id, book_ref_fn(&source_id)) {
            entries.push(index_entry);
        }
    }
    Ok(DigestIndex {
        entries,
        built_at: Instant::now(),
        source_dir_mtime: mtime,
    })
}

fn build_index_entry(path: &Path, source_id: &str, book_ref: String) -> Option<IndexEntry> {
    let content = fs::read_to_string(path).ok()?;
    let mut shallow_data = None;
    let mut latest_deep = None;
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match value.get("stage").and_then(Value::as_str) {
            Some("shallow") => shallow_data = parse_shallow(&value),
            Some("deep") => latest_deep = parse_deep(&value),
            _ => {}
        }
    }
    let shallow = shallow_data?;
    let (
        concepts,
        novel_ideas,
        patterns,
        applicability,
        integration_hooks,
        comparable,
        confidence,
        triad_passed,
        policy_readiness,
    ) = latest_deep
        .as_ref()
        .and_then(|deep: &DeepSlice| deep.extracted.as_ref().map(|knowledge| (deep, knowledge)))
        .map(|(deep, knowledge)| {
            (
                knowledge.concepts.clone(),
                knowledge.novel_ideas.clone(),
                knowledge.patterns.clone(),
                knowledge.applicability_to_arda.clone(),
                knowledge.integration_hooks.clone(),
                knowledge.comparable_systems.clone(),
                knowledge.confidence_self_report,
                deep.triad_passed,
                deep.policy_readiness.clone(),
            )
        })
        .unwrap_or_default();
    Some(IndexEntry {
        source_id: source_id.to_string(),
        book_ref,
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
            .map(|deep| deep.full_summary.clone())
            .unwrap_or_default(),
        extraction_status: latest_deep
            .as_ref()
            .map(|deep| deep.extraction_status.clone())
            .unwrap_or_default(),
        digest_status: if latest_deep.is_some() {
            "deep".to_string()
        } else {
            "shallow".to_string()
        },
        confidence_self_report: confidence,
        triad_passed,
        policy_readiness,
        has_extracted_knowledge: latest_deep
            .as_ref()
            .is_some_and(|deep| deep.extracted.is_some()),
    })
}

pub(super) fn refresh_index_entry(
    index_path: &Path,
    books_dir: &Path,
    source_id: &str,
    book_ref: String,
) -> Result<DigestIndex> {
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(index_path.with_extension("lock"))?;
    lock.lock_exclusive()?;
    let result = (|| -> Result<DigestIndex> {
        let mut entries = load_persisted(index_path)?
            .map(|persisted| persisted.entries)
            .unwrap_or_default();
        entries.retain(|entry| entry.source_id != source_id);
        let book_path = books_dir.join(format!("{source_id}.jsonl"));
        if let Some(entry) = build_index_entry(&book_path, source_id, book_ref) {
            entries.push(entry);
        }
        entries.sort_by(|left, right| left.source_id.cmp(&right.source_id));
        let index = DigestIndex {
            entries,
            built_at: Instant::now(),
            source_dir_mtime: books_dir_mtime(books_dir),
        };
        persist_payload_unlocked(
            index_path,
            &PersistedDigestIndex {
                schema_version: INDEX_SCHEMA_VERSION,
                source_revision: source_revision(index.source_dir_mtime),
                entries: index.entries.clone(),
            },
        )?;
        Ok(index)
    })();
    let _ = lock.unlock();
    result
}

fn persist_payload_unlocked(index_path: &Path, persisted: &PersistedDigestIndex) -> Result<()> {
    let bytes = serde_json::to_vec(persisted)
        .map_err(|err| athena_error(format!("serialize digest index: {err}")))?;
    let temp_path = index_path.with_extension(format!("tmp-{}", std::process::id()));
    let mut temp = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temp_path)?;
    temp.write_all(&bytes)?;
    temp.sync_all()?;
    fs::rename(temp_path, index_path)?;
    Ok(())
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
    triad_passed: bool,
    policy_readiness: String,
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
        triad_passed: data
            .get("triad_analysis")
            .and_then(|triad| triad.get("passed"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        policy_readiness: data
            .get("policy_readiness")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
    })
}

#[cfg(test)]
pub(super) fn score_entry(query_terms: &[String], entry: &IndexEntry) -> ScoreHit {
    score_entry_in_corpus(query_terms, entry, std::slice::from_ref(entry))
}

pub(super) fn score_entry_in_corpus(
    query_terms: &[String],
    entry: &IndexEntry,
    corpus: &[IndexEntry],
) -> ScoreHit {
    let mut score = 0.0_f64;
    let mut concepts_hit: Vec<String> = Vec::new();
    let mut citation_spans = Vec::new();
    for term in query_terms {
        if term.is_empty() {
            continue;
        }
        let document_frequency = corpus
            .iter()
            .filter(|candidate| entry_contains_term(candidate, term))
            .count();
        let idf = bm25_idf(corpus.len().max(1), document_frequency);
        if let Some(field_score) = bm25_field_score(&entry.title, term, 2.5, idf) {
            score += field_score;
            push_citation_span(&mut citation_spans, "title", &entry.title, term);
        }
        if let Some(field_score) = bm25_field_score(&entry.summary, term, 1.5, idf) {
            score += field_score;
            push_citation_span(&mut citation_spans, "summary", &entry.summary, term);
        }
        if let Some(field_score) = bm25_field_score(&entry.applicability, term, 1.5, idf) {
            score += field_score;
            push_citation_span(
                &mut citation_spans,
                "applicability",
                &entry.applicability,
                term,
            );
        }
        if let Some(field_score) = bm25_field_score(&entry.deep_summary, term, 1.2, idf) {
            score += field_score;
            push_citation_span(
                &mut citation_spans,
                "deep_summary",
                &entry.deep_summary,
                term,
            );
        }
        for (index, tag) in entry.relevance_tags.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(tag, term, 1.0, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("relevance_tags[{index}]"),
                    tag,
                    term,
                );
            }
        }
        for (index, concept) in entry.concepts.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(concept, term, 2.0, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("concepts[{index}]"),
                    concept,
                    term,
                );
                if !concepts_hit.iter().any(|c| c == concept) {
                    concepts_hit.push(concept.clone());
                }
            }
        }
        for (index, idea) in entry.novel_ideas.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(idea, term, 1.8, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("novel_ideas[{index}]"),
                    idea,
                    term,
                );
            }
        }
        for (index, pattern) in entry.patterns.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(pattern, term, 1.4, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("patterns[{index}]"),
                    pattern,
                    term,
                );
            }
        }
        for (index, hook) in entry.integration_hooks.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(hook, term, 1.2, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("integration_hooks[{index}]"),
                    hook,
                    term,
                );
            }
        }
        for (index, comp) in entry.comparable_systems.iter().enumerate() {
            if let Some(field_score) = bm25_field_score(comp, term, 0.8, idf) {
                score += field_score;
                push_citation_span(
                    &mut citation_spans,
                    &format!("comparable_systems[{index}]"),
                    comp,
                    term,
                );
            }
        }
    }

    // small deep-confidence bonus so a deep+high-confidence match beats a
    // shallow-only match at equal text-score. Only applied when the text
    // score is already positive, otherwise no-hit sources would surface.
    if score > 0.0
        && entry.digest_status == "deep"
        && entry.confidence_self_report > 0.5
        && entry.triad_passed
        && entry.policy_readiness == "policy_ready"
    {
        score += 0.5 * entry.confidence_self_report;
    }

    ScoreHit {
        score,
        concepts_hit,
        citation_spans,
    }
}

fn entry_contains_term(entry: &IndexEntry, term: &str) -> bool {
    std::iter::once(entry.title.as_str())
        .chain(std::iter::once(entry.summary.as_str()))
        .chain(entry.relevance_tags.iter().map(String::as_str))
        .chain(entry.concepts.iter().map(String::as_str))
        .chain(entry.novel_ideas.iter().map(String::as_str))
        .chain(entry.patterns.iter().map(String::as_str))
        .chain(std::iter::once(entry.applicability.as_str()))
        .chain(entry.integration_hooks.iter().map(String::as_str))
        .chain(entry.comparable_systems.iter().map(String::as_str))
        .chain(std::iter::once(entry.deep_summary.as_str()))
        .any(|field| {
            normalized_field_tokens(field)
                .iter()
                .any(|token| token == term)
        })
}

fn bm25_idf(document_count: usize, document_frequency: usize) -> f64 {
    let n = document_count as f64;
    let df = document_frequency as f64;
    (1.0 + (n - df + 0.5) / (df + 0.5)).ln()
}

fn bm25_field_score(field: &str, term: &str, weight: f64, idf: f64) -> Option<f64> {
    let tokens = normalized_field_tokens(field);
    let term_frequency = tokens.iter().filter(|token| token.as_str() == term).count() as f64;
    if term_frequency == 0.0 {
        return None;
    }
    let field_length = tokens.len().max(1) as f64;
    let k1 = 1.2;
    let b = 0.75;
    let normalized_length = 1.0 - b + b * field_length;
    Some(
        7.0 * weight * idf * (term_frequency * (k1 + 1.0))
            / (term_frequency + k1 * normalized_length),
    )
}

fn normalized_field_tokens(value: &str) -> Vec<String> {
    normalized_token_spans(value)
        .into_iter()
        .map(|(token, _, _)| token)
        .collect()
}

fn normalized_token_spans(value: &str) -> Vec<(String, usize, usize)> {
    let mut spans = Vec::new();
    let mut start = None;
    for (offset, ch) in value.char_indices() {
        if ch.is_alphanumeric() || ch == '_' || ch == '-' || ch == '/' {
            start.get_or_insert(offset);
        } else if let Some(token_start) = start.take() {
            spans.push((
                normalize_token(&value[token_start..offset]),
                token_start,
                offset,
            ));
        }
    }
    if let Some(token_start) = start {
        spans.push((
            normalize_token(&value[token_start..]),
            token_start,
            value.len(),
        ));
    }
    spans.retain(|(token, _, _)| token.len() > 1 && !STOPWORDS.contains(&token.as_str()));
    spans
}

fn normalize_token(token: &str) -> String {
    let mut normalized = token.to_ascii_lowercase();
    if normalized == "ran" {
        return "run".to_string();
    }
    if normalized == "auth" || normalized.starts_with("authenticat") {
        return "auth".to_string();
    }
    if normalized.len() > 5 && normalized.ends_with("ing") {
        normalized.truncate(normalized.len() - 3);
        let bytes = normalized.as_bytes();
        if bytes.len() >= 2 && bytes[bytes.len() - 1] == bytes[bytes.len() - 2] {
            normalized.pop();
        }
    } else if normalized.len() > 4 && normalized.ends_with("ied") {
        normalized.truncate(normalized.len() - 3);
        normalized.push('y');
    } else if normalized.len() > 4 && normalized.ends_with("ed") {
        normalized.truncate(normalized.len() - 2);
    } else if normalized.len() > 4 && normalized.ends_with("ies") {
        normalized.truncate(normalized.len() - 3);
        normalized.push('y');
    } else if normalized.len() > 4 && normalized.ends_with("es") {
        normalized.truncate(normalized.len() - 2);
    } else if normalized.len() > 3 && normalized.ends_with('s') {
        normalized.pop();
    }
    normalized
}

pub(super) struct ScoreHit {
    pub score: f64,
    pub concepts_hit: Vec<String>,
    pub citation_spans: Vec<CitationSpan>,
}

fn push_citation_span(spans: &mut Vec<CitationSpan>, field: &str, original: &str, term: &str) {
    let token_span = normalized_token_spans(original)
        .into_iter()
        .find(|(token, _, _)| token == term)
        .map(|(_, start, end)| (start, end));
    let exact_span = original
        .to_ascii_lowercase()
        .find(term)
        .map(|start| (start, start + term.len()));
    let Some((start, end)) = token_span.or(exact_span) else {
        return;
    };
    if spans
        .iter()
        .any(|span| span.field == field && span.start == start && span.end == end)
    {
        return;
    }
    spans.push(CitationSpan {
        field: field.to_string(),
        start,
        end,
        text: original[start..end].to_string(),
    });
}

pub(super) fn tokenize_query(query: &str) -> Vec<String> {
    let mut tokens = normalized_field_tokens(query);
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
            triad_passed: false,
            policy_readiness: String::new(),
            has_extracted_knowledge: true,
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
    fn lexical_scoring_does_not_match_inside_unrelated_tokens() {
        let e = entry("s", "Thrust control", "orbital propulsion", &[], &[], "");
        let hit = score_entry(&tokenize_query("rust"), &e);
        assert_eq!(hit.score, 0.0, "rust must not match inside thrust");
    }

    #[test]
    fn normalization_collapses_inflection_and_authentication_aliases() {
        let inflections = tokenize_query("running ran runs");
        assert!(inflections.contains(&"run".to_string()));
        assert!(!inflections
            .iter()
            .any(|token| matches!(token.as_str(), "ran" | "running" | "runs")));
        let authentication = tokenize_query("auth authentication");
        assert!(authentication.contains(&"auth".to_string()));
        assert!(!authentication.iter().any(|token| token == "authentication"));

        let running = entry(
            "run",
            "Workers running safely",
            "Authentication middleware",
            &[],
            &[],
            "",
        );
        assert!(score_entry(&tokenize_query("ran"), &running).score > 0.0);
        assert!(score_entry(&tokenize_query("auth"), &running).score > 0.0);
    }

    #[test]
    fn deep_confidence_without_governance_does_not_change_score() {
        let mut deep = entry("d", "x", "", &["rust"], &[], "");
        deep.digest_status = "deep".into();
        deep.confidence_self_report = 0.9;
        let mut shallow = entry("s", "x", "", &["rust"], &[], "");
        shallow.digest_status = "shallow".into();
        shallow.confidence_self_report = 0.0;
        let d_score = score_entry(&tokenize_query("rust"), &deep).score;
        let s_score = score_entry(&tokenize_query("rust"), &shallow).score;
        assert_eq!(d_score, s_score, "deep={d_score} shallow={s_score}");
    }

    #[test]
    fn governed_deep_confidence_breaks_an_equal_lexical_tie() {
        let mut governed = entry("d", "x", "", &["rust"], &[], "");
        governed.triad_passed = true;
        governed.policy_readiness = "policy_ready".into();
        governed.confidence_self_report = 0.9;
        let mut shallow = governed.clone();
        shallow.digest_status = "shallow".into();
        shallow.confidence_self_report = 0.0;

        let governed_score = score_entry(&tokenize_query("rust"), &governed).score;
        let shallow_score = score_entry(&tokenize_query("rust"), &shallow).score;
        assert!(governed_score > shallow_score);
    }

    #[test]
    fn persisted_deep_governance_fields_enable_the_confidence_tie_break() {
        let dir = tempfile::tempdir().expect("tempdir");
        let book = dir.path().join("source.jsonl");
        fs::write(
            &book,
            concat!(
                "{\"stage\":\"shallow\",\"data\":{\"title\":\"Rust\",\"summary\":\"runtime\",\"relevance_tags\":[\"rust\"]}}\n",
                "{\"stage\":\"deep\",\"data\":{\"full_summary\":\"deep runtime\",\"triad_analysis\":{\"passed\":true},\"policy_readiness\":\"policy_ready\",\"extracted_knowledge\":{\"confidence_self_report\":0.9}}}\n"
            ),
        )
        .expect("write book");

        let indexed =
            build_index_entry(&book, "source", "books/source.jsonl".into()).expect("indexed entry");
        assert!(indexed.triad_passed);
        assert_eq!(indexed.policy_readiness, "policy_ready");
        assert_eq!(indexed.confidence_self_report, 0.9);
        assert!(indexed.has_extracted_knowledge);
    }
}
