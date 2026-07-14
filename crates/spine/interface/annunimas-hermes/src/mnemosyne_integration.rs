// sigil: EDGE_NODE_INTEGRATION
// Purpose: Integrate Mnemosyne memory query into subagent spawning flow
// This file demonstrates how to use edge nodes for memory enrichment

use anyhow::Result;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex; // Add this import

use crate::context_cache::{CacheMetrics, ContextCache, InternalCacheMetrics};

/// Cache for enriched context to avoid repeated Mnemosyne queries
static CONTEXT_CACHE: once_cell::sync::Lazy<Arc<Mutex<ContextCache<String, String>>>> =
    once_cell::sync::Lazy::new(|| {
        Arc::new(Mutex::new(ContextCache::new(100, Duration::from_secs(300))))
    });

/// Metrics for cache performance
static CACHE_METRICS: once_cell::sync::Lazy<Arc<Mutex<InternalCacheMetrics>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(InternalCacheMetrics::default())));

/// Get cache metrics for monitoring
pub fn get_cache_stats() -> CacheMetrics {
    let metrics = futures::executor::block_on(CACHE_METRICS.lock());
    CacheMetrics {
        hits: metrics.hits.load(Ordering::Relaxed),
        misses: metrics.misses.load(Ordering::Relaxed),
        size: metrics.size.load(Ordering::Relaxed),
        evictions: metrics.evictions.load(Ordering::Relaxed),
    }
}

/// Enrich context with memories from Mnemosyne, using cache to avoid repeated queries
pub async fn spawn_enriched_subagent(_task: &str, context: &str) -> Result<String> {
    // Generate cache key from task and context
    let cache_key = format!("{}|{}", _task, context);

    // Check cache first
    let mut cache = futures::executor::block_on(CONTEXT_CACHE.lock());
    if let Some(enriched) = cache.get(&cache_key) {
        // Cache hit - update metrics
        let metrics = futures::executor::block_on(CACHE_METRICS.lock());
        metrics.hits.fetch_add(1, Ordering::Relaxed);
        tracing::debug!(cache_key = %cache_key, "subagent context cache hit");
        return Ok(enriched.clone());
    }

    tracing::debug!(cache_key = %cache_key, "subagent context cache miss");

    // Cache miss - query Mnemosyne
    let mnemosyne = annunimas_mnemosyne::MnemosyneService::from_default_or_fallback()?;
    let identity = mnemosyne.identity_state()?; // This is synchronous, not async

    // Build enriched context
    let mut enriched = context.to_string();

    // Add memory counts summary
    enriched.push_str("\n\n=== MEMORY SUMMARY ===\n");
    enriched.push_str(&format!(
        "  Core memories: {} (unique, high-significance)\n",
        identity.core_memory_count
    ));
    enriched.push_str(&format!(
        "  Active memories: {} (recent, task-relevant)\n",
        identity.active_memory_count
    ));
    enriched.push_str(&format!(
        "  Peripheral memories: {} (contextual, background)\n",
        identity.peripheral_memory_count
    ));
    enriched.push_str(&format!(
        "  Transient memories: {} (ephemeral, short-term)\n",
        identity.transient_memory_count
    ));

    // Add recent events if available
    if !identity.recent_events.is_empty() {
        let events_str = identity
            .recent_events
            .iter()
            .take(5)
            .map(|e| {
                format!(
                    "[{}] {:<20} sig: {:.2} | {}",
                    e.event_type, e.content, e.significance, e.ts_utc
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        enriched.push_str(&format!("\n=== RECENT SESSION EVENTS ===\n{}", events_str));
    }

    // Add mission focus
    if !identity.current_mission_focus.is_empty() {
        enriched.push_str(&format!(
            "\n=== CURRENT MISSION FOCUS ===\n{}\n",
            identity.current_mission_focus
        ));
    }

    // Store in cache
    cache.put(cache_key.clone(), enriched.clone());
    let size = cache.len();

    // Update metrics
    let metrics = futures::executor::block_on(CACHE_METRICS.lock());
    metrics.misses.fetch_add(1, Ordering::Relaxed);
    metrics.size.store(size, Ordering::Relaxed);

    Ok(enriched)
}
