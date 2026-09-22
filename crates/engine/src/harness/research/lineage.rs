//! Offline consistency checks against service-controlled local capture records.
//! This does not authenticate original HTTP bytes or resist same-UID store edits.
use super::*;
use std::io::{BufRead, BufReader, Read};
use std::path::Path as FsPath;

const MAX_RECORD_BYTES: u64 = 256 * 1024;

// Memory is bounded per record, not by the lifetime size of an append-only ledger.
fn any_record(path: &FsPath, matches: impl Fn(&serde_json::Value) -> bool) -> Result<bool, String> {
    let file = std::fs::File::open(path).map_err(|_| "capture lineage ledger unavailable")?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        if (&mut reader)
            .take(MAX_RECORD_BYTES + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|_| "capture lineage ledger unreadable")?
            == 0
        {
            return Ok(false);
        }
        if bytes.len() as u64 > MAX_RECORD_BYTES {
            if bytes.last() != Some(&b'\n') {
                loop {
                    let buffer = reader
                        .fill_buf()
                        .map_err(|_| "capture lineage ledger unreadable")?;
                    if buffer.is_empty() {
                        break;
                    }
                    let newline = buffer.iter().position(|&b| b == b'\n');
                    let consumed = newline.map_or(buffer.len(), |index| index + 1);
                    reader.consume(consumed);
                    if newline.is_some() {
                        break;
                    }
                }
            }
            tracing::warn!("Ignoring oversized local capture ledger record");
            continue;
        }
        if bytes.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        match serde_json::from_slice(&bytes) {
            Ok(record) if matches(&record) => return Ok(true),
            Ok(_) => {}
            Err(_) => tracing::warn!("Ignoring malformed local capture ledger record"),
        }
    }
}

/// Only these bindings were checked. No body hash, excerpt, timestamp or fitness
/// claim from the caller's citation is promoted to verified evidence.
pub(super) struct VerifiedLocalLineage {
    pub source_id: String,
    pub pipeline_id: String,
    pub evaluation_digest: String,
}

/// Verify persisted local lineage, not freshness or execution eligibility.
/// Derive every read path from the host store root, never from citation paths.
pub(super) fn verify(
    root: &FsPath,
    citation: &BriefCitation,
    query: &str,
    context: &str,
) -> Result<VerifiedLocalLineage, String> {
    let id = &citation.varda_source_id;
    let suffix = id.strip_prefix("src_").ok_or("invalid capture source ID")?;
    if suffix.len() != 8
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err("invalid capture source ID".into());
    }
    if citation.varda_pipeline_id.trim().is_empty() {
        return Err("missing capture pipeline ID".into());
    }
    let (recorded_url, expected_id) =
        arda_varda::ingest::AthenaStore::canonical_source_identity(&citation.canonical_url);
    if *id != expected_id {
        return Err("capture source ID does not match the canonical source identity".into());
    }
    if !any_record(&root.join("digest.jsonl"), |record| {
        record["id"] == *id
            && record["pipeline_id"] == citation.varda_pipeline_id
            && record["url"] == recorded_url
            && record["submitted_by"] == "workbench_research"
            && record["task_context"] == context
            && record["quarantine"] == false
            && record["error"].is_null()
    })? {
        return Err("capture citation has no matching local ingest record".into());
    }
    let expected_artifact = root.join("crawls").join(format!("{id}.md"));
    if citation.crawl_receipt_path != expected_artifact.to_string_lossy() {
        return Err("capture artifact path does not match the trusted store layout".into());
    }
    if !any_record(&root.join("crawl_receipts.jsonl"), |receipt| {
        receipt["source_id"] == *id
            && receipt["pipeline_id"] == citation.varda_pipeline_id
            && receipt["url"] == recorded_url
            && receipt["query"] == query
            && receipt["task_context"] == context
            && receipt["submitted_by"] == "workbench_research"
            && receipt["crawl_service_url"] == "workbench://canonical-http-fetch"
            && receipt["filter"] == "canonical_http_text"
            && receipt["success"] == true
            && receipt["artifact_path"] == citation.crawl_receipt_path
    })? {
        return Err("capture citation has no matching local crawl receipt".into());
    }
    if !any_record(&root.join("books").join(format!("{id}.jsonl")), |entry| {
        if entry["stage"] != "deep" || entry["pipeline_id"] != citation.varda_pipeline_id {
            return false;
        }
        let data = &entry["data"];
        data["policy_readiness"] == citation.policy_readiness
            && data["confidence"].as_f64() == Some(citation.confidence)
            && digest_value(&serde_json::json!({
                "policy_readiness": data["policy_readiness"], "confidence": data["confidence"],
                "triad_passed": data["triad_analysis"]["passed"],
                "extraction_status": data["extraction_status"],
            })) == citation.evaluation_digest
    })? {
        return Err("capture citation has no matching local deep evaluation".into());
    }
    // content_sha256 belongs to the raw HTTP response. Markdown is normalized,
    // mutable by source ID, and cannot be substituted as proof of those bytes.
    Ok(VerifiedLocalLineage {
        source_id: expected_id,
        pipeline_id: citation.varda_pipeline_id.clone(),
        evaluation_digest: citation.evaluation_digest.clone(),
    })
}

#[cfg(test)]
mod tests;
