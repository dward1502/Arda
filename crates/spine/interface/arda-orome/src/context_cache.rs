// sigil: EDGE_NODE_INTEGRATION
// Purpose: LRU/TTL cache for enriched context; async wrapper provided below

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// Internal metrics with atomics for concurrent updates
#[derive(Debug, Default)]
pub(crate) struct InternalCacheMetrics {
    pub hits: AtomicUsize,
    pub misses: AtomicUsize,
    pub size: AtomicUsize,
    pub evictions: AtomicUsize,
}

/// Public metrics snapshot for reporting
#[derive(Debug, Clone, Copy, Default)]
pub struct CacheMetrics {
    pub hits: usize,
    pub misses: usize,
    pub size: usize,
    pub evictions: usize,
}

/// LRU cache with TTL and metrics
pub struct ContextCache<K: Eq + Hash + Clone, V> {
    max_entries: usize,
    ttl: Duration,
    cache: HashMap<K, (V, Instant)>,
    lru: VecDeque<K>,
    metrics: InternalCacheMetrics,
}

impl<K: Eq + Hash + Clone, V> ContextCache<K, V> {
    /// Create a new cache with maximum entries and TTL
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            max_entries,
            ttl,
            cache: HashMap::new(),
            lru: VecDeque::new(),
            metrics: InternalCacheMetrics::default(),
        }
    }

    /// Get a value if present and not expired
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let now = Instant::now();
        let is_expired = {
            if let Some((_, timestamp)) = self.cache.get(key) {
                now.duration_since(*timestamp) > self.ttl
            } else {
                self.metrics.misses.fetch_add(1, Ordering::Relaxed);
                return None;
            }
        };
        if is_expired {
            self.cache.remove(key);
            if let Some(pos) = self.lru.iter().position(|k| k == key) {
                self.lru.remove(pos);
            }
            self.metrics.misses.fetch_add(1, Ordering::Relaxed);
            None
        } else {
            if let Some(pos) = self.lru.iter().position(|k| k == key) {
                self.lru.remove(pos);
            }
            self.lru.push_back(key.clone());
            self.metrics.hits.fetch_add(1, Ordering::Relaxed);
            Some(&self.cache[key].0)
        }
    }

    /// Insert a key-value pair into the cache
    pub fn put(&mut self, key: K, value: V) {
        if self.cache.contains_key(&key) {
            if let Some(pos) = self.lru.iter().position(|k| k == &key) {
                self.lru.remove(pos);
            }
        } else if self.lru.len() >= self.max_entries {
            if let Some(oldest) = self.lru.pop_front() {
                self.cache.remove(&oldest);
                self.metrics.evictions.fetch_add(1, Ordering::Relaxed);
            }
        }

        self.cache.insert(key.clone(), (value, Instant::now()));
        self.lru.push_back(key);
        self.metrics.size.store(self.cache.len(), Ordering::Relaxed);
    }

    /// Remove a key from the cache
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(entry) = self.cache.remove(key) {
            if let Some(pos) = self.lru.iter().position(|k| k == key) {
                self.lru.remove(pos);
            }
            self.metrics.size.store(self.cache.len(), Ordering::Relaxed);
            Some(entry.0)
        } else {
            None
        }
    }

    /// Clear the cache
    pub fn clear(&mut self) {
        self.cache.clear();
        self.lru.clear();
        self.metrics.size.store(0, Ordering::Relaxed);
    }

    /// Number of entries
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// True when empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Metrics snapshot
    pub fn metrics(&self) -> CacheMetrics {
        CacheMetrics {
            hits: self.metrics.hits.load(Ordering::Relaxed),
            misses: self.metrics.misses.load(Ordering::Relaxed),
            size: self.metrics.size.load(Ordering::Relaxed),
            evictions: self.metrics.evictions.load(Ordering::Relaxed),
        }
    }
}

/// Async wrapper around an LRU/TTL cache, bounded by `max_entries`.
pub struct AsyncContextCache<K: Eq + Hash + Clone, V> {
    inner: std::sync::Mutex<ContextCache<K, V>>,
}

impl<K: Eq + Hash + Clone, V> AsyncContextCache<K, V> {
    /// Create a new bounded async cache.
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            inner: std::sync::Mutex::new(ContextCache::new(max_entries, ttl)),
        }
    }

    /// Get a value without removing it.
    pub async fn get(&self, key: &K) -> Option<V>
    where
        V: Clone,
    {
        let mut guard = self.inner.lock().ok()?;
        guard.get(key).cloned()
    }

    /// Insert a value, evicting oldest entry if over capacity.
    pub async fn put(&self, key: K, value: V) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.put(key, value);
        }
    }

    /// Current metrics snapshot.
    pub async fn metrics(&self) -> CacheMetrics {
        let guard = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return CacheMetrics::default(),
        };
        guard.metrics()
    }
}
