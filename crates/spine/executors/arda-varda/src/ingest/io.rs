// sigil: REPAIR
//
// Store-local persistence helpers: JSONL append/readback, crawl-capture
// receipts, source provenance accounting, and stable book references.

use arda_core::error::{ArdaError, Result};
use chrono::Utc;
use fs2::FileExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use super::source::{canonicalize_ingest_input, source_id_from_input};
use super::{
    AthenaStore, CrawlCaptureReceipt, CrawlMarkdownResult, DeepBookEntry, IngestRecord,
    KnowledgeTriageEntry, KnowledgeTriageSoterion, SourceType,
};

impl AthenaStore {
    pub fn record_crawl_capture(
        &self,
        url: &str,
        submitted_by: &str,
        task_context: &str,
        crawl_service_url: &str,
        crawl: &CrawlMarkdownResult,
    ) -> Result<CrawlCaptureReceipt> {
        let canonical_url = canonicalize_ingest_input(url);
        let source_id = source_id_from_input(&canonical_url);
        let artifact_path = self.crawl_artifacts_dir.join(format!("{source_id}.md"));
        fs::write(&artifact_path, crawl.markdown.as_bytes())?;

        let receipt = CrawlCaptureReceipt {
            source_id: source_id.clone(),
            url: canonical_url,
            captured_at_utc: Utc::now().to_rfc3339(),
            submitted_by: submitted_by.to_string(),
            task_context: task_context.to_string(),
            filter: crawl.filter.clone(),
            query: crawl.query.clone(),
            markdown_bytes: crawl.markdown.len(),
            artifact_path: artifact_path.display().to_string(),
            crawl_service_url: crawl_service_url.to_string(),
            success: crawl.success,
        };
        self.append_jsonl(&self.crawl_receipts_path, &receipt)?;
        Ok(receipt)
    }

    pub fn read_digest(
        &self,
        source_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<serde_json::Value>> {
        let content = fs::read_to_string(&self.digest_path)?;
        let mut items = Vec::new();

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if let Some(source_id) = source_id {
                let matches_id = value.get("id").and_then(|v| v.as_str()) == Some(source_id)
                    || value.get("source_id").and_then(|v| v.as_str()) == Some(source_id);
                if !matches_id {
                    continue;
                }
            }
            items.push(value);
        }

        if items.len() > limit {
            let start = items.len().saturating_sub(limit);
            Ok(items.split_off(start))
        } else {
            Ok(items)
        }
    }

    pub(super) fn append_jsonl<T: Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        file.lock_exclusive()?;
        let line = serde_json::to_string(value)?;
        let write_result = (|| -> Result<()> {
            writeln!(file, "{line}")?;
            file.sync_data()?;
            Ok(())
        })();
        let unlock_result = file.unlock().map_err(ArdaError::Ledger);
        write_result?;
        unlock_result?;
        Ok(())
    }

    pub(super) fn latest_ingest_record(&self, source_id: &str) -> Result<Option<IngestRecord>> {
        let digest = fs::read_to_string(&self.digest_path)?;
        for line in digest.lines().rev() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let matches = value.get("id").and_then(|v| v.as_str()) == Some(source_id);
            if !matches {
                continue;
            }
            if value.get("raw_input").is_none() {
                continue;
            }
            if let Ok(record) = serde_json::from_value::<IngestRecord>(value) {
                return Ok(Some(record));
            }
        }
        Ok(None)
    }

    pub(super) fn latest_deep_book_entry(&self, source_id: &str) -> Result<Option<DeepBookEntry>> {
        let book_path = self.books_dir.join(format!("{source_id}.jsonl"));
        if !book_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&book_path)?;
        for line in content.lines().rev() {
            if line.trim().is_empty() {
                continue;
            }
            let value: serde_json::Value = match serde_json::from_str(line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if value.get("stage").and_then(|v| v.as_str()) != Some("deep") {
                continue;
            }
            if let Ok(entry) = serde_json::from_value::<DeepBookEntry>(value) {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }

    pub(super) fn source_provenance_coverage_ratio(&self) -> Result<f64> {
        let mut total = 0usize;
        let mut covered = 0usize;
        for entry in fs::read_dir(&self.books_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("jsonl") {
                continue;
            }
            let Some(source_id) = path.file_stem().and_then(|value| value.to_str()) else {
                continue;
            };
            total += 1;
            if let Some(record) = self.latest_ingest_record(source_id)? {
                let has_input = !record.raw_input.trim().is_empty();
                let has_context = !record.task_context.trim().is_empty();
                let has_book_ref = !record.book_ref.trim().is_empty();
                if has_input && has_context && has_book_ref {
                    covered += 1;
                }
            }
        }
        if total == 0 {
            return Ok(1.0);
        }
        Ok(covered as f64 / total as f64)
    }

    pub(super) fn book_ref_for(&self, source_id: &str) -> String {
        self.root
            .join("books")
            .join(format!("{source_id}.jsonl"))
            .to_string_lossy()
            .replace('\\', "/")
    }

    pub fn emit_knowledge_triage_entry(&self, entry: &KnowledgeTriageEntry) -> Result<()> {
        let Some(canonical_path) = self.knowledge_triage_registry_path() else {
            tracing::debug!(
                store_root = %self.root.display(),
                "ATHENA triage registry emission skipped for non-canonical store root"
            );
            return Ok(());
        };
        if knowledge_triage_entry_exists(&canonical_path, entry)? {
            return Ok(());
        }
        self.append_jsonl(&canonical_path, entry)?;
        Ok(())
    }

    fn knowledge_triage_registry_path(&self) -> Option<PathBuf> {
        if let Ok(path) = std::env::var("ARDA_KNOWLEDGE_TRIAGE_REGISTRY_PATH") {
            return Some(PathBuf::from(path));
        }

        let root = super::layout::arda_root();
        let canonical_store_root = root.join("data/athena");
        if std::env::var_os("ARDA_ROOT").is_none() && self.root != canonical_store_root {
            return None;
        }

        Some(root.join("core/state/knowledge_triage_registry.jsonl"))
    }

    pub(super) fn emit_ingest_triage_entry(&self, record: &IngestRecord) -> Result<()> {
        if is_fixture_triage_source(record) {
            return Ok(());
        }
        let (classification, soterion, authority, action, rationale) =
            ingest_triage_posture(&record.source_type);
        let path = format!("data/athena/books/{}.jsonl", record.id);
        let bytes = fs::metadata(&record.book_ref)
            .map(|meta| meta.len() as usize)
            .unwrap_or(0);
        let entry = KnowledgeTriageEntry {
            schema_version: "arda.knowledge_triage.v1".to_string(),
            path,
            title: record.shallow.title.clone(),
            classification: classification.to_string(),
            soterion,
            canonical_home: "data/athena".to_string(),
            domain: "athena_ingest".to_string(),
            authority: authority.to_string(),
            recommended_action: action.to_string(),
            rationale: rationale.to_string(),
            headings: vec![record.shallow.title.clone()],
            bytes,
            sha256_12: sha256_12(&record.raw_input),
            triaged_at_utc: Utc::now().to_rfc3339(),
        };
        self.emit_knowledge_triage_entry(&entry)
    }
}

fn knowledge_triage_entry_exists(path: &Path, candidate: &KnowledgeTriageEntry) -> Result<bool> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(false);
    };
    let candidate = knowledge_triage_semantic_value(serde_json::to_value(candidate)?);

    for line in raw.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(existing) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if knowledge_triage_semantic_value(existing) == candidate {
            return Ok(true);
        }
    }

    Ok(false)
}

fn knowledge_triage_semantic_value(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("triaged_at_utc");
    }
    value
}

fn is_fixture_triage_source(record: &IngestRecord) -> bool {
    let Some(url) = record.url.as_deref() else {
        return false;
    };
    let lower = url.to_ascii_lowercase();
    let fixture_domain = has_fixture_host(&lower, "https://example.com")
        || has_fixture_host(&lower, "http://example.com")
        || has_fixture_host(&lower, "https://www.example.com")
        || has_fixture_host(&lower, "http://www.example.com");
    if !fixture_domain {
        return false;
    }

    let context = record.task_context.to_ascii_lowercase();
    let submitter = record.submitted_by.to_ascii_lowercase();
    !(context.contains("explicit_test_data") || submitter.contains("fixture_promoter"))
}

fn has_fixture_host(url: &str, prefix: &str) -> bool {
    let Some(rest) = url.strip_prefix(prefix) else {
        return false;
    };
    rest.is_empty()
        || matches!(
            rest.as_bytes().first(),
            Some(b'/') | Some(b'?') | Some(b'#')
        )
}

fn ingest_triage_posture(
    source_type: &SourceType,
) -> (
    &'static str,
    KnowledgeTriageSoterion,
    &'static str,
    &'static str,
    &'static str,
) {
    match source_type {
        SourceType::RawNote
        | SourceType::ChatExport
        | SourceType::XBookmark
        | SourceType::XPost => (
            "memory_seed",
            KnowledgeTriageSoterion {
                sigil: "MNEMOSYNE".to_string(),
                glyph: "🜄".to_string(),
                retention: "encode_or_link".to_string(),
            },
            "curated_memory",
            "encode/link as Mnemosyne recall context after ATHENA digest",
            "ATHENA ingest produced a source likely to carry operational or human memory.",
        ),
        _ => (
            "reference",
            KnowledgeTriageSoterion {
                sigil: "SCROLL".to_string(),
                glyph: "📜".to_string(),
                retention: "keep".to_string(),
            },
            "canonical",
            "keep as ATHENA reference evidence and promote derived conclusions when stable",
            "ATHENA ingest produced reference evidence for future synthesis.",
        ),
    }
}

fn sha256_12(input: &str) -> String {
    let digest = Sha256::digest(input.as_bytes());
    digest
        .iter()
        .take(6)
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
