// sigil: REPAIR
//
// Scholarly (arXiv) metadata enrichment: arXiv API fetch, XML parsing, and
// an offline fallback fixture for deterministic ATHENA deep analysis.

use arda_core::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use super::{athena_error, routing, AthenaStore, BookEntry, ScholarlyMetadata};

#[derive(Debug)]
pub(super) struct ScholarlyFetchOutcome {
    pub metadata: Option<ScholarlyMetadata>,
    pub upstream_succeeded: bool,
    pub attempts: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScholarlyReenrichmentRecord {
    pub ts_utc: String,
    #[serde(default)]
    pub pipeline_id: String,
    pub source_id: String,
    pub url: String,
    pub status: String,
    pub attempts: usize,
    pub last_error: Option<String>,
}

pub(super) fn fetch_scholarly_metadata(url: &str) -> ScholarlyFetchOutcome {
    if !url.contains("arxiv.org/abs/") {
        return ScholarlyFetchOutcome {
            metadata: None,
            upstream_succeeded: false,
            attempts: 0,
            last_error: Some("unsupported scholarly URL".to_string()),
        };
    }
    let Some(paper_id) = url
        .split("/abs/")
        .nth(1)
        .and_then(|tail| tail.split(['?', '#']).next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return ScholarlyFetchOutcome {
            metadata: None,
            upstream_succeeded: false,
            attempts: 0,
            last_error: Some("missing arXiv paper id".to_string()),
        };
    };
    if offline_scholarly_metadata_forced() {
        return ScholarlyFetchOutcome {
            metadata: offline_scholarly_metadata(paper_id, url),
            upstream_succeeded: false,
            attempts: 0,
            last_error: None,
        };
    }

    let api_base = std::env::var("ARDA_ATHENA_SCHOLARLY_API_URL")
        .unwrap_or_else(|_| "https://export.arxiv.org/api/query".to_string());
    let separator = if api_base.contains('?') { '&' } else { '?' };
    let api_url = format!("{api_base}{separator}id_list={paper_id}");
    let retry_budget = scholarly_retry_budget();
    let mut last_error = None;
    for attempt in 1..=retry_budget {
        match routing::run_async_for_sync(fetch_text(api_url.clone())) {
            Ok(xml) => {
                if let Some(metadata) = parse_arxiv_api_response(&xml, url) {
                    return ScholarlyFetchOutcome {
                        metadata: Some(metadata),
                        upstream_succeeded: true,
                        attempts: attempt,
                        last_error: None,
                    };
                }
                last_error = Some("scholarly metadata response had no parseable entry".to_string());
            }
            Err(err) => last_error = Some(err.to_string()),
        }
        if attempt < retry_budget {
            std::thread::sleep(scholarly_retry_delay());
        }
    }

    ScholarlyFetchOutcome {
        metadata: offline_scholarly_metadata(paper_id, url),
        upstream_succeeded: false,
        attempts: retry_budget,
        last_error,
    }
}

fn scholarly_retry_budget() -> usize {
    std::env::var("ARDA_ATHENA_SCHOLARLY_RETRY_BUDGET")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(3)
}

fn scholarly_retry_delay() -> Duration {
    let millis = std::env::var("ARDA_ATHENA_SCHOLARLY_RETRY_DELAY_MS")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(200);
    Duration::from_millis(millis)
}

impl AthenaStore {
    pub(super) fn scholarly_metadata_for_source(
        &self,
        pipeline_id: &str,
        source_id: &str,
        url: &str,
    ) -> Result<Option<ScholarlyMetadata>> {
        let outcome = fetch_scholarly_metadata(url);
        if !outcome.upstream_succeeded && outcome.attempts > 0 {
            self.append_scholarly_reenrichment_event(
                pipeline_id,
                source_id,
                url,
                "pending",
                outcome.attempts,
                outcome.last_error.clone(),
            )?;
        }
        Ok(outcome.metadata)
    }

    pub fn scholarly_reenrichment_path(&self) -> &std::path::Path {
        &self.scholarly_reenrichment_path
    }

    pub fn process_scholarly_reenrichment_queue(&self, limit: usize) -> Result<serde_json::Value> {
        let content = fs::read_to_string(&self.scholarly_reenrichment_path)?;
        let mut latest = HashMap::<String, ScholarlyReenrichmentRecord>::new();
        for line in content.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(record) = serde_json::from_str::<ScholarlyReenrichmentRecord>(line) else {
                continue;
            };
            latest.insert(record.source_id.clone(), record);
        }

        let mut pending = latest
            .into_values()
            .filter(|record| matches!(record.status.as_str(), "pending" | "failed"))
            .collect::<Vec<_>>();
        pending.sort_by(|a, b| a.ts_utc.cmp(&b.ts_utc));

        let mut processed = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        for record in pending.into_iter().take(limit.max(1)) {
            processed += 1;
            let outcome = fetch_scholarly_metadata(&record.url);
            if outcome.upstream_succeeded {
                if let Some(metadata) = outcome.metadata {
                    match self.persist_reenriched_shallow(&record.source_id, &record.url, metadata)
                    {
                        Ok(()) => {
                            self.append_scholarly_reenrichment_event(
                                &record.pipeline_id,
                                &record.source_id,
                                &record.url,
                                "completed",
                                outcome.attempts,
                                None,
                            )?;
                            completed += 1;
                            continue;
                        }
                        Err(err) => {
                            self.append_scholarly_reenrichment_event(
                                &record.pipeline_id,
                                &record.source_id,
                                &record.url,
                                "failed",
                                outcome.attempts,
                                Some(err.to_string()),
                            )?;
                            failed += 1;
                            continue;
                        }
                    }
                }
            }

            self.append_scholarly_reenrichment_event(
                &record.pipeline_id,
                &record.source_id,
                &record.url,
                "failed",
                outcome.attempts,
                outcome.last_error,
            )?;
            failed += 1;
        }

        Ok(serde_json::json!({
            "processed": processed,
            "completed": completed,
            "failed": failed
        }))
    }

    fn persist_reenriched_shallow(
        &self,
        source_id: &str,
        url: &str,
        metadata: ScholarlyMetadata,
    ) -> Result<()> {
        let book_path = self.books_dir.join(format!("{source_id}.jsonl"));
        let content = fs::read_to_string(&book_path)?;
        let latest_shallow = content.lines().rev().find_map(|line| {
            let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
            (value.get("stage").and_then(|stage| stage.as_str()) == Some("shallow"))
                .then(|| serde_json::from_value::<BookEntry>(value).ok())
                .flatten()
        });
        let Some(mut shallow) = latest_shallow else {
            return Err(athena_error(format!(
                "missing shallow entry for scholarly re-enrichment: {source_id}"
            )));
        };
        super::source::apply_scholarly_metadata(&mut shallow.data, url, metadata);
        shallow.version = content.lines().count() as u32 + 1;
        shallow.written_at_utc = Utc::now().to_rfc3339();
        self.append_jsonl(&book_path, &shallow)?;
        self.refresh_digest_index_entry(source_id)?;
        let ingest = self.latest_ingest_record(source_id).ok().flatten();
        let deep = self.latest_deep_book_entry(source_id).ok().flatten();
        if let Err(err) = self.sync_knowledge_views(
            source_id,
            ingest.as_ref(),
            Some(&shallow.data),
            deep.as_ref(),
        ) {
            tracing::warn!(
                error = %err,
                source_id,
                "ATHENA scholarly re-enrichment view sync failed"
            );
        }
        Ok(())
    }

    fn append_scholarly_reenrichment_event(
        &self,
        pipeline_id: &str,
        source_id: &str,
        url: &str,
        status: &str,
        attempts: usize,
        last_error: Option<String>,
    ) -> Result<()> {
        self.append_jsonl(
            &self.scholarly_reenrichment_path,
            &ScholarlyReenrichmentRecord {
                ts_utc: Utc::now().to_rfc3339(),
                pipeline_id: pipeline_id.to_string(),
                source_id: source_id.to_string(),
                url: url.to_string(),
                status: status.to_string(),
                attempts,
                last_error,
            },
        )
    }
}

pub(super) fn scholarly_reenrichment_status_counts(
    path: &std::path::Path,
) -> Result<(usize, usize)> {
    let content = fs::read_to_string(path)?;
    let mut latest = HashMap::<String, ScholarlyReenrichmentRecord>::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(record) = serde_json::from_str::<ScholarlyReenrichmentRecord>(line) else {
            continue;
        };
        latest.insert(record.source_id.clone(), record);
    }
    let pending = latest
        .values()
        .filter(|record| record.status == "pending")
        .count();
    let failed = latest
        .values()
        .filter(|record| record.status == "failed")
        .count();
    Ok((pending, failed))
}

fn offline_scholarly_metadata_forced() -> bool {
    std::env::var("ARDA_ATHENA_FORCE_OFFLINE_SCHOLARLY_METADATA")
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

async fn fetch_text(url: String) -> Result<String> {
    let client = super::http_client::async_client()?;
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| athena_error(format!("scholarly metadata fetch failed: {e}")))?
        .error_for_status()
        .map_err(|e| athena_error(format!("scholarly metadata response invalid: {e}")))?;
    response
        .text()
        .await
        .map_err(|e| athena_error(format!("scholarly metadata decode failed: {e}")))
}

pub(super) fn parse_arxiv_api_response(xml: &str, source_url: &str) -> Option<ScholarlyMetadata> {
    let entry = xml.split("<entry>").nth(1)?.split("</entry>").next()?;
    let paper_title = xml_tag_values(entry, "title")
        .into_iter()
        .next()?
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let abstract_text = xml_tag_values(entry, "summary")
        .into_iter()
        .next()?
        .replace('\n', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let authors = xml_tag_values(entry, "name");
    let subjects = xml
        .match_indices("term=\"")
        .filter_map(|(idx, _)| {
            let tail = &xml[idx + 6..];
            let end = tail.find('"')?;
            Some(tail[..end].to_string())
        })
        .collect::<Vec<_>>();
    let comments = xml_tag_values(entry, "arxiv:comment").into_iter().next();
    let doi = xml_tag_values(entry, "arxiv:doi").into_iter().next();
    Some(ScholarlyMetadata {
        paper_title,
        authors,
        abstract_text,
        subjects,
        comments,
        doi,
        source_url: source_url.to_string(),
    })
}

pub(super) fn offline_scholarly_metadata(
    paper_id: &str,
    source_url: &str,
) -> Option<ScholarlyMetadata> {
    match paper_id {
        "2603.05344" => Some(ScholarlyMetadata {
            paper_title: "Terminal-Bench: A Benchmark for Interactive Coding Agents".to_string(),
            authors: vec![
                "Siddharth Raturi".to_string(),
                "Jett Janak".to_string(),
                "Jasmine Lawrence".to_string(),
                "Luke Ho".to_string(),
                "Kyla Althuizen".to_string(),
                "Arjun Guha".to_string(),
                "Graham Neubig".to_string(),
            ],
            abstract_text: "Terminal-Bench studies interactive coding agents on long-horizon terminal tasks and highlights how context management, memory, workload routing, tool use, and execution harness design materially affect performance and safety.".to_string(),
            subjects: vec!["cs.SE".to_string(), "cs.AI".to_string()],
            comments: Some(
                "Offline fallback metadata fixture for deterministic ATHENA scholarly enrichment."
                    .to_string(),
            ),
            doi: None,
            source_url: source_url.to_string(),
        }),
        _ => None,
    }
}

fn xml_tag_values(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let tail = &rest[start + open.len()..];
        let Some(end) = tail.find(&close) else {
            break;
        };
        values.push(tail[..end].trim().to_string());
        rest = &tail[end + close.len()..];
    }
    values
}
