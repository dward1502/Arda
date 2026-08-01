// sigil: MEMORY_ENRICHMENT
// Purpose: Context enrichment layer for Annunimas

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnrichedContext {
    pub memory_summary: MemorySummary,
    pub top_memories: Vec<RankedMemory>,
    pub mission_context: MissionContext,
    pub recent_events: Vec<RankedMemory>,
    pub enrichment_config: EnrichmentConfigSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichmentConfigSnapshot {
    pub include_recent_events: bool,
    pub include_identity: bool,
    pub include_mission_focus: bool,
    pub max_events: usize,
    pub max_memories_per_tier: usize,
    pub min_significance_threshold: f64,
    pub scoring_weights: ScoringWeights,
}

impl Default for EnrichmentConfigSnapshot {
    fn default() -> Self {
        Self {
            include_recent_events: true,
            include_identity: true,
            include_mission_focus: true,
            max_events: 10,
            max_memories_per_tier: 5,
            min_significance_threshold: 0.0,
            scoring_weights: ScoringWeights::default(),
        }
    }
}

/// Cache for enriched context to reduce Mnemosyne queries
/// Uses the same cache configuration as mnemosyne_integration.rs
static CONTEXT_CACHE: once_cell::sync::Lazy<
    Arc<Mutex<crate::context_cache::ContextCache<String, EnrichedContext>>>,
> = once_cell::sync::Lazy::new(|| {
    Arc::new(Mutex::new(crate::context_cache::ContextCache::new(
        100,
        Duration::from_secs(300),
    )))
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub significance_weight: f64,
    pub recency_weight: f64,
    pub tier_weight: f64,
    pub tag_match_weight: f64,
    pub query_relevance_weight: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            significance_weight: 0.40,
            recency_weight: 0.25,
            tier_weight: 0.15,
            tag_match_weight: 0.10,
            query_relevance_weight: 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemorySummary {
    pub core_memories: usize,
    pub active_memories: usize,
    pub peripheral_memories: usize,
    pub transient_memories: usize,
    pub total_memories: usize,
    pub avg_significance: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankedMemory {
    pub memory_id: String,
    pub source_crate: String,
    pub event_type: String,
    pub memory_scope: String,
    pub content: String,
    pub sigil: String,
    pub tags: Vec<String>,
    pub significance: f64,
    pub score: f64,
    pub tier: MemoryTier,
    pub ts_utc: String,
    pub relevance_reasons: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum MemoryTier {
    Core,
    Active,
    Peripheral,
    Transient,
}

impl fmt::Display for MemoryTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Core => "CORE",
            Self::Active => "ACTIVE",
            Self::Peripheral => "PERIPHERAL",
            Self::Transient => "TRANSIENT",
        };
        write!(f, "{label}")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MissionContext {
    pub current_focus: String,
    pub active_directives: Vec<String>,
    pub pending_decisions: Vec<String>,
}

#[derive(Default)]
pub struct ContextEnrichmentService {
    scoring_weights: ScoringWeights,
}

impl ContextEnrichmentService {
    pub fn new(scoring_weights: ScoringWeights) -> Self {
        Self { scoring_weights }
    }

    pub fn enrichment_config(&self) -> EnrichmentConfigSnapshot {
        EnrichmentConfigSnapshot {
            include_recent_events: true,
            include_identity: true,
            include_mission_focus: true,
            max_events: 10,
            max_memories_per_tier: 5,
            min_significance_threshold: 0.0,
            scoring_weights: self.scoring_weights.clone(),
        }
    }

    pub fn enrich_prompt(&self, task_description: &str) -> Result<EnrichedContext> {
        // Check cache first
        let cache_key = task_description.to_string();

        let mut cache = futures::executor::block_on(CONTEXT_CACHE.lock());
        if let Some(cached) = cache.get(&cache_key) {
            tracing::debug!(cache_key = %cache_key, "context cache hit");
            return Ok(cached.clone());
        }

        tracing::debug!(cache_key = %cache_key, "context cache miss");

        let mnemosyne = arda_vaire::MnemosyneService::from_default_or_fallback()?;
        let identity = mnemosyne.identity_state()?;
        let min_sig = 0.0;
        let relevant_memories = mnemosyne.recall_relevant(task_description, 72, None, None, 10)?;

        let summary = self.build_memory_summary(&identity);

        let mut recent_events: Vec<RankedMemory> = relevant_memories
            .iter()
            .filter(|event| event.significance >= min_sig)
            .take(10)
            .map(|event| {
                let query_terms = Self::query_terms(task_description);
                let query_score =
                    Self::query_match_score(&query_terms, &event.content, &event.tags);
                let score =
                    (event.significance * 0.40 + query_score * 0.10 + 0.4 * 0.25).clamp(0.0, 1.0);

                let mut reasons = Vec::new();
                if event.significance >= 0.7 {
                    reasons.push(format!("high significance: {:.2}", event.significance));
                }
                if query_score > 0.0 {
                    reasons.push("query match".to_string());
                }
                if event
                    .tags
                    .iter()
                    .any(|tag| matches!(tag.as_str(), "checkpoint" | "decision" | "boardroom"))
                {
                    reasons.push("governance-tagged".to_string());
                }
                if reasons.is_empty() {
                    reasons.push("recent session event".to_string());
                }

                RankedMemory {
                    memory_id: event.memory_id.clone(),
                    source_crate: event.source_crate.clone(),
                    event_type: event.event_type.clone(),
                    memory_scope: event.memory_scope.clone(),
                    content: event.content.clone(),
                    sigil: event.sigil.clone(),
                    tags: event.tags.clone(),
                    significance: event.significance,
                    score,
                    tier: MemoryTier::Active,
                    ts_utc: event.ts_utc.clone(),
                    relevance_reasons: reasons,
                }
            })
            .collect();

        recent_events.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let top_memories_vec: Vec<RankedMemory> = recent_events.iter().take(5).cloned().collect();

        let enriched = EnrichedContext {
            memory_summary: summary.clone(),
            top_memories: top_memories_vec.clone(),
            mission_context: self.build_mission_context(&identity),
            recent_events: if true {
                recent_events.clone()
            } else {
                Vec::new()
            },
            enrichment_config: self.enrichment_config(),
        };

        // Store in cache
        cache.put(cache_key.clone(), enriched.clone());

        Ok(enriched)
    }

    fn build_memory_summary(&self, identity: &arda_vaire::service::IdentityState) -> MemorySummary {
        let recent = &identity.recent_events;
        let avg_significance = if recent.is_empty() {
            0.0
        } else {
            recent.iter().map(|event| event.significance).sum::<f64>() / recent.len() as f64
        };

        MemorySummary {
            core_memories: identity.core_memory_count,
            active_memories: identity.active_memory_count,
            peripheral_memories: identity.peripheral_memory_count,
            transient_memories: identity.transient_memory_count,
            total_memories: identity.core_memory_count
                + identity.active_memory_count
                + identity.peripheral_memory_count
                + identity.transient_memory_count,
            avg_significance,
        }
    }

    fn build_mission_context(
        &self,
        identity: &arda_vaire::service::IdentityState,
    ) -> MissionContext {
        let mut context = MissionContext::default();
        if true {
            context.current_focus = identity.current_mission_focus.clone();
        }

        for event in &identity.recent_events {
            if event
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "decision" | "directive"))
            {
                context.active_directives.push(event.content.clone());
            }
            if event
                .tags
                .iter()
                .any(|tag| matches!(tag.as_str(), "pending" | "todo" | "followup"))
            {
                context.pending_decisions.push(event.content.clone());
            }
        }

        context
    }

    pub fn format_for_agent(&self, context: &EnrichedContext, task: &str) -> Result<String> {
        let mut output = String::new();
        output.push_str("=== CONTEXT ENRICHMENT START ===\n");
        output.push_str(&format!("Task: {task}\n"));
        output.push_str(&format!("Generated: {}\n", chrono::Utc::now().to_rfc3339()));
        output.push_str("================================\n\n");

        output.push_str("=== MEMORY SUMMARY ===\n");
        output.push_str(&format!(
            "  Core: {}\n",
            context.memory_summary.core_memories
        ));
        output.push_str(&format!(
            "  Active: {}\n",
            context.memory_summary.active_memories
        ));
        output.push_str(&format!(
            "  Peripheral: {}\n",
            context.memory_summary.peripheral_memories
        ));
        output.push_str(&format!(
            "  Transient: {}\n",
            context.memory_summary.transient_memories
        ));
        output.push_str(&format!(
            "  Total: {}\n",
            context.memory_summary.total_memories
        ));
        output.push_str(&format!(
            "  Avg Sig: {:.2}\n",
            context.memory_summary.avg_significance
        ));
        output.push_str("================================\n\n");

        if !context.top_memories.is_empty() {
            output.push_str("=== TOP MEMORIES ===\n");
            for (idx, mem) in context.top_memories.iter().enumerate() {
                output.push_str(&format!(
                    "[{}] {} {:.3} {}\n",
                    idx + 1,
                    mem.tier,
                    mem.score,
                    mem.content
                ));
            }
            output.push_str("================================\n\n");
        }

        if !context.mission_context.current_focus.is_empty() {
            output.push_str("=== MISSION CONTEXT ===\n");
            output.push_str(&format!(
                "Current Focus: {}\n",
                context.mission_context.current_focus
            ));
            output.push_str("================================\n\n");
        }

        Ok(output)
    }

    pub fn format_as_json(&self, context: &EnrichedContext) -> Result<String> {
        serde_json::to_string_pretty(context)
            .map_err(|err| anyhow::anyhow!("JSON serialization failed: {err}"))
    }

    pub fn get_quick_context(&self, context: &EnrichedContext) -> Result<String> {
        let mut output = String::new();
        output.push_str(&format!(
            "Context for subagent spawning. Top memories (score >= {:.2}): {}\n",
            context.enrichment_config.min_significance_threshold,
            context.top_memories.len()
        ));

        for mem in context.top_memories.iter().take(5) {
            output.push_str(&format!(
                "[{:.3}] Tier: {:<10} Sig: {:.2} | {}\n",
                mem.score,
                mem.tier,
                mem.significance,
                mem.content.chars().take(80).collect::<String>()
            ));
        }

        if !context.mission_context.current_focus.is_empty() {
            output.push_str(&format!(
                "Mission Focus: {}\n",
                context.mission_context.current_focus
            ));
        }

        Ok(output)
    }

    fn query_terms(task_description: &str) -> Vec<String> {
        task_description
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scoring_weights_defaults() {
        let weights = ScoringWeights::default();
        assert!((weights.significance_weight - 0.40).abs() < 0.01);
        assert!((weights.recency_weight - 0.25).abs() < 0.01);
    }

    #[test]
    fn test_memory_tier_display() {
        assert_eq!(MemoryTier::Core.to_string(), "CORE");
        assert_eq!(MemoryTier::Active.to_string(), "ACTIVE");
    }

    #[test]
    fn test_ranked_memory_serialization() {
        let mem = RankedMemory {
            memory_id: "test_id".to_string(),
            source_crate: "test_crate".to_string(),
            event_type: "test_event".to_string(),
            memory_scope: "system_continuity".to_string(),
            content: "test content".to_string(),
            sigil: "TEST".to_string(),
            tags: vec!["tag1".to_string()],
            significance: 0.85,
            score: 0.92,
            tier: MemoryTier::Active,
            ts_utc: chrono::Utc::now().to_rfc3339(),
            relevance_reasons: vec!["high significance".to_string()],
        };

        let json = serde_json::to_string(&mem).unwrap();
        let parsed: RankedMemory = serde_json::from_str(&json).unwrap();
        assert_eq!(mem.memory_id, parsed.memory_id);
        assert_eq!(mem.score, parsed.score);
    }

    #[test]
    fn test_enriched_context_serialization() {
        let context = EnrichedContext {
            memory_summary: MemorySummary {
                core_memories: 5,
                active_memories: 10,
                peripheral_memories: 20,
                transient_memories: 15,
                total_memories: 50,
                avg_significance: 0.75,
            },
            top_memories: vec![],
            mission_context: MissionContext {
                current_focus: "test mission".to_string(),
                active_directives: vec!["directive1".to_string()],
                pending_decisions: vec![],
            },
            recent_events: vec![],
            enrichment_config: EnrichmentConfigSnapshot::default(),
        };

        let json = serde_json::to_string_pretty(&context).unwrap();
        let parsed: EnrichedContext = serde_json::from_str(&json).unwrap();
        assert_eq!(
            context.memory_summary.core_memories,
            parsed.memory_summary.core_memories
        );
        assert_eq!(
            context.mission_context.current_focus,
            parsed.mission_context.current_focus
        );
    }
}
