use super::{MemoryCounts, MnemosyneService, MnemosyneStats};
use crate::schema::CONTINUITY_SCHEMA_VERSION;
use crate::service::retrieval::checkpoint_policy;
use crate::service::store::{episodic_schema_counts, walk_dir};
use arda_core::error::Result;
use chrono::{DateTime, Duration, Utc};
use std::path::Path;

impl MnemosyneService {
    pub fn stats(&self) -> Result<MnemosyneStats> {
        let records = self.read_episodic_records()?;
        let (legacy_episodic_records, unsupported_episodic_records) =
            episodic_schema_counts(&self.episodic_root);
        let recent = self.recall_recent(48, None)?;
        let mut counts = MemoryCounts {
            core: 0,
            active: 0,
            peripheral: 0,
            transient: 0,
            consolidated: 0,
            archived: 0,
            released: 0,
        };

        for record in records {
            match record.sigil.as_str() {
                "MNEME_CORE" => counts.core += 1,
                "MNEME_ACTIVE" => counts.active += 1,
                "MNEME_PERIPHERAL" => counts.peripheral += 1,
                "MNEME_TRANSIENT" => counts.transient += 1,
                "MNEME_CONSOLIDATED" => counts.consolidated += 1,
                "MNEME_ARCHIVED" => counts.archived += 1,
                _ => {}
            }
        }

        counts.consolidated += count_jsonl_files(&self.semantic_root)?;
        counts.archived += count_jsonl_lines(&self.archive_root.join("consolidation.jsonl"))?;
        counts.released += count_jsonl_lines(&self.noise_ledger_path)?;

        let last_consolidation_utc = std::fs::read_to_string(&self.last_consolidation_path)
            .ok()
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty());

        let next_consolidation_utc = last_consolidation_utc
            .as_deref()
            .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
            .map(|dt| (dt + Duration::hours(24)).with_timezone(&Utc).to_rfc3339())
            .unwrap_or_else(|| (Utc::now() + Duration::hours(24)).to_rfc3339());

        let chain_integrity = if self.chain_head_path.exists() {
            "head_present"
        } else {
            "missing"
        };

        Ok(MnemosyneStats {
            schema_version: CONTINUITY_SCHEMA_VERSION.to_owned(),
            generated_at_utc: Utc::now().to_rfc3339(),
            memory_counts: counts,
            last_consolidation_utc,
            next_consolidation_utc,
            chain_integrity: chain_integrity.to_owned(),
            informants_connected: 0,
            checkpoint_policy: checkpoint_policy(&recent),
            malformed_noise_records: count_malformed_jsonl(&self.noise_ledger_path),
            malformed_obsidian_records: count_malformed_jsonl(&self.obsidian_index_path),
            malformed_archive_records: count_malformed_jsonl(
                &self.archive_root.join("consolidation.jsonl"),
            ),
            malformed_episodic_records: count_malformed_episodic_records(&self.episodic_root),
            legacy_episodic_records,
            unsupported_episodic_records,
            observability: self.observability_snapshot(),
        })
    }

    pub fn status(&self) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "schema_version": CONTINUITY_SCHEMA_VERSION,
            "ok": true,
            "status": self.stats()?,
        }))
    }

    pub fn recent_noise_events(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.noise_ledger_path, limit)
    }

    pub fn recent_obsidian_entries(&self, limit: usize) -> Vec<serde_json::Value> {
        read_recent_jsonl(&self.obsidian_index_path, limit)
    }

    pub fn paths(&self) -> serde_json::Value {
        serde_json::json!({
            "root": self.root,
            "episodic": self.episodic_root,
            "semantic": self.semantic_root,
            "procedural": self.procedural_root,
            "archive": self.archive_root,
            "obsidian_index": self.obsidian_index_path,
            "chain_head": self.chain_head_path,
            "last_consolidation_utc": self.last_consolidation_path,
        })
    }
}

fn count_jsonl_lines(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let content = std::fs::read_to_string(path)?;
    Ok(content.lines().count())
}

fn count_jsonl_files(root: &Path) -> Result<usize> {
    if !root.exists() {
        return Ok(0);
    }
    let mut count = 0usize;
    for entry in walk_dir(root)? {
        if entry.extension().and_then(|v| v.to_str()) == Some("jsonl") {
            count += 1;
        }
    }
    Ok(count)
}

fn count_malformed_jsonl(path: &Path) -> usize {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(_) => return 0,
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter(|line| serde_json::from_str::<serde_json::Value>(line).is_err())
        .count()
}

fn count_malformed_episodic_records(root: &Path) -> usize {
    let mut malformed = 0usize;
    for entry in walk_dir(root).unwrap_or_default() {
        if entry.extension().and_then(|v| v.to_str()) != Some("jsonl") {
            continue;
        }
        let content = match std::fs::read_to_string(&entry) {
            Ok(content) => content,
            Err(_) => {
                malformed += 1;
                continue;
            }
        };
        let mut lines = content.lines().filter(|line| !line.trim().is_empty());
        let Some(header_line) = lines.next() else {
            continue;
        };
        if serde_json::from_str::<serde_json::Value>(header_line).is_err() {
            malformed += 1;
            continue;
        }
        let Some(body_line) = lines.next() else {
            malformed += 1;
            continue;
        };
        if serde_json::from_str::<serde_json::Value>(body_line).is_err() {
            malformed += 1;
        }
    }
    malformed
}

fn read_recent_jsonl(path: &Path, limit: usize) -> Vec<serde_json::Value> {
    let content = match std::fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut values = Vec::new();
    for line in content.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            values.push(value);
            if values.len() >= limit.max(1) {
                break;
            }
        }
    }
    values.reverse();
    values
}
