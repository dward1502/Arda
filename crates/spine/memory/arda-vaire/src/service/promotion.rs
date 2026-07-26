use super::{
    ConsolidationReport, InformantEvent, MnemosyneService, ObsidianSyncReport, RecallRecentEntry,
};
use crate::service::store::append_jsonl;
use arda_core::error::{ArdaError, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

impl MnemosyneService {
    pub fn consolidate(&self, hours: i64) -> Result<ConsolidationReport> {
        let recent = self.recall_recent(hours.max(1), None)?;

        let mut grouped_by_tag: HashMap<String, Vec<RecallRecentEntry>> = HashMap::new();
        for entry in &recent {
            if entry.tags.is_empty() {
                grouped_by_tag
                    .entry("untagged".to_owned())
                    .or_default()
                    .push(entry.clone());
            } else {
                for tag in &entry.tags {
                    grouped_by_tag
                        .entry(tag.to_lowercase())
                        .or_default()
                        .push(entry.clone());
                }
            }
        }

        let mut semantic_written = 0usize;
        let mut promotion_receipts_written = 0usize;
        let mut consolidation_depth = 0usize;
        for (tag, cluster) in grouped_by_tag {
            if cluster.len() < 2 {
                continue;
            }
            let avg_significance =
                cluster.iter().map(|e| e.significance).sum::<f64>() / cluster.len() as f64;
            if avg_significance < 0.4 {
                continue;
            }

            let domain = sanitize_tag(&tag);
            let domain_dir = self.semantic_root.join(domain);
            fs::create_dir_all(&domain_dir)?;
            let pattern_id = format!("pat_{}", uuid::Uuid::new_v4().simple());
            let receipt_id = format!("promotion_{}", uuid::Uuid::new_v4().simple());
            let pattern_path = domain_dir.join(format!("{pattern_id}.jsonl"));
            let source_memory_ids = cluster
                .iter()
                .map(|entry| entry.memory_id.clone())
                .collect::<Vec<_>>();

            let summary = cluster
                .iter()
                .take(3)
                .map(|e| e.content.as_str())
                .collect::<Vec<_>>()
                .join(" | ");

            let record = serde_json::json!({
                "pattern_id": pattern_id,
                "memory_type": "semantic",
                "tag": tag,
                "created_at_utc": Utc::now().to_rfc3339(),
                "cluster_size": cluster.len(),
                "average_significance": avg_significance,
                "event_types": unique_values(cluster.iter().map(|e| e.event_type.as_str())),
                "source_crates": unique_values(cluster.iter().map(|e| e.source_crate.as_str())),
                "summary": summary,
                "promotion_receipt_id": receipt_id,
                "source_memory_ids": source_memory_ids,
                "average_confidence": cluster.iter().map(|entry| entry.confidence).sum::<f64>() / cluster.len() as f64,
                "average_trust": cluster.iter().map(|entry| entry.trust).sum::<f64>() / cluster.len() as f64,
            });
            append_jsonl(&pattern_path, &record)?;
            append_jsonl(
                &self.archive_root.join("promotion_receipts.jsonl"),
                &serde_json::json!({
                    "receipt_id": receipt_id,
                    "promoted_record_id": pattern_id,
                    "promoted_kind": "semantic",
                    "source_memory_ids": source_memory_ids,
                    "created_at_utc": Utc::now().to_rfc3339(),
                }),
            )?;
            semantic_written += 1;
            promotion_receipts_written += 1;
            consolidation_depth = consolidation_depth.max(cluster.len());
        }

        let mut procedural_map: HashMap<String, Vec<RecallRecentEntry>> = HashMap::new();
        for entry in &recent {
            let event = entry.event_type.to_lowercase();
            if event.contains("completed")
                || event.contains("delegated")
                || event.contains("ingest")
            {
                let key = format!("{}::{}", entry.source_crate, event);
                procedural_map.entry(key).or_default().push(entry.clone());
            }
        }

        let mut procedural_written = 0usize;
        for (skill_key, entries) in procedural_map {
            if entries.len() < 2 {
                continue;
            }
            let skill_id = format!("skill_{}", uuid::Uuid::new_v4().simple());
            let receipt_id = format!("promotion_{}", uuid::Uuid::new_v4().simple());
            let path = self.procedural_root.join(format!("{skill_id}.jsonl"));
            let source_memory_ids = entries
                .iter()
                .map(|entry| entry.memory_id.clone())
                .collect::<Vec<_>>();
            let sample = entries
                .iter()
                .take(3)
                .map(|e| e.content.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
            let value = serde_json::json!({
                "skill_id": skill_id,
                "memory_type": "procedural",
                "skill_key": skill_key,
                "created_at_utc": Utc::now().to_rfc3339(),
                "repetition_count": entries.len(),
                "summary": sample,
                "promotion_receipt_id": receipt_id,
                "source_memory_ids": source_memory_ids,
                "average_confidence": entries.iter().map(|entry| entry.confidence).sum::<f64>() / entries.len() as f64,
                "average_trust": entries.iter().map(|entry| entry.trust).sum::<f64>() / entries.len() as f64,
            });
            append_jsonl(&path, &value)?;
            append_jsonl(
                &self.archive_root.join("promotion_receipts.jsonl"),
                &serde_json::json!({
                    "receipt_id": receipt_id,
                    "promoted_record_id": skill_id,
                    "promoted_kind": "procedural",
                    "source_memory_ids": source_memory_ids,
                    "created_at_utc": Utc::now().to_rfc3339(),
                }),
            )?;
            procedural_written += 1;
            promotion_receipts_written += 1;
            consolidation_depth = consolidation_depth.max(entries.len());
        }

        let archived = serde_json::json!({
            "ts_utc": Utc::now().to_rfc3339(),
            "action": "consolidation_sweep",
            "window_hours": hours.max(1),
            "episodic_scanned": recent.len(),
            "semantic_patterns_written": semantic_written,
            "procedural_patterns_written": procedural_written,
        });
        let archive_log_path = self.archive_root.join("consolidation.jsonl");
        append_jsonl(&archive_log_path, &archived)?;

        let now = Utc::now().to_rfc3339();
        std::fs::write(&self.last_consolidation_path, &now)?;

        self.observe_consolidation(consolidation_depth, promotion_receipts_written);
        Ok(ConsolidationReport {
            consolidated_at_utc: now,
            window_hours: hours.max(1),
            episodic_scanned: recent.len(),
            semantic_patterns_written: semantic_written,
            procedural_patterns_written: procedural_written,
            archived_records_written: 1,
            promotion_receipts_written,
            consolidation_depth,
        })
    }

    pub fn sync_obsidian(
        &self,
        vault_path: impl AsRef<Path>,
        max_files: usize,
    ) -> Result<ObsidianSyncReport> {
        let vault_path = vault_path.as_ref();
        if !vault_path.exists() {
            return Err(ArdaError::Agent {
                agent: "mnemosyne".to_owned(),
                message: format!("obsidian path not found: {}", vault_path.display()),
            });
        }

        let mut files = Vec::new();
        collect_obsidian_files(vault_path, &mut files)?;
        files.sort();
        files.truncate(max_files.max(1));

        let mut notes_indexed = 0usize;
        let mut memories_encoded = 0usize;

        for file in &files {
            let content = fs::read_to_string(file).unwrap_or_default();
            let snippet = content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(8)
                .collect::<Vec<_>>()
                .join(" ");
            let rel = file
                .strip_prefix(vault_path)
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or_else(|| file.to_str().unwrap_or("unknown"));

            append_jsonl(
                &self.obsidian_index_path,
                &serde_json::json!({
                    "sigil": "SCROLL",
                    "ts_utc": Utc::now().to_rfc3339(),
                    "path": rel,
                    "absolute_path": file.display().to_string(),
                    "snippet": snippet,
                }),
            )?;
            notes_indexed += 1;

            let event = InformantEvent {
                informant_id: "obsidian_bridge".to_owned(),
                crate_name: "illuvatar_obsidian".to_owned(),
                event_type: "obsidian_note_sync".to_owned(),
                ts_utc: Utc::now().to_rfc3339(),
                content: format!("Synced Obsidian note {rel}: {snippet}"),
                confidence_hint: Some(0.65),
                tags: vec!["obsidian".to_owned(), "human".to_owned()],
            };
            if self.encode(event)?.is_some() {
                memories_encoded += 1;
            }
        }

        Ok(ObsidianSyncReport {
            synced_at_utc: Utc::now().to_rfc3339(),
            vault_path: vault_path.display().to_string(),
            files_scanned: files.len(),
            notes_indexed,
            memories_encoded,
            index_path: self.obsidian_index_path.display().to_string(),
        })
    }
}

fn sanitize_tag(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_owned()
}

fn unique_values<'a, I>(values: I) -> Vec<String>
where
    I: Iterator<Item = &'a str>,
{
    let mut set = std::collections::BTreeSet::new();
    for value in values {
        if !value.is_empty() {
            set.insert(value.to_owned());
        }
    }
    set.into_iter().collect()
}

fn collect_obsidian_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_obsidian_files(&path, out)?;
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext == "md" || ext == "canvas" {
            out.push(path);
        }
    }
    Ok(())
}
