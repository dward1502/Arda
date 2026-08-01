// sigil: EDGE_NODE_INTEGRATION
// Purpose: Integrate Mnemosyne memory query into subagent spawning flow
// This file demonstrates how to use edge nodes for memory enrichment

use anyhow::Result;

use crate::context_cache::AsyncContextCache;

/// Async-shared cache for enriched context to avoid repeated Mnemosyne queries
static CONTEXT_CACHE: once_cell::sync::Lazy<AsyncContextCache<String, String>> =
    once_cell::sync::Lazy::new(|| AsyncContextCache::new(100, std::time::Duration::from_secs(300)));

/// Enrich context with memories from Mnemosyne, using cache to avoid repeated queries
pub async fn spawn_enriched_subagent(_task: &str, context: &str) -> Result<String> {
    let cache_key = format!("{}|{}", _task, context);

    if let Some(enriched) = CONTEXT_CACHE.get(&cache_key).await {
        tracing::debug!(cache_key = %cache_key, "subagent context cache hit");
        return Ok(enriched);
    }

    tracing::debug!(cache_key = %cache_key, "subagent context cache miss");

    let mnemosyne = arda_vaire::MnemosyneService::from_default_or_fallback()?;
    let identity = mnemosyne.identity_state()?;

    let mut enriched = context.to_string();
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

    if !identity.current_mission_focus.is_empty() {
        enriched.push_str(&format!(
            "\n=== CURRENT MISSION FOCUS ===\n{}\n",
            identity.current_mission_focus
        ));
    }

    CONTEXT_CACHE.put(cache_key, enriched.clone()).await;

    Ok(enriched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn bounded_async_cache_get_put_and_metrics() {
        let cache: AsyncContextCache<String, String> =
            AsyncContextCache::new(2, std::time::Duration::from_secs(60));
        let missing = "missing".to_string();
        let a = "a".to_string();
        let b = "b".to_string();
        let c = "c".to_string();
        assert!(cache.get(&missing).await.is_none());

        cache.put(a.clone(), "1".to_string()).await;
        cache.put(b.clone(), "2".to_string()).await;

        assert_eq!(cache.get(&a).await, Some("1".to_string()));
        assert_eq!(cache.get(&b).await, Some("2".to_string()));

        cache.put(c.clone(), "3".to_string()).await;

        assert!(cache.get(&a).await.is_none());
        assert_eq!(cache.get(&b).await, Some("2".to_string()));
        assert_eq!(cache.get(&c).await, Some("3".to_string()));

        let metrics = cache.metrics().await;
        assert_eq!(metrics.size, 2);
        assert!(metrics.evictions >= 1);
    }
}
