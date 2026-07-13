// sigil: EDGE_NODE_INTEGRATION
// Purpose: Test the context cache implementation

use annunimas_hermes::context_cache::ContextCache;
use std::thread;
use std::time::Duration;

#[test]
fn test_lru_cache() {
    let mut cache = ContextCache::new(3, Duration::from_secs(60));

    // Add three items
    cache.put("a", 1);
    cache.put("b", 2);
    cache.put("c", 3);

    // Check size
    assert_eq!(cache.len(), 3);

    // Access "a" to make it most recently used
    assert_eq!(cache.get(&"a"), Some(&1));

    // Add a fourth item - should evict "b" (least recently used)
    cache.put("d", 4);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.get(&"b"), None);
    assert_eq!(cache.get(&"a"), Some(&1));
    assert_eq!(cache.get(&"c"), Some(&3));
    assert_eq!(cache.get(&"d"), Some(&4));

    // Test TTL expiration
    let mut cache = ContextCache::new(3, Duration::from_secs(1));
    cache.put("x", 10);
    thread::sleep(Duration::from_secs(2));
    assert_eq!(cache.get(&"x"), None);
}

#[test]
fn test_cache_metrics() {
    let mut cache = ContextCache::new(3, Duration::from_secs(60));

    // First access should be a miss
    assert_eq!(cache.get(&"test"), None);

    // Put something and then get it - should be a hit
    cache.put("test", 42);
    assert_eq!(cache.get(&"test"), Some(&42));

    // Check metrics
    let metrics = cache.metrics();
    assert_eq!(metrics.hits, 1);
    assert_eq!(metrics.misses, 1);
    assert_eq!(metrics.size, 1);
    assert_eq!(metrics.evictions, 0);
}
