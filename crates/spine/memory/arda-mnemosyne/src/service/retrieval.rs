use super::{
    IdentityState, KnowledgeSeedRecallEntry, MemoryCheckpointPolicy, MnemosyneService,
    RecallRecentEntry,
};
use arda_core::error::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

impl MnemosyneService {
    pub fn recall_recent(
        &self,
        hours: i64,
        crate_filter: Option<&str>,
    ) -> Result<Vec<RecallRecentEntry>> {
        self.recall_recent_scoped(hours, crate_filter, None)
    }

    pub fn recall_relevant(
        &self,
        query: &str,
        hours: i64,
        crate_filter: Option<&str>,
        scope_filter: Option<&str>,
        limit: usize,
    ) -> Result<Vec<RecallRecentEntry>> {
        let mut out = self.recall_recent_scoped(hours, crate_filter, scope_filter)?;
        let query_terms = query_terms(query);
        if !query_terms.is_empty() {
            out.sort_by(|a, b| {
                let a_score = self.query_aware_relevance_score(a, &query_terms);
                let b_score = self.query_aware_relevance_score(b, &query_terms);
                b_score
                    .partial_cmp(&a_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        out.truncate(limit.max(1));
        Ok(out)
    }

    pub fn recall_knowledge_seeds(
        &self,
        query: Option<&str>,
        limit: usize,
    ) -> Result<Vec<KnowledgeSeedRecallEntry>> {
        let entries = read_triage_registry()?;
        let delete_paths = entries
            .iter()
            .filter(|entry| {
                entry.get("classification").and_then(|v| v.as_str()) == Some("delete_candidate")
            })
            .filter_map(|entry| {
                entry
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(str::to_owned)
            })
            .collect::<HashSet<_>>();
        let query_terms = query.map(query_terms).unwrap_or_default();

        let mut out = entries
            .into_iter()
            .filter_map(|entry| {
                if entry.get("classification").and_then(|v| v.as_str()) == Some("memory_seed") {
                    triage_memory_seed_entry(entry, &delete_paths, &query_terms)
                } else {
                    triage_athena_deep_github_seed_entry(entry, &delete_paths, &query_terms)
                }
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
        });
        out.dedup_by(|a, b| a.path == b.path);
        out.truncate(limit.max(1));
        Ok(out)
    }

    pub fn recall_recent_scoped(
        &self,
        hours: i64,
        crate_filter: Option<&str>,
        scope_filter: Option<&str>,
    ) -> Result<Vec<RecallRecentEntry>> {
        let cutoff = Utc::now() - Duration::hours(hours.max(1));
        let mut out = Vec::new();

        for record in self.read_episodic_records()? {
            if let Ok(dt) = DateTime::parse_from_rfc3339(&record.ts_utc) {
                if dt.with_timezone(&Utc) < cutoff {
                    continue;
                }
            }
            if let Some(filter) = crate_filter {
                if record.source_crate != filter {
                    continue;
                }
            }
            if let Some(scope) = scope_filter {
                if record.memory_scope != scope {
                    continue;
                }
            }
            out.push(RecallRecentEntry {
                memory_id: record.memory_id,
                source_crate: record.source_crate,
                event_type: record.event_type,
                memory_scope: record.memory_scope,
                significance: record.significance,
                sigil: record.sigil,
                content: record.content,
                ts_utc: record.ts_utc,
                tags: record.tags,
            });
        }

        out.sort_by(|a, b| {
            let a_relevance = self.relevance_score(a);
            let b_relevance = self.relevance_score(b);
            b_relevance
                .partial_cmp(&a_relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(out)
    }

    fn relevance_score(&self, entry: &RecallRecentEntry) -> f64 {
        if let Ok(ts) = DateTime::parse_from_rfc3339(&entry.ts_utc) {
            let ts_utc = ts.with_timezone(&Utc);
            let age_hours = (Utc::now() - ts_utc).num_hours() as f64;
            let time_decay = (-age_hours.ln() / 12.0).exp().min(1.0);
            let base_score = entry.significance;
            let content_len = entry.content.len() as f64;
            let content_bonus = (content_len / 100.0).min(0.5);
            let tag_bonus = (entry.tags.len() as f64 / 5.0).min(0.3);
            let sigil_multiplier = match entry.sigil.as_str() {
                "MNEME_CORE" => 1.5,
                "MNEME_ACTIVE" => 1.2,
                "MNEME_PERIPHERAL" => 1.0,
                "MNEME_TRANSIENT" => 0.8,
                _ => 0.5,
            };

            (base_score * time_decay * sigil_multiplier + content_bonus + tag_bonus).clamp(0.0, 2.0)
        } else {
            0.0
        }
    }

    fn query_aware_relevance_score(
        &self,
        entry: &RecallRecentEntry,
        query_terms: &[String],
    ) -> f64 {
        let base = self.relevance_score(entry);
        let query_match = query_match_score(query_terms, &entry.content, &entry.tags);
        let scope_bonus = if matches!(
            entry.memory_scope.as_str(),
            "boardroom_council" | "human_context"
        ) {
            0.08
        } else {
            0.0
        };
        (base + query_match * 0.9 + scope_bonus).clamp(0.0, 4.0)
    }

    pub fn identity_state(&self) -> Result<IdentityState> {
        let recent = self.recall_recent(48, None)?;
        let mut core_count = 0usize;
        let mut active_count = 0usize;
        let mut peripheral_count = 0usize;
        let mut transient_count = 0usize;
        for r in &recent {
            match r.sigil.as_str() {
                "MNEME_CORE" => core_count += 1,
                "MNEME_ACTIVE" => active_count += 1,
                "MNEME_PERIPHERAL" => peripheral_count += 1,
                "MNEME_TRANSIENT" => transient_count += 1,
                _ => {}
            }
        }
        let focus = recent
            .first()
            .map(|r| r.content.clone())
            .unwrap_or_else(|| "No recent memory focus.".to_owned());

        Ok(IdentityState {
            generated_at_utc: Utc::now().to_rfc3339(),
            core_memory_count: core_count,
            active_memory_count: active_count,
            peripheral_memory_count: peripheral_count,
            transient_memory_count: transient_count,
            recent_events: recent.into_iter().take(8).collect(),
            current_mission_focus: focus,
        })
    }
}

pub(super) fn checkpoint_policy(recent: &[RecallRecentEntry]) -> MemoryCheckpointPolicy {
    let mut tag_counts: HashMap<String, usize> = HashMap::new();
    for entry in recent {
        for tag in &entry.tags {
            *tag_counts.entry(tag.to_ascii_lowercase()).or_default() += 1;
        }
    }
    let mut priority_tags = tag_counts.into_iter().collect::<Vec<_>>();
    priority_tags.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let priority_tags = priority_tags
        .into_iter()
        .take(4)
        .map(|(tag, _)| tag)
        .collect::<Vec<_>>();
    let recent_len = recent.len();
    let memory_pressure = if recent_len >= 18 {
        "high"
    } else if recent_len >= 8 {
        "medium"
    } else {
        "low"
    };
    let recall_window_hours = if memory_pressure == "high" { 24 } else { 48 };
    let checkpoint_interval_events = match memory_pressure {
        "high" => 4,
        "medium" => 6,
        _ => 8,
    };
    let consolidation_bias = if priority_tags.iter().any(|tag| {
        matches!(
            tag.as_str(),
            "decision"
                | "routing"
                | "safety"
                | "memory"
                | "boardroom"
                | "interrupt"
                | "delegation"
                | "completion"
        )
    }) {
        "procedural"
    } else {
        "semantic"
    };

    MemoryCheckpointPolicy {
        checkpoint_interval_events,
        recall_window_hours,
        priority_tags,
        consolidation_bias: consolidation_bias.to_owned(),
        memory_pressure: memory_pressure.to_owned(),
    }
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_alphanumeric())
        .filter(|term| term.len() >= 4)
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

fn query_match_score(query_terms: &[String], content: &str, tags: &[String]) -> f64 {
    if query_terms.is_empty() {
        return 0.0;
    }

    let haystack = format!(
        "{} {}",
        content.to_ascii_lowercase(),
        tags.join(" ").to_ascii_lowercase()
    );
    let matches = query_terms
        .iter()
        .filter(|term| haystack.contains(term.as_str()))
        .count();

    (matches as f64 / query_terms.len() as f64).clamp(0.0, 1.0)
}

fn triage_memory_seed_entry(
    entry: Value,
    delete_paths: &HashSet<String>,
    query_terms: &[String],
) -> Option<KnowledgeSeedRecallEntry> {
    let path = value_string(&entry, "path")?;
    if delete_paths.contains(&path) {
        return None;
    }
    if path_exists(&path) == Some(false) {
        return None;
    }
    let title = value_string(&entry, "title").unwrap_or_else(|| title_from_path(&path));
    let canonical_home = value_string(&entry, "canonical_home").unwrap_or_default();
    let domain = value_string(&entry, "domain").unwrap_or_default();
    let authority = value_string(&entry, "authority").unwrap_or_default();
    let recommended_action = value_string(&entry, "recommended_action").unwrap_or_default();
    let rationale = value_string(&entry, "rationale").unwrap_or_default();
    let triaged_at_utc = value_string(&entry, "triaged_at_utc").unwrap_or_default();
    let soterion = entry.get("soterion").and_then(|value| value.as_object());
    let soterion_glyph = soterion
        .and_then(|value| value.get("glyph"))
        .and_then(|value| value.as_str())
        .unwrap_or("🜄")
        .to_string();
    let soterion_sigil = soterion
        .and_then(|value| value.get("sigil"))
        .and_then(|value| value.as_str())
        .unwrap_or("MNEMOSYNE")
        .to_string();
    let haystack = format!(
        "{path} {title} {canonical_home} {domain} {authority} {recommended_action} {rationale}"
    );
    let match_score = if query_terms.is_empty() {
        0.25
    } else {
        let lower = haystack.to_ascii_lowercase();
        query_terms
            .iter()
            .filter(|term| lower.contains(term.as_str()))
            .count() as f64
            / query_terms.len() as f64
    };
    if !query_terms.is_empty() && match_score == 0.0 {
        return None;
    }
    let authority_bonus = if authority == "curated_memory" {
        0.25
    } else {
        0.0
    };
    Some(KnowledgeSeedRecallEntry {
        path,
        title,
        classification: "memory_seed".to_string(),
        canonical_home,
        domain,
        authority,
        recommended_action,
        rationale,
        soterion_glyph,
        soterion_sigil,
        triaged_at_utc,
        score: (match_score + authority_bonus).clamp(0.0, 2.0),
    })
}

fn triage_athena_deep_github_seed_entry(
    entry: Value,
    delete_paths: &HashSet<String>,
    query_terms: &[String],
) -> Option<KnowledgeSeedRecallEntry> {
    if entry.get("classification").and_then(|value| value.as_str()) == Some("delete_candidate") {
        return None;
    }
    if entry.get("domain").and_then(|value| value.as_str()) != Some("athena_ingest") {
        return None;
    }
    let path = value_string(&entry, "path")?;
    if !path.starts_with("data/athena/books/") || delete_paths.contains(&path) {
        return None;
    }
    if path_exists(&path) == Some(false) {
        return None;
    }

    let deep = read_deep_athena_book_entry(&path)?;
    let data = deep.get("data")?;
    let relevance_tags = data
        .get("relevance_tags")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();
    let implementation_brief = data
        .get("implementation_brief")
        .and_then(|value| value.as_object());
    let source_url = implementation_brief
        .and_then(|value| value.get("source_url"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let title = data
        .get("title")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| value_string(&entry, "title"))
        .unwrap_or_else(|| title_from_path(&path));
    let full_summary = data
        .get("full_summary")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let policy_readiness = data
        .get("policy_readiness")
        .and_then(|value| value.as_str())
        .unwrap_or("deep");
    let method_summary = implementation_brief
        .and_then(|value| value.get("method_summary"))
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let github_haystack =
        format!("{source_url} {title} {relevance_tags} {full_summary}").to_ascii_lowercase();
    if !(github_haystack.contains("github.com") || github_haystack.contains("githubrepo")) {
        return None;
    }

    let canonical_home = value_string(&entry, "canonical_home").unwrap_or_default();
    let domain = value_string(&entry, "domain").unwrap_or_default();
    let recommended_action =
        "encode/link deep Athena GitHub record as Mnemosyne recall context".to_string();
    let rationale = format!(
        "ATHENA deep GitHub record with policy_readiness={policy_readiness}; {method_summary}"
    );
    let triaged_at_utc = value_string(&entry, "triaged_at_utc").unwrap_or_default();
    let haystack = format!(
        "{path} {title} {canonical_home} {domain} {recommended_action} {rationale} {source_url} {relevance_tags} {full_summary}"
    )
    .to_ascii_lowercase();
    let match_score = if query_terms.is_empty() {
        0.5
    } else {
        query_terms
            .iter()
            .filter(|term| haystack.contains(term.as_str()))
            .count() as f64
            / query_terms.len() as f64
    };
    if !query_terms.is_empty() && match_score == 0.0 {
        return None;
    }

    Some(KnowledgeSeedRecallEntry {
        path,
        title,
        classification: "memory_seed".to_string(),
        canonical_home,
        domain,
        authority: "curated_memory".to_string(),
        recommended_action,
        rationale,
        soterion_glyph: "🜄".to_string(),
        soterion_sigil: "MNEMOSYNE".to_string(),
        triaged_at_utc,
        score: (match_score + 0.2).clamp(0.0, 2.0),
    })
}

fn read_deep_athena_book_entry(path: &str) -> Option<Value> {
    let root = arda_root();
    let path = if path.starts_with('/') {
        PathBuf::from(path)
    } else {
        root.join(path)
    };
    let content = std::fs::read_to_string(path).ok()?;
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|entry| entry.get("stage").and_then(|value| value.as_str()) == Some("deep"))
}

fn read_triage_registry() -> Result<Vec<Value>> {
    let root = arda_root();
    let path = root.join("core/state/knowledge_triage_registry.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(path)?;
    Ok(content
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect())
}

fn value_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn title_from_path(path: &str) -> String {
    Path::new(path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .replace(['_', '-'], " ")
}

fn path_exists(path: &str) -> Option<bool> {
    if path.starts_with('/') {
        return Some(Path::new(path).exists());
    }
    Some(arda_root().join(path).exists())
}

fn arda_root() -> PathBuf {
    if let Ok(path) = std::env::var("ARDA_ROOT") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}
